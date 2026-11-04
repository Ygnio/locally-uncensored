// Actual isolated Tauri WebView, shared stores, canonical native backend and real models.
import { createRequire } from 'node:module'
import { readFileSync, writeFileSync, existsSync } from 'node:fs'
import { resolve, join } from 'node:path'
import assert from 'node:assert/strict'
const root=resolve('.lucille-qa-runtime/lucille-synthetic-real22')
assert.equal(readFileSync(join(root,'.lucille-isolated'),'utf8'),'synthetic-data-only\n')
const {chromium}=createRequire(import.meta.url)('@playwright/test')
const browser=await chromium.connectOverCDP('http://127.0.0.1:15287')
let page
for(let i=0;i<120;i++){page=browser.contexts().flatMap(c=>c.pages()).find(p=>p.url()==='http://127.0.0.1:15283/');if(page)break;await new Promise(r=>setTimeout(r,500))}
assert(page,'Dedicated WebView loaded')
await page.getByRole('button',{name:'Settings',exact:true}).waitFor({timeout:60000})
await page.evaluate(async()=>{globalThis.__p22=Object.assign({},...await Promise.all([import('/src/stores/chatStore.ts'),import('/src/stores/codexStore.ts'),import('/src/stores/modelStore.ts'),import('/src/stores/agentModeStore.ts'),import('/src/stores/uiStore.ts'),import('/src/stores/generationStore.ts'),import('/src/stores/settingsStore.ts'),import('/src/lib/lucille-context.ts')]))})
const stage=process.argv[2]??'long20'
const stateFile=join(root,'reports',`gui-${stage}.json`)
assert(!existsSync(stateFile),'Never overwrite completed evidence')
const fixture={stage,scope:'p22-'+crypto.randomUUID().slice(0,8),turns:[],started:new Date().toISOString()}
const scope=fixture.scope,project='cedro'
async function poll(fn,label,limit=480){for(let i=0;i<limit;i++){if(await fn())return;await new Promise(r=>setTimeout(r,250))}throw Error('Timeout '+label)}
const manage=request=>page.evaluate(request=>globalThis.__p22.lucilleManage(request),request)
const details=()=>manage({operation:'details',scope,project})
const metrics=()=>manage({operation:'learning_metrics',scope,project})
async function settings(){await page.evaluate(()=>globalThis.__p22.useUIStore.getState().openSettingsAt({tab:'agent'}));const load=page.getByRole('button',{name:'Load identity and scoped records',exact:true});if(!await load.isVisible())await page.getByRole('button',{name:'Memory',exact:true}).click();await load.click()}
async function conversation(mode='chat'){
 return page.evaluate(({mode,scope,project})=>{const p=globalThis.__p22;p.useModelStore.getState().setActiveModel('openai::lucille-real-gemma');p.useCodexStore.getState().setChatMode(mode==='code'?'codex':'lu');const id=p.useChatStore.getState().createConversation('openai::lucille-real-gemma','',mode==='code'?'codex':'lu');p.useChatStore.getState().setConversationLucille(id,{profile:'lucille',scope,project,tokenBudget:8192});if(mode==='agent')p.useAgentModeStore.getState().setAgentModeActive(id,true);p.useUIStore.getState().setView('chat');return id},{mode,scope,project})
}
async function send(text){
 await page.evaluate(()=>globalThis.__p22.useUIStore.getState().setView('chat'))
 const before=await page.evaluate(()=>{const p=globalThis.__p22;return p.useChatStore.getState().conversations.find(c=>c.id===p.useChatStore.getState().activeConversationId).messages.map(m=>m.id)})
 const started=Date.now();await page.locator('textarea').first().fill(text);await page.getByRole('button',{name:'Send message',exact:true}).click()
 await poll(()=>page.evaluate(before=>{const p=globalThis.__p22,c=p.useChatStore.getState().conversations.find(c=>c.id===p.useChatStore.getState().activeConversationId);return c.messages.some(m=>m.role==='assistant'&&!before.includes(m.id)&&m.content.length&&(m.finishReason||m.usage))&&!Object.keys(p.useGenerationStore.getState().aborters).length},before),'actual reply')
 await new Promise(r=>setTimeout(r,600))
 await poll(async()=>{const jobs=await manage({operation:'learning_jobs',scope,project});return jobs.every(j=>['done','cancelled','failed'].includes(j.state))},'extraction drain')
 await new Promise(r=>setTimeout(r,400))
 const result=await page.evaluate(()=>{const p=globalThis.__p22,c=p.useChatStore.getState().conversations.find(c=>c.id===p.useChatStore.getState().activeConversationId);return {conversation_id:c.id,messages:c.messages.slice(-2)} })
 fixture.turns.push({text,...result,elapsed_ms:Date.now()-started,metrics:await metrics()});writeFileSync(join(root,'reports',`gui-${stage}-progress.json`),JSON.stringify(fixture,null,2));console.log(fixture.turns.length,text.slice(0,65),Date.now()-started)
 return result
}
async function approveAll(){
 await settings()
 for(let i=0;i<12;i++){
  const rows=await details();const row=rows.find(d=>['candidate','conflict'].includes(d.memory.state)&&['project','decision','hardware','goal'].includes(d.memory.input.kind))
  if(!row)return
  await page.getByLabel('Filter Lucille category').selectOption(row.memory.input.kind)
  await page.getByLabel('Filter Lucille state').selectOption(row.memory.state)
  await page.getByRole('button',{name:row.memory.state==='conflict'?'Approve replacement':'approve',exact:true}).first().click()
  await new Promise(r=>setTimeout(r,400))
 }
 throw Error('review did not drain')
}
function lastInference(question){
 const calls=readFileSync(join(root,'reports','real-broker-calls.ndjson'),'utf8').trim().split('\n').map(s=>JSON.parse(s))
 return calls.findLast(c=>c.kind==='gemma'&&!c.request.messages?.[0]?.content?.startsWith('LUCILLE_ATOMIC_EXTRACTION_')&&c.request.messages?.some(m=>m.role==='user'&&m.content===question))
}
async function probe(item){
 await conversation(item.mode??'chat');await send(item.text)
 const call=lastInference(item.text);assert(call,'Actual primary inference captured')
 const context=call.request.messages.filter(m=>typeof m.content==='string'&&m.content.startsWith('Retrieved contextual data')).map(m=>m.content).join('\n')
 const answer=call.response.choices[0].message.content
 const checks={retrieval_expected:item.context_all.every(v=>context.toLowerCase().includes(v.toLowerCase())),retrieval_forbidden:!(item.context_none??[]).some(v=>context.toLowerCase().includes(v.toLowerCase())),answer_expected:new RegExp(item.answer_pattern,'i').test(answer),answer_current:!(item.answer_none??[]).some(v=>new RegExp(v,'i').test(answer)),no_announcement:!/minhas mem[oó]rias|na mem[oó]ria salva|lembro que|I remember|stored preference/i.test(answer)}
 fixture.probes??=[];fixture.probes.push({item,context,answer,checks,usage:call.response.usage})
 // Preserve every scored failure; the final report decides the gate.
 console.log('PROBE',JSON.stringify(checks))
}
try{
 const plan=JSON.parse(readFileSync(resolve(process.argv[3]),'utf8'))
 fixture.plan=plan.id;fixture.plan_sha256=(await import('node:crypto')).createHash('sha256').update(readFileSync(resolve(process.argv[3]))).digest('hex')
 await page.evaluate(()=>globalThis.__p22.useSettingsStore.getState().updateSettings({thinkingEnabled:false,maxTokens:128,temperature:0}))
 await conversation();await settings()
 for(const label of ['Enable Lucille automatic learning','Enable safe autonomous preferences','Automatically update Lucille index']){const c=page.getByLabel(label,{exact:true});if(!await c.isChecked())await c.check();await new Promise(r=>setTimeout(r,350))}
 await page.getByLabel('Lucille extraction mode').selectOption('llm');await new Promise(r=>setTimeout(r,350))
 fixture.policy=await manage({operation:'learning_policy'});fixture.initial_metrics=await metrics()
 for(const turn of plan.turns){
  if(turn.new_session)await conversation(turn.mode??'chat')
  await send(turn.text)
  if(turn.review)await approveAll()
 }
 fixture.after_long=await details();await poll(async()=>(await metrics()).pending_vectors===0,'index drained')
 for(const probeItem of plan.probes)await probe(probeItem)
 fixture.final_metrics=await metrics();fixture.final_details=await details();fixture.identity=await manage({operation:'identity'});fixture.index=await manage({operation:'index_status'});fixture.deleted=await manage({operation:'deleted',scope,project});fixture.finished=new Date().toISOString()
 fixture.passed=fixture.probes.every(p=>Object.values(p.checks).every(Boolean))
 await page.screenshot({path:join(root,'reports',`gui-${stage}.png`),fullPage:true})
 writeFileSync(stateFile,JSON.stringify(fixture,null,2));console.log('COMPLETE',stage,'passed',fixture.passed)
}catch(error){fixture.error=String(error);writeFileSync(join(root,'reports',`gui-${stage}-failure-${Date.now()}.json`),JSON.stringify(fixture,null,2));throw error}finally{await browser.close()}
