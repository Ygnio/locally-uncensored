"""One real Gemma V3 response per frozen source, unchanged native modes, risk scored separately."""
import argparse,hashlib,importlib.util,json,re,time,urllib.request,sys
sys.dont_write_bytecode=True
from pathlib import Path
spec=importlib.util.spec_from_file_location('previous',Path(__file__).with_name('test-lucille-real-extraction.py'))
h=importlib.util.module_from_spec(spec);spec.loader.exec_module(h)
def verify_freeze(a,repo):
 if not a.freeze:return
 manifest=json.loads(Path(a.freeze).read_text(encoding="utf-8"))
 for path,digest in manifest["source_sha256_lf"].items():assert hashlib.sha256((repo/path).read_bytes().replace(b"\r\n",b"\n")).hexdigest()==digest,path
 for path,digest in manifest["corpus_sha256"].items():assert hashlib.sha256((repo/path).read_bytes()).hexdigest()==digest,path
 assert hashlib.sha256(Path(a.bridge).read_bytes()).hexdigest()==manifest["bridge_sha256"]
def main(a):
 repo=Path(__file__).resolve().parents[1];root=Path(a.root).resolve(strict=True)
 assert (root/'.lucille-isolated').read_bytes()==b'synthetic-data-only\n'
 verify_freeze(a,repo)
 if a.label.startswith('final'):assert a.freeze,'Final requires freeze'
 dataset_path=Path(a.dataset);data=json.loads(dataset_path.read_text(encoding='utf-8'))
 digest=hashlib.sha256(dataset_path.read_bytes()).hexdigest()
 prompt=re.search(r'CONVERSATION_EXTRACTION_PROMPT = `([^`]+)`',(repo/'src/lib/lucille-atomic.ts').read_text(encoding='utf-8')).group(1)
 progress=root/'reports'/f'{a.label}-extraction-progress.ndjson';out=root/'reports'/f'{a.label}-extraction.json'
 assert not out.exists(),'Never overwrite completed results'
 prior=[json.loads(s) for s in progress.read_text(encoding='utf-8').splitlines()] if progress.exists() else []
 modes={m:{'tp':0,'fp':0,'missed':0,'active_correct':0,'active_wrong':0,'critical_writes':0,'scope_errors':0,'by_category':{}} for m in ['rules','llm','hybrid']}
 for i,case in enumerate(data['cases']):
  if i<len(prior):
   c=prior[i];assert c['id']==case['id'] and c['dataset_sha256']==digest
  else:
   assert not (root/'gemma.stop').exists(),'Owned model stop requested'
   scope=case.get('scope','eval22');project=case.get('project','cedro')
   payload={'model':'lucille-real-gemma','temperature':0,'max_tokens':768,'messages':[{'role':'system','content':prompt},{'role':'user','content':json.dumps({'scope':scope,'project':project,'user_text':case['text']},ensure_ascii=False)}]}
   start=time.monotonic();req=urllib.request.Request('http://127.0.0.1:15284/v1/chat/completions',data=json.dumps(payload).encode(),headers={'Content-Type':'application/json'})
   with urllib.request.urlopen(req,timeout=120) as res:reply=json.load(res)
   output=reply['choices'][0]['message']['content'];raw=json.loads(output)
   c={'id':case['id'],'dataset_sha256':digest,'response':reply,'latency_seconds':time.monotonic()-start,'raw_claims':raw['claims'],'native_modes':{}}
   for mode in modes:
    c['native_modes'][mode]=h.bridge(Path(a.bridge).resolve(),root/a.label/('lucille-synthetic-'+mode+'-'+case['id']),{'operation':'learn','mode':mode,'autonomy':True,'automatic_indexing':True,'text':case['text'],'scope':scope,'project':project,'message_id':case['id'],'output':output})
   with progress.open('a',encoding='utf-8') as f:f.write(json.dumps(c,ensure_ascii=False)+'\n')
  for mode,stats in modes.items():
   details=c['native_modes'][mode]['details'];tp,fp,miss=h.score(details,case['expected'])
   for k,v in [('tp',tp),('fp',fp),('missed',miss)]:stats[k]+=v
   b=stats['by_category'].setdefault(case['category'],{'tp':0,'fp':0,'missed':0,'active_correct':0,'active_wrong':0,'critical_writes':0,'n':0});b['n']+=1
   for k,v in [('tp',tp),('fp',fp),('missed',miss)]:b[k]+=v
   active=[d for d in details if d['memory']['state']=='active'];atp,afp,_=h.score(active,case['expected'] if case.get('risk')=='A' else [])
   for k,v in [('active_correct',atp),('active_wrong',afp),('critical_writes',len(details) if case.get('critical') else 0)]:stats[k]+=v;b[k]+=v
   stats['scope_errors']+=sum(d['memory']['input']['scope']!=case.get('scope','eval22') or d['memory']['input']['project']!=case.get('project','cedro') for d in details)
  print(i+1,case['id'],[(m,h.score(c['native_modes'][m]['details'],case['expected'])) for m in modes],flush=True)
 verify_freeze(a,repo)
 for stats in modes.values():
  stats['precision']=stats['tp']/max(1,stats['tp']+stats['fp']);stats['recall']=stats['tp']/max(1,stats['tp']+stats['missed']);stats['promotion_precision']=stats['active_correct']/max(1,stats['active_correct']+stats['active_wrong'])
  stats['passed']=stats['precision']>=.95 and stats['recall']>=.8 and stats['promotion_precision']>=.99 and not any(stats[k] for k in ['active_wrong','critical_writes','scope_errors'])
 out.write_bytes((json.dumps({'dataset_sha256':digest,'cases':len(data['cases']),'modes':modes,'raw_proposals':sum(len(c['raw_claims']) for c in ([json.loads(s) for s in progress.read_text(encoding='utf-8').splitlines()]))},indent=2,ensure_ascii=False)+'\n').encode())
 print(json.dumps(modes,indent=2))
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--root',required=True);p.add_argument('--bridge',required=True);p.add_argument('--dataset',required=True);p.add_argument('--label',required=True);p.add_argument('--freeze');main(p.parse_args())
