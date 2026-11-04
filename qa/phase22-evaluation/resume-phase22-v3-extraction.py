"""Measurement-only recovery: retain exact response and explicit bridge failures.

The frozen source is not edited. Ineligible-source exit is a rejection/miss,
not a fabricated accepted output. Other process errors still abort the run.
"""
import argparse,hashlib,importlib.util,io,json,subprocess,sys,urllib.request
from pathlib import Path
repo=Path(__file__).resolve().parents[2]
root=repo/'.lucille-qa-runtime/lucille-synthetic-real22';reports=root/'reports'
spec=importlib.util.spec_from_file_location('frozen_phase22',repo/'scripts/test-lucille-phase22-extraction.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
bridge=m.h.bridge;urlopen=urllib.request.urlopen
data=json.loads((repo/'crates/lucille-memory/tests/fixtures/phase22/extraction-v3.json').read_text(encoding='utf-8'))
case=data['cases'][34];calls=[json.loads(s) for s in (reports/'real-broker-calls.ndjson').read_text(encoding='utf-8').splitlines()]
cached=next(c for c in reversed(calls) if case['text'] in str(c.get('request')))
def cached_open(request,*args,**kwargs):
 if getattr(request,'data',None):
  payload=json.loads(request.data)
  if any(case['text'] in x.get('content','') for x in payload.get('messages',[])):
   return io.BytesIO(json.dumps(cached['response']).encode())
 return urlopen(request,*args,**kwargs)
def retained_bridge(exe,path,request):
 try:return bridge(exe,path,request)
 except subprocess.CalledProcessError as error:
  if error.returncode!=1 or error.stderr.strip()!='Ineligible extraction source or response size':raise
  scoped={'scope':request['scope'],'project':request['project']}
  result={'details':bridge(exe,path,{'operation':'details',**scoped}),'metrics':bridge(exe,path,{'operation':'metrics',**scoped}),'bridge_error':{'returncode':error.returncode,'stderr':error.stderr,'mode':request['mode']},'measurement_note':'Native bridge refused source. No claims or jobs fabricated; no cancellation or repair applied. Count missing gold as missed.'}
  with (reports/'v3-bridge-rejections.ndjson').open('a',encoding='utf-8') as out:out.write(json.dumps({'case':request['message_id'],**result},ensure_ascii=False)+'\n')
  return result
note=reports/'v3-measurement-recovery.json'
if not note.exists():note.write_text(json.dumps({'original_completed_cases':34,'aborted_case':case['id'],'cached_response':cached['response'],'cached_response_completed_unix':cached['completed_unix'],'helper_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'algorithm_and_corpora_unchanged':True,'manual_diagnostic_replays':['rules exit0, ignored, empty details','llm exit0, cancelled, empty details','hybrid exit1: Ineligible extraction source or response size'],'limitation':'Interrupted case is replayed idempotently using the same real response; latency for this case is cached replay, not real generation. Hybrid fallback bridge can leave a running lease on ineligible source; preserved as evaluator failure, not successful queue completion.'},indent=2,ensure_ascii=False),encoding='utf-8')
m.h.bridge=retained_bridge;urllib.request.urlopen=cached_open
args=argparse.Namespace(root=str(root),bridge=str(repo.parent/'build/memory/debug/examples/real_validation_bridge.exe'),dataset=str(repo/'crates/lucille-memory/tests/fixtures/phase22/extraction-v3.json'),label='final-v3',freeze=str(repo/'crates/lucille-memory/tests/fixtures/phase22/freeze-v3.json'))
m.main(args)
