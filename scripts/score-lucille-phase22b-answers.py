"""Score independently adjudicated literal answers, never the verifier's verdict.

The ledger names immutable GUI fixtures, exact answer SHA256s and literal claims.
Blocked answers remain unavailable responses with lost utility, not correct answers.
Raw attempts and displayed answers have separate denominators. The ledger must
cover every assistant response in each selected fixture, including blocked ones.
"""
import argparse
import hashlib
import json
from collections import defaultdict
from pathlib import Path


def digest(text):
    return hashlib.sha256(text.encode('utf-8')).hexdigest()


def bucket():
    return dict(assertions=0, supported=0, unsupported=0, critical=0,
                responses=0, delivered=0, blocked=0, useful_expected=0,
                useful_retained=0, regenerated=0, checked=0)


def annotate(text, annotation, stats):
    assert annotation['sha256'] == digest(text), 'Adjudication belongs to another answer'
    assert annotation['reviewed_entire_answer'] is True
    assert isinstance(annotation['claims'], list)
    for claim in annotation['claims']:
        assert claim['literal'] and claim['literal'] in text
        assert isinstance(claim['supported'], bool) and isinstance(claim['critical'], bool)
        assert claim['reason'].strip(), 'Independent evidence/reason required'
        if claim['supported']:
            assert claim['evidence'], 'A supported assertion needs independent source evidence'
        stats['assertions'] += 1
        stats['supported' if claim['supported'] else 'unsupported'] += 1
        stats['critical'] += int(not claim['supported'] and claim['critical'])


def main(a):
    reports = Path(a.reports).resolve(strict=True)
    ledger = json.loads(Path(a.ledger).read_text(encoding='utf-8'))
    assert ledger['adjudication_method'] == 'independent_literal_review'
    assert ledger['rubric_frozen_sha256'] == hashlib.sha256(Path(a.rubric).read_bytes()).hexdigest()
    rubric = json.loads(Path(a.rubric).read_text(encoding='utf-8'))
    required = set(rubric['required_scenarios'])
    by_category = defaultdict(lambda: {'raw': bucket(), 'final': bucket()})
    totals = {'raw': bucket(), 'final': bucket()}
    source_fixtures = {}
    expected_keys = set()
    for name, file_hash in ledger['fixtures'].items():
        assert Path(name).name == name and name.endswith('.json')
        file = reports / name
        assert hashlib.sha256(file.read_bytes()).hexdigest() == file_hash
        fixture = json.loads(file.read_text(encoding='utf-8'))
        assert fixture.get('execution_complete') is True
        source_fixtures[name] = fixture
        for index, turn in enumerate(fixture['turns']):
            for message in turn['messages']:
                if message['role'] == 'assistant':
                    expected_keys.add((name, index, message['id']))
    seen = set()
    for sample in ledger['samples']:
        key = (sample['fixture'], sample['turn'], sample['message_id'])
        assert key in expected_keys and key not in seen
        seen.add(key)
        turn = source_fixtures[key[0]]['turns'][key[1]]
        message = next(m for m in turn['messages'] if m['id'] == key[2])
        assert sample['query'] == turn['text']
        category = sample['scenario']
        assert category in required or category == 'longitudinal_nonprobe'
        report = (message.get('lucilleFidelity') or [None])[-1]
        assert report, 'Every evaluated Lucille response must record its actual intervention'
        assert len(sample['raw']) == len(report['attempts'])
        for target in (totals, by_category[category]):
            for raw, annotation in zip(report['attempts'], sample['raw']):
                annotate(raw['rawAnswer'], annotation, target['raw'])
                target['raw']['responses'] += 1
            final = target['final']
            final['responses'] += 1
            blocked = report['status'] == 'blocked'
            assert sample['delivered'] is (not blocked and not bool(message.get('error')))
            final['blocked'] += int(blocked)
            final['delivered'] += int(sample['delivered'])
            final['regenerated'] += int(report['status'] == 'regenerated')
            final['checked'] += int(report['status'] == 'checked')
            assert 0 <= sample['useful_retained'] <= sample['useful_expected']
            assert sample['delivered'] or sample['useful_retained'] == 0
            final['useful_expected'] += sample['useful_expected']
            final['useful_retained'] += sample['useful_retained']
            annotate(message['content'], sample['final'], final)
            if blocked:
                assert not sample['final']['claims'], 'A retained answer is a non-answer'
    assert seen == expected_keys, 'Missing responses cannot disappear from the denominator'
    for group in [totals, *by_category.values()]:
        for stats in group.values():
            stats['fidelity'] = stats['supported'] / stats['assertions'] if stats['assertions'] else None
            stats['utility_loss'] = stats['useful_expected'] - stats['useful_retained']
    covered = required.intersection(by_category)
    final = totals['final']
    result = dict(method=ledger['adjudication_method'], totals=totals,
                  by_category=dict(by_category), missing_scenarios=sorted(required-covered),
                  answer_gate=bool(final['assertions'] and final['fidelity'] >= .95
                                   and not final['critical'] and covered == required
                                   and all(v['final']['critical'] == 0 and (v['final']['fidelity'] is None or v['final']['fidelity'] >= .95) for v in by_category.values())
                                   and all(by_category[c]['final']['delivered'] > 0 and (by_category[c]['final']['useful_expected'] == 0 or by_category[c]['final']['useful_retained'] > 0) for c in required)),
                  availability_gate=bool(final['responses'] and final['delivered'] == final['responses'] and final['utility_loss'] == 0),
                  limitation='Literal independent adjudication, with exact hashes and coverage; not a universal truth proof. Non-answers and useful omissions are reported separately and never auto-approved.')
    out = Path(a.output)
    assert not out.exists(), 'Never overwrite a completed evaluation'
    out.write_bytes((json.dumps(result, indent=2, ensure_ascii=False)+'\n').encode('utf-8'))
    print(json.dumps(result, ensure_ascii=True))


if __name__ == '__main__':
    p = argparse.ArgumentParser()
    for name in ['reports', 'ledger', 'rubric', 'output']:
        p.add_argument('--'+name, required=True)
    main(p.parse_args())
