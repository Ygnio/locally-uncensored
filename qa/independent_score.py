"""Aggregate independent human judgments against the frozen oracle, offline."""
import argparse
import json
from pathlib import Path

SUITE = Path(__file__).resolve().parents[1]/'docs/evidence/phase22g/independent-evaluation'


def require(value, message):
    if not value:
        raise ValueError(message)


def summarize(gold, ledger):
    expected = {c['id']:c for c in gold['cases']}
    rows = ledger['queries']
    require(len(rows)==len(expected) and {r['id'] for r in rows}==set(expected),'List every case exactly once')
    counts = dict.fromkeys(('observed','delivered','complete','useful','faithful_complete','claims','supported',
                            'relevant','retrieved','hits','mandatory','false_accept','false_retain','correct_drafts','incorrect_drafts','drafts'),0)
    for r in rows:
        case = expected[r['id']]
        needed = {x['fact'] for x in case['required_references']}
        counts['mandatory'] += len(needed)
        require(r['status'] in ('observed','interrupted','pending'),'Unknown case status')
        if r['status']!='observed':
            continue
        require(r['evidence'].strip(),'Missing evidence locator')
        counts['observed'] += 1
        for k in ('delivered','complete','useful','faithful'):
            require(type(r[k]) is bool,k)
        require(r['delivered'] or not (r['complete'] or r['useful'] or r['faithful'] or r['assertions']),'Undelivered response graded')
        for k in ('complete','useful','delivered'):
            counts[k]+=r[k]
        counts['faithful_complete']+=r['complete'] and r['faithful']
        for claim in r['assertions']:
            require(type(claim['supported']) is bool and claim['text'].strip() and claim['basis'].strip(),'Invalid claim')
            counts['claims']+=1
            counts['supported']+=claim['supported']
        refs=set()
        occurrences={}
        for ref in r['retrieval']:
            require(ref['native_version'].strip() and type(ref['relevant']) is bool,'Missing actual retrieval identifier')
            authorized=ref['fact'] in case['authorized_facts']
            require(not ref['relevant'] or authorized,'Unauthorized reference marked relevant')
            if authorized: refs.add(ref['fact'])
            key=ref['native_version']
            occurrences[key]=occurrences.get(key,False) or ref['relevant']
        counts['relevant']+=sum(occurrences.values())
        counts['retrieved']+=len(occurrences)
        counts['hits']+=len(needed & refs)
        for d in r['drafts']:
            require(d in ('correct_accepted','incorrect_accepted','correct_blocked','incorrect_blocked'),'Invalid draft disposition')
            counts['drafts']+=1
            counts['correct_drafts' if d.startswith('correct_') else 'incorrect_drafts']+=1
            counts['false_accept']+=d=='incorrect_accepted'
            counts['false_retain']+=d=='correct_blocked'
    facts=ledger['materialization']
    require(set(facts)==set(gold['facts']) and all(v in ('correct','failed','unobserved') for v in facts.values()),'Grade every source fact')
    events=ledger['safety_events']
    for e in events:
        require(e['kind'] in ('critical_delivery','premature','resurrection','isolation') and e['evidence'].strip(),'Invalid safety event')
    checked=sum(v!='unobserved' for v in facts.values())
    ratios={k:[counts[k],len(rows)] for k in ('delivered','complete','useful','faithful_complete')}
    ratios.update(conditional_fidelity=[counts['faithful_complete'],counts['complete']],
                  assertion_fidelity=[counts['supported'],counts['claims']],
                  retrieval_precision=[counts['relevant'],counts['retrieved']],
                  mandatory_recovery=[counts['hits'],counts['mandatory']],
                  false_acceptance=[counts['false_accept'],counts['incorrect_drafts']],
                  false_retention=[counts['false_retain'],counts['correct_drafts']],
                  materialization=[sum(v=='correct' for v in facts.values()),len(facts)])
    require(type(ledger['closed_cleanly']) is bool,'Missing closure verdict')
    finished=counts['observed']==len(rows) and checked==len(facts) and ledger['closed_cleanly']
    passes=finished and not events and all(n and 100*x>=95*n for x,n in
            (ratios[k] for k in ('delivered','faithful_complete','conditional_fidelity','assertion_fidelity'))) and counts['hits']==counts['mandatory']
    return dict(status='pass' if passes else 'fail' if finished or events else 'incomplete',counts=counts,
                ratios=ratios,materialization_observed=checked,safety_events=events)


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('ledger',type=Path)
    args=parser.parse_args()
    print(json.dumps(summarize(json.loads((SUITE/'oracle.json').read_text(encoding='utf8')),
                               json.loads(args.ledger.read_text(encoding='utf8'))),indent=2))
