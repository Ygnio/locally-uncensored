"""Measure C1 canonical retrieval and C2 actual USER → answer independently."""

import argparse
import json
from pathlib import Path
import re
import subprocess
import sys
import time
import uuid

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'qa'))
from phase22f_evaluation import BRIDGE, REPO, Evaluation, EvaluationFailure, failure, module, proof_matches_source, read, require_product, slug


WEIGHTS = {'fts5': 1.0, 'nomic': 0.0, 'hybrid': 0.5, 'fallback': 0.5}
MODELS = {'nomic', 'hybrid'}
FIELDS = ('subject', 'property', 'value', 'temporal')


def binding(item):
    scope, project = item['scope'], item['project']
    if not re.fullmatch(r'phase22[ef]-[a-z0-9_.-]{1,100}', scope):
        raise ValueError('An explicit synthetic phase scope is required')
    if not re.fullmatch(r'[a-z0-9][a-z0-9_.-]{0,60}', project):
        raise ValueError('An explicit synthetic project is required')
    return dict(scope=scope, project=project)


def check_plan(scenarios, track, *, final=False):
    if not isinstance(scenarios, list) or not scenarios:
        raise ValueError('At least one independent scenario is required')
    seen_scenarios, seen_queries = set(), set()
    for scenario in scenarios:
        name = slug(scenario['id'])
        if name in seen_scenarios:
            raise ValueError('Duplicate scenario')
        seen_scenarios.add(name)
        labels, records = set(), set()
        for event in scenario['events']:
            label = slug(event['label'])
            if label in labels:
                raise ValueError('Duplicate preparation label')
            action = event['action']
            if action in ('delete', 'obsolete'):
                if event['target'] not in records:
                    raise ValueError('State operation requires an earlier record')
            elif action == ('seed' if track == 'c1' else 'learn'):
                binding(event)
                if final and not event['scope'].startswith('phase22f-'):
                    raise ValueError('Final preparation requires a 2.2F scope')
                if not all(isinstance(event['fact'][key], str) and event['fact'][key].strip()
                           for key in (*FIELDS, 'kind', 'content') if key != 'content' or track == 'c1'):
                    raise ValueError('Typed preparation fact required')
                if track == 'c2' and not event['text'].strip():
                    raise ValueError('C2 requires literal USER source text')
                if event.get('state', 'active') not in ('active', 'obsolete'):
                    raise ValueError('C1 fixture state must be active or obsolete')
                if event.get('replaces') and (track != 'c1' or event['replaces'] not in records):
                    raise ValueError('Canonical replacement must reference an earlier typed fixture')
                records.add(label)
            else:
                raise ValueError('Unsupported preparation event')
            labels.add(label)
        if not scenario['queries']:
            raise ValueError('Scenario has no planned queries')
        for query in scenario['queries']:
            key = slug(query['id'])
            binding(query)
            if final and not query['scope'].startswith('phase22f-'):
                raise ValueError('Final query requires a 2.2F scope')
            required, expected, forbidden = (query.get(key, []) for key in ('requires', 'expected', 'forbidden'))
            if 'requires' not in query or 'expected' not in query or not query['query'].strip():
                raise ValueError('Explicit query gold and preparation dependencies required')
            if any(len(set(items)) != len(items) for items in (required, expected, forbidden)):
                raise ValueError('Duplicate query dependencies or gold')
            if (key in seen_queries or set(required) - labels or
                    (set(expected) | set(forbidden)) - records or
                    (set(expected) | set(forbidden)) - set(required) or set(expected) & set(forbidden)):
                raise ValueError('Invalid query ID, gold, or dependency graph')
            if query.get('mode', 'chat') not in ('chat', 'agent', 'code'):
                raise ValueError('Invalid synthetic mode')
            if track == 'c2' and query.get('logical_offset_seconds', 0) != 0:
                raise ValueError('C2 answers must use the actual clock for canonical verification')
            seen_queries.add(key)


