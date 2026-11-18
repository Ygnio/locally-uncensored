"""Lifecycle for existing pinned Phase22 weights only; never installs or stops foreign processes."""
import argparse,json,os,shutil,socket,subprocess,time
from pathlib import Path
repo=Path(__file__).resolve().parents[1]
def ready(root,kind):
 try:
  d=json.loads((root/(kind+'-process.json')).read_text(encoding='utf-8'));return d.get('ready') and 'exit_code' not in d
 except (OSError,ValueError):return False
def port_open(port):
 try:
  with socket.create_connection(('127.0.0.1',port),timeout=.4):return True
 except OSError:return False
def stop(root,kinds):
 for kind in kinds:(root/(kind+'.stop')).write_bytes(b'owned phase22 lifecycle request\n')
 deadline=time.monotonic()+40
 while time.monotonic()<deadline:
  if all(not port_open({'gemma':15288,'nomic':15289,'broker':15284}[k]) for k in kinds):return
  time.sleep(.5)
 raise RuntimeError('Own graceful stop did not finish; no foreign termination attempted')
def archive(root,kinds):
 dest=root/'reports'/('epoch-'+str(time.time_ns()));dest.mkdir()
 for k in kinds:
  manifest=root/(k+'-process.json')
  if manifest.exists() and k!='broker':
   d=json.loads(manifest.read_text(encoding='utf-8'));assert 'exit_code' in d and d.get('stop_identity_verified'),'Wait for own supervisor shutdown'
  for name in [k+'-process.json',k+'.stop']:
   p=root/name
   if p.exists():shutil.copyfile(p,dest/name);p.unlink()
  for name in [k+'.stdout.log',k+'.stderr.log',k+'-resources.ndjson',k+'-supervisor.stdout.log',k+'-supervisor.stderr.log']:
   p=root/'reports'/name
   if p.exists():shutil.copyfile(p,dest/name)
def launch(root,kind,contract):
 if kind=='broker':
  args=['scripts/lucille-real-broker.py','--root',str(root),'--contract',str(root/'attested-contract.json')];stem='broker'
 else:
  args=['scripts/lucille-real-server.py','--root',str(root),'--executable',contract['runtime_path'],'--model',contract['paths'][kind],'--kind',kind];stem=kind+'-supervisor'
  if kind=='gemma':args+=['--prompt-cache-mib','0','--context-checkpoints','0','--disable-cuda-graphs','--lazy-embeddings']
 with (root/'reports'/(stem+'.stdout.log')).open('wb') as out,(root/'reports'/(stem+'.stderr.log')).open('wb') as err:
  p=subprocess.Popen([__import__('sys').executable,*args],cwd=repo,stdout=out,stderr=err,creationflags=subprocess.CREATE_NO_WINDOW)
 print('Started own supervisor',kind,p.pid,flush=True)
 deadline=time.monotonic()+180
 while time.monotonic()<deadline:
  if p.poll() is not None:raise RuntimeError(kind+' supervisor exited; inspect retained private logs')
  if (kind=='broker' and port_open(15284)) or (kind!='broker' and ready(root,kind)):return
  time.sleep(1)
 raise RuntimeError('Own endpoint startup timeout: '+kind)
def main(a, phase='22e'):
 assert phase in ('22e','22f')
 root=Path(a.root).resolve(strict=True);assert root==repo/'.lucille-qa-runtime'/('lucille-synthetic-real'+phase)
 assert (root/'.lucille-isolated').read_bytes()==b'synthetic-data-only\n'
 contract=json.loads((root/'attested-contract.json').read_text(encoding='utf-8'));assert contract['memory_protocol']==3 and contract['models']['gemma']['context_tokens']==65536
 kinds=['nomic','gemma','broker'] if a.operation in ['start','stop'] else ['nomic'] if a.operation in ['nomic-restart','nomic-stop'] else ['broker']
 if a.operation!='start':stop(root,kinds)
 if a.operation in ['stop','nomic-stop','broker-stop']:return
 assert not any(port_open({'gemma':15288,'nomic':15289,'broker':15284}[k]) for k in kinds),'Own requested ports occupied; no stop attempted'
 time.sleep(3);archive(root,kinds)
 for k in kinds:launch(root,k,contract)
def cli(phase='22e'):
 p=argparse.ArgumentParser();p.add_argument('--root',required=True);p.add_argument('--operation',choices=['start','stop','nomic-restart','nomic-stop','broker-restart','broker-stop'],required=True);main(p.parse_args(),phase)

if __name__=='__main__':cli()
