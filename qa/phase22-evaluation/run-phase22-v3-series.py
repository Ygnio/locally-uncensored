"""External orchestration only; never changes frozen application or gold labels."""
import datetime,json,subprocess,sys,time,uuid,shutil
from pathlib import Path
repo=Path(__file__).resolve().parents[2]
root=repo/'.lucille-qa-runtime/lucille-synthetic-real22'; reports=root/'reports'
fixtures=repo/'crates/lucille-memory/tests/fixtures/phase22'
logs=repo.parent/'logs'; bridge=repo.parent/'build/memory/debug/examples/real_validation_bridge.exe'
pwsh=shutil.which('pwsh'); assert pwsh
def run(args,name):
 with (logs/name).open('wb') as out:subprocess.run(args,cwd=repo,stdout=out,stderr=subprocess.STDOUT,check=True)
def control(label,plan,project='candeia',resume=True,**extra):
 data={'id':str(uuid.uuid4()),'label':label,'scope':'phase22-long-heldout-v3','project':project,'resume':resume,'plan':plan,**extra}
 (reports/'control.json').write_text(json.dumps(data,ensure_ascii=False),encoding='utf-8')
 end=time.monotonic()+5400
 while time.monotonic()<end:
  failures=list(reports.glob('driver-'+label+'-failure-*.json'))
  if failures:raise RuntimeError('GUI stage failed: '+str(failures[-1]))
  final=reports/('driver-'+label+'-final.json')
  if final.exists():return final
  if (root/'gemma.stop').exists():raise RuntimeError('Owned model guard stopped')
  time.sleep(2)
 raise TimeoutError(label)
def score(final,label):
 run([sys.executable,'scripts/score-lucille-phase22-gui.py','--root',str(root),'--fixture',str(final),'--output',str(reports/(label+'-scored.json'))],label+'-score.log')
 result=json.loads((reports/(label+'-scored.json')).read_text(encoding='utf-8'))
 print(label,result['totals'],flush=True)
 if result['totals']['critical_references']:raise RuntimeError('Critical references; no later stage')
 return result['totals']
def start():run([pwsh,'-NoProfile','-File','scripts/start-lucille-real-gui.ps1','-Phase','22','-QaExe',str(repo.parent/'build/tauri/x86_64-pc-windows-msvc/debug/lucille-full-validation.exe')],'phase22-gui-start-v3.log')
def stop():run([pwsh,'-NoProfile','-File','scripts/stop-lucille-real-gui.ps1','-Phase','22'],'phase22-gui-stop-v3.log')
def main():
 while not (reports/'final-v3-extraction.json').exists():
  if (root/'gemma.stop').exists():raise RuntimeError('Model stopped before extraction completed')
  time.sleep(5)
 extraction=json.loads((reports/'final-v3-extraction.json').read_text())
 print('Extraction', {k:{x:v[x] for x in ['precision','recall','active_wrong','critical_writes']} for k,v in extraction['modes'].items()},flush=True)
 if any(v['critical_writes'] or v['active_wrong'] for v in extraction['modes'].values()):raise RuntimeError('Critical extraction/promotions: do not continue')
 evaluation=root/'evaluation-v3/lucille-synthetic-evaluation-v3';evaluation.mkdir(parents=True,exist_ok=False);(evaluation/'.lucille-isolated').write_bytes(b'synthetic-data-only\n');(evaluation/'reports').mkdir()
 run([sys.executable,'scripts/test-lucille-phase22-retrieval.py','--root',str(evaluation),'--bridge',str(bridge),'--freeze',str(fixtures/'freeze-v3.json')],'phase22-retrieval-final-v3.log')
 print('Retrieval complete',flush=True); start(); results={}
 for n in [20,50,100,200]:
  if n==200:
   sample=json.loads((reports/'system-resources-v2.ndjson').read_text().splitlines()[-1]);physical=sample['available_mib'];commit=sample['commit_limit']-sample['committed_bytes']
   if physical<1280 or commit<3*1024**3:
    results['200_skipped']={'reason':'Optional larger stage insufficient current headroom','measurement':sample};break
  plan=json.loads((fixtures/f'long-{n}-v3.json').read_text(encoding='utf-8'));label=f'heldout-v3-long{n}'
  final=control(label,plan,plan['project'],n!=20);results[str(n)]=score(final,label)
 (reports/'v3-long-series-summary.json').write_text(json.dumps(results,indent=2),encoding='utf-8')
 print('Long series completed; lifecycle remains separately orchestrated',flush=True)
if __name__=='__main__':main()
