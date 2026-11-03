// Complete QA WebView, actual Gemma/Nomic requests, canonical native SQLite.
import { createRequire } from 'node:module'
import { readFileSync, writeFileSync } from 'node:fs'
import { resolve, join } from 'node:path'
import assert from 'node:assert/strict'
const root=resolve('.lucille-qa-runtime/lucille-synthetic-real21c')
assert.equal(readFileSync(join(root,'.lucille-isolated'),'utf8'),'synthetic-data-only\n')
const {chromium}=createRequire(import.meta.url)('@playwright/test')
const browser=await chromium.connectOverCDP('http://127.0.0.1:15287')
const page=browser.contexts().flatMap(c=>c.pages()).find(p=>p.url()==='http://127.0.0.1:15283/')
assert(page)
await page.getByRole('button',{name:'Settings',exact:true}).waitFor({timeout:60000})
await page.evaluate(async()=>{globalThis.__real=Object.assign({},...await Promise.all([import('/src/stores/chatStore.ts'),import('/src/stores/codexStore.ts'),import('/src/stores/modelStore.ts'),import('/src/stores/agentModeStore.ts'),import('/src/stores/uiStore.ts'),import('/src/stores/generationStore.ts'),import('/src/lib/lucille-context.ts')]))})
const stateFile=join(root,'reports','real-gui-fixture.json')
const stage=process.argv[2]??'stage'
let fixture=stage==='stage'?{scope:'p21c-'+crypto.randomUUID().slice(0,8),conversations:[],passed:false}:JSON.parse(readFileSync(stateFile,'utf8'))
const scope=fixture.scope
async function poll(fn,label,limit=240){for(let i=0;i<limit;i++){if(await fn())return;await new Promise(r=>setTimeout(r,250))}throw Error('Timeout: '+label)}
const details=()=>page.evaluate(scope=>globalThis.__real.lucilleManage({operation:'details',scope,project:'atlas'}),scope)
async function settings(){await page.evaluate(()=>globalThis.__real.useUIStore.getState().openSettingsAt({tab:'agent'}));const load=page.getByRole('button',{name:'Load identity and scoped records',exact:true});if(!await load.isVisible())await page.getByRole('button',{name:'Memory',exact:true}).click();await load.click()}
async function chat(){await page.evaluate(()=>globalThis.__real.useUIStore.getState().setView('chat'));await page.locator('textarea').first().waitFor()}
async function conversation(mode){const id=await page.evaluate(({mode,scope})=>{const p=globalThis.__real;p.useModelStore.getState().setActiveModel('openai::lucille-real-gemma');p.useCodexStore.getState().setChatMode(mode==='code'?'codex':'lu');const id=p.useChatStore.getState().createConversation('openai::lucille-real-gemma','',mode==='code'?'codex':'lu');p.useChatStore.getState().setConversationLucille(id,{profile:'lucille',scope,project:'atlas',tokenBudget:8192});if(mode==='agent')p.useAgentModeStore.getState().setAgentModeActive(id,true);p.useUIStore.getState().setView('chat');return id},{mode,scope});fixture.conversations.push(id);return id}
async function send(text,expectedJobs){
 const before=await page.evaluate(()=>globalThis.__real.useChatStore.getState().conversations.find(c=>c.id===globalThis.__real.useChatStore.getState().activeConversationId).messages.map(m=>m.id))
 await chat();await page.locator('textarea').first().fill(text);await page.getByRole('button',{name:'Send message',exact:true}).click()
 await poll(async()=>page.evaluate(async({before,scope,expectedJobs})=>{const p=globalThis.__real;const c=p.useChatStore.getState().conversations.find(c=>c.id===p.useChatStore.getState().activeConversationId);const done=c.messages.some(m=>m.role==='assistant'&&!before.includes(m.id)&&m.content.length>0&&!m.content.includes('[SIMULATED:'));const jobs=await p.lucilleManage({operation:'learning_jobs',scope,project:'atlas'});return done&&!Object.keys(p.useGenerationStore.getState().aborters).length&&jobs.length>=expectedJobs&&jobs.every(j=>['done','cancelled'].includes(j.state))},{before,scope,expectedJobs}),'real generation and durable extraction')
 return page.evaluate(()=>{const p=globalThis.__real,c=p.useChatStore.getState().conversations.find(c=>c.id===p.useChatStore.getState().activeConversationId),m=c.messages.findLast(m=>m.role==='user');return {conversation_id:c.id,message_id:m.lucilleInteractionId??m.id,user_text:m.content}})
}
async function approve(kind,state='candidate'){await settings();await page.getByLabel('Filter Lucille category').selectOption(kind);await page.getByLabel('Filter Lucille state').selectOption(state);await page.getByRole('button',{name:state==='conflict'?'Approve replacement':'approve',exact:true}).first().click();await new Promise(r=>setTimeout(r,350))}
if(stage==='stage'){
 await page.evaluate(async()=>{const p=globalThis.__real,old=await p.lucilleManage({operation:'identity'});await p.lucilleManage({operation:'configure_identity',expected_version:old.version,profile:{name:'Lucille Synthetic Real QA',description:'Fictional test identity only',personality:'Concise',instructions:'Use only synthetic QA data. Do not invoke tools for these memory checks.'}},true)})
 await conversation('chat');await settings();await page.getByLabel('Lucille extraction mode').selectOption('llm');const enabled=page.getByLabel('Enable Lucille automatic learning');if(!await enabled.isChecked())await enabled.check();await new Promise(r=>setTimeout(r,400))
 await send('Prefiro café sem açúcar.',1)
 assert((await details()).some(d=>d.memory.state==='active'&&d.quality?.value==='without_sugar'))
 await settings();await page.getByLabel('Lucille extraction mode').selectOption('hybrid');await new Promise(r=>setTimeout(r,400))
 await conversation('agent');await send('Meu notebook do trabalho tem 8 GB de RAM.',2);await approve('hardware')
 await conversation('code');await send('Meu computador principal tem 16 GB de RAM.',3);await approve('hardware')
 await send('Atualizei o computador principal para 32 GB.',4);await approve('hardware','conflict')
 let records=await details();assert(records.some(d=>d.memory.state==='active'&&d.quality?.subject==='device.primary_computer'&&d.quality.value==='32_gb'));assert(records.some(d=>d.memory.state==='active'&&d.quality?.subject==='device.work_laptop'&&d.quality.value==='8_gb'))
 fixture.withdrawal=await send('Não gosto mais de café.',5);await approve('preference','conflict')
 records=await details();assert(records.some(d=>d.memory.state==='active'&&d.quality?.value==='dislikes_coffee'));assert(records.some(d=>d.memory.state==='obsolete'&&d.quality?.value==='without_sugar'))
 await settings();await page.getByRole('button',{name:'Build real Nomic test index',exact:true}).click();await page.getByText(/REAL Nomic index: \d+ current vectors/).waitFor({timeout:30000});await page.getByRole('button',{name:'Build real Nomic test index',exact:true}).scrollIntoViewIfNeeded();await page.screenshot({path:join(root,'reports','real-gui-index.png'),fullPage:true})
 fixture.identity=(await page.evaluate(()=>globalThis.__real.lucilleManage({operation:'identity'}))).version
 fixture.active=records.filter(d=>d.memory.state==='active').map(d=>({id:d.memory.input.id,version:d.memory.version,value:d.quality?.value}))
 fixture.real_index=await page.evaluate(()=>globalThis.__real.lucilleManage({operation:'index_status'}))
 assert.equal(fixture.real_index.space.fingerprint,'f7af6f66802f4df86eda10fe9bbcfc75c39562bed48ef6ace719a251cf1c2fdb')
 writeFileSync(stateFile,JSON.stringify(fixture,null,2));console.log('PASS: real complete Chat/Agent/Code, LLM coffee + hybrid corrections, device separation and actual Nomic index; staged native reopen.')
}else if(stage==='verify'){
 const result=await page.evaluate(async({conversations})=>({identity:(await globalThis.__real.lucilleManage({operation:'identity'})).version,index:await globalThis.__real.lucilleManage({operation:'index_status'}),bindings:globalThis.__real.useChatStore.getState().conversations.filter(c=>conversations.includes(c.id)).map(c=>c.lucille)}),fixture)
 assert.equal(result.identity,fixture.identity);assert.equal(result.bindings.length,3);assert(result.bindings.every(b=>b.scope===scope));assert.equal(result.index.indexed,fixture.real_index.indexed)
 assert.deepEqual((await details()).filter(d=>d.memory.state==='active').map(d=>({id:d.memory.input.id,version:d.memory.version,value:d.quality?.value})),fixture.active)
 await page.evaluate(id=>globalThis.__real.useChatStore.getState().setActiveConversation(id),fixture.conversations[0])
 await send('Qual é a capacidade de RAM do computador principal?',5)
 const correctionCalls=readFileSync(join(root,'reports','real-broker-calls.ndjson'),'utf8').trim().split('\n').map(s=>JSON.parse(s))
 const correction=correctionCalls.findLast(c=>c.kind==='gemma'&&c.request.messages?.some(m=>String(m.content).includes('Qual é a capacidade de RAM do computador principal?')))
 const retrieved=correction.request.messages.filter(m=>typeof m.content==='string'&&m.content.startsWith('Retrieved contextual data')).map(m=>m.content).join('\n')
 assert(retrieved.includes('32 GB')&&!retrieved.includes('16 GB'));assert(/32\s*GB/i.test(correction.response.choices[0].message.content))
 await settings();await page.getByLabel('Filter Lucille category').selectOption('preference');await page.getByLabel('Filter Lucille state').selectOption('active');await page.getByRole('button',{name:'delete',exact:true}).click();await new Promise(r=>setTimeout(r,350))
 await page.evaluate(async({withdrawal,scope})=>globalThis.__TAURI_INTERNALS__.invoke('lucille_learn',{interaction:{...withdrawal,scope,project:'atlas',mode:'chat',role:'user'}}),{withdrawal:fixture.withdrawal,scope})
 assert(!(await details()).some(d=>d.memory.state==='active'&&d.memory.input.kind==='preference'))
 fixture.passed=true;fixture.real_native_reopen=true;fixture.deleted_replay_fenced=true;fixture.real_gemma=true;fixture.real_nomic=true;fixture.llm_quality_approved=false;fixture.retrieval_quality_approved=false
 writeFileSync(join(root,'reports','real-gui-results.json'),JSON.stringify(fixture,null,2));console.log('PASS: native reopening retained actual vector index/identity/bindings, real generation after reopen, deleted preference did not reappear.')
}else if(stage==='cancel'){
 const before=await details()
 await settings();await page.getByLabel('Lucille extraction mode').selectOption('llm');await new Promise(r=>setTimeout(r,350))
 const job=await page.evaluate(async({scope,conversation})=>{const job=await globalThis.__TAURI_INTERNALS__.invoke('lucille_learn',{interaction:{conversation_id:conversation,message_id:crypto.randomUUID(),scope,project:'atlas',mode:'chat',role:'user',user_text:'Prefiro café com açúcar.'}});void import('/src/lib/lucille-llm.ts').then(m=>m.tickLucilleLlm());return job},{scope,conversation:fixture.conversations[0]})
 await poll(async()=>page.evaluate(async({scope,id})=>(await globalThis.__real.lucilleManage({operation:'learning_jobs',scope,project:'atlas'})).find(j=>j.id===id)?.state==='running',{scope,id:job.id}),'real extractor lease running',80)
 await page.getByRole('button',{name:'Refresh processing',exact:true}).click();await page.getByRole('button',{name:'Cancel processing',exact:true}).first().click()
 await poll(async()=>page.evaluate(async({scope,id})=>(await globalThis.__real.lucilleManage({operation:'learning_jobs',scope,project:'atlas'})).find(j=>j.id===id)?.state==='cancelled',{scope,id:job.id}),'cancelled job durable')
 await new Promise(r=>setTimeout(r,6000))
 assert.deepEqual(await details(),before)
 const purged=await page.evaluate(async id=>{const backup=JSON.parse(await globalThis.__real.lucilleManage({operation:'export'},true));return !backup.intelligent.tables.learning_inputs.some(row=>row[0]===id)&&backup.intelligent.tables.extraction_jobs.find(row=>row[0]===id)[8]==='[]'},job.id)
 assert(purged)
 const calls=readFileSync(join(root,'reports','real-broker-calls.ndjson'),'utf8').trim().split('\n').map(s=>JSON.parse(s));const real=calls.findLast(c=>c.kind==='gemma'&&c.request.messages?.[0]?.content?.startsWith('LUCILLE_STRUCTURED_EXTRACTION_V1'))
 assert.equal(JSON.parse(real.request.messages.at(-1).content).user_text,'Prefiro café com açúcar.')
 writeFileSync(join(root,'reports','real-gui-cancellation.json'),JSON.stringify({passed:true,real_inference_completed:true,native_job_cancelled:true,late_commit_fenced:true,source_purged:true,note:'Bounded broker call can finish after native cancellation; cancellation fences writes, not immediate GPU preemption.'},null,2));console.log('PASS: real in-flight extraction cancelled; late model output wrote nothing and source was purged.')
}else if(stage==='fallback'){
 const before=await page.evaluate(scope=>globalThis.__real.lucilleManage({operation:'learning_jobs',scope,project:'atlas'}),scope)
 await page.evaluate(id=>globalThis.__real.useChatStore.getState().setActiveConversation(id),fixture.conversations[0])
 await send('Quanto é 7 vezes 9?',before.length)
 const calls=readFileSync(join(root,'reports','real-broker-calls.ndjson'),'utf8').trim().split('\n').map(s=>JSON.parse(s));const last=calls.findLast(c=>c.kind==='gemma'&&!c.request.messages?.[0]?.content?.startsWith('LUCILLE_STRUCTURED_EXTRACTION_V1'))
 assert(!JSON.stringify(last.request.messages).includes('Retrieved contextual data'))
 await settings();await page.getByText(/lexical fallback; embedding endpoint unavailable or unverified/).waitFor();await page.getByRole('button',{name:'Build real Nomic test index',exact:true}).scrollIntoViewIfNeeded();await page.screenshot({path:join(root,'reports','real-gui-fallback.png'),fullPage:true})
 writeFileSync(join(root,'reports','real-gui-fallback.json'),JSON.stringify({passed:true,real_gemma_answer:true,nomic_owned_instance_stopped:true,lexical_fallback_visible:true,irrelevant_question_context_empty:true},null,2));console.log('PASS: unavailable owned Nomic -> visible lexical fallback; real Gemma answered irrelevant question with empty retrieved context.')
}else throw Error('stage, verify, cancel or fallback required')
await browser.close()