def fixture(scenario, identity, policy):
    """Logical production backup shape. The isolated Store validates and restores it."""
    now = int(time.time())
    tables = {'memory_quality': [], 'memory_provenance': [], 'memory_revisions': [],
              'learning_policy': [[1, json.dumps(policy)]]}
    result = dict(schema=5, identity=dict(profile=identity['profile'], version=identity['version'],
                                         created_at=now, updated_at=now), memories=[], history=[], audits=[],
                  intelligent={'tables': tables})
    ids = {}
    for event in scenario['events']:
        if event['action'] != 'seed':
            continue
        label, fact = event['label'], event['fact']
        rid = str(uuid.uuid5(uuid.NAMESPACE_OID, 'phase22f-c1/' + scenario['id'] + '/' + label))
        state = event.get('state', 'active')
        # Canonical decision and execution tracks can coexist. Preserve distinct
        # fact keys where the scenario authors explicitly distinguish them.
        key = fact.get('fact_key', fact['subject'] + '.' + fact['property'])
        source_id = 'c1-fixture-' + label
        item = dict(id=rid, **binding(event), kind=fact['kind'], content=fact['content'], fact_key=key,
                    source='reviewed_import', source_id=source_id)
        replaces_id = ids[event['replaces']] if event.get('replaces') else None
        quality = dict(subject=fact['subject'], property=fact['property'], value=fact['value'], confidence=1.0,
                       relevance=1.0, temporal=fact['temporal'], expires_at=None, priority=2 if replaces_id else 1,
                       replaces_id=replaces_id, replaces_version=2 if replaces_id else None,
                       facets=fact.get('facets', []), evidence_rule='reviewed_synthetic_fixture')
        tables['memory_quality'].append([rid, *(quality[k] for k in ('subject', 'property', 'value', 'confidence',
                                          'relevance', 'temporal', 'expires_at', 'priority', 'replaces_id', 'replaces_version')),
                                          json.dumps(quality['facets']), quality['evidence_rule']])
        reviewed = dict(input=item, state='active', version=2, created_at=now, updated_at=now)
        result['history'].extend([dict(record_id=rid, operation='create', version=1, at=now),
                                  dict(record_id=rid, operation='approve', version=2, at=now)])
        if state == 'obsolete':
            # Native historical retrieval requires the immediately preceding
            # active revision, as written when Store obsoletes a reviewed row.
            tables['memory_revisions'].append([rid, 2, json.dumps(dict(memory=reviewed, quality=quality)), now])
            result['history'].append(dict(record_id=rid, operation='obsolete', version=3, at=now))
            result['memories'].append(dict(reviewed, state='obsolete', version=3))
        else:
            result['memories'].append(reviewed)
        tables['memory_provenance'].append([rid, source_id, fact['content'], now])
        ids[label] = rid
    return result, ids


def matches(detail, fact):
    return (detail['memory']['input']['kind'] == fact['kind'] and detail['quality'] is not None
            and all(detail['quality'][key] == fact[key] for key in FIELDS))


def prepare_learn(run, native, scenario, event, row):
    run.guard('gemma')
    scoped = binding(event)
    before = native(dict(operation='details', **scoped))
    prompt = re.search(r'CONVERSATION_EXTRACTION_PROMPT = `([^`]+)`',
                       (REPO / 'src/lib/lucille-atomic.ts').read_text(encoding='utf-8'))[1]
    request = dict(model='lucille-real-gemma', temperature=0, max_tokens=768, stream=False,
                   reasoning_effort='none', chat_template_kwargs={'enable_thinking': False}, messages=[
                       dict(role='system', content=prompt), dict(role='user', content=json.dumps(
                           dict(**scoped, user_text=event['text']), ensure_ascii=False))])
    row['stage'] = 'gemma_extraction'
    response = run.api('/v1/chat/completions', request)
    row['generation'] = dict(request=request, response=response)
    output = response['choices'][0]['message']['content']
    row['stage'] = 'native_materialization'
    result = native(dict(operation='learn', mode='hybrid', autonomy=True, automatic_indexing=True,
                         conversation_id='phase22f-c2-' + scenario, message_id=event['label'],
                         interaction_mode=event.get('mode', 'chat'), text=event['text'], output=output,
                         learning_policy=run.freeze.get('policy') or {}, **scoped))
    row['native'] = result
    job_id = result['enqueue'].get('id')
    jobs = [job for job in result['jobs'] if job['id'] == job_id]
    require_product(len(jobs) == 1 and jobs[0]['state'] == 'done' and not result.get('rejected'),
                    'Exact source did not complete native extraction and materialization')
    ids = {detail['memory']['input']['id'] for detail in before}
    records = [detail for detail in result['details']
               if detail['memory']['input']['id'] not in ids or any(
                   source['interaction_id'] == job_id for source in detail['provenance'])]
    require_product(len(records) == 1 and matches(records[0], event['fact']),
                    'Materialization differs from the single typed source fact')
    require_product(proof_matches_source(records[0], event['text'], job_id),
                    'Canonical record lacks provenance and native proof for the exact USER source')
    memory = records[0]['memory']
    if memory['state'] in ('candidate', 'conflict') and event.get('approve', True):
        row['stage'] = 'explicit_review'
        memory = native(dict(operation='change', id=memory['input']['id'], version=memory['version'],
                             action={'action': 'approve'}, **scoped))
    require_product(memory['state'] == 'active', 'Canonical fact is unavailable after policy and review')
    return memory


