"""Run the three frozen outages in the dedicated consolidation GUI."""
import argparse
import json
import os
import sys
from pathlib import Path
from types import SimpleNamespace

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'qa'))
from phase22f_evaluation import QA_ROOT, controller_lock, failure, module, read, send_control, slug


def main(prefix):
    manifest = read(QA_ROOT / 'gui-processes.json')
    if Path(manifest['root']).name != 'lucille-synthetic-full22g':
        raise ValueError('Dedicated consolidation host required')
    if os.environ.get('LUCILLE_G22_BUDGET') != '1':
        raise ValueError('Missing budget across broker restarts')
    run = SimpleNamespace(root=QA_ROOT, reports=QA_ROOT / 'reports')
    transport = module('scripts/test-lucille-phase22f-transport.py')
    wait = module('qa/phase22f-artifact-check.py').read_report
    fixtures = ROOT / 'crates/lucille-memory/tests/fixtures/phase22f/final-v1/faults.json'
    out = ROOT / 'docs/evidence/phase22g/consolidation/gui.json'
    if out.exists():
        raise FileExistsError(out)
    results = []
    with controller_lock(run):
        for case in transport.cases_from(read(fixtures)):
            label = slug(prefix + '-' + case['mode'])
            scope = 'phase22f-fault-' + label
            if list(run.reports.glob('driver-' + label + '-*')):
                raise FileExistsError(label)
            row = dict(mode=case['mode'], scope=scope, project=case['project'], source=case['text'])
            try:
                try:
                    transport.broker(run, 'broker-stop')
                    send_control(run, label, dict(scope=scope, project=case['project'], resume=True,
                        expect_transport_failure=True, plan=dict(id=label, project=case['project'],
                        turns=[dict(text=case['text'], new_session=True, mode=case['mode'])], probes=[])))
                    error = wait(run.reports / f'driver-{label}-transport-progress.json', timeout=90)
                    row['error_messages'] = error['messages']
                finally:
                    transport.broker(run, 'broker-restart')
                result = wait(run.reports / f'driver-{label}-final.json', timeout=180)
                row.update(identity=result['identity'], details=result['final_details'], metrics=result['final_metrics'],
                    turns=[{k:t.get(k) for k in ('conversation_id','messages','domPaints','operationSnapshots','mode')}
                           for t in result['turns']])
                send_control(run, label + '-copy', dict(copy=True, observe_copy_payload=True))
                row['copy'] = wait(run.reports / f'driver-{label}-copy-copy.json', timeout=30)
                row['verified'] = transport.verify_recovery(case, error, result)
                if row['copy'].get('status') != 'refused_unverified':
                    raise ValueError('Unexpected copy result after a transport failure')
                row['status'] = 'measured'
            except Exception as exc:
                row.update(failure(exc))
            results.append(row)
            out.write_text(json.dumps(results, ensure_ascii=False, indent=2)+'\n', encoding='utf8')
            print(label, row['status'], flush=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('prefix', type=slug)
    main(parser.parse_args().prefix)
