"""Offline replay using the original C1 evaluator, fixture and archived Gemma nominations."""
import argparse
import json
import sys
import tempfile
from pathlib import Path
from types import SimpleNamespace

ROOT = Path(__file__).resolve().parents[1]
sys.dont_write_bytecode = True
sys.path.insert(0,str(ROOT/'qa'))
from phase22f_evaluation import module, read, QA_ROOT

FIX = ROOT/'crates/lucille-memory/tests/fixtures/phase22f/final-v1'
OUT = ROOT/'docs/evidence/phase22g'


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('bridge',type=Path)
    parser.add_argument('output',type=Path)
    args=parser.parse_args()
    exe=args.bridge.resolve(strict=True)
    native=module('scripts/test-lucille-real-extraction.py').bridge
    evaluator=module('scripts/test-lucille-phase22f-retrieval.py')
    freeze=read(FIX/'freeze.json')
    results=dict(c1=[],learning=[])
    with tempfile.TemporaryDirectory(prefix='phase22g-',dir=QA_ROOT) as tmp:
        path=Path(tmp)
        run=SimpleNamespace(root=QA_ROOT,label=path.name,bridge_path=exe,freeze=freeze,add=lambda row:None)
        for s in read(FIX/'retrieval-c1.json')['scenarios']:
            rows=[]
            evaluator.execute_scenario(run,s,'c1',native,None,None,['fallback'],[],rows,None)
            for r in rows:
                obs=r['modes']['fallback']
                q=r['query']
                assert obs['status']=='measured', obs
                results['c1'].append(dict(id=q['id'],mandatory=q.get('lexical_mandatory',False),expected=q['expected'],
                    actual=obs['actual'],exact=not any(obs[k] for k in ['fp','missed','unsafe_current','scope_errors'])))
        nominations=read(OUT/'nominations.json')
        ram=next(e for e in nominations if e['id']=='fv1-main-t15')
        nominations.append(dict(id='ram-hybrid-control',request=dict(ram['request'],mode='hybrid')))
        for event in nominations:
            payload=event['request']
            result=native(exe,path/('lucille-synthetic-'+event['id']),payload)
            results['learning'].append(dict(id=event['id'],enqueue=result['enqueue']['state'],
                jobs=[j['state'] for j in result['jobs']],facts=[dict(
                    subject=d['quality']['subject'],property=d['quality']['property'],value=d['quality']['value'],
                    state=d['memory']['state'],source=d['provenance'][0]['evidence']) for d in result['details']]))
    # Preserve every originally exact C1 row and the three demonstrated repairs.
    frozen=read(ROOT/'docs/evidence/phase22f/final-v1/retrieval-slots.json')['tracks']['c1']['queries']
    required={q['id'] for q in frozen if set(q['expected'])==set(q['modes']['fallback']['actual'])}
    required.update(['f1q-06','f1q-19','f1q-22'])
    assert required <= {q['id'] for q in results['c1'] if q['exact']}
    assert len(results['c1'])==26
    args.output.write_text(json.dumps(results,ensure_ascii=False,indent=2)+'\n',encoding='utf8')


if __name__=='__main__':
    main()
