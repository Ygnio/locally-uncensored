"""Recheck the prepared C2 stores with the exact cached query vectors, offline."""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
sys.path.insert(0, str(ROOT / 'qa'))
from phase22f_evaluation import module, QA_ROOT, read

out = ROOT / 'docs/evidence/phase22g/consolidation'
data = read(out / 'c2-native.json')
native = module('scripts/test-lucille-real-extraction.py').bridge
evaluation = module('scripts/test-lucille-phase22f-retrieval.py')
bridge = ROOT.parent / 'build/phase22g/real_validation_bridge.exe'
rows = []
for q in data['queries']:
    store = QA_ROOT / data['profile'] / ('lucille-synthetic-c2-' + q['scenario'])
    payload = read(store / (q['id'] + '.json'))
    labels = {p['id']: dict(input=dict(id=p['bound_records'][0]['id'])) for p in data['preparations']
              if p['scenario'] == q['scenario'] and p.get('bound_records')}
    row = dict(id=q['id'], query=q, modes={})
    for mode in ('fallback', 'hybrid'):
        vector = payload['query_vector'] if mode == 'hybrid' else None
        native(bridge, store, dict(operation='policy', lexical_weight=.5, minimum_similarity=.6))
        trace = native(bridge, store, dict(operation='explain',request=payload['request'],query_vector=vector))
        result = evaluation.assess(trace, {**payload['request'], **q}, labels)
        del result['trace']
        row['modes'][mode] = result
    context = native(bridge,store,dict(operation='context', **payload))
    row['references'] = context['references']
    row['facts'] = context['facts']
    rows.append(row)
summary = {mode:evaluation.summarize(rows,mode) for mode in ('fallback','hybrid')}
(out/'c2-retrieval-final.json').write_text(json.dumps(dict(summary=summary,rows=[
    dict(id=row['id'],modes=row['modes'],references=row['references'],facts=row['facts']) for row in rows]),ensure_ascii=False,indent=2)+'\n',encoding='utf8')
print(summary)
