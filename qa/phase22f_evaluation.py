"""Shared F QA inputs, accounting, and isolated controller contracts."""

from contextlib import contextmanager
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import subprocess
import time
import urllib.error
import urllib.request
import uuid


REPO = Path(__file__).resolve().parents[1]
QA_ROOT = REPO / '.lucille-qa-runtime/lucille-synthetic-real22f'
BRIDGE = REPO.parent / 'build/phase22f/real_validation_bridge.exe'
MARKER = b'synthetic-data-only\n'


def module(path):
    spec = importlib.util.spec_from_file_location(Path(path).stem, REPO / path)
    loaded = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(loaded)
    return loaded


def read(path):
    return json.loads(Path(path).read_text(encoding='utf-8'))


def write(path, data):
    with path.open('x', encoding='utf-8', newline='\n') as stream:
        stream.write(json.dumps(data, ensure_ascii=False, indent=2) + '\n')


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def slug(value):
    if not isinstance(value, str) or not re.fullmatch(r'[a-z0-9][a-z0-9-]{0,59}', value):
        raise ValueError('Invalid synthetic case or report label')
    return value


class EvaluationFailure(Exception):
    def __init__(self, status, reason):
        super().__init__(reason)
        self.status = status


def require_product(condition, reason):
    if not condition:
        raise EvaluationFailure('product_failure', reason)


def proof_matches_source(detail, text, job_id):
    quality = detail.get('quality') or {}
    match = re.search(r'\|native_source_authority_v\d+:(\{.*\})$', quality.get('evidence_rule', ''))
    if not match:
        return False
    proof = json.loads(match[1])
    source = text.encode()
    start, end = proof['start'], proof['end']
    if not (0 <= start < end <= len(source)):
        return False
    evidence = source[start:end]
    return (proof['source_digest'] == hashlib.sha256(source).hexdigest() and
            proof['evidence_digest'] == hashlib.sha256(evidence).hexdigest() and
            proof['statement_author'] == proof['message_author'] == 'user' and
            any(row['interaction_id'] == job_id and row['evidence'].encode() == evidence
                for row in detail['provenance']))


def failure(error):
    if isinstance(error, EvaluationFailure):
        status = error.status
    elif isinstance(error, urllib.error.HTTPError) and error.code in (502, 503, 504):
        status = 'infrastructure_failure'
    elif isinstance(error, (urllib.error.URLError, TimeoutError)):
        status = 'infrastructure_failure'
    elif isinstance(error, subprocess.TimeoutExpired):
        status = 'infrastructure_failure'
    else:
        status = 'evaluator_failure'
    return dict(status=status, error_type=type(error).__name__, error=str(error))


