"""Join frozen observations to their original USER inputs without opening a database."""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FIX = ROOT / 'crates/lucille-memory/tests/fixtures/phase22f/final-v1'
DATA = ROOT / 'docs/evidence/phase22f/final-v1'
OUT = ROOT / 'docs/evidence/phase22g'


def read(path):
    return json.loads(path.read_text(encoding='utf8'))


def main():
    rows = []
    for track, observations in read(DATA / 'retrieval-slots.json')['tracks'].items():
        if track == 'c1_offline':  # same selections as logical fallback in the frozen report
            continue
        scenarios = read(FIX / f'retrieval-{track}.json')['scenarios']
        events = {e['label']: e for s in scenarios for e in s['events']}
        queries = {q['id']: q for s in scenarios for q in s['queries']}
        failed = set()
        for p in observations['preparations_and_indexes']:
            if p.get('reason') == 'not_eligible':
                failed.add(p['label'])
                rows.append(dict(category='A', source=events[p['label']], observed=p,
                    last='source submitted', first='admission before job allocation', gemma=False, job=False, receipt=False,
                    experiment='Replay enqueue and inspect authority spans and typed claims.'))
        for q in observations['queries']:
            obs = q['modes']['fallback' if track == 'c1' else 'hybrid']
            skipped = obs['status'] != 'measured'
            if not skipped and set(obs['actual']) == set(q['expected']):
                continue
            rows.append(dict(category='A (dependency)' if skipped else 'C', query=queries[q['id']],
                records=[events[k] for k in q['requires'] if k in events], observed=obs,
                blockers=sorted(failed.intersection(q['requires'])), last='plan' if skipped else 'indexed records',
                first='precondition' if skipped else 'selection', gemma='no query generation' if skipped or track=='c1' else 'see literal slots',
                job='n/a for lookup', receipt='n/a for lookup', experiment='Recheck prerequisites individually.' if skipped else 'Replay selectors on identical indexed records.'))
    faults=read(FIX/'faults.json')['transport']
    for t in read(DATA/'native-protocols.json')['stages']['transport']['outcome']:
        if t['status']=='measured':
            continue
        chat=t['mode']=='chat'
        rows.append(dict(category='B' if chat else 'A/B (underlying cause F)', source=next(x for x in faults if x['mode']==t['mode']),
            observed=t, last='valid extraction proposal' if chat else 'persisted USER', first='materialization' if chat else 'no job observed',
            gemma=chat, job=chat, receipt=False,experiment='Replay source through admission, then validate proposal and job completion separately.'))
    for s in read(DATA/'literal-slots.json')['slots']:
        if s['stage']!='GUI' or (s['useful'] and not s['claims'].get('critical') and len(s['retained_atom_ids'])==len(s['expected_atoms'])):
            continue
        fixture=read(FIX/s['file'])
        turn=next(t for t in fixture['turns']+fixture.get('probes',[]) if t['id']==s['id'])
        rejected=s['status']=='blocked'
        rows.append(dict(category='E' if s['claims'].get('critical') else 'F',source=turn,observed=s,
            last='generation' if rejected else 'display',first='no verified delivery' if rejected else 'literal mismatch',
            gemma=True,job='queued at display' if s['id']=='fv1-forget-t01' else 'unknown',
            receipt={'fv1-forget-t01':'first read 7624 ms, display 2684 ms','fv1-main-t15':'none'}.get(s['id'],'unknown'),
            experiment='Replay original snapshot and both candidates, separating correct holds from false holds.'))
    (OUT/'audit.json').write_text(json.dumps(rows,ensure_ascii=False,indent=2)+'\n',encoding='utf8')


if __name__=='__main__':
    main()


