"""Actual scoped native RPC/worker faults. Private leases stay in marked QA.

No fabricated extraction output: late completion uses the actual broker response.
Only the manifest-verified owned host may be terminated for the restart case.
"""
import argparse
import ctypes
from ctypes import wintypes
import datetime
import hashlib
import importlib.util
import json
from pathlib import Path
import sqlite3
import subprocess
import sys
import time
import uuid
sys.dont_write_bytecode = True


def main(a):
    repo = Path(__file__).resolve().parents[1]
    root = Path(a.root).resolve(strict=True)
    assert root == repo / '.lucille-qa-runtime/lucille-synthetic-real22d'
    assert (root / '.lucille-isolated').read_bytes() == b'synthetic-data-only\n'
    spec = importlib.util.spec_from_file_location('check', repo/'scripts/test-lucille-real-quality-extraction.py')
    check = importlib.util.module_from_spec(spec); spec.loader.exec_module(check)
    freeze = check.verify(repo, a.freeze)
    corpus = Path(a.dataset).resolve(strict=True)
    assert corpus.relative_to(repo).as_posix() in freeze['corpus_sha256']
    cases = json.loads(corpus.read_text(encoding='utf-8'))['cases']
    reports = root / 'reports'
    output = reports / (a.label + '-faults.json')
    assert not output.exists()
    lock = reports / 'phase22d-controller.lock'
    nonce = str(uuid.uuid4())
    with lock.open('x', encoding='utf-8') as f: f.write(nonce)
    assert 1 <= len(a.label) <= 50 and all(c.islower() or c.isdigit() or c=='-' for c in a.label)
    rows = []

    def wait_file(path, timeout=120):
        until = time.monotonic()+timeout
        while time.monotonic() < until:
            if path.exists(): return json.loads(path.read_text(encoding='utf-8'))
            time.sleep(.1)
        raise RuntimeError('Own QA report timeout: '+path.name)

    def control(label, body, suffix='fault'):
        path = reports / ('driver-'+label+'-'+suffix+'.json')
        assert not path.exists()
        (reports/'control.json').write_text(json.dumps(dict(id=str(uuid.uuid4()), label=label, **body), ensure_ascii=False), encoding='utf-8')
        return wait_file(path)

    def rpc(label, request):
        return control(label, {'fault':request})

    def result(value):
        assert not value.get('rejected'), value
        return value['result']

    def details(label, scope, project):
        return result(rpc(label, dict(operation='details', scope=scope, project=project)))

    def publish(label, proof):
        return rpc(label, dict(native_command='lucille_learn', args=dict(interaction=proof['interaction'], source=proof)))

    db = repo/'.lucille-qa-runtime/lucille-synthetic-full22d/lucille-phase1.sqlite3'
    def sql(query, args=()):
        with sqlite3.connect(db.resolve().as_uri()+'?mode=ro', uri=True) as connection:
            connection.row_factory=sqlite3.Row
            return [dict(r) for r in connection.execute(query, args)]

    def jobs(scope, project):
        return sql('SELECT id,state,attempts,lease_token,lease_until,evidence_json FROM extraction_jobs WHERE scope=? AND project=? ORDER BY rowid', (scope, project))

    def wait_terminal(scope, project, timeout=120):
        until=time.monotonic()+timeout
        while time.monotonic()<until:
            value=jobs(scope, project)
            if value and all(r['state'] not in ['queued','retry','running'] for r in value):return value
            time.sleep(.2)
        raise RuntimeError('Scoped worker did not settle')

    def real_reply(scope, source, offset, timeout=120):
        path=reports/'real-broker-calls.ndjson'; until=time.monotonic()+timeout
        while time.monotonic()<until:
            with path.open(encoding='utf-8') as f:
                f.seek(offset)
                for line in f:
                    if not line.endswith('\n'):continue
                    call=json.loads(line)
                    for message in call.get('request',{}).get('messages',[]):
                        if message.get('role')!='user':continue
                        try:payload=json.loads(message.get('content',''))
                        except (ValueError,TypeError):continue
                        if isinstance(payload,dict) and payload.get('scope')==scope and payload.get('user_text')==source:
                            return call
            time.sleep(.2)
        raise RuntimeError('No actual scoped Gemma response observed')

    def stop_owned_gui_fast():
        manifest=json.loads((root/'gui-processes.json').read_text(encoding='utf-8'))['gui']
        expected=Path(manifest['executable']).resolve(strict=True)
        assert expected == (repo.parent/'build/tauri/x86_64-pc-windows-msvc/debug/lucille-full-validation.exe').resolve(strict=True)
        with expected.open('rb') as stream: assert hashlib.file_digest(stream,'sha256').hexdigest()==freeze['native_gui_sha256']
        kernel=ctypes.WinDLL('kernel32', use_last_error=True)
        kernel.OpenProcess.restype=wintypes.HANDLE
        kernel.OpenProcess.argtypes=[wintypes.DWORD,wintypes.BOOL,wintypes.DWORD]
        handle=kernel.OpenProcess(0x1000|0x100000|1,False,manifest['pid'])
        if not handle:raise OSError('Cannot retain owned host handle')
        try:
            image=ctypes.create_unicode_buffer(32768); size=wintypes.DWORD(len(image))
            kernel.QueryFullProcessImageNameW.argtypes=[wintypes.HANDLE,wintypes.DWORD,wintypes.LPWSTR,ctypes.POINTER(wintypes.DWORD)]
            assert kernel.QueryFullProcessImageNameW(handle,0,image,ctypes.byref(size))
            assert Path(image.value).resolve(strict=True)==expected
            times=[wintypes.FILETIME() for _ in range(4)]
            kernel.GetProcessTimes.argtypes=[wintypes.HANDLE,*([ctypes.POINTER(wintypes.FILETIME)]*4)]
            assert kernel.GetProcessTimes(handle,*[ctypes.byref(t) for t in times])
            ticks=times[0].dwHighDateTime<<32|times[0].dwLowDateTime
            created=datetime.datetime(1601,1,1,tzinfo=datetime.timezone.utc)+datetime.timedelta(microseconds=ticks//10)
            expected_created=datetime.datetime.fromisoformat(manifest['created'].replace('Z','+00:00')).astimezone(datetime.timezone.utc)
            assert created==expected_created, 'Owned creation identity mismatch; no stop'
            kernel.TerminateProcess.argtypes=[wintypes.HANDLE,wintypes.UINT]
            assert kernel.TerminateProcess(handle,0)
        finally:
            kernel.CloseHandle.argtypes=[wintypes.HANDLE];kernel.CloseHandle(handle)
        return dict(pid=manifest['pid'],image_verified=True,creation_verified=True)

    try:
        for case in cases:
            label=case['id']; assert len(label)<=36 and all(c.islower() or c.isdigit() or c=='-' for c in label)
            scope='phase22d-fault-'+label;project=case['project']
            assert not jobs(scope,project), 'Never reuse a completed fault scope'
            row=dict(case=case,scope=scope,started_utc=datetime.datetime.now(datetime.timezone.utc).isoformat())
            if case['type']=='prepared_cancel':
                interaction=dict(conversation_id='fault-'+label,message_id='source-'+label,scope=scope,project=project,mode=case['mode'],role='user',user_text=case['text'])
                prepared=result(rpc(label+'-prepare',dict(operation='prepare_user_turn',input=interaction)))
                proof=prepared['proof']; assert proof['job_id']
                cancelled=result(rpc(label+'-cancel',dict(operation='cancel_user_turn',source=proof)))
                replay=result(publish(label+'-replay',proof)); retry=result(publish(label+'-retry',proof))
                invalid={**proof,'job_id':'0'*64}
                missing=publish(label+'-missing',invalid); assert missing.get('rejected') or missing.get('result',{}).get('state')=='ignored'
                assistant=rpc(label+'-assistant',dict(operation='prepare_user_turn',input={**interaction,'message_id':'assistant-'+label,'role':'assistant'}));assert assistant.get('rejected')
                found=details(label+'-details',scope,project);state=jobs(scope,project)
                assert cancelled['cancelled'] and not found and len(state)==1 and state[0]['state']=='cancelled'
                assert state[0]['lease_token'] is None and state[0]['lease_until'] is None
                assert not sql('SELECT job_id FROM learning_inputs WHERE job_id=?',(proof['job_id'],))
                row.update(cancelled=cancelled,replay=replay,retry=retry,missing=missing,assistant=assistant,records=found,jobs=state)
            elif case['type']=='ordered_sources':
                proofs=[]
                for index,text in enumerate(case['texts']):
                    interaction=dict(conversation_id='fault-'+label,message_id='source-'+str(index),scope=scope,project=project,mode=case['mode'],role='user',user_text=text)
                    proofs.append(result(rpc(label+'-prepare-'+str(index),dict(operation='prepare_user_turn',input=interaction)))['proof'])
                result(publish(label+'-newer',proofs[1]))
                until=time.monotonic()+120
                while time.monotonic()<until:
                    if next(j for j in jobs(scope,project) if j['id']==proofs[1]['job_id'])['state']=='done':break
                    time.sleep(.2)
                else:raise RuntimeError('Newer source did not finish')
                before=details(label+'-before',scope,project)
                result(publish(label+'-older',proofs[0]));state=wait_terminal(scope,project)
                after=details(label+'-after',scope,project)
                assert before==after and len(after)==1 and after[0]['quality']['value']==case['expected_value']
                assert all(j['state']=='done' for j in state)
                row.update(before=before,after=after,jobs=state)
            elif case['type'] in ['leased_cancel','restart']:
                offset=(reports/'real-broker-calls.ndjson').stat().st_size
                prepared=control(label,dict(scope=scope,project=project,resume=True,plan=dict(id='fault-'+label,project=project,turns=[],probes=[]),interrupt_source=case['text']),'prepared')
                source=prepared['interrupted_job'];state=jobs(scope,project);assert len(state)==1 and state[0]['state']=='running'
                old=state[0];lease=dict(id=old['id'],token=old['lease_token'],scope=scope,project=project,claims=json.loads(old['evidence_json']),attempts=old['attempts'])
                if case['type']=='leased_cancel':
                    interaction={k:source[k] for k in ['conversation_id','message_id','scope','project']};interaction.update(mode='chat',role='user',user_text=source['text'])
                    cancelled=result(rpc(label+'-cancel',dict(operation='cancel_user_turn',source=dict(interaction=interaction,job_id=old['id']))));assert cancelled['cancelled']
                else:
                    row['termination']=stop_owned_gui_fast()
                    assert jobs(scope,project)[0]['state']=='running', 'Extraction completed before restart; no restart success claimed'
                    subprocess.run(['pwsh','-NoProfile','-File',str(repo/'scripts/stop-lucille-real-gui.ps1'),'-Phase','22d'],cwd=repo,check=True)
                    subprocess.run(['pwsh','-NoProfile','-File',str(repo/'scripts/start-lucille-real-gui.ps1'),'-QaExe',str(repo.parent/'build/tauri/x86_64-pc-windows-msvc/debug/lucille-full-validation.exe'),'-Phase','22d','-StaticFrontend'],cwd=repo,check=True)
                response=real_reply(scope,case['text'],offset)
                state=wait_terminal(scope,project)
                assert len(state)==1
                before=details(label+'-before-late',scope,project)
                late=result(rpc(label+'-late',dict(operation='complete_llm',lease=lease,output=response['response']['choices'][0]['message']['content'])))
                after=details(label+'-after-late',scope,project)
                assert late['completed'] is False and before==after
                if case['type']=='leased_cancel':assert not after and state[0]['state']=='cancelled'
                else:
                    assert state[0]['state']=='done' and state[0]['attempts']>=2 and len(after)==case['expected_records']
                    assert len({m['memory']['input']['fact_key'] for m in after})==len(after)
                row.update(actual_response=response,old_lease=lease,before=before,after=after,late=late,jobs=state)
            else:raise ValueError('Unsupported frozen fault type')
            row['passed']=True;row['finished_utc']=datetime.datetime.now(datetime.timezone.utc).isoformat()
            rows.append(row)
            (reports/(a.label+'-faults-progress.json')).write_text(json.dumps(rows,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
            print(label,'passed',flush=True)
        check.verify(repo,a.freeze)
        with output.open('x',encoding='utf-8') as f:json.dump(dict(freeze_sha256=hashlib.sha256(Path(a.freeze).read_bytes()).hexdigest(),rows=rows,passed=all(r['passed'] for r in rows)),f,ensure_ascii=False,indent=2)
    finally:
        if lock.exists() and lock.read_text(encoding='utf-8')==nonce:lock.unlink()


if __name__=='__main__':
    parser=argparse.ArgumentParser()
    for name in ['root','freeze','dataset','label']:parser.add_argument('--'+name,required=True)
    main(parser.parse_args())
