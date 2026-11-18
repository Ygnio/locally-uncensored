"""Run the frozen directed dialogues on the isolated Tauri validation host."""
import json
import os
import subprocess
import sys
import time
from pathlib import Path
from types import SimpleNamespace

ROOT = Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT / 'qa'))
from phase22f_evaluation import QA_ROOT, controller_lock, read, send_control

OUT = ROOT / 'docs/evidence/phase22g/continuity'
REPORTS = QA_ROOT / 'reports'


def gui(start):
    cmd = ['pwsh', '-NoProfile', '-File', 'scripts/'+('start' if start else 'stop')+'-lucille-real-gui.ps1','-Phase','22f']
    if start:
        cmd += ['-Checkpoint22g','-StaticFrontend','-QaExe','../build/phase22g/lucille-full-validation.exe']
    subprocess.run(cmd, cwd=ROOT,check=True,timeout=65)


def terminal(label):
    deadline=time.monotonic()+600
    while time.monotonic()<deadline:
        paths=list(REPORTS.glob('driver-'+label+'-failure-*.json'))+[REPORTS/('driver-'+label+'-final.json')]
        for p in paths:
            if p.exists():
                try:return read(p)
                except (ValueError, PermissionError):pass
        time.sleep(1)
    raise TimeoutError(label)


def main():
    if os.environ.get('LUCILLE_G22_BUDGET') != 'continuity':
        raise ValueError('Continuity budget required')
    spec=read(OUT/'protocol.json')
    run=SimpleNamespace(root=QA_ROOT,reports=REPORTS)
    control=REPORTS/'control.json'
    previous=control.read_bytes()
    with controller_lock(run):
        try:
            for scenario in spec['scenarios']:
                for stage,turns in [('conversation',scenario['turns'][:4]),('restart',scenario['turns'][4:])]:
                    label='g22c-'+scenario['project']+'-'+stage
                    if list(REPORTS.glob('driver-'+label+'-*')):raise FileExistsError(label)
                    control.write_text('{}',encoding='utf8')
                    try:
                        gui(True)
                        host=read(QA_ROOT/'gui-processes.json')
                        if Path(host['root']).name!=spec['profile']:raise ValueError('Wrong profile')
                        send_control(run,label,dict(resume=True,observe_canonical=True,scope=scenario['scope'],project=scenario['project'],
                            plan=dict(id=label,project=scenario['project'],turns=turns,probes=[])))
                        result=terminal(label)
                        result['observed_host']={k:host['gui'][k] for k in ('pid','created')}
                        (OUT/(label+'.json')).write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
                        print(label,len(result['turns']),result.get('error','completed'),flush=True)
                    finally:gui(False)
        finally:control.write_bytes(previous)


if __name__=='__main__':main()
