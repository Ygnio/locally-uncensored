"""One frozen real V2 response per case, native rules/LLM/hybrid replay, no repair."""
import argparse
from collections import defaultdict
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import sys
import time
import urllib.request
sys.dont_write_bytecode=True
spec=importlib.util.spec_from_file_location('baseline',Path(__file__).with_name('test-lucille-real-extraction.py'))
h=importlib.util.module_from_spec(spec);spec.loader.exec_module(h)

def verify(repo,freeze):
    data=json.loads(Path(freeze).read_text(encoding='utf-8'))
    for path,digest in data['source_sha256_lf'].items():
        assert hashlib.sha256((repo/path).read_bytes().replace(b'\r\n',b'\n')).hexdigest()==digest,path
    for path,digest in data['corpus_sha256'].items():
        assert hashlib.sha256((repo/path).read_bytes()).hexdigest()==digest,path
    return data

def main(a):
    repo=Path(__file__).resolve().parents[1];root=Path(a.root).resolve(strict=True)
    assert (root/'.lucille-isolated').read_bytes()==b'synthetic-data-only\n'
    freeze=verify(repo,a.freeze);dataset=json.loads((repo/freeze['extraction_corpus']).read_text(encoding='utf-8'))
    out=root/'reports/quality-extraction-final.json';progress=root/'reports/quality-extraction-progress.ndjson'
    if out.exists():raise RuntimeError('Final evaluation already complete; never overwrite')
    prior=[json.loads(line) for line in progress.read_text(encoding='utf-8').splitlines()] if progress.exists() else []
    assert all(x['freeze_sha256']==hashlib.sha256(Path(a.freeze).read_bytes()).hexdigest() for x in prior)
    modes={m:{'tp':0,'fp':0,'missed':0,'active':0,'candidate_or_conflict':0,'critical_writes':0,'scope_errors':0,'identity_changes':0,'by_category':defaultdict(lambda:[0,0,0])} for m in ['rules','llm','hybrid']}
    prompt=re.search(r'ATOMIC_EXTRACTION_PROMPT = `([^`]+)`',(repo/'src/lib/lucille-atomic.ts').read_text(encoding='utf-8')).group(1)
    captures=[]
    for i,case in enumerate(dataset['cases']):
        if i<len(prior):
            capture=prior[i];assert capture['id']==case['id']
        else:
            if (root/'gemma.stop').exists():raise RuntimeError('Own stop requested')
            payload={'model':'lucille-real-gemma','temperature':0,'max_tokens':768,'messages':[{'role':'system','content':prompt},{'role':'user','content':json.dumps({'scope':case.get('scope','lucille'),'project':case.get('project','aurora'),'user_text':case['text']},ensure_ascii=False)}]}
            req=urllib.request.Request('http://127.0.0.1:15284/v1/chat/completions',data=json.dumps(payload).encode(),headers={'Content-Type':'application/json'})
            start=time.monotonic()
            with urllib.request.urlopen(req,timeout=120) as response:reply=json.load(response)
            output=reply['choices'][0]['message']['content'];raw=json.loads(output)
            capture={'id':case['id'],'text':case['text'],'response':reply,'latency_seconds':time.monotonic()-start,'raw_proposals':len(raw['claims']),'raw_complete_spans':[x['evidence'] for x in raw['claims']],'native_modes':{},'freeze_sha256':hashlib.sha256(Path(a.freeze).read_bytes()).hexdigest()}
            for mode in modes:
                capture['native_modes'][mode]=h.bridge(Path(a.bridge).resolve(),root/'final-extraction'/('lucille-synthetic-'+mode+'-'+case['id']),{'operation':'learn','mode':mode,'text':case['text'],'scope':case.get('scope','lucille'),'project':case.get('project','aurora'),'message_id':case['id'],'output':output})
            with progress.open('a',encoding='utf-8') as stream:stream.write(json.dumps(capture,ensure_ascii=False)+'\n')
        for mode,stats in modes.items():
            result=capture['native_modes'][mode];tp,fp,miss=h.score(result['details'],case['expected'])
            for field,v in [('tp',tp),('fp',fp),('missed',miss)]:stats[field]+=v
            for j,v in enumerate([tp,fp,miss]):stats['by_category'][case['category']][j]+=v
            stats['active']+=sum(d['memory']['state']=='active' for d in result['details'])
            stats['candidate_or_conflict']+=sum(d['memory']['state'] in ['candidate','conflict'] for d in result['details'])
            if case.get('critical'):stats['critical_writes']+=len(result['details'])
            stats['scope_errors']+=sum(d['memory']['input']['scope']!=case.get('scope','lucille') or d['memory']['input']['project']!=case.get('project','aurora') for d in result['details'])
            stats['identity_changes']+=int(result['identity']['version']!=1)
        captures.append(capture);print(i+1,case['id'],[(m,h.score(capture['native_modes'][m]['details'],case['expected'])) for m in modes],flush=True)
    for stats in modes.values():
        stats['precision']=stats['tp']/max(1,stats['tp']+stats['fp']);stats['recall']=stats['tp']/max(1,stats['tp']+stats['missed'])
        stats['passed']=stats['precision']>=.95 and stats['recall']>=.8 and all(stats[k]==0 for k in ['critical_writes','scope_errors','identity_changes'])
    verify(repo,a.freeze)
    report={'evaluation':'unseen frozen phase21d extraction','freeze':freeze,'modes':modes,'raw_proposals':sum(c['raw_proposals'] for c in captures),'captures':captures,'note':'One actual Gemma response replayed unchanged into native LLM/hybrid. Rules do not consume output. Extraction validation and active promotion are separate.'}
    out.write_text(json.dumps(report,indent=2,ensure_ascii=False),encoding='utf-8');print(json.dumps(modes,indent=2))
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--root',required=True);p.add_argument('--bridge',required=True);p.add_argument('--freeze',required=True);main(p.parse_args())
