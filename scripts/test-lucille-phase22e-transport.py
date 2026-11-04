"""Real isolated composer outage/recovery, one owned broker outage per mode.

The bundled observer captures the actual error before waiting for durable
learning. No OS input, model replacement, source resend or canonical SQL writes.
"""
import argparse
import importlib.util
import json
from pathlib import Path
import re
import subprocess
import sys
import uuid
sys.dont_write_bytecode = True


def main(a):
    repo = Path(__file__).resolve().parents[1]
    root = Path(a.root).resolve(strict=True)
    assert root == repo/'.lucille-qa-runtime/lucille-synthetic-real22e'
    assert (root/'.lucille-isolated').read_bytes() == b'synthetic-data-only\n'
    spec = importlib.util.spec_from_file_location('freeze_check', repo/'scripts/test-lucille-real-quality-extraction.py')
    check = importlib.util.module_from_spec(spec); spec.loader.exec_module(check)
    freeze = check.verify(repo, a.freeze)
    artifact_spec = importlib.util.spec_from_file_location('artifact_check', repo/'qa/phase22e-artifact-check.py')
    artifacts = importlib.util.module_from_spec(artifact_spec); artifact_spec.loader.exec_module(artifacts)
    initial_artifacts = artifacts.verify_artifacts(repo, root, freeze)
    dataset = Path(a.dataset).resolve(strict=True)
    assert dataset.relative_to(repo).as_posix() in freeze['corpus_sha256']
    cases = json.loads(dataset.read_text(encoding='utf-8'))['cases']
    assert re.fullmatch(r'[a-z0-9-]{1,30}', a.label)
    reports = root/'reports'; output = reports/(a.label+'-transport.json')
    assert not output.exists()
    lock = reports/'phase22e-controller.lock'; nonce = str(uuid.uuid4())
    with lock.open('x', encoding='utf-8') as stream: stream.write(nonce)
    rows = []

    def wait(path, seconds=180):
        return artifacts.read_report(path, timeout=seconds)

    def owned_stop():
        spec = importlib.util.spec_from_file_location('owned', repo/'scripts/run-lucille-phase22e-owned.py')
        owned = importlib.util.module_from_spec(spec); spec.loader.exec_module(owned)
        owned.stop(root, ['broker'])

    def restore():
        subprocess.run([sys.executable, 'scripts/run-lucille-phase22e-owned.py', '--root', str(root), '--operation', 'broker-restart'], cwd=repo, check=True)

    try:
        for mode in ['chat', 'agent', 'code']:
            case = next(c for c in cases if c['type']=='prepared_cancel' and c['mode']==mode)
            label = a.label+'-'+mode; scope = 'phase22e-fault-'+label
            assert not list(reports.glob('driver-'+label+'-*')), 'Never repeat an outage label'
            owned_stop()
            try:
                control = dict(id=str(uuid.uuid4()), label=label, scope=scope, project=case['project'], resume=True, expect_transport_failure=True,
                               plan=dict(id=label, project=case['project'], turns=[dict(text=case['text'], new_session=True, mode=mode)], probes=[]))
                (reports/'control.json').write_bytes((json.dumps(control, ensure_ascii=False)+'\n').encode('utf-8'))
                error = wait(reports/('driver-'+label+'-transport-progress.json'), 60)
                assert any(m['role']=='assistant' and re.search(r'network|not answering|failed to fetch|connection', m.get('content',''), re.I) for m in error['messages'])
            finally:
                restore()
            result = wait(reports/('driver-'+label+'-final.json'))
            assert result['execution_complete'] and len(result['turns'])==1
            turn = result['turns'][0]; details = result['final_details']
            assert turn['transport_failure'] and turn['text']==case['text']
            assert len(details)==1 and details[0]['memory']['state']=='active' and details[0]['quality']['value']=='without_sugar'
            assert '|native_source_authority_v2:' in details[0]['quality']['evidence_rule']
            assert any('network' in p['text'].lower() or 'not answering' in p['text'].lower() for p in turn['domPaints'])
            rows.append(dict(mode=mode, scope=scope, source=case['text'], actual_error=error, native_result=result, passed=True))
            (reports/(a.label+'-transport-progress.json')).write_bytes((json.dumps(rows,ensure_ascii=False,indent=2)+'\n').encode('utf-8'))
            print(mode, 'actual error displayed; original USER persisted after broker recovery', flush=True)
        check.verify(repo, a.freeze)
        final_artifacts = artifacts.verify_artifacts(repo, root, freeze)
        with output.open('xb') as stream: stream.write((json.dumps(dict(rows=rows,passed=True,initial_artifacts=initial_artifacts,final_artifacts=final_artifacts),ensure_ascii=False,indent=2)+'\n').encode('utf-8'))
    finally:
        if lock.exists() and lock.read_text(encoding='utf-8')==nonce: lock.unlink()


if __name__=='__main__':
    parser=argparse.ArgumentParser()
    for name in ['root','freeze','dataset','label']: parser.add_argument('--'+name,required=True)
    main(parser.parse_args())
