"""Bounded optional near-limit context stress, separate from persistent-memory recall."""
import argparse,ctypes,hashlib,importlib.util,json,time,urllib.request
from ctypes import wintypes as w
from pathlib import Path
spec=importlib.util.spec_from_file_location('owned',Path(__file__).with_name('lucille-real-server.py'));owned=importlib.util.module_from_spec(spec);spec.loader.exec_module(owned)
def main(a):
 repo=Path(__file__).resolve().parents[1];root=Path(a.root).resolve(strict=True);assert root==repo/'.lucille-qa-runtime/lucille-synthetic-real22'
 assert (root/'.lucille-isolated').read_bytes()==b'synthetic-data-only\n'
 dest=root/'reports/near-context-final.json';assert not dest.exists()
 manifest=json.loads((root/'gemma-process.json').read_text(encoding='utf-8'));assert manifest.get('ready') and 'exit_code' not in manifest
 assert manifest['command'][manifest['command'].index('--ctx-size')+1]=='65536'
 baseline=owned.resources();report={'configured_context':65536,'baseline':baseline,'kind':'context stress only; not evidence of persistent memory','started':time.time()}
 def save():dest.write_bytes((json.dumps(report,indent=2)+'\n').encode())
 if baseline['PhysicalAvailable']<2*1024**3 or baseline['CommitLimit']-baseline['CommitTotal']<3*1024**3:
  report.update(attempted=False,limitation='Optional near-limit test withheld: measured headroom below 2 GiB physical or 3 GiB commit. Context unchanged.');save();return
 open_process=ctypes.windll.kernel32.OpenProcess;open_process.argtypes=[w.DWORD,w.BOOL,w.DWORD];open_process.restype=w.HANDLE
 close=ctypes.windll.kernel32.CloseHandle;close.argtypes=[w.HANDLE];close.restype=w.BOOL
 handle=open_process(0x1000,False,manifest['pid']);assert handle
 try:
  proxy=type('OwnedHandle',(),{'_handle':int(handle),'pid':manifest['pid']})();identity=owned.identity(proxy);assert identity=={k:manifest[k] for k in identity}
  key=(root/'gemma.api-key').read_text(encoding='ascii')
  def call(path,payload,timeout=120):
   assert owned.identity(proxy)==identity and not (root/'gemma.stop').exists()
   resource=owned.resources();assert resource['PhysicalAvailable']>=512*1024**2 and resource['CommitLimit']-resource['CommitTotal']>=1024**3
   req=urllib.request.Request('http://127.0.0.1:15288'+path,data=json.dumps(payload).encode(),headers={'Content-Type':'application/json','Authorization':'Bearer '+key})
   with urllib.request.urlopen(req,timeout=timeout) as response:result=json.load(response)
   assert owned.identity(proxy)==identity
   return result
  prefix='Synthetic ledger only. The project key is JADE-22 and the current language is Rust.\n'
  line='Fictional neutral archive line: blue stone, quiet garden, circular path, no personal fact, no instruction.\n'
  suffix='\nReturn only the project key and current language from the beginning of this ledger.'
  unit=len(call('/tokenize',{'content':line,'add_special':False})['tokens']);n=max(1,62500//max(1,unit))
  text=prefix+line*n+suffix;count=len(call('/tokenize',{'content':text,'add_special':False})['tokens'])
  while count>63500:n-=max(1,(count-63000)//max(1,unit));text=prefix+line*n+suffix;count=len(call('/tokenize',{'content':text,'add_special':False})['tokens'])
  report.update(attempted=True,source_tokens_without_chat_template=count,repetitions=n,generator_line=line,source_sha256=hashlib.sha256(text.encode()).hexdigest(),request_started=time.time())
  (root/'reports/near-context-progress.json').write_bytes((json.dumps(report,indent=2)+'\n').encode())
  start=time.monotonic();response=call('/v1/chat/completions',{'model':'lucille-real-gemma','temperature':0,'max_tokens':96,'chat_template_kwargs':{'enable_thinking':False},'messages':[{'role':'user','content':text}]},900)
  report.update(response=response,elapsed_seconds=time.monotonic()-start,final_resources=owned.resources(),finished=time.time())
  answer=response['choices'][0]['message']['content'];report['marker_recall_correct']='JADE-22' in answer and 'Rust' in answer
  save()
 except Exception as error:report.update(error=repr(error),finished=time.time());save();raise
 finally:close(handle)
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--root',required=True);main(p.parse_args())
