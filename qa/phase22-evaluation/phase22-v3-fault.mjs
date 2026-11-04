// Actual isolated GUI faults. No production endpoints or operating-system input.
import {createRequire} from 'node:module'
import {readFileSync,writeFileSync,existsSync} from 'node:fs'
import {resolve,join} from 'node:path'
import {execFileSync} from 'node:child_process'
import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
const repo=resolve(process.cwd())
const root=join(repo,'.lucille-qa-runtime/lucille-synthetic-real22'),reports=join(root,'reports')
assert.equal(readFileSync(join(root,'.lucille-isolated'),'utf8'),'synthetic-data-only\n')
const kind=process.argv[2],suffix=process.argv[3]??'1';assert(['cancel','restart'].includes(kind)&&/^\d+$/.test(suffix))
const label=`v3-${kind}-fault-${suffix}`,out=join(reports,`${label}-fault.json`);assert(!existsSync(out))
const lifecycle=JSON.parse(readFileSync(join(repo,'crates/lucille-memory/tests/fixtures/phase22/lifecycle-v3.json'),'utf8'))
const scenario=lifecycle[kind==='cancel'?'cancel':'restart_during_extraction'],scope=scenario.scope+'-'+suffix,project=lifecycle.project
const {chromium}=createRequire(join(repo,'package.json'))('@playwright/test')
const sleep=ms=>new Promise(r=>setTimeout(r,ms));let browser,page
async function connect(){
 browser=await chromium.connectOverCDP('http://127.0.0.1:15287');page=browser.contexts().flatMap(c=>c.pages()).find(p=>p.url()==='http://127.0.0.1:15283/');assert(page)
 assert(await page.evaluate(()=>Boolean(globalThis.__TAURI_INTERNALS__)))
 await page.evaluate(async()=>{globalThis.__fault22=Object.assign({},...await Promise.all([import('/src/lib/lucille-context.ts'),import('/src/stores/chatStore.ts'),import('/src/stores/uiStore.ts')]))})
}
const manage=request=>page.evaluate(request=>globalThis.__fault22.lucilleManage(request),request)
const scoped=operation=>manage({operation,scope,project})
async function wait(fn,timeout=180000){const start=Date.now();while(Date.now()-start<timeout){const v=await fn();if(v)return v;await sleep(50)}throw Error('Bounded fault timeout')}
function control(data){writeFileSync(join(reports,'control.json'),JSON.stringify({id:crypto.randomUUID(),scope,project,resume:true,...data}))}
function completedCall(after){
 const calls=readFileSync(join(reports,'real-broker-calls.ndjson'),'utf8').trim().split('\n').map(s=>JSON.parse(s))
 return calls.find(c=>c.completed_unix>=after&&(c.request.messages??[]).some(m=>m.role==='user'&&m.content.includes(scenario.source)&&m.content.includes(scope)))
}
const report={kind,label,scope,project,started:new Date().toISOString()}
let oldLease
function runNativeScript(script,args,action){
 const py="import subprocess,sys\nwith open(sys.argv[2],'wb') as log: subprocess.run(sys.argv[3:],cwd=sys.argv[1],stdin=subprocess.DEVNULL,stdout=log,stderr=subprocess.STDOUT,check=True,timeout=100)"
 execFileSync('python',['-c',py,repo,join(reports,`${label}-${action}-launcher.log`),'pwsh','-NoProfile','-File',script,...args],{cwd:repo,windowsHide:true,stdio:'ignore',timeout:120000})
}
try{
 await connect();const gm=JSON.parse(readFileSync(join(root,'gui-processes.json'),'utf8'));report.initial_gui=gm.gui
 control({label,plan:{id:label,turns:[],probes:[]},interrupt_source:scenario.source})
 const prepared=await wait(()=>existsSync(join(reports,`driver-${label}-prepared.json`))?JSON.parse(readFileSync(join(reports,`driver-${label}-prepared.json`),'utf8')):null)
 report.prepared=prepared;const jobid=prepared.interrupted_job.id
 const inflight=await wait(async()=>{
  const record=JSON.parse(readFileSync(join(reports,'inflight-gemma.json'),'utf8'))
  if(!record.extraction||record.scope!==scope)return null
  const job=(await scoped('learning_jobs')).find(j=>j.id===jobid)
  if(job?.state!=='running')return null
  return {request:record,job,observed_unix:Date.now()/1000}
 },15000)
 report.actual_inflight=inflight;report.before_details=await scoped('details')
 const sqlReader=`import sqlite3,json,sys\nfrom pathlib import Path\np=Path(sys.argv[1]);c=sqlite3.connect(p.resolve().as_uri()+'?mode=ro',uri=True);row=c.execute('SELECT id,lease_token,scope,project,evidence_json,attempts FROM extraction_jobs WHERE id=? AND state=?',(sys.argv[2],'running')).fetchone();assert row and row[1];print(json.dumps(dict(zip(['id','token','scope','project','claims','attempts'],[row[0],row[1],row[2],row[3],json.loads(row[4]),row[5]]))));c.close()`
 oldLease=JSON.parse(execFileSync('python',['-c',sqlReader,join(repo,'.lucille-qa-runtime/lucille-synthetic-full22/lucille-phase1.sqlite3'),jobid],{cwd:repo,windowsHide:true,encoding:'utf8'}))
 report.original_lease_sha256=createHash('sha256').update(JSON.stringify(oldLease)).digest('hex')
 if(kind==='cancel'){
  await page.getByRole('button',{name:'Refresh processing',exact:true}).click()
  const row=page.locator('li').filter({hasText:jobid.slice(0,8)})
  await row.getByRole('button',{name:'Cancel processing',exact:true}).click({timeout:3000})
  report.action_unix=Date.now()/1000
  report.cancelled_job=await wait(async()=>{const j=(await scoped('learning_jobs')).find(j=>j.id===jobid);return j?.state==='cancelled'?j:null},5000)
  report.completed_after_cancel=await wait(()=>completedCall(inflight.request.began_unix),120000)
  await sleep(2000);report.after_details=await scoped('details');report.final_metrics=await scoped('learning_metrics')
  report.canonical_unchanged=JSON.stringify(report.before_details)===JSON.stringify(report.after_details)
  report.actual_overlap=report.action_unix<report.completed_after_cancel.completed_unix
  report.old_callback_result=await page.evaluate(({lease,output})=>globalThis.__fault22.lucilleManage({operation:'complete_llm',lease,output},true),{lease:oldLease,output:report.completed_after_cancel.response.choices[0].message.content})
  report.old_callback_canonical_unchanged=JSON.stringify(report.after_details)===JSON.stringify(await scoped('details'))
 }else{
  report.stop_requested_unix=Date.now()/1000
  runNativeScript('scripts/stop-lucille-real-gui.ps1',['-Phase','22'],'stop')
  report.stopped_unix=Date.now()/1000;await browser.close();browser=null
  report.old_response=await wait(()=>completedCall(inflight.request.began_unix),120000)
  report.actual_overlap=report.stopped_unix<report.old_response.completed_unix
  runNativeScript('scripts/start-lucille-real-gui.ps1',['-Phase','22','-QaExe',join(repo,'../build/tauri/x86_64-pc-windows-msvc/debug/lucille-full-validation.exe')],'start')
  await connect();report.reopened_unix=Date.now()/1000;report.reopened_gui=JSON.parse(readFileSync(join(root,'gui-processes.json'),'utf8')).gui
  report.recovered_before_probe=await wait(async()=>{const j=(await scoped('learning_jobs')).find(j=>j.id===jobid);return j&&['done','failed','cancelled'].includes(j.state)?j:null},180000)
  await wait(async()=>(await scoped('learning_metrics')).pending_vectors===0,60000)
  const beforeOldCallback=await scoped('details')
  report.old_callback_result=await page.evaluate(({lease,output})=>globalThis.__fault22.lucilleManage({operation:'complete_llm',lease,output},true),{lease:oldLease,output:report.old_response.response.choices[0].message.content})
  report.old_callback_canonical_unchanged=JSON.stringify(beforeOldCallback)===JSON.stringify(await scoped('details'))
  const probeLabel=`${label}-after`
  control({label:probeLabel,plan:{id:probeLabel,turns:[],probes:[{text:'Qual Ã© meu idioma preferido para conversar?',expected:[{property:'response_language',value:'pt'}],answer_pattern:'portugu',category:'response_language'}]}})
  const final=await wait(()=>existsSync(join(reports,`driver-${probeLabel}-final.json`))?JSON.parse(readFileSync(join(reports,`driver-${probeLabel}-final.json`),'utf8')):null,240000)
  report.after=final;report.recovered_job=(await scoped('learning_jobs')).find(j=>j.id===jobid);report.after_details=await scoped('details');report.final_metrics=await scoped('learning_metrics')
 }
 report.finished=new Date().toISOString();report.execution_complete=true
}catch(error){report.error=String(error);report.finished=new Date().toISOString();process.exitCode=1}
finally{writeFileSync(out,JSON.stringify(report,null,2));if(browser)await browser.close();console.log(JSON.stringify({kind,label,error:report.error,actual_overlap:report.actual_overlap,canonical_unchanged:report.canonical_unchanged,job:report.recovered_job??report.cancelled_job}))}
