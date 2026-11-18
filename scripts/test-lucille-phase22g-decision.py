"""Directed decision checkpoint. A fresh marked profile, native reviews and a shared four-call broker."""
import json
import os
import sqlite3
import subprocess
import sys
from pathlib import Path
from types import SimpleNamespace

ROOT = Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT / 'qa'))
from phase22f_evaluation import QA_ROOT, controller_lock, module, read, send_control

EVIDENCE = ROOT / 'docs/evidence/phase22g'
OUT = EVIDENCE / 'decision-update'
PROFILE = QA_ROOT.parent / 'lucille-synthetic-full22g-retest-decision'
STORE = QA_ROOT / 'g22decision' / 'lucille-synthetic-c2-preparation'
BRIDGE = ROOT.parent / 'build/phase22g/real_validation_bridge.exe'
REPORTS = QA_ROOT / 'reports'
SPEC = read(EVIDENCE / 'continuity/protocol.json')['scenarios'][0]
BINDING = {key: SPEC[key] for key in ('project', 'scope')}
native = module('scripts/test-lucille-real-extraction.py').bridge
gui = module('scripts/test-lucille-phase22g-continuity.py')


def save(name, data):
    (OUT / (name + '.json')).write_text(json.dumps(data, ensure_ascii=False, indent=2) + '\n', encoding='utf8')


def readonly(db):
    return sqlite3.connect(db.resolve().as_uri() + '?mode=ro', uri=True)


def snapshot():
    with readonly(PROFILE / 'lucille-phase1.sqlite3') as conn:
        conn.row_factory = sqlite3.Row
        assert conn.execute('pragma integrity_check').fetchone()[0] == 'ok'
        assert not conn.execute('pragma foreign_key_check').fetchall()
        return {table: [dict(row) for row in conn.execute('select * from ' + table)] for table in
                ('memories','memory_quality','memory_provenance','history','memory_revisions',
                 'memory_source_operations','memory_source_operation_records')}


def prepare():
    assert not PROFILE.exists() and not STORE.exists()
    OUT.mkdir(exist_ok=True)
    initial = SPEC['turns'][0]['text']
    prior = read(EVIDENCE / 'persistence-retest/persist-alfenim-seed.json')
    nomination = read(EVIDENCE / 'persistence-retest/broker.json')[1]
    assert json.loads(nomination['request']['messages'][-1]['content'])['user_text'] == initial
    native(BRIDGE, STORE, dict(operation='configure_identity', expected_version=0, profile=prior['identity']['profile'], **BINDING))
    job = native(BRIDGE, STORE, dict(operation='learn', mode='llm', autonomy=True, automatic_indexing=False,
                 conversation_id='offline-seed', message_id='initial-choice', text=initial, **BINDING,
                 output=nomination['response']['choices'][0]['message']['content']))
    for detail in job['details']:
        m = detail['memory']
        assert m['state'] == 'candidate'
        native(BRIDGE, STORE, dict(operation='change', id=m['input']['id'], version=m['version'],
                                   action={'action':'approve'}, **BINDING))
    PROFILE.mkdir()
    (PROFILE / '.lucille-isolated').write_bytes(b'synthetic-data-only\n')
    for folder in ('platform/roaming','platform/local','tmp','home'):
        (PROFILE / folder).mkdir(parents=True)
    with readonly(STORE / 'synthetic.sqlite3') as src, sqlite3.connect(PROFILE / 'lucille-phase1.sqlite3') as dst:
        src.backup(dst)
    initial_db = snapshot()
    assert len(initial_db['memories']) == 2 and all(m['state']=='active' for m in initial_db['memories'])
    assert {q['property'] for q in initial_db['memory_quality']} == {'description','database'}
    assert any(q['property']=='database' and q['value']=='sqlite' for q in initial_db['memory_quality'])
    save('initial', dict(source=initial, method='Archived first extraction replayed into a new store, versioned native approvals', db=initial_db))


def live(stage):
    assert os.environ.get('LUCILLE_G22_BUDGET') == 'decision'
    label = 'decision-alfenim-' + stage
    control = REPORTS / 'control.json'
    run = SimpleNamespace(root=QA_ROOT, reports=REPORTS)
    turn = SPEC['turns'][2 if stage=='update' else 3]
    with controller_lock(run):
        assert not list(REPORTS.glob('driver-' + label + '-*'))
        saved = control.read_bytes()
        try:
            control.write_text('{}', encoding='utf8')
            subprocess.run(['pwsh','-NoProfile','-File','scripts/start-lucille-real-gui.ps1','-Phase','22f',
                '-Checkpoint22g','-RetestProfile','decision','-StaticFrontend','-QaExe',
                '../build/phase22g/lucille-full-validation.exe'], cwd=ROOT, check=True, timeout=90)
            host = read(QA_ROOT / 'gui-processes.json')
            assert Path(host['root']).resolve() == PROFILE
            send_control(run,label,dict(resume=True,observe_projection=True,observe_canonical=True,**BINDING,
                         plan=dict(id=label,project=SPEC['project'],turns=[turn],probes=[])))
            result = gui.terminal(label)
            result['observed_host'] = {key:host['gui'][key] for key in ('pid','created')}
            save(stage, result)
            assert result.get('execution_complete'), result.get('error')
            db = snapshot()
            if stage=='update':
                save('after-update', db)
                tracks = {q['value']:q for q in db['memory_quality'] if q['property']=='database'}
                records = {m['id']:m for m in db['memories']}
                assert len(records)==3 and set(tracks)=={'sqlite','postgresql'}
                assert records[tracks['sqlite']['memory_id']]['state']=='obsolete'
                assert records[tracks['postgresql']['memory_id']]['state']=='active'
                assert tracks['postgresql']['replaces_id']==tracks['sqlite']['memory_id']
            print(stage, result['turns'][0]['messages'][-1]['content'], flush=True)
        finally:
            try: gui.gui(False)
            finally: control.write_bytes(saved)


if __name__ == '__main__':
    stage = sys.argv[1]
    if stage=='prepare': prepare()
    elif stage in ('update','history'): live(stage)
    else: raise ValueError(stage)
