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
import socket
import sys
import time
import uuid

sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location('freeze_check', Path(__file__).with_name('test-lucille-real-quality-extraction.py'))
h = importlib.util.module_from_spec(spec)
spec.loader.exec_module(h)
artifact_spec = importlib.util.spec_from_file_location('artifact_check', Path(__file__).resolve().parents[1]/'qa/phase22e-artifact-check.py')
artifacts = importlib.util.module_from_spec(artifact_spec)
artifact_spec.loader.exec_module(artifacts)


def main(a, phase='22e'):
    repo = Path(__file__).resolve().parents[1]
    root = Path(a.root).resolve(strict=True)
    assert phase in ('22e','22f')
    assert root == repo / '.lucille-qa-runtime' / ('lucille-synthetic-real' + phase)
    assert (root / '.lucille-isolated').read_bytes() == b'synthetic-data-only\n'
    assert re.fullmatch(r'[a-z0-9-]{1,50}', a.label)
    assert re.fullmatch(r'phase' + phase + r'-[a-z0-9-]{1,55}', a.scope) or a.scope == 'phase22c-final-historical-series'
    reports = root / 'reports'
    static = repo / ('qa-dist-' + phase)
    expected_native, expected_bridge = artifacts.artifact_paths(repo, phase)
    if phase == '22f':
        assert Path(a.bridge).resolve(strict=True) == expected_bridge.resolve(strict=True)
    freeze = h.verify(repo, a.freeze)
    assert hashlib.sha256(Path(a.bridge).read_bytes()).hexdigest() == freeze['bridge_sha256']
    for relative, sha in freeze.get('frontend_sha256', {}).items():
        assert hashlib.sha256((static / relative).read_bytes()).hexdigest() == sha
    plan = json.loads(Path(a.plan).read_text(encoding='utf-8'))
    assert str(Path(a.plan).resolve().relative_to(repo)).replace('\\', '/') in freeze['corpus_sha256']
    gui = json.loads((root / 'gui-processes.json').read_text(encoding='utf-8'))
    native = Path(gui['gui']['executable']).resolve(strict=True)
    assert native == expected_native.resolve(strict=True)
    assert hashlib.sha256(native.read_bytes()).hexdigest() == freeze['native_gui_sha256']
    if phase == '22f':
        dependencies = {p.name:artifacts.digest(p) for p in sorted(native.parent.glob('*.dll')) if p.is_file()}
        assert dependencies == freeze['native_local_dependencies_sha256']
        assert artifacts.digest(reports / 'native-build-contract.json') == freeze['native_build_contract_sha256']
        artifacts.running_gui(gui['gui'], native)
    assert json.loads((reports / 'real-public-pin.json').read_text(encoding='utf-8')) == freeze['model_pin']
    ready = artifacts.read_report(reports / 'driver-ready-progress.json', timeout=5)
    assert ready['native'] is True
    assert datetime.datetime.fromisoformat(ready['ready'].replace('Z', '+00:00')) >= datetime.datetime.fromisoformat(gui['gui']['created'])
    final = reports / ('driver-' + a.label + '-final.json')
    assert not final.exists() and not list(reports.glob('driver-' + a.label + '-failure-*.json'))
    lock = reports / ('phase' + phase + '-controller.lock')
    nonce = str(uuid.uuid4())
    with lock.open('x', encoding='utf-8') as f:
        f.write(nonce)
    try:
        control = dict(id=str(uuid.uuid4()), label=a.label, scope=a.scope,
                       project=plan['project'], resume=not a.initialize, plan=plan)
        if a.nomic_off:
            assert phase == '22f' and not a.initialize
            try:
                with socket.create_connection(('127.0.0.1', 15289), timeout=5):
                    raise RuntimeError('Expected the owned Nomic endpoint to be down')
            except ConnectionRefusedError:
                pass
            control['allow_pending_index'] = True
        (reports / 'control.json').write_bytes((json.dumps(control, ensure_ascii=False) + '\n').encode('utf-8'))
        began = time.monotonic()
        previous = None
        while not final.exists():
            failures = list(reports.glob('driver-' + a.label + '-failure-*.json'))
            if failures:
                failure = artifacts.read_report(failures[0], timeout=5)
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
                except (json.JSONDecodeError, UnicodeDecodeError):
                    pass  # A progress write can be observed mid-write, never final evidence.
            time.sleep(2)
        d = artifacts.read_report(final, timeout=5)
        assert d['execution_complete'] is True and d['scope'] == a.scope
        assert len(d['probes']) == len(plan.get('probes', []))
        assert len(d['turns']) == len(plan.get('turns', [])) + len(plan.get('probes', []))
        if a.observe_copy:
            assert phase == '22f'
            copy_path = reports / ('driver-' + a.label + '-copy.json')
            assert not copy_path.exists()
            copy_control = dict(id=str(uuid.uuid4()), label=a.label, copy=True, observe_copy_payload=True)
            (reports / 'control.json').write_bytes((json.dumps(copy_control) + '\n').encode('utf-8'))
            copy = artifacts.read_report(copy_path, timeout=35)
            print(a.label, 'copy payload observation', copy.get('status'), copy.get('handler_payload_matches_verified'), flush=True)
        h.verify(repo, a.freeze)
        assert hashlib.sha256(native.read_bytes()).hexdigest() == freeze['native_gui_sha256']
        if phase == '22f':
            assert {p.name:artifacts.digest(p) for p in sorted(native.parent.glob('*.dll')) if p.is_file()} == dependencies
        assert json.loads((reports / 'real-public-pin.json').read_text(encoding='utf-8')) == freeze['model_pin']
        for relative, sha in freeze.get('frontend_sha256', {}).items():
            assert hashlib.sha256((static / relative).read_bytes()).hexdigest() == sha
        print(a.label, 'execution complete; quality requires independent adjudication', flush=True)
    finally:
        if lock.exists() and lock.read_text(encoding='utf-8') == nonce:
            lock.unlink()


def cli(phase='22e'):
    p = argparse.ArgumentParser()
    for name in ['root', 'bridge', 'freeze', 'plan', 'label', 'scope']:
        p.add_argument('--' + name, required=True)
    p.add_argument('--initialize', action='store_true', help='Configure only the fresh synthetic fixture identity before development; never used for a frozen final run.')
    p.add_argument('--nomic-off', action='store_true', help='Record a Phase 2.2F lexical run with the owned Nomic listener closed.')
    p.add_argument('--observe-copy', action='store_true', help='Observe the real Phase 2.2F Copy payload without writing the system clipboard.')
    p.add_argument('--timeout', type=int, default=7200)
    main(p.parse_args(), phase)


if __name__ == '__main__':
    cli()
