"""Compare frozen same-data FTS5/real Nomic/hybrid through canonical SQLite."""
import argparse
import hashlib
import importlib.util
import json
import math
from pathlib import Path
import time
import sqlite3
import subprocess
import sys
import urllib.request
import uuid

sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location('extraction', Path(__file__).with_name('test-lucille-real-extraction.py'))
helpers = importlib.util.module_from_spec(spec)
spec.loader.exec_module(helpers)


def interval(k, n):
    z = 1.96
    p = k / max(1, n)
    center = (p + z*z/(2*n))/(1+z*z/n)
    radius = z*math.sqrt(p*(1-p)/n+z*z/(4*n*n))/(1+z*z/n)
    return [max(0, center-radius), min(1, center+radius)]


def lifecycle(args):
    # Reuse already measured REAL vectors, without reloading either model or
    # rerunning/altering the sealed retrieval scores. Canonical Store alone writes.
    root = Path(args.root).resolve(strict=True)
    assert (root / '.lucille-isolated').read_bytes() == b'synthetic-data-only\n'
    store = root / 'retrieval/lucille-synthetic-real-nomic'
    executable = Path(args.bridge).resolve(strict=True)
    native = lambda request: helpers.bridge(executable, store, request)
    calls = [json.loads(line) for line in (root / 'reports/real-broker-calls.ndjson').read_text(encoding='utf-8').splitlines()]
    original = next(c for c in calls if c['kind']=='nomic' and c['request']['input']=='search_document: I prefer unsweetened coffee.')
    vector = original['response']['data'][0]['embedding']
    space = json.loads((root / 'attested-contract.json').read_text(encoding='utf-8'))['models']['nomic']
    memory_id = str(uuid.uuid5(uuid.NAMESPACE_URL,'lucille-phase21c-concept-1'))
    memory = next(d['memory'] for d in native({'operation':'details'}) if d['memory']['input']['id']==memory_id)
    def vector_count(current=False):
        with sqlite3.connect((store/'synthetic.sqlite3').as_uri()+'?mode=ro',uri=True) as db:
            if current:
                return db.execute("SELECT count(*) FROM memory_vectors v JOIN memories m ON m.id=v.memory_id WHERE m.id=? AND m.version=v.memory_version AND m.state='active'",(memory_id,)).fetchone()[0]
            return db.execute('SELECT count(*) FROM memory_vectors WHERE memory_id=?',(memory_id,)).fetchone()[0]
    assert vector_count()==1
    revised = 'Fictional coffee preference revised: sugar added.'
    if memory['state']=='candidate' and memory['input']['content']==revised:
        # Resume an interrupted lifecycle check without changing any benchmark.
        edited, old_version = memory, memory['version']-1
    else:
        old_version=memory['version']
        edited=native({'operation':'change','id':memory_id,'version':old_version,'action':{'action':'update','content':revised,'fact_key':'concept-1'}})
    assert edited['state']=='candidate' and vector_count(current=True)==0
    approved = native({'operation':'change','id':memory_id,'version':edited['version'],'action':{'action':'approve'}})
    assert approved['state']=='active'
    rejected = False
    try:
        native({'operation':'put_vector','id':memory_id,'version':old_version,'space':space,'vector':vector})
    except subprocess.CalledProcessError as error:
        rejected = 'Stale memory version or embedding space' in error.stderr
    assert rejected and vector_count(current=True)==0
    native({'operation':'policy','lexical_weight':0.})
    context = native({'operation':'context','request':{'conversation_id':'vector-lifecycle','message_id':'after-edit','scope':'lucille','project':'atlas','mode':'chat','query':'coffee','token_budget':8192},'query_vector':{'space':space,'vector':vector}})
    assert not any(m['input']['id']==memory_id for m in context['memories'])
    retained_before_delete=vector_count()
    native({'operation':'change','id':memory_id,'version':approved['version'],'action':{'action':'delete'}})
    assert vector_count()==0
    result={'passed':True,'real_previously_generated_vector':True,'live_inference_in_this_check':False,'edit_invalidated_vector_logically':True,'old_vector_rows_retained_but_ineligible_until_delete':retained_before_delete,'reapproval_did_not_restore_stale_vector':True,'old_version_write_rejected':True,'stale_vector_not_retrieved':True,'delete_physically_removed_vector':True,'only_canonical_store_wrote':True}
    (root/'reports/real-vector-lifecycle.json').write_text(json.dumps(result,indent=2),encoding='utf-8')
    print(json.dumps(result))


