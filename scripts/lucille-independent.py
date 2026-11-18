"""Frozen independent QA campaign. 'check' and 'smoke' never start an inference service."""
import argparse
import hashlib
import json
import os
import re
import socket
import sqlite3
import subprocess
import sys
import time
import uuid
from collections import Counter
from pathlib import Path
from types import SimpleNamespace

ROOT = Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT / 'qa'))
from phase22f_evaluation import QA_ROOT, controller_lock, digest, module, read, send_control, write

SUITE = ROOT / 'docs/evidence/phase22g/independent-evaluation'
REPORTS = QA_ROOT / 'reports'
RUN = QA_ROOT / 'independent-v1'
PORTS = (15283,15284,15287,15288,15289)
PROFILES = tuple('i'+str(i) for i in range(1,6))
MARKER = b'synthetic-data-only\n'
BUDGET = REPORTS / 'g22-independent-budget'
ABORT = REPORTS / 'g22-independent-abort'
TABLES = ('memories','memory_quality','memory_provenance','memory_revisions','memory_tombstones',
          'history','extraction_jobs','memory_source_operations','memory_source_operation_records')


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def port_open(n):
    with socket.socket() as sock:
        sock.settimeout(.3)
        return sock.connect_ex(('127.0.0.1',n)) == 0


def profile(token):
    require(token in (*PROFILES,'smoke'), 'Unknown QA profile')
    p=QA_ROOT.parent/('lucille-synthetic-full22g-retest-'+token)
    require(p.resolve()==p and p.parent==QA_ROOT.parent.resolve(), 'Profile outside direct QA root')
    return p


def new_profile(token):
    p=profile(token)
    p.mkdir(exist_ok=False)
    (p/'.lucille-isolated').write_bytes(MARKER)
    for name in ('platform/roaming','platform/local','tmp','home'):
        (p/name).mkdir(parents=True)
    return p


def db_state(p):
    require((p/'.lucille-isolated').read_bytes()==MARKER,'Wrong marker')
    db=p/'lucille-phase1.sqlite3'
    require(db.resolve().parent==p,'SQLite escaped QA profile')
    with sqlite3.connect(db.resolve().as_uri()+'?mode=ro',uri=True) as c:
        c.row_factory=sqlite3.Row
        c.execute('begin')
        require(c.execute('pragma integrity_check').fetchone()[0]=='ok' and not list(c.execute('pragma foreign_key_check')), 'SQLite integrity')
        return {table:[{k:(v.hex() if isinstance(v,bytes) else v) for k,v in dict(r).items()} for r in c.execute('select * from '+table)] for table in TABLES}


