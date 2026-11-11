"""Evaluator contract regressions. No inference or application data mutation."""

from copy import deepcopy
import hashlib
import json
from pathlib import Path
from runpy import run_path
import sys
import tempfile
from types import SimpleNamespace
import unittest

sys.dont_write_bytecode = True
from phase22f_evaluation import EvaluationFailure, REPO, protocol_plan, read


retrieval = run_path(str(REPO / 'scripts/test-lucille-phase22f-retrieval.py'))
transport = run_path(str(REPO / 'scripts/test-lucille-phase22f-transport.py'))
faults = run_path(str(REPO / 'scripts/test-lucille-phase22f-faults.py'))
ANNOTATIONS = REPO / 'crates/lucille-memory/tests/fixtures/phase22f/development'


def known(name, key):
    data = read(ANNOTATIONS / name)
    return SimpleNamespace(data=data, freeze={'kind': 'development', 'corpus_sha256': {data[key]['corpus']: 'known'}})


class EvaluationTests(unittest.TestCase):
    def test_known_plans_preserve_sources_questions_gold_and_all_slots(self):
        run = known('known-retrieval-annotations.json', 'known_replay')
        original = read(REPO / run.data['known_replay']['corpus'])
        for track in ('c1', 'c2'):
            scenarios = retrieval['scenarios_from'](run, track)
            retrieval['check_plan'](scenarios, track)
            self.assertEqual(len(scenarios[0]['events']), 17)
            self.assertEqual(len(scenarios[0]['queries']), 29)
            self.assertEqual(sum(query['lexical_mandatory'] for query in scenarios[0]['queries']), 20)
            for old, new in zip(original['queries'], scenarios[0]['queries']):
                self.assertEqual({key: new[key] for key in old}, old)
                if 'deleted-creation' in new['requires']:
                    self.assertIn('delete-deleted-creation', new['requires'])

    def test_failed_preparation_keeps_dependent_slot_and_measures_independent_query(self):
        scope, project = 'phase22f-test', 'fixture'
        fact = dict(kind='preference', subject='user', property='interface_theme', value='dark', temporal='preference',
                    content='I prefer a dark theme.')
        query = dict(query='Qual o tema?', scope=scope, project=project, lexical_mandatory=True)
        scenario = dict(id='dep', events=[dict(action='seed', label='bad', scope=scope, project=project, fact=fact)], queries=[
            dict(query, id='dep-one', expected=['bad'], requires=['bad']),
            dict(query, id='dep-two', expected=[], requires=[]),
        ])
        retrieval['check_plan']([scenario], 'c1', final=True)
        rows, prep, queries = [], [], []

        def bridge(_exe, _store, payload):
            if payload['operation'] == 'restore_canonical_fixture':
                self.assertEqual(json.loads(payload['backup'])['schema'], 5)
            elif payload['operation'] == 'details':
                return []
            elif payload['operation'] == 'explain':
                self.assertTrue(payload['request']['message_id'].startswith('dep-two'))
                return {'context': {'memories': []}}

        with tempfile.TemporaryDirectory() as path:
            run = SimpleNamespace(root=Path(path), label='synthetic', bridge_path=None, freeze={}, add=rows.append)
            retrieval['execute_scenario'](run, scenario, 'c1', bridge, None, None, ('fallback',), prep, queries, None)
        self.assertEqual(prep[0]['status'], 'product_failure')
        self.assertEqual(queries[0]['modes']['fallback']['status'], 'not_executed')
        self.assertEqual(queries[1]['modes']['fallback']['status'], 'measured')
        scores = retrieval['summarize'](queries, 'fallback')
        self.assertEqual((scores['planned'], scores['measured'], scores['unmeasured_expected']), (2, 1, 1))
        self.assertEqual(scores['recall_planned'], 0)
        self.assertFalse(scores['passed'])

    def test_transport_gold_is_bound_to_each_source_and_commit(self):
        run = known('known-protocol-annotations.json', 'known_protocol')
        dataset = protocol_plan(run)
        cases = transport['cases_from'](dataset)
        self.assertEqual({case['mode']: case['expected']['property'] for case in cases}, {
            'chat': 'coffee_sugar', 'agent': 'interface_theme', 'code': 'games_class_selection',
        })
        self.assertEqual(len(faults['check_cases'](dataset)), 7)
        case = next(case for case in cases if case['mode'] == 'agent')
        text = case['text']
        sha = hashlib.sha256(text.encode()).hexdigest()
        proof = dict(start=0, end=len(text.encode()), source_digest=sha, evidence_digest=sha,
                     statement_author='user', message_author='user')
        detail = dict(memory={'input': {'kind': 'preference', 'id': 'record'}, 'state': 'active', 'version': 1},
                      quality=dict(case['expected'], evidence_rule='fixture|native_source_authority_v2:' + json.dumps(proof)),
                      provenance=[dict(interaction_id='job', evidence=text)])
        message = dict(id='source', role='user', content=text)
        receipt = dict(id='job', profile='lucille', identity_version=3, mode='agent', action='learn', state='done',
                       conversation_id='c', source_message_id='source', scope='s', project=case['project'],
                       records=[dict(id='record', version=1, state='active')])
        turn = dict(transport_failure=True, mode='agent', conversation_id='c', messages=[message],
                    domPaints=[dict(at_ms=5, text='network error')], operationSnapshots=[dict(
                        job={'state': 'done'}, learning_receipt=receipt, read_started_ms=6, read_finished_ms=7)])
        result = dict(scope='s', execution_complete=True, turns=[turn], initial_details=[], initial_deleted=[],
                      final_details=[detail], identity={'version': 3}, final_metrics={'jobs': [{'state': 'done', 'count': 1}]})
        verify = transport['verify_recovery']
        self.assertEqual(verify(case, {'messages': [message]}, result)['canonical']['value'], 'dark')
        for key, wrong in [('property', 'coffee_sugar'), ('value', 'without_sugar')]:
            bad = deepcopy(case)
            bad['expected'][key] = wrong
            with self.assertRaises(EvaluationFailure):
                verify(bad, {'messages': [message]}, result)
        for key, wrong in [('source_message_id', 'earlier'), ('identity_version', 2), ('mode', 'chat')]:
            bad = deepcopy(result)
            bad['turns'][0]['operationSnapshots'][0]['learning_receipt'][key] = wrong
            with self.assertRaises(EvaluationFailure):
                verify(case, {'messages': [message]}, bad)


if __name__ == '__main__':
    unittest.main()