def assess(trace, query, labels):
    reverse = {value['input']['id']: name for name, value in labels.items()}
    memories = trace['context']['memories']
    actual = [reverse.get(memory['input']['id'], 'unlabelled:' + memory['input']['id']) for memory in memories]
    gold = set(query['expected'])
    hits = len(set(actual) & gold)
    return dict(status='measured', actual=actual, tp=hits, fp=len(actual) - hits, missed=len(gold) - hits,
                unsafe_current=sum(label in query.get('forbidden', []) for label in actual),
                scope_errors=sum(any(m['input'][key] != query[key] for key in ('scope', 'project')) for m in memories),
                negative_injection=bool(not gold and actual), trace=trace)


def summarize(planned, mode):
    rows = [row['modes'][mode] for row in planned]
    measured = [row for row in rows if row['status'] == 'measured']
    result = {key: sum(row[key] for row in measured) for key in ('tp', 'fp', 'missed', 'unsafe_current', 'scope_errors', 'negative_injection')}
    result.update(planned=len(rows), measured=len(measured),
                  unmeasured_expected=sum(len(row['query']['expected']) for row in planned
                                          if row['modes'][mode]['status'] != 'measured'))
    tp, fp, missed = (result[key] for key in ('tp', 'fp', 'missed'))
    result['precision'] = tp / (tp + fp) if tp + fp else None
    result['recall'] = tp / (tp + missed) if tp + missed else None
    result['recall_planned'] = tp / (tp + missed + result['unmeasured_expected']) if tp + missed + result['unmeasured_expected'] else None
    result['passed'] = (len(rows) == len(measured) and result['precision'] is not None and result['precision'] >= .9
                        and result['recall'] is not None and result['recall'] >= .8
                        and not any(result[key] for key in ('unsafe_current', 'scope_errors', 'negative_injection')))
    return result


