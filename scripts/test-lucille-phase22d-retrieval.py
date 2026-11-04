"""Frozen 2.2D retrieval: actual Nomic index 5, canonical state and injected context."""
import argparse
from collections import defaultdict
import hashlib
import importlib.util
import json
import re
from pathlib import Path
import sys
import time
import urllib.request
sys.dont_write_bytecode=True
spec=importlib.util.spec_from_file_location('quality',Path(__file__).with_name('test-lucille-real-quality-extraction.py'))
h=importlib.util.module_from_spec(spec);spec.loader.exec_module(h)

def main(a):
    repo=Path(__file__).resolve().parents[1];root=Path(a.root).resolve(strict=True);freeze=h.verify(repo,a.freeze)
    assert root==repo/'.lucille-qa-runtime/lucille-synthetic-real22d'
    assert re.fullmatch(r'[a-z0-9-]{1,60}',a.label)
    assert hashlib.sha256(Path(a.bridge).read_bytes()).hexdigest()==freeze['bridge_sha256']
    assert json.loads((root/'reports/real-public-pin.json').read_text(encoding='utf-8'))==freeze['model_pin']
    assert (root/'.lucille-isolated').read_bytes()==b'synthetic-data-only\n'
    out=root/'reports'/(a.label+'-retrieval.json')
    if out.exists():raise RuntimeError('Final retrieval evaluation already exists; never overwrite')
    dataset=json.loads((repo/freeze['retrieval_corpus']).read_text(encoding='utf-8'))
    store=root/a.label/'lucille-synthetic-retrieval';exe=Path(a.bridge).resolve();vectors={};calls=[]
    def native(data):return h.h.bridge(exe,store,data)
    def api(path,data=None):
        req=urllib.request.Request('http://127.0.0.1:15284'+path,data=None if data is None else json.dumps(data).encode(),headers={'Content-Type':'application/json'})
        start=time.monotonic()
        with urllib.request.urlopen(req,timeout=120) as response:reply=json.load(response)
        calls.append({'path':path,'latency_seconds':time.monotonic()-start,'usage':reply.get('usage')});return reply
    space=api('/lucille/embedding-manifest')['space'];assert space['index_version']==5
    def encode(text,document):
        key=(space['document_prefix'] if document else space['query_prefix'])+text
        if key not in vectors:vectors[key]=api('/v1/embeddings',{'model':space['model'],'input':key,'encoding_format':'float'})['data'][0]['embedding']
        return vectors[key]
    labels={};seed_evidence=[]
    for item in dataset['events']:
        scope=item.get('scope','ret22');project=item.get('project','cedro')
        def scoped(op):return native({**op,'scope':scope,'project':project})
        if item['action']=='learn':
            before={d['memory']['input']['id'] for d in scoped({'operation':'details'})} if store.exists() else set()
            result=scoped({'operation':'learn','mode':'rules','autonomy':True,'automatic_indexing':True,'text':item['text'],'message_id':item['label']})
            new=[d['memory'] for d in result['details'] if d['memory']['input']['id'] not in before]
            assert len(new)==1,(item,result)
            m=new[0]
            if m['state']!='active' and item.get('approve',True):m=scoped({'operation':'change','id':m['input']['id'],'version':m['version'],'action':{'action':'approve'}})
            labels[item['label']]={'id':m['input']['id'],'scope':scope,'project':project}
            rep=None;vector=None
            if m['state']=='active':
                native({'operation':'configure_embedding','space':space})
                rep=scoped({'operation':'representation','id':m['input']['id'],'style':'gloss'})
                vector=encode(rep['text'],True);scoped({'operation':'put_vector','id':m['input']['id'],'version':m['version'],'space':space,'vector':vector})
            seed_evidence.append({'event':item,'memory':m,'representation':rep,'vector':vector})
        else:
            target=labels[item['target']];scope=target['scope'];project=target['project']
            m=next(d['memory'] for d in scoped({'operation':'details'}) if d['memory']['input']['id']==target['id'])
            scoped({'operation':'change','id':m['input']['id'],'version':m['version'],'action':{'action':item['action']}})
    reverse={x['id']:k for k,x in labels.items()};stats={mode:{'tp':0,'fp':0,'missed':0,'negative_injections':0,'unsafe_current':0,'scope_errors':0,'precision_at_3_sum':0,'recall_at_3_sum':0,'ranking_queries':0,'by_category':defaultdict(lambda:[0,0,0])} for mode in ['fts5','nomic','hybrid']};samples=[]
    for i,q in enumerate(dataset['queries']):
        query_vector=encode(q['query'],False);sample={'id':q['id'],'query':q,'modes':{}}
        for mode,lw in [('fts5',1.),('nomic',0.),('hybrid',.5)]:
            native({'operation':'policy','lexical_weight':lw,'minimum_similarity':.6})
            request={'conversation_id':'phase22d-frozen-final','message_id':q['id']+'-'+mode,'scope':q.get('scope','ret22'),'project':q.get('project','cedro'),'mode':q.get('mode','chat'),'query':q['query'],'token_budget':8192}
            start=time.monotonic();trace=native({'operation':'explain','request':request,'query_vector':None if mode=='fts5' else {'space':space,'vector':query_vector},'logical_offset_seconds':q.get('logical_offset_seconds',0)});latency=time.monotonic()-start
            actual=[reverse[m['input']['id']] for m in trace['context']['memories']];gold=q['expected'];tp=len(set(actual)&set(gold));fp=len(set(actual)-set(gold));miss=len(set(gold)-set(actual));s=stats[mode]
            for k,n in [('tp',tp),('fp',fp),('missed',miss)]:s[k]+=n
            for j,n in enumerate([tp,fp,miss]):s['by_category'][q['category']][j]+=n
            s['negative_injections']+=int(not gold and bool(actual));s['unsafe_current']+=sum(x in q.get('forbidden',[]) for x in actual)
            s['scope_errors']+=sum(labels[x]['scope']!=request['scope'] or labels[x]['project']!=request['project'] for x in actual)
            # Raw ranking is FTS BM25 for fts5 and Nomic cosine for nomic/hybrid diagnostics; actual injected context is scored separately.
            rows=trace['trace'];ranked=sorted([x for x in rows if x['state']=='active' and (x['lexical_rank_bm25'] is not None if mode=='fts5' else x['cosine'] is not None)],key=lambda x:x['lexical_rank_bm25'][0] if mode=='fts5' else -x['cosine'])
            rank=[reverse[x['id']] for x in ranked];hits=len(set(rank[:3])&set(gold))
            if gold:s['precision_at_3_sum']+=hits/3;s['recall_at_3_sum']+=hits/len(gold);s['ranking_queries']+=1
            sample['modes'][mode]={'actual':actual,'tp':tp,'fp':fp,'missed':miss,'native_latency_seconds':latency,'rank_before_threshold':rank,'rank_precision_at_3':hits/3,'trace':trace}
        samples.append(sample);print(i+1,q['id'],[(m,sample['modes'][m]['actual']) for m in stats],flush=True)
    for mode,s in stats.items():
        s['precision']=s['tp']/max(1,s['tp']+s['fp']);s['recall']=s['tp']/max(1,s['tp']+s['missed']);s['precision_at_3']=s['precision_at_3_sum']/max(1,s['ranking_queries']);s['recall_at_3']=s['recall_at_3_sum']/max(1,s['ranking_queries']);s['passed']=s['precision']>=.9 and s['recall']>=.8 and not s['negative_injections'] and not s['unsafe_current'] and not s['scope_errors']
    h.verify(repo,a.freeze)
    assert hashlib.sha256(Path(a.bridge).read_bytes()).hexdigest()==freeze['bridge_sha256']
    assert json.loads((root/'reports/real-public-pin.json').read_text(encoding='utf-8'))==freeze['model_pin']
    out.write_text(json.dumps({'evaluation':'unseen frozen actual Nomic phase22d','freeze':freeze,'space':space,'modes':stats,'seed_evidence':seed_evidence,'samples':samples,'calls':calls,'note':'Native V3 rules + scoped QA review of B seed retrieval independently of Gemma; eligible A can promote by proof. Hybrid pre-gate ranking fields are Nomic diagnostics, not a claimed reconstruction of the final fused selector. All vectors actual. Logical expiry uses Store.context_at, never OS clock. JSON byte/context ceiling is a conservative token upper bound, not tokenizer measurements.'},indent=2,ensure_ascii=False),encoding='utf-8')
    print(json.dumps(stats,indent=2))
if __name__=='__main__':
    p=argparse.ArgumentParser();p.add_argument('--root',required=True);p.add_argument('--bridge',required=True);p.add_argument('--freeze',required=True);p.add_argument('--label',required=True);main(p.parse_args())