def validate(inputs, gold, config):
    cases=gold['cases']; facts=gold['facts']; source_keys={s['id']:s for s in gold['sources']}
    require(config['gemma_limit']==160 and config['context']==65536,'Campaign contract')
    require(len(cases)==40 and Counter(c['category'] for c in cases)=={c:5 for c in config['categories']} and len(config['categories'])==8,'Category balance')
    by_step={c['step']:c for c in cases}; require(len(by_step)==len({c['id'] for c in cases})==40,'Duplicate case')
    seen={}; states={p:{} for p in PROFILES}
    for i in inputs:
        sid=i['id']; p=i['profile']
        require(p in PROFILES and sid not in seen and re.fullmatch('phase22f-[a-z0-9-]{1,55}',i['scope']),'Invalid binding/ID')
        require(i['mode'] in ('chat','code','agent') and isinstance(i['text'],str) and i['text'].strip(),'Empty text or mode')
        require(set(i)<=set('id profile identity scope project mode text kind case restart replay_source'.split()),'Unexpected model-input field')
        if i['kind']=='source':
            row=source_keys[sid]
            require(states[p]==row['initial'],'Source precondition')
            changed={k:v for k,v in row['expected'].items() if states[p].get(k)!=v}
            for k,v in changed.items():
                f=facts[k]
                require((f['profile'],f['scope'],f['project'])==(p,i['scope'],i['project']),'Cross-bound mutation')
                require((v=='current' and f['source']==sid) or (v in ('historical','deleted') and states[p].get(k)=='current'),'Bad lifecycle')
            states[p]=row['expected']
        else:
            row=by_step[sid]
            require(i['kind']=='query' and all(row[k]==i[k] for k in ('profile','mode','identity','scope','project')) and row['id']==i['case'],'Query binding')
            require(states[p]==row['initial']==row['expected'] and row['required_claims'] and row['forbidden_claims'],'Undefined oracle')
            require(all(s in seen or s==sid for s in row['original_sources']), 'Future source')
            allowed={k for k,v in states[p].items() if v!='deleted' and facts[k]['scope']==i['scope'] and facts[k]['project']==i['project']}
            require(set(row['authorized_facts'])==allowed,'Incorrect reference allowlist')
            for ref in row['required_references']:
                require(ref['fact'] in allowed and states[p][ref['fact']]==ref['state'],'Invalid mandatory reference')
        if 'replay_source' in i:
            old=seen[i['replay_source']]
            require(all(old[k]==i[k] for k in ('profile','scope','project')) and old['kind']=='source','Replay binding')
        seen[sid]=i
    require(set(by_step)|set(source_keys)==set(seen) and len(inputs)==68,'Unscheduled cases or sources')
    require(len({i['text'] for i in inputs})==len(inputs),'Duplicate utterance')


def control_for(i, sources, replay=False, initialize=False):
    # Positive allowlist: oracle, expected records and rubric are not arguments.
    turn={'text':i['text'],'new_session':True,'mode':i['mode'],'review':i['kind']=='source','review_preferences':i['kind']=='source'}
    plan={'id':i['id'],'project':i['project'],'turns':[] if replay else [turn],'probes':[]}
    if replay:
        old=sources[i['replay_source']]
        plan['replay_persisted_sources']=[{'text':old['text'],'mode':old['mode']}]
    return dict(plan=plan,scope=i['scope'],project=i['project'],resume=not initialize,
                synthetic_identity=i['identity'],observe_projection=True,observe_canonical=True,expect_transport_failure=True)


def files_hash(folder):
    require((folder/'index.html').is_file(),'Missing QA frontend')
    h=hashlib.sha256()
    for f in sorted(folder.rglob('*')):
        if f.is_file():
            h.update((f.relative_to(folder).as_posix()+'\0'+digest(f)+'\n').encode())
    return h.hexdigest()


def integrity():
    rows=read(REPORTS/'protected-before-qa.json')['files']
    require(len(rows)==104 and all(digest(Path(r['path']))==r['sha256'].lower() for r in rows),'Protected files changed')
    return len(rows)


def frozen(commit):
    require(bool(re.fullmatch('[0-9a-f]{40}',commit)),'Full freeze commit required')
    rel=SUITE.relative_to(ROOT).as_posix()
    raw=subprocess.check_output(['git','show',commit+':'+rel+'/manifest.json'],cwd=ROOT)
    manifest=json.loads(raw)
    require(manifest==read(SUITE/'manifest.json'),'Manifest differs from Git anchor')
    require(set(f.name for f in SUITE.iterdir())==set(manifest['sha256'])|{'manifest.json'},'Suite membership changed')
    for name,expected in manifest['sha256'].items():
        require(Path(name).name==name and digest(SUITE/name)==expected,'Frozen file changed: '+name)
    for name,expected in read(SUITE/'regressions.json')['sha256'].items():
        require(digest(ROOT/name)==expected,'Regression anchor changed: '+name)
    subprocess.run(['git','diff','--quiet',commit,'--','.'],cwd=ROOT,check=True)
    return manifest


