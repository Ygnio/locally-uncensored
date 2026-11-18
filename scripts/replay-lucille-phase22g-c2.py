"""Rehydrate C2 from the frozen USER inputs and exact archived extractor responses."""
import argparse
import json
import sys
import urllib.request
from pathlib import Path
from types import SimpleNamespace

ROOT=Path(__file__).resolve().parents[1]
sys.dont_write_bytecode=True
sys.path.insert(0,str(ROOT/'qa'))
from phase22f_evaluation import module, read, QA_ROOT
FIX=ROOT/'crates/lucille-memory/tests/fixtures/phase22f/final-v1'
OUT=ROOT/'docs/evidence/phase22g/consolidation'
BRIDGE=ROOT.parent/'build/phase22g/real_validation_bridge.exe'


def http(path,payload=None):
    body=json.dumps(payload).encode() if payload is not None else None
    req=urllib.request.Request('http://127.0.0.1:15284'+path,data=body,headers={'Content-Type':'application/json'})
    with urllib.request.urlopen(req,timeout=100) as response:
        return json.load(response)


def main(label):
    root=QA_ROOT/label
    root.mkdir(exist_ok=False)
    freeze=read(FIX/'freeze.json')
    spec=read(FIX/'retrieval-c2.json')
    archived=[json.loads(l) for l in (QA_ROOT/'reports/final22f-c2-c2-progress.ndjson').read_text(encoding='utf8').splitlines()]
    nominations={json.dumps(r['generation']['request'],sort_keys=True):r['generation']['response'] for r in archived if 'generation' in r}
    ev=module('scripts/test-lucille-phase22f-retrieval.py')
    bridge=module('scripts/test-lucille-real-extraction.py').bridge
    pin=http('/lucille/embedding-manifest')
    assert all(pin[k]==v for k,v in freeze['model_pin'].items())
    space=pin['space']
    cached={}
    # Reuse only identical prefixed input from the sealed evaluation epoch.
    start=min(r['generation']['response']['created'] for r in archived if 'generation' in r)
    for line in (QA_ROOT/'reports/real-broker-calls.ndjson').open(encoding='utf8'):
        r=json.loads(line)
        if r['kind']=='nomic' and r['completed_unix']>=start:
            cached[r['request']['input']]=r['response']['data'][0]['embedding']
    fresh=[]
    def encode(text,document):
        key=space['document_prefix' if document else 'query_prefix']+text
        if key not in cached:
            cached[key]=http('/v1/embeddings',dict(model=space['model'],input=key,encoding_format='float'))['data'][0]['embedding']
            fresh.append(key)
        return cached[key]
    def replay(endpoint,body):
        assert endpoint=='/v1/chat/completions'
        return nominations[json.dumps(body,sort_keys=True)]
    # Intercept only generation. Selection, index, review and native snapshots are real.
    def capture(run,store,request,vector):
        context=bridge(BRIDGE,store,dict(operation='context',request=request,query_vector=vector))
        stamp=dict(request=request,query_vector=vector)
        (store/(request['message_id']+'.json')).write_text(json.dumps(stamp),encoding='utf8')
        return dict(status='pending',facts=context['facts'],references=context['references'])
    ev.answer=capture
    run=SimpleNamespace(root=QA_ROOT,label=label,bridge_path=BRIDGE,freeze=freeze,guard=lambda *k:None,api=replay,add=lambda row:None)
    result=dict(profile=label,preparations=[],indexes=[],queries=[],new_embeddings=fresh)
    def record(row):
        if row['kind']=='index':
            result['indexes'].append(row)
    run.add=record
    for s in spec['scenarios']:
        prep=[]
        rows=[]
        ev.execute_scenario(run,s,'c2',bridge,space,encode,['fallback','hybrid'],prep,rows,None)
        for p in prep:
            event=p.get('event',{})
            entry=dict(id=p['label'],scenario=s['id'],status=p['status'],stage=p.get('stage'),action=event.get('action'),disposition=p.get('disposition'))
            if 'error' in p: entry['error']=p['error']
            if 'native' in p:
                job=p['native']['enqueue'].get('id')
                entry.update(source=event['text'],job=job,proposal=p['generation']['response']['choices'][0]['message']['content'],bound_records=[
                    dict(id=d['memory']['input']['id'],state=d['memory']['state'],value=d['quality']['value'])
                    for d in p['native']['details'] if any(v['interaction_id']==job for v in d['provenance'])])
            result['preparations'].append(entry)
        for r in rows:
            q=r['query']
            result['queries'].append(dict(scenario=s['id'],id=q['id'],query=q['query'],required=q['requires'],expected=q['expected'],forbidden=q.get('forbidden',[]),
                modes={k:{key:v for key,v in value.items() if key!='trace'} for k,value in r['modes'].items()},context=r.get('answer')))
    (OUT/'c2-native.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf8')


if __name__=='__main__':
    p=argparse.ArgumentParser()
    p.add_argument('label',type=lambda s:s if s.isalnum() and s.startswith('g22') else p.error('isolated g22 label required'))
    main(p.parse_args().label)
