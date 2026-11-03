"""Frozen real Gemma output -> unchanged canonical validation/queue/SQLite.

One real response per case is replayed into LLM and hybrid modes, controlling
stochastic variation; rules see the same text. No mock output or response repair.
"""
import argparse
from collections import defaultdict
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import time
import urllib.request


def bridge(executable, root, request):
    root.mkdir(parents=True, exist_ok=True)
    (root / '.lucille-isolated').write_bytes(b'synthetic-data-only\n')
    env = dict(os.environ, LUCILLE_QA_ROOT=str(root.resolve()))
    result = subprocess.run([str(executable)], input=json.dumps(request), text=True,
                            encoding='utf-8', capture_output=True, env=env, check=True,
                            creationflags=subprocess.CREATE_NO_WINDOW if os.name == 'nt' else 0)
    return json.loads(result.stdout)


def score(actual, gold):
    matched = set()
    tp = fp = 0
    for record in actual:
        memory, quality = record['memory'], record['quality']
        target = next((i for i, e in enumerate(gold) if i not in matched
                       and memory['input']['kind'] == e['kind']
                       and quality['subject'] == e['subject']
                       and ('property' not in e or quality['property'] == e['property'])
                       and ('property_prefix' not in e or quality['property'].startswith(e['property_prefix']))
                       and ('value' not in e or quality['value'] == e['value'])
                       and ('temporal' not in e or quality['temporal'] == e['temporal'])
                       and ('correction' not in e or (quality['priority'] == 2) == e['correction'])), None)
        if target is None:
            fp += 1
        else:
            matched.add(target)
            tp += 1
    return tp, fp, len(gold) - len(matched)


def main(args):
    repo = Path(__file__).resolve().parent.parent
    root = Path(args.root).resolve(strict=True)
    if (root / '.lucille-isolated').read_bytes() != b'synthetic-data-only\n':
        raise ValueError('Marked root required')
    out = root / 'reports' / 'extraction-final.json'
    if out.exists():
        raise RuntimeError('Final results already exist; never silently overwrite')
    fixture = repo / 'crates/lucille-memory/tests/fixtures/phase21c-real-final.json'
    manifest = json.loads(fixture.with_suffix('.manifest.json').read_text(encoding='utf-8'))
    assert hashlib.sha256(fixture.read_bytes()).hexdigest() == manifest['sha256']
    prompt = re.search(r'EXTRACTION_PROMPT = `([^`]+)`', (repo / 'src/lib/lucille-llm.ts').read_text(encoding='utf-8')).group(1)
    freeze = json.loads(Path(args.freeze).read_text(encoding='utf-8'))
    for path, digest in freeze['core_source_sha256'].items():
        # Git's text checkout can convert CRLF; the sealed algorithm uses LF.
        raw = (repo / path).read_bytes().replace(b'\r\n', b'\n')
        assert hashlib.sha256(raw).hexdigest() == digest, path
    assert hashlib.sha256(prompt.encode()).hexdigest() == freeze['prompt_sha256_utf8']
    key = (root / 'gemma.api-key').read_text(encoding='ascii')
    executable = Path(args.bridge).resolve(strict=True)
    dataset = json.loads(fixture.read_text(encoding='utf-8'))
    modes = {m: {'tp': 0, 'fp': 0, 'missed': 0, 'critical_writes': 0, 'scope_errors': 0,
                 'identity_changes': 0, 'by_category': defaultdict(lambda: [0, 0, 0]),
                 'failures': []} for m in ['rules', 'llm', 'hybrid']}
    captures = []
    for case in dataset['cases']:
        if (root / 'gemma.stop').exists():
            raise RuntimeError('Stop requested; no new inference')
        payload = {'model': 'lucille-real-gemma', 'temperature': 0, 'max_tokens': 768,
                   'messages': [{'role': 'system', 'content': prompt},
                                {'role': 'user', 'content': json.dumps({'scope': 'lucille', 'project': 'atlas', 'user_text': case['text']}, ensure_ascii=False)}]}
        request = urllib.request.Request('http://127.0.0.1:15288/v1/chat/completions',
                                        data=json.dumps(payload).encode(), headers={
                                            'Content-Type': 'application/json', 'Authorization': 'Bearer ' + key})
        started = time.monotonic()
        with urllib.request.urlopen(request, timeout=120) as response:
            reply = json.load(response)
        elapsed = time.monotonic() - started
        choice = reply['choices'][0]
        output = choice['message']['content']
        capture = {'id': case['id'], 'text': case['text'], 'response': reply,
                   'latency_seconds': elapsed, 'native_modes': {}}
        (root / 'reports' / (case['id'] + '-gemma.json')).write_text(json.dumps(capture, indent=2, ensure_ascii=False), encoding='utf-8')
        for mode, stats in modes.items():
            result = bridge(executable, root / 'evaluation' / f'lucille-synthetic-{mode}-{case["id"]}',
                            {'operation': 'learn', 'mode': mode, 'text': case['text'],
                             'message_id': case['id'], 'output': output})
            capture['native_modes'][mode] = result
            tp, fp, missed = score(result['details'], case['expected'])
            for field, n in [('tp', tp), ('fp', fp), ('missed', missed)]:
                stats[field] += n
            for i, n in enumerate([tp, fp, missed]):
                stats['by_category'][case['category']][i] += n
            stats['critical_writes'] += len(result['details']) if case['critical'] else 0
            stats['scope_errors'] += sum(d['memory']['input']['scope'] != 'lucille' or d['memory']['input']['project'] != 'atlas' for d in result['details'])
            stats['identity_changes'] += result['identity']['version'] != 1
            if fp or missed:
                stats['failures'].append({'id': case['id'], 'tp': tp, 'fp': fp, 'missed': missed, 'rejected': result['rejected']})
        captures.append(capture)
        # Preserve evidence after every real call, including incomplete runs.
        (root / 'reports' / 'extraction-progress.json').write_text(json.dumps(captures, indent=2, ensure_ascii=False), encoding='utf-8')
        print(f'{case["id"]}: real response {elapsed:.2f}s, finish={choice["finish_reason"]}', flush=True)
    for stats in modes.values():
        stats['precision'] = stats['tp'] / max(1, stats['tp'] + stats['fp'])
        stats['recall'] = stats['tp'] / max(1, stats['tp'] + stats['missed'])
        stats['passed'] = stats['precision'] >= .95 and stats['recall'] >= .8 and not any(stats[k] for k in ['critical_writes', 'scope_errors', 'identity_changes'])
    times = sorted(x['latency_seconds'] for x in captures)
    result = {'dataset': manifest, 'real_gemma': True, 'simulation': False,
              'same_response_used_for_llm_and_hybrid': True, 'modes': modes,
              'requests': len(captures), 'latency_p50_s': times[(len(times)-1)//2],
              'latency_p95_s': times[int((len(times)-1)*.95)],
              'prompt_tokens': sum(x['response'].get('usage', {}).get('prompt_tokens', 0) for x in captures),
              'completion_tokens': sum(x['response'].get('usage', {}).get('completion_tokens', 0) for x in captures)}
    out.write_text(json.dumps(result, indent=2), encoding='utf-8')
    print(json.dumps(result, indent=2), flush=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', required=True)
    parser.add_argument('--bridge', required=True)
    parser.add_argument('--freeze', required=True)
    main(parser.parse_args())