def environment():
    require((QA_ROOT/'.lucille-isolated').read_bytes()==MARKER,'Bad QA root')
    require(not any(port_open(n) for n in PORTS),'QA listener occupied')
    require(not any(profile(p).exists() for p in PROFILES),'Campaign profile already exists')
    require(not any(p.exists() for p in (BUDGET,ABORT,RUN)),'Previous campaign evidence exists')
    pin=read(QA_ROOT/'attested-contract.json')
    require(pin['models']['gemma']['context_tokens']==65536 and pin['memory_protocol']==3,'Invalid pin')
    for key in ('gemma','nomic'):
        require(digest(Path(pin['paths'][key]))==pin['models'][key]['fingerprint'],'Weights changed')
    require(digest(Path(pin['runtime_path']))==pin['runtime_sha256'],'Inference binary changed')
    return dict(protected=integrity(),models=pin['models'],runtime_sha256=pin['runtime_sha256'],
                host_sha256=digest(ROOT.parent/'build/phase22g/lucille-full-validation.exe'),
                loader_sha256=digest(ROOT.parent/'build/phase22g/WebView2Loader.dll'),
                frontend_sha256=files_hash(ROOT/'qa-dist-22g'))


def gui_start(token):
    subprocess.run(['pwsh','-NoProfile','-File','scripts/start-lucille-real-gui.ps1','-Phase','22f',
        '-Checkpoint22g','-StaticFrontend','-RetestProfile',token,'-QaExe','../build/phase22g/lucille-full-validation.exe'],cwd=ROOT,check=True,timeout=95)
    host=read(QA_ROOT/'gui-processes.json')
    require(Path(host['root']).resolve()==profile(token) and Path(host['gui']['executable']).resolve()==(ROOT.parent/'build/phase22g/lucille-full-validation.exe').resolve(),'Host ownership mismatch')
    return {k:host['gui'][k] for k in ('pid','created')}


def gui_stop(token):
    manifest=QA_ROOT/'gui-processes.json'
    if manifest.exists() and Path(read(manifest)['root']).resolve()==profile(token):
        subprocess.run(['pwsh','-NoProfile','-File','scripts/stop-lucille-real-gui.ps1','-Phase','22f'],cwd=ROOT,check=True,timeout=50)
    deadline=time.monotonic()+10
    while any(port_open(n) for n in (15283,15287)) and time.monotonic()<deadline: time.sleep(.25)
    require(not any(port_open(n) for n in (15283,15287)),'Uncontrolled QA GUI process')


def smoke():
    require(not any(port_open(n) for n in PORTS),'Smoke requires empty QA ports')
    p=profile('smoke') if profile('smoke').exists() else new_profile('smoke')
    require((p/'.lucille-isolated').read_bytes()==MARKER,'Smoke marker')
    run=SimpleNamespace(root=QA_ROOT,reports=REPORTS); control=REPORTS/'control.json'
    hosts=[]
    with controller_lock(run):
        previous=control.read_bytes()
        try:
            for _ in range(2):
                control.write_text('{}',encoding='utf8')
                try:
                    hosts.append(gui_start('smoke'))
                    label='i-smoke-'+uuid.uuid4().hex[:10]
                    send_control(run,label,{'inspect':True})
                    path=REPORTS/('driver-'+label+'-inspection.json')
                    deadline=time.monotonic()+45
                    while not path.exists() and time.monotonic()<deadline: time.sleep(.25)
                    result=read(path)
                    require(not result['conversations'] and not result['generation_active'],'Unexpected smoke conversation')
                finally: gui_stop('smoke')
        finally: control.write_bytes(previous)
    require(hosts[0]!=hosts[1] and not any(port_open(n) for n in PORTS),'Shutdown/restart failed')
    return {'real_hosts':hosts,'dispatched_user_turns':0,'broker_available':False,'final_ports_closed':list(PORTS)}


def campaign_guard():
    require(not ABORT.exists(), 'Supervisor or budget stop')
    require(module('scripts/lucille-real-server.py').resources()['PhysicalAvailable'] >= 1024**3, 'RAM floor')
    for name, port in [('gemma',15288),('nomic',15289),('broker',15284)]:
        require(port_open(port) and not (QA_ROOT/(name+'.stop')).exists(), 'Lost '+name)
        if name != 'broker':
            manifest=read(QA_ROOT/(name+'-process.json'))
            require(manifest.get('ready') and 'exit_code' not in manifest, 'Supervisor exited: '+name)
            heartbeat=REPORTS/(name+'-resources.ndjson')
            require(heartbeat.exists() and time.time()-heartbeat.stat().st_mtime<25,'Supervisor heartbeat lost: '+name)


