import { useState, useEffect } from 'react'
import { createRoot } from 'react-dom/client'
import { invoke } from '@tauri-apps/api/core'
import { lucilleManage, withLucilleContext, type LucilleMemory } from '../src/lib/lucille-context'
import { CODEX_SYSTEM_PROMPT } from '../src/lib/codex-prompts'
import { executeParallel } from '../src/api/agents/tool-executor'
import { executorEvidence, type ToolEvidence, type LucilleMode } from '../src/lib/verified-environment'
import type { ChatMessage } from '../src/api/providers/types'

const scope='lucille', project='synthetic-a'
function App() {
  const [profile,setProfile]=useState<{profile:{name:string};version:number}|null>(null)
  const [entries,setEntries]=useState<LucilleMemory[]>([])
  const [mode,setMode]=useState<LucilleMode>('chat')
  const [enabled,setEnabled]=useState(localStorage.getItem('qa-lucille-enabled')==='true')
  const [result,setResult]=useState('No request sent. No inference model is loaded.')
  const [payload,setPayload]=useState<ChatMessage[]>([])
  const [receipts,setReceipts]=useState<ToolEvidence[]>([])
  const [busy,setBusy]=useState(false)
  async function refresh(){
    try{setProfile(await lucilleManage({operation:'identity'}))}catch{setProfile(null)}
    setEntries(await lucilleManage({operation:'list',scope,project}))
  }
  useEffect(()=>{void refresh()},[])
  async function run(fn:()=>Promise<void>){setBusy(true);try{await fn()}catch(error){setResult(String(error))}finally{setBusy(false)}}
  async function seed(){
    await lucilleManage({operation:'configure_identity',expected_version:profile?.version??0,profile:{name:'Synthetic Aurora',description:'Fictional QA identity',personality:'Concise',instructions:'Use only synthetic examples.'}},true)
    const existing=await lucilleManage<LucilleMemory[]>({operation:'list',scope,project})
    if(!existing.length){const memory=await lucilleManage<LucilleMemory>({operation:'create',candidate:{id:crypto.randomUUID(),scope,project,kind:'fact',content:'The synthetic telescope is blue.',fact_key:'telescope.color',source:'manual',source_id:'qa-fixture'}},true)
      await lucilleManage({operation:'change',id:memory.input.id,scope,project,expected_version:memory.version,change:{action:'approve'}},true)}
    await refresh();setResult('Synthetic identity and memory stored in real SQLite. Refresh or restart this QA window to verify persistence.')
  }
  async function send(){
    const base:ChatMessage[]=[{role:'system',content:mode==='code'?CODEX_SYSTEM_PROMPT:'Operational mode: '+mode+'. Answer the current request.'},{role:'user',content:'What is your name, the current local date, and the color of the synthetic telescope?'}]
    const conversation={id:'qa-'+mode,...(enabled?{lucille:{profile:'lucille' as const,scope,project,tokenBudget:8192}}:{})}
    const tools=mode==='chat'?[]:['file_read','file_write','file_edit']
    const messages=await withLucilleContext(conversation,crypto.randomUUID(),'telescope',mode,base,undefined,undefined,{backend:'synthetic-loopback',model:'payload-inspector',availableTools:tools,receipts})
    setPayload(messages)
    const response=await fetch('http://127.0.0.1:15274/v1/chat/completions',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({model:'payload-inspector',messages,mode,tools})})
    if(!response.ok)throw new Error('Synthetic server unavailable')
    setResult((await response.json()).choices[0].message.content)
  }
  async function files(){
    const runtime={getTool:(name:string)=>['file_read','file_write','file_edit'].includes(name)?{name}:undefined,awaitApproval:async()=>true,
      execute:async(name:string,args:Record<string,unknown>)=>invoke<string>('qa_file',{operation:name,path:args.path,content:args.content??null})}
    const evidence:ToolEvidence[]=[]
    for(const [toolName,path,content] of [['file_write','fixture.txt','blue telescope'],['file_read','fixture.txt',undefined],['file_edit','fixture.txt',undefined],['file_read','fixture.txt',undefined],['file_read','../outside.txt',undefined]]){
      const results=await executeParallel([{id:crypto.randomUUID(),toolName:toolName!,args:{path,content}}],runtime)
      evidence.push(...results.map(executorEvidence))
    }
    setReceipts(evidence);setResult('Real synthetic filesystem calls completed. Escape request failed. See executor receipts; model prose is not evidence.')
  }
  async function change(action:'update'|'delete'){
    const memory=entries[0];if(!memory)throw new Error('No memory')
    const updated=await lucilleManage<LucilleMemory|null>({operation:'change',id:memory.input.id,scope,project,expected_version:memory.version,change:action==='update'?{action,content:'The synthetic telescope is green.',fact_key:'telescope.color'}:{action}},true)
    if(action==='update'&&updated)await lucilleManage({operation:'change',id:updated.input.id,scope,project,expected_version:updated.version,change:{action:'approve'}},true)
    await refresh();setResult('Memory '+action+' completed. Send a new request to inspect fresh context.')
  }
  return <main><h1>Lucille Phase 1.5 — isolated synthetic QA</h1><p className="notice">Real Tauri + SQLite + shared adapter/executor. Mock model server; payload validation only, no real inference. This is a dedicated integration surface, not the operational app.</p>
    <fieldset disabled={busy}><button onClick={()=>void run(seed)}>Initialize synthetic identity</button><button onClick={()=>void run(refresh)}>Refresh persisted state</button><p>Applied profile: {profile?.profile.name??'Not configured'} · version {profile?.version??0} · scope profile:lucille</p>
    <label><input type="checkbox" checked={enabled} onChange={event=>{setEnabled(event.target.checked);localStorage.setItem('qa-lucille-enabled',String(event.target.checked))}}/> Explicit Lucille profile binding</label>
    <p>{(['chat','agent','code'] as const).map(value=><label key={value}><input type="radio" name="mode" checked={mode===value} onChange={()=>setMode(value)}/>{value.toUpperCase()} </label>)}</p>
    <button onClick={()=>void run(send)}>Send synthetic request</button><button onClick={()=>void run(files)}>Verify real file tools</button>
    <h2>Canonical synthetic memory</h2><pre>{entries.map(entry=>`${entry.state} v${entry.version}: ${entry.input.content}`).join('\n')||'No records'}</pre>
    <button onClick={()=>void run(()=>change('update'))}>Update synthetic memory</button><button onClick={()=>void run(()=>change('delete'))}>Delete synthetic memory</button></fieldset>
    <h2>Result / verified execution</h2><pre>{result}</pre><pre>{JSON.stringify(receipts,null,2)}</pre><h2>Captured send payload</h2><pre>{JSON.stringify(payload,null,2)}</pre></main>
}
const style=document.createElement('style');style.textContent='body{margin:0;background:#101522;color:#e9effc;font:16px Segoe UI,sans-serif}main{max-width:1020px;margin:24px auto;padding:20px}h1{font-size:26px}h2{font-size:19px;color:#b7c9ff}.notice{background:#24324b;padding:12px;border-radius:8px}fieldset{border:1px solid #516587;border-radius:8px;padding:16px}button{background:#d4dfff;color:#15203d;border:0;padding:10px;margin:5px;border-radius:5px;font-weight:600}label{padding:8px}pre{white-space:pre-wrap;overflow-wrap:anywhere;background:#1b2437;padding:12px;font:13px Consolas,monospace}';document.head.append(style)
createRoot(document.getElementById('root')!).render(<App/> )
