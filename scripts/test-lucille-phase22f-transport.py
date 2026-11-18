"""Actual composer outages with a frozen typed expectation for each source."""

import argparse
from pathlib import Path
import re
import subprocess
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'qa'))
from phase22f_evaluation import Evaluation, EvaluationFailure, REPO, controller_lock, failure, proof_matches_source, protocol_plan, report_counts, require_product, send_control, slug


NETWORK = re.compile(r'network|not answering|failed to fetch|connection', re.I)
FIELDS = ('subject', 'property', 'value')


def verify_recovery(case, error, result):
    require_product(result['execution_complete'] and len(result['turns']) == 1 and not result['initial_details'] and
                    not result['initial_deleted'], 'One actual recovered turn in an empty initial scope required')
    turn = result['turns'][0]
    original = [message for message in turn['messages'] if message['role'] == 'user']
    original_at_error = [message for message in error['messages'] if message['role'] == 'user']
    require_product(turn['transport_failure'] and turn['mode'] == case['mode'] and len(original) == 1 and
                    original == original_at_error and original[0]['content'] == case['text'],
                    'Recovery did not preserve the single exact composer USER source')
    painted = [event for event in turn['domPaints'] if NETWORK.search(event['text'])]
    require_product(bool(painted), 'Transport error was not painted in DOM/transcript')
    details = result['final_details']
    expect = case['expected']
    require_product(len(details) == 1 and details[0]['memory']['state'] == expect.get('state', 'active') and
                    details[0]['memory']['input']['kind'] == expect['kind'] and details[0]['quality'] is not None and
                    all(details[0]['quality'][key] == expect[key] for key in FIELDS),
                    'Recovered fact does not match the typed gold for this source')
    detail = details[0]
    snapshots = turn['operationSnapshots']
    confirmed = [row for row in snapshots if row.get('learning_receipt') and (row.get('job') or {}).get('state') == 'done']
    require_product(bool(confirmed), 'No observed durable completion after reconnect')
    terminal = confirmed[-1]
    receipt = terminal['learning_receipt']
    record = detail['memory']
    require_product(proof_matches_source(detail, case['text'], receipt['id']),
                    'Native proof does not bind the exact USER source and job')
    require_product(result['final_metrics']['jobs'] == [{'count': 1, 'state': 'done'}],
                    'Recovered scope must contain exactly one terminal job')
    require_product(receipt['source_message_id'] == original[0]['id'] and receipt['conversation_id'] == turn['conversation_id'] and
                    receipt['scope'] == result['scope'] and receipt['project'] == case['project'] and
                    receipt.get('profile') == 'lucille' and receipt.get('identity_version') == result['identity']['version'] and
                    receipt.get('mode') == case['mode'] and receipt['action'] == 'learn' and receipt['state'] == 'done' and
                    any(entry['id'] == record['input']['id'] and entry['version'] == record['version'] and
                        entry['state'] == record['state'] for entry in receipt['records']),
                    'Receipt does not bind source, scoped record version, and final state')
    require_product(terminal['read_started_ms'] >= painted[0]['at_ms'], 'Receipt observation must follow the painted error')
    return dict(first_error_paint_ms=painted[0]['at_ms'], receipt_read_ms=terminal['read_finished_ms'],
                canonical={key: detail['quality'][key] for key in FIELDS})


def cases_from(data):
    cases = data.get('transport', [])
    if sorted(case['mode'] for case in cases) != ['agent', 'chat', 'code']:
        raise ValueError('One frozen transport case per mode is required')
    for case in cases:
        if not case['text'].strip() or not case['project'].strip() or not all(
                isinstance(case['expected'][key], str) and case['expected'][key] for key in (*FIELDS, 'kind')):
            raise ValueError('Transport scenario requires a literal source and typed canonical gold')
    return cases


def broker(run, operation):
    try:
        subprocess.run([sys.executable, 'scripts/run-lucille-phase22f-owned.py', '--root', str(run.root),
                        '--operation', operation], cwd=REPO, check=True, timeout=90)
    except subprocess.CalledProcessError as error:
        raise EvaluationFailure('infrastructure_failure', f'Owned {operation} failed') from error


def main(args):
    run = Evaluation(args, 'transport', gui=True)
    cases = cases_from(protocol_plan(run))
    with controller_lock(run):
        for case in cases:
            mode = case['mode']
            label = slug(run.label + '-' + mode)
            scope = 'phase22f-fault-' + label
            row = dict(mode=mode, scope=scope, case=case)
            try:
                if len(label) > 50 or list(run.reports.glob('driver-' + label + '-*')):
                    raise ValueError('Unique supported transport driver label required')
                run.guard('gemma', 'nomic')
                run.live_pin('extraction')
                row['stage'] = 'broker_outage'
                try:
                    broker(run, 'broker-stop')
                    send_control(run, label, dict(scope=scope, project=case['project'], resume=True,
                                                 expect_transport_failure=True, plan=dict(id=label, project=case['project'],
                                                 turns=[dict(text=case['text'], new_session=True, mode=mode)], probes=[])))
                    row['actual_error'] = run.artifacts.read_report(run.reports / f'driver-{label}-transport-progress.json', timeout=90)
                finally:
                    broker(run, 'broker-restart')
                row['stage'] = 'composer_recovery'
                row['native_result'] = run.artifacts.read_report(run.reports / f'driver-{label}-final.json', timeout=180)
                require_product(any(m['role'] == 'assistant' and NETWORK.search(m.get('content', ''))
                                    for m in row['actual_error']['messages']), 'Actual assistant error not captured')
                row['verified'] = verify_recovery(case, row['actual_error'], row['native_result'])
                row['status'] = 'measured'
            except Exception as error:
                row.update(failure(error))
            run.add(row)
            print(mode, row['status'], flush=True)
        counts = report_counts(run.rows)
        return run.finish(passed=counts['measured'] == counts['planned'], counts=counts, calls=run.calls)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    for name in ('root', 'freeze', 'dataset', 'label'):
        parser.add_argument('--' + name, required=True)
    raise SystemExit(main(parser.parse_args()))