def lifecycle(operation):
    subprocess.run([sys.executable, 'scripts/run-lucille-phase22f-owned.py','--root',str(QA_ROOT),
                    '--operation',operation], cwd=ROOT, check=True, timeout=620,
                   env={**os.environ,'LUCILLE_G22_BUDGET':'independent'})


def dispatch(run, i, sources, replay, initialize):
    campaign_guard()
    require(not BUDGET.exists() or len(list(BUDGET.iterdir()))<160,'Budget fully reserved')
    label='g22ie-'+i['id']+('-replay' if replay else '')
    require(not list(REPORTS.glob('driver-'+label+'-*.json')), 'Stage previously exposed')
    send_control(run,label,control_for(i,sources,replay,initialize))
    deadline=time.monotonic()+480
    while time.monotonic()<deadline:
        campaign_guard()
        failed=list(REPORTS.glob('driver-'+label+'-failure-*.json'))
        require(not failed,'Driver failure: '+str(failed))
        target=REPORTS/('driver-'+label+'-final.json')
        if target.exists():
            return read(target)
        time.sleep(1)
    raise RuntimeError('Driver timeout: '+label)


def audit_prompt(i, calls, database):
    records={r['id']:r for r in database['memories']}
    generation=False
    for call in calls:
        if call['kind']!='gemma':
            continue
        messages=call['request']['messages']
        if any('LUCILLE_ATOMIC_EXTRACTION_V' in str(m.get('content')) for m in messages if m['role']=='system'):
            continue
        for message in messages:
            content=message.get('content')
            if not isinstance(content,str) or not content.startswith('Retrieved contextual data,'):
                continue
            for ref in json.loads(content.splitlines()[1]):
                canonical=records.get(ref['id'])
                require(canonical and all(canonical[k]==i[k] for k in ('scope','project')), 'Cross-bound retrieval')
        if not generation:
            users=[m for m in messages if m['role']=='user']
            require(not any(m['role'] in ('assistant','tool') for m in messages),'Earlier conversation in first request')
            require(any(i['text'] in str(m['content']) for m in users),'Missing measured query')
            require(all(i['text'] in str(m['content']) or str(m['content']).startswith('Retrieved contextual data,') for m in users),'Earlier USER message')
        generation=True
    return generation


def adjudicate(step,decision,note):
    require(re.fullmatch('i[1-5]-[0-9]{2}',step) and len(note.strip())>=3,'Invalid review')
    require((RUN/(step+'-observed.json')).exists() and not (RUN/'closure.json').exists(),'Step unavailable')
    write(RUN/(step+'-review.json'),dict(decision=decision,note=note))