def execute_scenario(run, scenario, track, bridge, space, encode, modes, prepare, queries, extraction_failure):
    name = scenario['id']
    store = run.root / run.label / ('lucille-synthetic-' + track + '-' + name)
    if store.exists():
        raise FileExistsError('Never reuse a C1 or C2 scenario database')

    def native(data):
        return bridge(run.bridge_path, store, data)

    identity = run.freeze.get('identity') or {'version': 1, 'profile': {
        'name': 'Lucille Test Fixture', 'description': 'Fictional QA profile', 'personality': 'Clear and concise',
        'instructions': 'Use synthetic examples.'}}
    initialization = None
    ids = {}
    try:
        if track == 'c1':
            backup, ids = fixture(scenario, identity, run.freeze.get('policy') or {})
            native(dict(operation='restore_canonical_fixture', backup=json.dumps(backup, ensure_ascii=False)))
        else:
            for version in range(identity['version']):
                native(dict(operation='configure_identity', expected_version=version, profile=identity['profile']))
    except Exception as error:
        initialization = failure(error)

    labels, prepared = {}, {}
    for event in scenario['events']:
        label, action = event['label'], event['action']
        row = dict(kind='preparation', scenario=name, event=event, label=label, status='measured')
        try:
            if initialization:
                row.update(status='not_executed', cause=initialization)
            elif action == 'seed':
                row['stage'] = 'confirm_restored_record'
                details = native(dict(operation='details', **binding(event)))
                records = [d for d in details if d['memory']['input']['id'] == ids[label]]
                require_product(len(records) == 1 and matches(records[0], event['fact']) and
                                records[0]['memory']['state'] == event.get('state', 'active') and
                                records[0]['memory']['input']['source'] == 'reviewed_import',
                                'Validated restore did not preserve the exact typed fixture record')
                row['detail'] = records[0]
                labels[label] = records[0]['memory']
            elif action == 'learn':
                if extraction_failure:
                    row.update(status='not_executed', cause=extraction_failure)
                else:
                    labels[label] = prepare_learn(run, native, name, event, row)
            elif prepared.get(event['target'], {}).get('status') != 'measured':
                row.update(status='not_executed', cause='Target seed was not prepared')
            else:
                row['stage'] = 'change_and_confirm'
                target = labels[event['target']]
                scoped = binding(target['input'])
                row['operation_result'] = native(dict(operation='change', id=target['input']['id'],
                                                      version=target['version'], action={'action': action}, **scoped))
                if action == 'delete':
                    row['tombstones'] = native(dict(operation='deleted', **scoped))
                    require_product(any(t['id'] == target['input']['id'] for t in row['tombstones']),
                                    'Native delete did not produce the expected tombstone')
                else:
                    require_product(row['operation_result']['state'] == 'obsolete', 'Record did not become historical')
                    labels[event['target']] = row['operation_result']
        except Exception as error:
            row.update(failure(error))
        prepared[label] = row
        prepare.append(row)
        run.add(row)

    indexing = {}
    if not initialization and space:
        for label, memory in labels.items():
            scoped = binding(memory['input'])
            row = dict(kind='index', scenario=name, label=label, status='measured')
            try:
                details = native(dict(operation='details', **scoped))
                active = next((d['memory'] for d in details if d['memory']['input']['id'] == memory['input']['id']
                               and d['memory']['state'] == 'active'), None)
                if active:
                    native(dict(operation='configure_embedding', space=space))
                    representation = native(dict(operation='representation', id=active['input']['id'], style='gloss', **scoped))
                    vector = encode(representation['text'], True)
                    native(dict(operation='put_vector', id=active['input']['id'], version=active['version'],
                                space=space, vector=vector, **scoped))
                    row.update(representation=representation, version=active['version'])
                else:
                    row['disposition'] = 'not_current_or_deleted'
            except Exception as error:
                row.update(failure(error))
            indexing[label] = row
            run.add(row)

    for query in scenario['queries']:
        row = dict(kind='query', scenario=name, query=query, modes={})
        missing = [label for label in query['requires'] if prepared[label]['status'] != 'measured']
        unavailable = [label for label in query['requires'] if label in indexing and indexing[label]['status'] != 'measured']
        request = dict(conversation_id='phase22f-' + track + '-' + name, message_id=query['id'],
                       mode=query.get('mode', 'chat'), query=query['query'], token_budget=8192, **binding(query))
        for mode in modes:
            if initialization or missing or (mode in MODELS and unavailable):
                row['modes'][mode] = dict(status='not_executed', missing_preparation=missing,
                                          unavailable_indexes=unavailable if mode in MODELS else [], cause=initialization)
                continue
            try:
                query_vector = dict(space=space, vector=encode(query['query'], False)) if mode in MODELS else None
                native(dict(operation='policy', lexical_weight=WEIGHTS[mode], minimum_similarity=.6))
                started = time.monotonic()
                trace = native(dict(operation='explain', request={**request, 'message_id': query['id'] + '-' + mode},
                                    query_vector=query_vector, logical_offset_seconds=query.get('logical_offset_seconds', 0)))
                result = assess(trace, query, labels)
                result['native_latency_seconds'] = time.monotonic() - started
                row['modes'][mode] = result
                if mode == 'hybrid' and track == 'c2':
                    row['answer'] = dict(status='not_executed', cause=extraction_failure) if extraction_failure else answer(run, store, request, query_vector)
            except Exception as error:
                row['modes'][mode] = failure(error)
        if track == 'c2' and 'answer' not in row:
            row['answer'] = dict(status='not_executed', reason='Hybrid context unavailable')
        queries.append(row)
        run.add(row)
        print(name, query['id'], {mode: result['status'] for mode, result in row['modes'].items()}, flush=True)


def answer(run, store, request, vector):
    # Model/transport failure here must not erase the retrieval measurement.
    try:
        run.guard('gemma')
        result = subprocess.run(['node', str(REPO / 'qa/phase22f-canonical-answer.mjs'), str(run.bridge_path), str(store)],
                                input=json.dumps(dict(request=request, query_vector=vector)), encoding='utf-8',
                                capture_output=True, timeout=270, check=True, cwd=REPO)
        return json.loads(result.stdout)
    except Exception as error:
        return failure(error)