def main(args):
    repo = Path(__file__).resolve().parent.parent
    root = Path(args.root).resolve(strict=True)
    assert (root / '.lucille-isolated').read_bytes() == b'synthetic-data-only\n'
    out = root / 'reports/real-retrieval-final.json'
    if out.exists():
        raise RuntimeError('Frozen result already exists; refuse overwrite')
    fixture = repo / 'crates/lucille-memory/tests/fixtures/phase21c-real-retrieval.json'
    seal = json.loads(Path(args.manifest).read_text(encoding='utf-8'))
    assert hashlib.sha256(fixture.read_bytes()).hexdigest() == seal['sha256']
    dataset = json.loads(fixture.read_text(encoding='utf-8'))
    executable = Path(args.bridge).resolve(strict=True)
    store = root / 'retrieval/lucille-synthetic-real-nomic'
    def native(request):
        return helpers.bridge(executable, store, request)
    def fetch(path, data=None):
        request = urllib.request.Request('http://127.0.0.1:15284' + path,
                                        data=None if data is None else json.dumps(data).encode(),
                                        headers={'Content-Type': 'application/json'})
        with urllib.request.urlopen(request, timeout=120) as response:
            return json.load(response)
    contract = json.loads((root / 'attested-contract.json').read_text(encoding='utf-8'))
    space = contract['models']['nomic']
    manifest = fetch('/lucille/embedding-manifest')
    assert manifest['simulation'] is False and manifest['space'] == space
    times = []
    def encode(text, document):
        started = time.monotonic()
        response = fetch('/v1/embeddings', {'model': space['model'], 'input': ('search_document: ' if document else 'search_query: ') + text, 'encoding_format': 'float'})
        times.append(time.monotonic() - started)
        vector = response['data'][0]['embedding']
        assert len(vector) == 768 and all(math.isfinite(v) for v in vector)
        assert abs(math.sqrt(sum(v*v for v in vector)) - 1) < .001
        return vector
    native({'operation': 'learn', 'mode': 'rules', 'text': 'A fictional neutral initialization turn.', 'message_id': 'init'})
    native({'operation': 'configure_embedding', 'space': space})
    vectors = {}
    ids_by_concept = {x['id']: str(uuid.uuid5(uuid.NAMESPACE_URL, 'lucille-phase21c-' + x['id'])) for x in dataset['concepts']}
    concepts_by_id = {v:k for k,v in ids_by_concept.items()}
    for concept in dataset['concepts']:
        memory_id = ids_by_concept[concept['id']]
        candidate = {'id': memory_id, 'scope': 'lucille', 'project': 'atlas', 'kind': 'fact',
                     'content': concept['memory'], 'fact_key': concept['id'], 'source': 'manual', 'source_id': 'synthetic-retrieval-gold'}
        memory = native({'operation': 'create', 'candidate': candidate})
        memory = native({'operation': 'change', 'id': memory_id, 'version': memory['version'], 'action': {'action': 'approve'}})
        vector = encode(concept['memory'], True)
        native({'operation': 'put_vector', 'id': memory_id, 'version': memory['version'], 'space': space, 'vector': vector})
        vectors[concept['id']] = vector
    repeat = encode(dataset['concepts'][0]['memory'], True)
    max_delta = max(abs(a-b) for a,b in zip(repeat,vectors['concept-0']))
    assert max_delta < 1e-5
    queries = []
    for concept in dataset['concepts']:
        for field in ['exact_query', 'paraphrase_query', 'irrelevant_query']:
            queries.append({'query': concept[field], 'expected': None if field == 'irrelevant_query' else concept['id'], 'type': field, 'vector': encode(concept[field], False)})
    reports = {}
    query_times = []
    for mode, weight in [('fts5', 1.), ('semantic', 0.), ('hybrid', .7)]:
        native({'operation': 'policy', 'lexical_weight': weight})
        tp = fp = missed = 0
        results = []
        for i, query in enumerate(queries):
            req = {'conversation_id': 'real-retrieval', 'message_id': f'{mode}-{i}', 'scope': 'lucille', 'project': 'atlas', 'mode': 'chat', 'query': query['query'], 'token_budget': 8192}
            started = time.monotonic()
            context = native({'operation': 'context', 'request': req, 'query_vector': None if mode == 'fts5' else {'space': space, 'vector': query['vector']}})
            query_times.append(time.monotonic()-started)
            ids = [concepts_by_id[m['input']['id']] for m in context['memories']]
            tp += bool(query['expected'] and query['expected'] in ids)
            missed += bool(query['expected'] and query['expected'] not in ids)
            fp += sum(id != query['expected'] for id in ids)
            assert all(m['input']['scope'] == 'lucille' and m['input']['project'] == 'atlas' and m['state'] == 'active' for m in context['memories'])
            results.append({'query': query['query'], 'type': query['type'], 'expected': query['expected'], 'actual_ids': ids})
        precision, recall = tp/max(1,tp+fp), tp/16
        reports[mode] = {'tp': tp, 'fp': fp, 'missed': missed, 'precision': precision, 'recall': recall,
                         'precision_wilson95': interval(tp,tp+fp) if tp+fp else None,
                         'recall_wilson95': interval(tp,16), 'results': results,
                         'passed': precision >= .95 and recall >= .8 and fp == 0}
    # Same real vectors must never cross project boundaries.
    req['project'] = 'boreal'
    req['query'] = dataset['concepts'][0]['exact_query']
    assert not native({'operation': 'context', 'request': req, 'query_vector': {'space': space, 'vector': vectors['concept-0']}})['memories']
    # Reopening is intrinsic: every request opens a fresh Store/process.
    details = native({'operation': 'details'})
    target = next(d['memory'] for d in details if d['memory']['input']['id'] == ids_by_concept['concept-0'])
    native({'operation': 'change', 'id': ids_by_concept['concept-0'], 'version': target['version'], 'action': {'action': 'delete'}})
    req['project'] = 'atlas'
    deleted = native({'operation': 'context', 'request': req, 'query_vector': {'space': space, 'vector': vectors['concept-0']}})
    assert all(m['input']['id'] != ids_by_concept['concept-0'] for m in deleted['memories'])
    summary = {'real_nomic': True, 'simulation': False, 'dataset': seal, 'space': space,
               'repeat_max_component_delta': max_delta, 'repeat_tolerance': 1e-5,
               'modes': reports, 'scope_fenced': True, 'delete_filtered_after_reopen': True,
               'embedding_calls': len(times), 'embedding_latency_p50_s': sorted(times)[len(times)//2],
               'embedding_latency_p95_s': sorted(times)[int((len(times)-1)*.95)],
               'native_roundtrip_latency_p50_s': sorted(query_times)[len(query_times)//2],
               'native_roundtrip_latency_p95_s': sorted(query_times)[int((len(query_times)-1)*.95)]}
    out.write_text(json.dumps(summary, indent=2), encoding='utf-8')
    print(json.dumps({k:v for k,v in summary.items() if k != 'modes'},indent=2))
    for mode,r in reports.items():
        print(mode,{k:v for k,v in r.items() if k != 'results'})


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', required=True)
    parser.add_argument('--bridge', required=True)
    parser.add_argument('--manifest', required=True)
    parser.add_argument('--lifecycle-only', action='store_true')
    args = parser.parse_args()
    lifecycle(args) if args.lifecycle_only else main(args)
