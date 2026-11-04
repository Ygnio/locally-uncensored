"""Separate proposal diagnostics, canonical validation and promotion metrics."""
import importlib.util,json,statistics
from pathlib import Path
repo=Path(__file__).resolve().parents[2];reports=repo/'.lucille-qa-runtime/lucille-synthetic-real22/reports';fixtures=repo/'crates/lucille-memory/tests/fixtures/phase22'
spec=importlib.util.spec_from_file_location('scorer',repo/'scripts/test-lucille-real-extraction.py');h=importlib.util.module_from_spec(spec);spec.loader.exec_module(h)
output={}
for version in ['v1','v2','v3']:
 path=reports/f'final-{version}-extraction.json'
 if not path.exists():continue
 final=json.loads(path.read_text(encoding='utf-8'));dataset=json.loads((fixtures/f'extraction-{version}.json').read_text(encoding='utf-8'));rows=[json.loads(x) for x in (reports/f'final-{version}-extraction-progress.ndjson').read_text(encoding='utf-8').splitlines()]
 cats={};byrisk={};latencies=[]
 for row,case in zip(rows,dataset['cases']):
  assert row['id']==case['id'];cat=cats.setdefault(case['category'],{'sources':0,'gold_claims':0,'raw_proposals':0,'raw_proposal_categories':{},'sources_with_proposals_but_no_gold':0,'raw_category_divergences':0,'modes':{}})
  cat['sources']+=1;cat['gold_claims']+=len(case['expected']);cat['raw_proposals']+=len(row['raw_claims']);cat['sources_with_proposals_but_no_gold']+=int(not case['expected'] and bool(row['raw_claims']))
  expected={g['kind'] for g in case['expected']}
  for c in row['raw_claims']:
   kind=c['category'];cat['raw_proposal_categories'][kind]=cat['raw_proposal_categories'].get(kind,0)+1
   cat['raw_category_divergences']+=int(bool(expected) and kind not in expected)
  if not(version=='v3' and case['id']=='v3-035'):latencies.append(row['latency_seconds'])
  risk=byrisk.setdefault(case.get('risk','unspecified'),{'sources':0,'gold_claims':0,'modes':{}});risk['sources']+=1;risk['gold_claims']+=len(case['expected'])
  for mode in ['rules','llm','hybrid']:
   details=row['native_modes'][mode]['details'];tp,fp,miss=h.score(details,case['expected']);pending=[d for d in details if d['memory']['state'] in ['candidate','conflict']];ptp,pfp,_=h.score(pending,case['expected']);active=[d for d in details if d['memory']['state']=='active'];atp,afp,_=h.score(active,case['expected'] if case.get('risk')=='A' else [])
   for parent in [cat,risk]:
    m=parent['modes'].setdefault(mode,{'tp':0,'fp':0,'missed':0,'correct_pending':0,'incorrect_pending':0,'correct_active':0,'incorrect_active':0})
    for k,v in [('tp',tp),('fp',fp),('missed',miss),('correct_pending',ptp),('incorrect_pending',pfp),('correct_active',atp),('incorrect_active',afp)]:m[k]+=v
 output[version]={'sources':len(rows),'gold_claims':sum(len(c['expected']) for c in dataset['cases']),'overall':final['modes'],'by_category':cats,'by_risk':byrisk,'end_to_end_broker_latency_seconds':{'n':len(latencies),'median':statistics.median(latencies),'minimum':min(latencies),'maximum':max(latencies)},'notes':['Raw category divergence is a taxonomy diagnostic, not equivalent to a canonical false positive; grounded aliases may be permitted. Raw proposals without gold remain visible even when validation rejects them.','The final gate evaluates independently validated canonical claims; hybrid fallback does not change the reported LLM-only result.','No-claim denominators are undefined precision, not 0% correctness. Categories fewer than five sources support counts only.','V3 interrupted-case latency excluded because the same cached actual response was replayed. Original failure and recovery instrumentation are retained.']}
(reports/'v3-extraction-category-review.json').write_text(json.dumps(output,indent=2,ensure_ascii=False)+'\n',encoding='utf-8')
print(json.dumps({k:{'sources':v['sources'],'modes':{m:{x:s[x] for x in ['tp','fp','missed','precision','recall','active_correct','active_wrong','critical_writes']} for m,s in v['overall'].items()}} for k,v in output.items()},indent=2))
