"""Generate one real, first-pass answer per selected prepared C2 request."""
import argparse
import json
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / 'docs/evidence/phase22g/consolidation'
QA = ROOT / '.lucille-qa-runtime/lucille-synthetic-real22f'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('ids', nargs='+')
    args = parser.parse_args()
    data = json.loads((EVIDENCE/'c2-native.json').read_text(encoding='utf8'))
    queries = {q['id']: q for q in data['queries']}
    if len(set(args.ids)) != len(args.ids) or not set(args.ids) <= queries.keys():
        parser.error('Every selected query must be known and unique')
    rows = []
    with args.output.open('x',encoding='utf8') as out:
        out.write('[]\n')
    for key in args.ids:
        q = queries[key]
        store = QA/data['profile']/('lucille-synthetic-c2-'+q['scenario'])
        request = json.loads((store/(key+'.json')).read_text(encoding='utf8'))
        answer = subprocess.run(['node','qa/phase22f-canonical-answer.mjs',
            '../build/phase22g/real_validation_bridge.exe',str(store)],cwd=ROOT,
            input=json.dumps(dict(**request,first_pass_only=True)),encoding='utf8',
            capture_output=True,timeout=180,check=True)
        result = json.loads(answer.stdout)
        rows.append(dict(id=key, **{k:result.get(k) for k in ('status','delivered','report','error','generations')}))
        args.output.write_text(json.dumps(rows,ensure_ascii=False,indent=2)+'\n',encoding='utf8')
        if not result['generations']:
            break


if __name__=='__main__':
    main()