class Evaluation:
    def __init__(self, args, suffix, *, gui=False):
        self.root = Path(args.root).resolve(strict=True)
        if self.root != QA_ROOT or (self.root / '.lucille-isolated').read_bytes() != MARKER:
            raise ValueError('Exact marked F QA root required')
        self.label = slug(args.label)
        self.reports = self.root / 'reports'
        self.output = self.reports / f'{self.label}-{suffix}.json'
        self.progress = self.reports / f'{self.label}-{suffix}-progress.ndjson'
        if self.output.exists() or self.progress.exists():
            raise FileExistsError('Never overwrite or replay an evaluation label')
        self.freeze_path = Path(args.freeze).resolve(strict=True)
        self.dataset_path = Path(args.dataset).resolve(strict=True)
        self.bridge_path = Path(getattr(args, 'bridge', BRIDGE)).resolve(strict=True)
        if self.bridge_path != BRIDGE.resolve(strict=True):
            raise ValueError('Dedicated F bridge required')
        self.verify_frozen = module('scripts/test-lucille-real-quality-extraction.py').verify
        self.freeze = self.verify_frozen(REPO, self.freeze_path)
        if self.label.startswith('final') and self.freeze.get('kind') != 'final':
            raise ValueError('Final run requires the final sealed manifest')
        dataset_key = self.dataset_path.relative_to(REPO).as_posix()
        if dataset_key not in self.freeze['corpus_sha256']:
            raise ValueError('Dataset must be sealed by the selected manifest')
        self.data = read(self.dataset_path)
        self.artifacts = module('qa/phase22f-artifact-check.py') if gui else None
        self.initial_artifacts = self.verify()
        self.calls = []
        self.rows = []

    def verify(self):
        if digest(self.bridge_path) != self.freeze['bridge_sha256']:
            raise ValueError('Frozen F bridge changed')
        if read(self.reports / 'real-public-pin.json') != self.freeze['model_pin']:
            raise ValueError('Frozen real model pin changed')
        if self.artifacts:
            return self.artifacts.verify_artifacts(REPO, self.root, self.freeze)
        return dict(bridge_verified=True, saved_model_pin_verified=True)

    def live_pin(self, kind):
        live = self.api('/lucille/' + kind + '-manifest')
        frozen = self.freeze['model_pin']
        if frozen.get('simulation') is not False or {key: live.get(key) for key in frozen} != frozen:
            raise EvaluationFailure('invalid_result', f'Live {kind} pin differs from freeze')
        return live

    def guard(self, *services):
        if any((self.root / (name + '.stop')).exists() for name in services):
            raise EvaluationFailure('infrastructure_failure', 'Owned stop requested')

    def api(self, path, data=None):
        request = urllib.request.Request(
            'http://127.0.0.1:15284' + path,
            data=json.dumps(data, ensure_ascii=False).encode() if data is not None else None,
            headers={'Content-Type': 'application/json'},
        )
        begin = time.monotonic()
        with urllib.request.urlopen(request, timeout=120 if data is not None else 15) as response:
            result = json.load(response)
        self.calls.append(dict(path=path, latency_seconds=time.monotonic() - begin, usage=result.get('usage')))
        return result

    def add(self, row):
        self.rows.append(row)
        with self.progress.open('a', encoding='utf-8', newline='\n') as stream:
            stream.write(json.dumps(row, ensure_ascii=False) + '\n')

    def finish(self, *, passed, **data):
        attestation = None
        error = None
        try:
            self.verify_frozen(REPO, self.freeze_path)
            attestation = self.verify()
        except Exception as caught:
            error = failure(caught)
        passed = bool(passed and error is None)
        write(self.output, dict(freeze_sha256=digest(self.freeze_path), dataset_sha256=digest(self.dataset_path),
                                passed=passed, initial_artifacts=self.initial_artifacts, final_artifacts=attestation,
                                final_attestation_error=error, rows=self.rows, **data))
        return 0 if passed else 1


@contextmanager
def controller_lock(run):
    lock = run.reports / 'phase22f-controller.lock'
    nonce = str(uuid.uuid4())
    with lock.open('x', encoding='utf-8') as stream:
        stream.write(nonce)
    try:
        yield
    finally:
        if lock.exists() and lock.read_text(encoding='utf-8') == nonce:
            lock.unlink()


def send_control(run, label, body):
    slug(label)
    target = run.reports / 'control.json'
    temp = run.reports / f'control-{uuid.uuid4()}.tmp'
    write(temp, dict(id=str(uuid.uuid4()), label=label, **body))
    for attempt in range(5):
        try:
            temp.replace(target)
            return
        except PermissionError:
            if attempt == 4:
                raise
            time.sleep(.05)


def report_counts(rows):
    counts = {'planned': len(rows)}
    for status in ('measured', 'product_failure', 'evaluator_failure', 'infrastructure_failure', 'not_executed', 'invalid_result'):
        counts[status] = sum(row['status'] == status for row in rows)
    return counts


def protocol_plan(run):
    """Known development annotations retain original source wording and golds."""
    if 'known_protocol' not in run.data:
        return run.data
    known = run.data['known_protocol']
    path = known['corpus']
    if run.freeze.get('kind') != 'development' or path not in run.freeze['corpus_sha256']:
        raise ValueError('Both the known protocol and annotations must be sealed for development')
    original = read(REPO / path)['cases']
    transport = [dict(mode=c['mode'], project=c['project'], text=c['text'], expected=known['expected'][c['id']])
                 for c in original if c['type'] == 'prepared_cancel']
    cases = [dict(c, expected=known['expected'].get(c['id'])) for c in original]
    if 'sqlite_busy_source' in known:
        source = next(case for case in original if case['id'] == known['sqlite_busy_source'])
        cases.append(dict(source, id='sqlite-busy', type='sqlite_busy'))
    return dict(cases=cases, transport=transport)
