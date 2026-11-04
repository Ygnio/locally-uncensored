"""Exactly one dispatch at pushed final HEAD, then bounded read-only audit."""
import datetime,json,subprocess,urllib.request,urllib.error,urllib.parse
from pathlib import Path
repo=Path(__file__).resolve().parents[2];out=repo.parent/'qa/phase22-final-github-audit.json';assert not out.exists(),'Never repeat final-head dispatch'
branch='feature/lucille-phase22-autonomous-memory';head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=repo,text=True).strip();remote=subprocess.check_output(['git','ls-remote','origin','refs/heads/'+branch],cwd=repo,text=True).split()[0];assert remote==head
c=subprocess.run(['git','credential','fill'],input='protocol=https\nhost=github.com\n\n',capture_output=True,text=True,cwd=repo,check=True);token=dict(x.split('=',1) for x in c.stdout.splitlines() if '=' in x)['password']
result={'utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'branch':branch,'head':head,'remote_head_verified':remote,'dispatch_attempts':0,'read_only':{}}
# Save intent before network: an interrupted dispatch is not automatically retried.
out.write_text(json.dumps(result,indent=2),encoding='utf-8')
def call(path,data=None):
 req=urllib.request.Request('https://api.github.com/repos/Ygnio/locally-uncensored'+path,data=None if data is None else json.dumps(data).encode(),headers={'Authorization':'Bearer '+token,'User-Agent':'lucille-isolated-qa','Accept':'application/vnd.github+json','Content-Type':'application/json'},method='GET' if data is None else 'POST')
 try:
  with urllib.request.urlopen(req,timeout=30) as r:
   raw=r.read();return {'http_status':r.status,'data':json.loads(raw) if raw else None}
 except urllib.error.HTTPError as e:return {'http_status':e.code,'message':json.load(e).get('message')}
result['dispatch_attempts']=1;result['dispatch']=call('/actions/workflows/lucille-memory.yml/dispatches',{'ref':branch});out.write_text(json.dumps(result,indent=2),encoding='utf-8')
for path in ['/actions/permissions','/actions/permissions/workflow','/actions/permissions/selected-actions','/actions/runs?branch='+urllib.parse.quote(branch,safe='')+'&per_page=20','/commits/'+head+'/check-suites','/actions/workflows?per_page=50','/commits/'+head+'/check-runs','/commits/'+head+'/status']:
 item=call(path);d=item.get('data')
 if isinstance(d,dict):
  for key,fields in [('workflow_runs',['id','name','event','head_sha','head_branch','status','conclusion','html_url']),('workflows',['id','name','path','state']),('check_suites',['id','status','conclusion']),('check_runs',['id','name','status','conclusion','html_url'])]:
   if key in d:item['data']={'total_count':d.get('total_count'),key:[{k:x.get(k) for k in fields} for x in d[key]]};break
  if path.endswith('/status'):item['data']={k:d.get(k) for k in ['state','sha','total_count','statuses']}
 result['read_only'][path]=item
out.write_text(json.dumps(result,indent=2),encoding='utf-8');print(json.dumps({'head':head,'dispatch':result['dispatch'],'audit_saved':True}))
