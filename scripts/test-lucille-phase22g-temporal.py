"""Run a single mixed-time recall in a fresh Tauri profile with no conversation history."""
import os
import sqlite3
import subprocess
import sys
from pathlib import Path
from types import SimpleNamespace

ROOT = Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT / 'qa'))
from phase22f_evaluation import QA_ROOT, controller_lock, module, read, send_control, write

decision = module('scripts/test-lucille-phase22g-decision.py')
PROFILE = QA_ROOT.parent / 'lucille-synthetic-full22g-retest-temporal'
OUT = ROOT / 'docs/evidence/phase22g/temporal-claims'
LABEL = 'temporal-alfenim-history'


def prepare():
    if PROFILE.exists():
        raise FileExistsError(PROFILE)
    source = decision.PROFILE / 'lucille-phase1.sqlite3'
    assert (source.parent / '.lucille-isolated').read_bytes() == b'synthetic-data-only\n'
    PROFILE.mkdir()
    (PROFILE / '.lucille-isolated').write_bytes(b'synthetic-data-only\n')
    for folder in ('platform/roaming', 'platform/local', 'tmp', 'home'):
        (PROFILE / folder).mkdir(parents=True)
    with decision.readonly(source) as src, sqlite3.connect(PROFILE / 'lucille-phase1.sqlite3') as dst:
        src.backup(dst)
        rows = dst.execute("""select m.id,m.version,m.state,q.subject,q.property,q.value,q.replaces_id
            from memories m join memory_quality q on q.memory_id=m.id where q.property='database'
            and m.scope=? and m.project=?""", (decision.SPEC['scope'], decision.SPEC['project'])).fetchall()
        assert len(rows) == 2
        old, new = sorted(rows, key=lambda r: r[2]=='active')
        assert old[2:6] == ('obsolete','project.alfenim','database','sqlite')
        assert new[2:] == ('active','project.alfenim','database','postgresql',old[0])
        assert dst.execute('pragma integrity_check').fetchone()[0]=='ok' and not list(dst.execute('pragma foreign_key_check'))
        write(OUT/'preparation.json',dict(method='SQLite backup of reviewed synthetic decision store only; fresh WebView profile',
            fields=['id','version','state','subject','property','value','replaces_id'],rows=rows))


def live():
    assert os.environ.get('LUCILLE_G22_BUDGET') == 'temporal'
    run=SimpleNamespace(root=QA_ROOT,reports=decision.REPORTS)
    control=run.reports/'control.json'
    with controller_lock(run):
        assert not list(run.reports.glob('driver-'+LABEL+'-*'))
        previous=control.read_bytes()
        try:
            control.write_text('{}',encoding='utf8')
            subprocess.run(['pwsh','-NoProfile','-File','scripts/start-lucille-real-gui.ps1','-Phase','22f',
                '-Checkpoint22g','-RetestProfile','temporal','-StaticFrontend','-QaExe',
                '../build/phase22g/lucille-full-validation.exe'],cwd=ROOT,check=True,timeout=90)
            host=read(QA_ROOT/'gui-processes.json')
            assert Path(host['root']).resolve() == PROFILE
            turn=decision.SPEC['turns'][3]
            send_control(run,LABEL,dict(resume=True,observe_canonical=True,**decision.BINDING,
                plan=dict(id=LABEL,project=decision.SPEC['project'],turns=[turn],probes=[])))
            result=decision.gui.terminal(LABEL)
            result['observed_host']={key:host['gui'][key] for key in ('pid','created')}
            write(OUT/'gui.json',result)
            assert result.get('execution_complete') and not result['existing_bindings']
            print(result['turns'][0]['messages'][-1]['content'],flush=True)
        finally:
            try: decision.gui.gui(False)
            finally: control.write_bytes(previous)


if __name__=='__main__':
    if sys.argv[1]=='prepare': prepare()
    elif sys.argv[1]=='live': live()
    else: raise ValueError(sys.argv[1])
