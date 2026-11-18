"""Exercise the real independent QA supervisors on loopback using GET /health only."""
import json
import os
import runpy
import sys
import tempfile
import time
import urllib.request
from pathlib import Path
from types import SimpleNamespace

ROOT = Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
qa = runpy.run_path(str(ROOT / 'scripts/lucille-independent.py'))
lifecycle = runpy.run_path(str(ROOT / 'scripts/run-lucille-phase22e-owned.py'))
MODELS = {'nomic': 15289, 'gemma': 15288}


def broker_offsets():
    reports = qa['REPORTS']
    return {name: (reports / name).stat().st_size for name in
            ('real-broker-calls.ndjson', 'real-broker-errors.ndjson')}


def collect(root, model):
    manifest = qa['read'](root / (model + '-process.json'))
    log = root / 'reports' / (model + '-resources.ndjson')
    with log.open(encoding='utf8') as stream:
        first = json.loads(stream.readline())
    qa['require'](first['PhysicalAvailable'] >= 1024**3 and first['process_working_set'] > 0,
                  model + ' has no valid first resource sample')
    command = manifest['command']
    ctx = int(command[command.index('--ctx-size') + 1])
    gpu = int(command[command.index('--gpu-layers') + 1])
    qa['require']((ctx, gpu) == ((2048, 0) if model == 'nomic' else (65536, 99)),
                  'Unexpected backend configuration')
    with urllib.request.urlopen(f'http://127.0.0.1:{MODELS[model]}/health', timeout=5) as response:
        qa['require'](response.status == 200 and manifest['ready'], 'Model not ready')
    return dict(pid=manifest['pid'], creation_filetime=manifest['creation_filetime'],
                context=ctx, gpu_layers=gpu, health_http=200, ready_seconds=manifest['initialization_seconds'],
                first_resource_sample=first)


def main():
    with qa['controller_lock'](SimpleNamespace(reports=qa['REPORTS'])):
        qa['require'](qa['environment']() == qa['read'](qa['SUITE'] / 'runtime-lock.json'),
                      'Pinned runtime mismatch')
        root = Path(tempfile.mkdtemp(prefix='lucille-synthetic-supervisor-test-', dir=qa['QA_ROOT'].parent))
        (root / '.lucille-isolated').write_bytes(qa['MARKER'])
        (root / 'reports').mkdir()
        contract = qa['read'](qa['QA_ROOT'] / 'attested-contract.json')
        before = broker_offsets()
        os.environ['LUCILLE_G22_BUDGET'] = 'independent'
        results = {}
        try:
            for model in MODELS:
                lifecycle['launch'](root, model, contract)
            for model in MODELS:
                results[model] = collect(root, model)
        finally:
            lifecycle['stop'](root, list(MODELS))
        for model in MODELS:
            deadline = time.monotonic() + 15
            while True:
                manifest = qa['read'](root / (model + '-process.json'))
                if 'exit_code' in manifest:
                    break
                qa['require'](time.monotonic() < deadline, 'No final manifest for ' + model)
                time.sleep(.25)
            qa['require'](manifest.get('stop_identity_verified') and manifest.get('stop_reason') == 'requested test shutdown'
                          and manifest['creation_filetime'] == results[model]['creation_filetime'], 'Unverified shutdown')
            results[model]['stop_identity_verified'] = True
        qa['require'](not any(qa['port_open'](p) for p in qa['PORTS']), 'QA listener still present')
        qa['require'](before == broker_offsets() and not qa['BUDGET'].exists(), 'Broker activity during test')
        qa['require'](not (root/'reports/g22-independent-abort').exists(), 'Supervisor resource abort')
        qa['verify_first_attempt']()
        result = dict(models=results, protected_files_verified=qa['integrity'](), ports_closed=qa['PORTS'],
                      campaign_inputs_sent=0, gemma_inferences=0, embeddings=0)
        qa['write'](root/'startup-test.json', result)
        print(json.dumps(result, ensure_ascii=False))


if __name__ == '__main__':
    main()
