"""Drive the isolated full native app through its bundled DOM QA observer.

No OS input, model substitutes, canonical writes or generated replacement answers.
The plan is immutable input; reviewed B records use the actual settings panel.
"""
import argparse
import datetime
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import sys
import time
import uuid

sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location('freeze_check', Path(__file__).with_name('test-lucille-real-quality-extraction.py'))
h = importlib.util.module_from_spec(spec)
spec.loader.exec_module(h)


def main(a):
    repo = Path(__file__).resolve().parents[1]
    root = Path(a.root).resolve(strict=True)
    assert root == repo / '.lucille-qa-runtime/lucille-synthetic-real22d'
    assert (root / '.lucille-isolated').read_bytes() == b'synthetic-data-only\n'
    assert re.fullmatch(r'[a-z0-9-]{1,50}', a.label)
    assert re.fullmatch(r'phase22d-[a-z0-9-]{1,55}', a.scope) or a.scope == 'phase22c-final-historical-series'
    reports = root / 'reports'
    freeze = h.verify(repo, a.freeze)
    assert hashlib.sha256(Path(a.bridge).read_bytes()).hexdigest() == freeze['bridge_sha256']
    for relative, sha in freeze.get('frontend_sha256', {}).items():
        assert hashlib.sha256((repo / 'qa-dist-22d' / relative).read_bytes()).hexdigest() == sha
    plan = json.loads(Path(a.plan).read_text(encoding='utf-8'))
    assert str(Path(a.plan).resolve().relative_to(repo)).replace('\\', '/') in freeze['corpus_sha256']
    gui = json.loads((root / 'gui-processes.json').read_text(encoding='utf-8'))
    native = Path(gui['gui']['executable']).resolve(strict=True)
    assert native == (repo.parent / 'build/tauri/x86_64-pc-windows-msvc/debug/lucille-full-validation.exe').resolve(strict=True)
    assert hashlib.sha256(native.read_bytes()).hexdigest() == freeze['native_gui_sha256']
    assert json.loads((reports / 'real-public-pin.json').read_text(encoding='utf-8')) == freeze['model_pin']
    ready = json.loads((reports / 'driver-ready-progress.json').read_text(encoding='utf-8'))
    assert ready['native'] is True
    assert datetime.datetime.fromisoformat(ready['ready'].replace('Z', '+00:00')) >= datetime.datetime.fromisoformat(gui['gui']['created'])
    final = reports / ('driver-' + a.label + '-final.json')
    assert not final.exists() and not list(reports.glob('driver-' + a.label + '-failure-*.json'))
    lock = reports / 'phase22d-controller.lock'
    nonce = str(uuid.uuid4())
    with lock.open('x', encoding='utf-8') as f:
        f.write(nonce)
    try:
        control = dict(id=str(uuid.uuid4()), label=a.label, scope=a.scope,
                       project=plan['project'], resume=not a.initialize, plan=plan)
        (reports / 'control.json').write_bytes((json.dumps(control, ensure_ascii=False) + '\n').encode('utf-8'))
        began = time.monotonic()
        previous = None
        while not final.exists():
            failures = list(reports.glob('driver-' + a.label + '-failure-*.json'))
            if failures:
                failure = json.loads(failures[0].read_text(encoding='utf-8'))
                raise RuntimeError('Actual full-app stage failed: ' + failure.get('error', 'unknown'))
            assert time.monotonic() - began < a.timeout, 'Full-app stage timeout; evidence retained'
            progress = reports / ('driver-' + a.label + '-progress.json')
            if progress.exists():
                try:
                    d = json.loads(progress.read_text(encoding='utf-8'))
                    state = (len(d['turns']), len(d['probes']))
                    if state != previous:
                        print(a.label, 'turns', state[0], 'probes', state[1], flush=True)
                        previous = state
                except json.JSONDecodeError:
                    pass  # A progress write can be observed mid-write, never final evidence.
            time.sleep(2)
        d = json.loads(final.read_text(encoding='utf-8'))
        assert d['execution_complete'] is True and d['scope'] == a.scope
        assert len(d['probes']) == len(plan.get('probes', []))
        assert len(d['turns']) == len(plan.get('turns', [])) + len(plan.get('probes', []))
        h.verify(repo, a.freeze)
        assert hashlib.sha256(native.read_bytes()).hexdigest() == freeze['native_gui_sha256']
        assert json.loads((reports / 'real-public-pin.json').read_text(encoding='utf-8')) == freeze['model_pin']
        for relative, sha in freeze.get('frontend_sha256', {}).items():
            assert hashlib.sha256((repo / 'qa-dist-22d' / relative).read_bytes()).hexdigest() == sha
        print(a.label, 'execution complete; quality requires independent adjudication', flush=True)
    finally:
        if lock.exists() and lock.read_text(encoding='utf-8') == nonce:
            lock.unlink()


if __name__ == '__main__':
    p = argparse.ArgumentParser()
    for name in ['root', 'bridge', 'freeze', 'plan', 'label', 'scope']:
        p.add_argument('--' + name, required=True)
    p.add_argument('--initialize', action='store_true', help='Configure only the fresh synthetic fixture identity before development; never used for a frozen final run.')
    p.add_argument('--timeout', type=int, default=7200)
    main(p.parse_args())