def scenarios_from(run, track):
    if 'scenarios' in run.data:
        return run.data['scenarios']
    known = run.data['known_replay']
    path = known['corpus']
    if run.freeze.get('kind') != 'development' or path not in run.freeze['corpus_sha256']:
        raise ValueError('The original known corpus and annotations must both be sealed for development')
    old = read(REPO / path)
    events = []
    operations = {}
    for event in old['events']:
        if event['action'] == 'learn':
            label = event['label']
            annotated = known['facts'][label]
            if track == 'c1':
                events.append(dict(action='seed', label=label, **binding(event),
                                   fact=dict(annotated, content=event['text']), **known.get('canonical_states', {}).get(label, {})))
            else:
                events.append(dict(event, fact=annotated))
        else:
            label = event['action'] + '-' + event['target']
            events.append(dict(event, label=label))
            operations[event['target']] = label
    queries = []
    for query in old['queries']:
        required = sorted(set(query['expected']) | set(query.get('forbidden', [])))
        required.extend(operations[label] for label in required.copy() if label in operations)
        queries.append(dict(query, requires=required, lexical_mandatory=query.get('mandatory_lexical', False)))
    return [dict(id=known['scenario'], events=events, queries=queries)]


def main(args):
    run = Evaluation(args, args.track + ('-offline' if args.offline else ''))
    scenarios = scenarios_from(run, args.track)
    check_plan(scenarios, args.track, final=run.freeze.get('kind') == 'final')
    (run.root / run.label).mkdir(exist_ok=False)
    prepare, queries = [], []
    services = {}
    space = None
    vectors = {}
    modes = ('fallback',) if args.offline else tuple(WEIGHTS)
    if not args.offline:
        try:
            space = run.live_pin('embedding')['space']
            if space['index_version'] != 5 or space['dimension'] != 768 or space['context_tokens'] != 2048:
                raise EvaluationFailure('invalid_result', 'Unexpected frozen Nomic index contract')
        except Exception as error:
            services['embedding'] = failure(error)
            space = None
    if args.track == 'c2':
        try:
            run.live_pin('extraction')
        except Exception as error:
            services['extraction'] = failure(error)

    def encode(text, document):
        if space is None:
            raise EvaluationFailure('infrastructure_failure', 'No verified live Nomic endpoint')
        run.guard('nomic')
        key = space['document_prefix' if document else 'query_prefix'] + text
        if key not in vectors:
            response = run.api('/v1/embeddings', dict(model=space['model'], input=key, encoding_format='float'))
            vectors[key] = response['data'][0]['embedding']
        return vectors[key]

    if args.offline:
        import socket
        try:
            with socket.create_connection(('127.0.0.1', 15289), timeout=5):
                raise RuntimeError('Nomic endpoint is still listening; offline run refused')
        except ConnectionRefusedError:
            services['nomic_loopback_port_closed'] = True
    bridge = module('scripts/test-lucille-real-extraction.py').bridge
    for scenario in scenarios:
        execute_scenario(run, scenario, args.track, bridge, space, encode, modes, prepare, queries, services.get('extraction'))
    summaries = {mode: summarize(queries, mode) for mode in modes}
    mandatory = [row for row in queries if row['query'].get('lexical_mandatory')]
    correct = sum(row['modes']['fallback']['status'] == 'measured' and
                  not any(row['modes']['fallback'][key] for key in ('missed', 'fp', 'scope_errors', 'unsafe_current'))
                  for row in mandatory)
    hybrid_ok = args.offline or summaries['hybrid']['passed']
    retrieval_passed = bool(hybrid_ok and mandatory and correct == len(mandatory) and
                  all(row['status'] == 'measured' for row in prepare) and
                  all(row['modes'][mode]['status'] == 'measured' for row in queries for mode in modes))
    answer_counts = {state: sum(row.get('answer', {}).get('status') == state for row in queries)
                     for state in ('delivered', 'blocked', 'not_executed', 'infrastructure_failure', 'evaluator_failure')}
    return run.finish(passed=retrieval_passed and args.track == 'c1', track=args.track,
                      retrieval_passed=retrieval_passed, ranking=summaries, services=services, calls=run.calls,
                      mandatory_fallback=dict(planned=len(mandatory), exact=correct,
                                              physical_endpoint_unavailable=args.offline),
                      answers=dict(planned=len(queries), counts=answer_counts, literal_review_required=True) if args.track == 'c2' else None,
                      contract='C1 uses validated Store restore for reviewed synthetic typed relations. C2 uses actual Gemma and native learning, and does not substitute a failed seed. Every mode keeps all planned queries in coverage.')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    for name in ('root', 'freeze', 'dataset', 'label'):
        parser.add_argument('--' + name, required=True)
    parser.add_argument('--bridge', default=BRIDGE)
    parser.add_argument('--track', choices=('c1', 'c2'), required=True)
    parser.add_argument('--offline', action='store_true', help='Only C1 fallback, with owned Nomic port verified closed')
    args = parser.parse_args()
    if args.offline and args.track != 'c1':
        parser.error('--offline is limited to independent C1')
    raise SystemExit(main(args))
