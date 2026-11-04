"""Bounded real fault attempts plus targeted fresh-session forget proof."""
import importlib.util,json,subprocess,sys,time,datetime
from pathlib import Path
here=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('series',here/'run-phase22-v3-series.py');s=importlib.util.module_from_spec(spec);spec.loader.exec_module(s)
repo=s.repo;reports=s.reports
while not (reports/'v3-lifecycle-review.json').exists():
 if (s.root/'gemma.stop').exists():raise RuntimeError('Own model stopped')
 time.sleep(2)
summary={'started':datetime.datetime.now(datetime.timezone.utc).isoformat(),'faults':{}}
for kind in ['cancel','restart']:
 attempts=[]
 for n in [1,2]:
  label=f'v3-{kind}-fault-{n}'
  with (s.logs/(label+'.log')).open('wb') as out:code=subprocess.run(['node',str(here/'phase22-v3-fault.mjs'),kind,str(n)],cwd=repo,stdout=out,stderr=subprocess.STDOUT).returncode
  path=reports/(label+'-fault.json');result=json.loads(path.read_text(encoding='utf-8')) if path.exists() else {'error':'Missing fault report'}
  attempts.append({'label':label,'exit_code':code,'actual_overlap':result.get('actual_overlap'),'error':result.get('error'),'old_callback_result':result.get('old_callback_result'),'old_callback_canonical_unchanged':result.get('old_callback_canonical_unchanged')})
  print(kind,attempts[-1],flush=True)
  if code==0 and result.get('actual_overlap') and result.get('old_callback_canonical_unchanged') and result.get('old_callback_result',{}).get('completed') is False:break
 summary['faults'][kind]=attempts
plan=json.loads((s.fixtures/'forget-supplement-v3.json').read_text(encoding='utf-8'));label='heldout-v3-forget-fresh';final=s.control(label,plan,scope=plan['scope'])
s.score(final,label);fixture=json.loads(final.read_text(encoding='utf-8'));inline=dict(fixture);inline['label']=label+'-inline';inline['probes']=[{'item':{**item,'text':fixture['turns'][item['turn_index']]['text']},'turn':fixture['turns'][item['turn_index']]} for item in plan['inline_probe_gold']]
adapter=reports/'v3-forget-inline-scoring-input.json';assert not adapter.exists();adapter.write_text(json.dumps(inline,indent=2,ensure_ascii=False),encoding='utf-8');summary['forget_inline_score']=s.score(adapter,label+'-inline');summary['finished']=datetime.datetime.now(datetime.timezone.utc).isoformat();(reports/'v3-postlife-review.json').write_text(json.dumps(summary,indent=2),encoding='utf-8');print('Post-lifecycle tests completed',flush=True)