def campaign(commit):
    contract=frozen(commit)
    inputs=read(SUITE/'inputs.json')
    validate(inputs,read(SUITE/'oracle.json'),read(SUITE/'config.json'))
    scope=SimpleNamespace(reports=REPORTS,root=QA_ROOT)
    seen={}; conversations=set(); hosts={}; current=None
    log=REPORTS/'real-broker-calls.ndjson'
    states={i['id']:'pending' for i in inputs}
    with controller_lock(scope):
        require(environment()==read(SUITE/'runtime-lock.json'),'Runtime lock mismatch')
        RUN.mkdir(exist_ok=False)
        write(RUN/'start.json',dict(freeze=commit,application=contract['application_commit'],states=states))
        control=REPORTS/'control.json'; previous=control.read_bytes()
        try:
            control.write_text('{}',encoding='utf8')
            lifecycle('start')
            for i in inputs:
                sid=i['id']; p=i['profile']; initialize=p not in hosts
                if current!=p or i['restart']:
                    if current:
                        gui_stop(current)
                        integrity()
                    control.write_text('{}',encoding='utf8')
                    current=p
                    if initialize: new_profile(p)
                    old=hosts.get(p)
                    hosts[p]=gui_start(p)
                    require(hosts[p]!=old,'Host did not restart')
                integrity()
                path=profile(p)
                before=({'uninitialized':True} if initialize and not (path/'lucille-phase1.sqlite3').exists() else db_state(path))
                if initialize: require(not before.get('memories') and not before.get('history'),'Contaminated initial database')
                write(RUN/(sid+'-before.json'),dict(host=hosts[p],database=before))
                offset=log.stat().st_size if log.exists() else 0
                states[sid]='interrupted'
                write(RUN/(sid+'-exposure.json'),dict(time=time.time(),offset=offset))
                try:
                    if 'replay_source' in i:
                        replay=dispatch(scope,i,seen,True,False)
                        write(RUN/(sid+'-replay.json'),replay)
                        require(all(x['canonical_unchanged'] and x['tombstones_unchanged'] for x in replay['replays']) and len(replay['replays'])==1,'Replay changed memory')
                    fixture=dispatch(scope,i,seen,False,initialize)
                finally:
                    if log.exists():
                        with log.open('rb') as stream:
                            stream.seek(offset)
                            (RUN/(sid+'-broker.ndjson')).write_bytes(stream.read())
                require(len(fixture['turns'])==1,'Wrong number of turns')
                turn=fixture['turns'][0]
                require(turn['text']==i['text'] and turn['mode']==i['mode'] and turn['conversation_id'] not in conversations,'Conversation contamination')
                conversations.add(turn['conversation_id'])
                write(RUN/(sid+'-observed.json'),dict(driver=fixture,database=db_state(path)))
                trace=RUN/(sid+'-broker.ndjson')
                calls=[json.loads(line) for line in trace.read_text(encoding='utf8').splitlines()] if trace.exists() else []
                audit_prompt(i,calls,read(RUN/(sid+'-observed.json'))['database'])
                states[sid]='observed'
                print('Review evidence and oracle, then adjudicate '+sid,flush=True)
                gate=RUN/(sid+'-review.json')
                while not gate.exists():
                    campaign_guard()
                    time.sleep(1)
                review=read(gate)
                require(review['decision'] in ('ok','functional','critical'),'Invalid adjudication')
                states[sid]=review['decision']
                require(review['decision']!='critical','Critical observation: '+sid)
                seen[sid]=i
        except BaseException as exc:
            write(RUN/'interruption.json',dict(reason=str(exc),states=states))
            raise
        finally:
            try:
                if current: gui_stop(current)
            finally:
                try: lifecycle('stop')
                finally:
                    control.write_bytes(previous)
                    write(RUN/'closure.json',dict(states=states,tickets=len(list(BUDGET.glob('*')))))
                    integrity()
                    frozen(commit)
                    require(not any(port_open(n) for n in PORTS),'Uncontrolled QA port')


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    subs=parser.add_subparsers(dest='action',required=True)
    for name in ('check','run'):
        subs.add_parser(name).add_argument('--freeze',required=True)
    subs.add_parser('smoke')
    review=subs.add_parser('adjudicate')
    review.add_argument('step')
    review.add_argument('decision',choices=('ok','functional','critical'))
    review.add_argument('note')
    args=parser.parse_args()
    if args.action=='smoke': print(json.dumps(smoke()))
    elif args.action=='adjudicate': adjudicate(args.step,args.decision,args.note)
    elif args.action=='run': campaign(args.freeze)
    else:
        frozen(args.freeze)
        validate(read(SUITE/'inputs.json'),read(SUITE/'oracle.json'),read(SUITE/'config.json'))
        require(environment()==read(SUITE/'runtime-lock.json'),'Local runtime differs')
        print('Freeze, runtime, 104 protected files and 68 scheduled steps verified offline.')


if __name__=='__main__':
    main()
