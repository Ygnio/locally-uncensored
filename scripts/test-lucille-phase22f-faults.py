"""Independent native fault cases through the marked F Tauri QA host."""

import argparse
import ctypes
from ctypes import wintypes
import datetime
import json
from pathlib import Path
import sqlite3
import subprocess
import sys
import time

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'qa'))
from phase22f_evaluation import Evaluation, EvaluationFailure, MARKER, REPO, controller_lock, digest, failure, protocol_plan, read, report_counts, require_product, send_control, slug


NATIVE = REPO.parent / 'build/phase22f/lucille-full-validation.exe'
PENDING = {'queued', 'retry', 'running'}
FACT_FIELDS = ('subject', 'property', 'value')


def matches(row, expected):
    return row['quality'] is not None and row['memory']['input']['kind'] == expected['kind'] and all(
        row['quality'][key] == expected[key] for key in FACT_FIELDS)


def active(details):
    return [row for row in details if row['memory']['state'] == 'active']


def terminate_owned_host(run):
    # Verify image and creation time on the same retained handle used to stop.
    record = read(run.root / 'gui-processes.json')['gui']
    path = NATIVE.resolve(strict=True)
    if Path(record['executable']).resolve(strict=True) != path or digest(path) != run.freeze['native_gui_sha256']:
        raise ValueError('Frozen owned host required before restart')
    kernel = ctypes.WinDLL('kernel32', use_last_error=True)
    kernel.OpenProcess.restype = wintypes.HANDLE
    kernel.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
    handle = kernel.OpenProcess(0x1000 | 0x100000 | 1, False, record['pid'])
    if not handle:
        raise OSError('Cannot retain the owned GUI handle')
    try:
        image = ctypes.create_unicode_buffer(32768)
        size = wintypes.DWORD(len(image))
        kernel.QueryFullProcessImageNameW.argtypes = [wintypes.HANDLE, wintypes.DWORD, wintypes.LPWSTR,
                                                      ctypes.POINTER(wintypes.DWORD)]
        if not kernel.QueryFullProcessImageNameW(handle, 0, image, ctypes.byref(size)) or Path(image.value).resolve() != path:
            raise ValueError('Owned image changed; restart refused')
        times = [wintypes.FILETIME() for _ in range(4)]
        kernel.GetProcessTimes.argtypes = [wintypes.HANDLE, *([ctypes.POINTER(wintypes.FILETIME)] * 4)]
        if not kernel.GetProcessTimes(handle, *(ctypes.byref(t) for t in times)):
            raise OSError('Could not verify host creation')
        ticks = times[0].dwHighDateTime << 32 | times[0].dwLowDateTime
        created = datetime.datetime(1601, 1, 1, tzinfo=datetime.timezone.utc) + datetime.timedelta(microseconds=ticks // 10)
        if created != run.artifacts.utc(record['created']):
            raise ValueError('Owned creation time changed; restart refused')
        kernel.TerminateProcess.argtypes = [wintypes.HANDLE, wintypes.UINT]
        if not kernel.TerminateProcess(handle, 0):
            raise OSError('Owned host termination failed')
    finally:
        kernel.CloseHandle.argtypes = [wintypes.HANDLE]
        kernel.CloseHandle(handle)
    return dict(pid=record['pid'], image_verified=True, creation_verified=True)


def restart_frontend():
    for script, arguments in [('stop', []), ('start', ['-QaExe', str(NATIVE), '-StaticFrontend'])]:
        subprocess.run(['pwsh', '-NoProfile', '-File', str(REPO / f'scripts/{script}-lucille-real-gui.ps1'),
                        '-Phase', '22f', *arguments], cwd=REPO, check=True, timeout=90)


class Case:
    def __init__(self, run, case, row):
        self.run = run
        self.case = case
        self.row = row
        self.label = slug(run.label + '-' + case['id'])
        self.scope = 'phase22f-fault-' + self.label
        self.project = case['project']
        self.row.update(scope=self.scope, project=self.project)
        root = (REPO / '.lucille-qa-runtime/lucille-synthetic-full22f').resolve(strict=True)
        if root != REPO / '.lucille-qa-runtime/lucille-synthetic-full22f' or (root / '.lucille-isolated').read_bytes() != MARKER:
            raise ValueError('Exact marked F GUI profile required')
        self.db = (root / 'lucille-phase1.sqlite3').resolve(strict=True)
        if self.db.parent != root:
            raise ValueError('Linked QA database refused')

    def control(self, suffix, body, report='fault'):
        label = slug(self.label + suffix)
        target = self.run.reports / f'driver-{label}-{report}.json'
        if len(label) > 50 or target.exists():
            raise ValueError('Fresh scoped driver label required')
        send_control(self.run, label, body)
        return self.run.artifacts.read_report(target)

    def rpc(self, suffix, operation=None, *, rejected=False, **data):
        request = dict(operation=operation, **data) if operation else data
        response = self.control(suffix, {'fault': request})
        self.row.setdefault('native_steps', {})[suffix] = response
        if rejected:
            return response
        require_product(not response.get('rejected'), f'Native operation rejected: {suffix}')
        return response['result']

    def interaction(self, message_id, text):
        return dict(conversation_id='fault-' + self.label, message_id=message_id, scope=self.scope, project=self.project,
                    mode=self.case.get('mode', 'chat'), role='user', user_text=text)

    def publish(self, suffix, proof, *, rejected=False):
        return self.rpc(suffix, native_command='lucille_learn', args=dict(interaction=proof['interaction'], source=proof),
                        rejected=rejected)

    def details(self, suffix):
        return self.rpc(suffix, 'details', scope=self.scope, project=self.project)

    def sql(self, query, args=()):
        with sqlite3.connect(self.db.as_uri() + '?mode=ro', uri=True) as connection:
            connection.execute('PRAGMA query_only=ON')
            connection.row_factory = sqlite3.Row
            return [dict(row) for row in connection.execute(query, args)]

    def jobs(self):
        return self.sql('SELECT id,state,attempts,lease_token,lease_until,evidence_json FROM extraction_jobs '
                        'WHERE scope=? AND project=? ORDER BY rowid', (self.scope, self.project))

    def wait_jobs(self, *, only=None):
        until = time.monotonic() + 120
        while time.monotonic() < until:
            rows = [row for row in self.jobs() if only is None or row['id'] == only]
            if rows and all(row['state'] not in PENDING for row in rows):
                return rows
            self.run.guard('gemma', 'nomic')
            time.sleep(.2)
        raise EvaluationFailure('infrastructure_failure', 'Scoped native worker did not settle')

    def real_reply(self, path, offset):
        until = time.monotonic() + 120
        while time.monotonic() < until:
            with path.open('rb') as stream:
                stream.seek(offset)
                for line in stream:
                    if not line.endswith(b'\n'):
                        continue
                    call = json.loads(line)
                    for message in call.get('request', {}).get('messages', []):
                        if message.get('role') != 'user':
                            continue
                        try:
                            payload = json.loads(message.get('content', ''))
                        except (ValueError, TypeError):
                            continue
                        if isinstance(payload, dict) and payload.get('scope') == self.scope and payload.get('user_text') == self.case['text']:
                            if call.get('response', {}).get('choices'):
                                return call
            self.run.guard('gemma', 'nomic')
            time.sleep(.2)
        raise EvaluationFailure('infrastructure_failure', 'No actual scoped Gemma response observed')

    def prepared_cancel(self):
        interaction = self.interaction('original', self.case['text'])
        prepared = self.rpc('-prepare', 'prepare_user_turn', input=interaction)
        proof = prepared['proof']
        require_product(bool(proof['job_id']), 'Prepared USER source has no native job')
        cancelled = self.rpc('-cancel', 'cancel_user_turn', source=proof)
        self.publish('-replay', proof)
        self.publish('-retry', proof)
        missing = self.publish('-missing', {**proof, 'job_id': '0' * 64}, rejected=True)
        assistant = self.rpc('-assistant', 'prepare_user_turn', rejected=True,
                             input={**interaction, 'message_id': 'assistant', 'role': 'assistant'})
        found, jobs = self.details('-details'), self.jobs()
        self.row.update(details=found, jobs=jobs)
        require_product(missing.get('rejected') or missing.get('result', {}).get('state') == 'ignored',
                        'Unbound source accepted')
        require_product(assistant.get('rejected') and cancelled['cancelled'] and not found and len(jobs) == 1 and
                        jobs[0]['state'] == 'cancelled' and jobs[0]['lease_token'] is None and jobs[0]['lease_until'] is None and
                        not self.sql('SELECT job_id FROM learning_inputs WHERE job_id=?', (proof['job_id'],)),
                        'Cancelled source can still publish, retain a lease or leave records')

    def ordered_sources(self):
        proofs = [self.rpc(f'-prepare-{index}', 'prepare_user_turn', input=self.interaction(f'source-{index}', text))['proof']
                  for index, text in enumerate(self.case['texts'])]
        require_product(all(proof['job_id'] for proof in proofs), 'One source was rejected before materialization')
        self.publish('-newer', proofs[1])
        self.wait_jobs(only=proofs[1]['job_id'])
        before = self.details('-before')
        self.publish('-older', proofs[0])
        jobs = self.wait_jobs()
        after = self.details('-after')
        self.row.update(before=before, after=after, jobs=jobs)
        expected = self.case['expected']
        current = active(before)
        require_product(len(current) == 1 and matches(current[0], expected),
                        'Newer typed correction never materialized before old source')
        after_current = active(after)
        require_product(len(after_current) == 1 and matches(after_current[0], expected) and
                        after_current[0]['memory']['input']['id'] == current[0]['memory']['input']['id'] and
                        after_current[0]['memory']['version'] == current[0]['memory']['version'] and
                        len(jobs) == 2 and all(job['state'] == 'done' for job in jobs),
                        'Older source changed the effective current correction or failed to settle')

    def leased(self, restart):
        path = self.run.reports / 'real-broker-calls.ndjson'
        offset = path.stat().st_size if path.exists() else 0
        plan = dict(id='fault-' + self.label, project=self.project, turns=[], probes=[])
        prepared = self.control('', dict(scope=self.scope, project=self.project, resume=True, plan=plan,
                                         interrupt_source=self.case['text']), 'prepared')
        source, jobs = prepared['interrupted_job'], self.jobs()
        self.row.update(prepared=source, jobs_at_interruption=jobs)
        if len(jobs) != 1 or jobs[0]['state'] != 'running':
            raise EvaluationFailure('invalid_result', 'No live leased extraction was captured')
        old = jobs[0]
        lease = dict(id=old['id'], token=old['lease_token'], scope=self.scope, project=self.project,
                     claims=json.loads(old['evidence_json']), attempts=old['attempts'])
        if restart:
            self.row['termination'] = terminate_owned_host(self.run)
            try:
                if self.jobs()[0]['state'] != 'running':
                    raise EvaluationFailure('invalid_result', 'Extraction completed before host restart')
            finally:
                restart_frontend()
            self.row['restarted_host'] = self.run.verify()
        else:
            interaction = dict(conversation_id=source['conversation_id'], message_id=source['message_id'], scope=self.scope,
                               project=self.project, mode='chat', role='user', user_text=source['text'])
            cancelled = self.rpc('-cancel', 'cancel_user_turn', source=dict(interaction=interaction, job_id=old['id']))
            require_product(cancelled['cancelled'], 'Lease cancellation refused')
        response = self.real_reply(path, offset)
        jobs = self.wait_jobs()
        before = self.details('-before-late')
        late = self.rpc('-late', 'complete_llm', lease=lease, output=response['response']['choices'][0]['message']['content'])
        after = self.details('-after-late')
        self.row.update(actual_response=response, jobs=jobs, before=before, after=after, late=late)
        require_product(len(jobs) == 1 and late['completed'] is False and before == after,
                        'Revoked lease accepted late completion or changed state')
        if restart:
            wanted = self.case['expected']
            require_product(jobs[0]['state'] == 'done' and jobs[0]['attempts'] >= 2 and len(after) == len(wanted) and
                            len({record['memory']['input']['fact_key'] for record in after}) == len(after) and
                            all(sum(matches(record, expected) for record in after) == 1 for expected in wanted),
                            'Restart did not durably materialize the expected facts exactly once')
        else:
            require_product(not after and jobs[0]['state'] == 'cancelled', 'Cancelled lease left memory')

    def sqlite_busy(self):
        # Reserve the writer lock of the exact marked synthetic database. The
        # fault connection makes no data writes and always rolls back.
        connection = sqlite3.connect(self.db.as_uri() + '?mode=rw', uri=True, timeout=0)
        try:
            connection.execute('BEGIN IMMEDIATE')
            rejected = self.rpc('-blocked', 'prepare_user_turn', rejected=True,
                                input=self.interaction('original', self.case['text']))
            require_product(rejected.get('rejected') and 'database operation failed' in rejected.get('error', '').lower(),
                            'SQLite writer lock did not cause the expected native failure')
            require_product(not self.jobs(), 'Failed SQLite operation left a queued source')
        finally:
            connection.rollback()
            connection.close()
        require_product(not self.details('-after-busy'), 'Failed SQLite operation left canonical memory')


def check_cases(data):
    cases = data['cases']
    if not cases or len({case['id'] for case in cases}) != len(cases):
        raise ValueError('Nonempty unique fault plan required')
    for case in cases:
        slug(case['id'])
        if not case['project'].strip() or case['type'] not in {'prepared_cancel', 'ordered_sources', 'leased_cancel', 'restart', 'sqlite_busy'}:
            raise ValueError('Unknown fault protocol or missing project')
        if case['type'] == 'ordered_sources' and len(case['texts']) != 2:
            raise ValueError('Out-of-order case requires two literal sources')
        if case['type'] in {'ordered_sources', 'restart'}:
            expected = case['expected'] if case['type'] == 'restart' else [case['expected']]
            if not expected or not all(row.get(key) for row in expected for key in ('kind', *FACT_FIELDS)):
                raise ValueError('Fault protocol requires typed facts for its actual source')
    return cases


def main(args):
    run = Evaluation(args, 'faults', gui=True)
    cases = check_cases(protocol_plan(run))
    with controller_lock(run):
        for case in cases:
            row = dict(case=case, stage=case['type'])
            protocol = None
            try:
                run.guard('gemma', 'nomic')
                protocol = Case(run, case, row)
                if len(protocol.label) > 37 or protocol.jobs() or list(run.reports.glob('driver-' + protocol.label + '-*')):
                    raise ValueError('Use a fresh bounded native fault scope')
                if case['type'] in ('leased_cancel', 'restart'):
                    protocol.leased(restart=case['type'] == 'restart')
                else:
                    getattr(protocol, case['type'])()
                row['status'] = 'measured'
            except Exception as error:
                row.update(failure(error))
                if protocol:
                    try:
                        row['jobs_at_failure'] = protocol.jobs()
                    except Exception as observation:
                        row['job_observation_error'] = failure(observation)
            run.add(row)
            print(case['id'], row['status'], flush=True)
        counts = report_counts(run.rows)
        return run.finish(passed=counts['measured'] == counts['planned'], counts=counts,
                          contract='Each case has its own scope and reports observed faults even if another case failed.')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    for name in ('root', 'freeze', 'dataset', 'label'):
        parser.add_argument('--' + name, required=True)
    raise SystemExit(main(parser.parse_args()))
