// QA orchestration revision only: Agent/Code completion is usage-backed, Chat also has finishReason.
// Frozen extraction/retrieval sources and corpora are unchanged. Prior timing failures retained.
// Complete QA WebView, actual Gemma/Nomic requests, canonical native SQLite.
import { createRequire } from 'node:module'
import { readFileSync, writeFileSync } from 'node:fs'
import { resolve, join } from 'node:path'
import assert from 'node:assert/strict'
const root=resolve('.lucille-qa-runtime/lucille-synthetic-real21d')
assert.equal(readFileSync(join(root,'.lucille-isolated'),'utf8'),'synthetic-data-only\n')
const {chromium}=createRequire(import.meta.url)('@playwright/test')
const browser=await chromium.connectOverCDP('http://127.0.0.1:15287')
const page=browser.contexts().flatMap(c=>c.pages()).find(p=>p.url()==='http://127.0.0.1:15283/')
assert(page)
await page.getByRole('button',{name:'Settings',exact:true}).waitFor({timeout:60000})
await page.evaluate(async()=>{globalThis.__real=Object.assign({},...await Promise.all([import('/src/stores/chatStore.ts'),import('/src/stores/codexStore.ts'),import('/src/stores/modelStore.ts'),import('/src/stores/agentModeStore.ts'),import('/src/stores/uiStore.ts'),import('/src/stores/generationStore.ts'),import('/src/lib/lucille-context.ts')]))})
const stateFile=join(root,'reports','real-gui-fixture.json')
const stage=process.argv[2]??'stage'
let fixture=['stage','development'].includes(stage)?{scope:'p21d-'+crypto.randomUUID().slice(0,8),conversations:[],passed:false}:JSON.parse(readFileSync(stateFile,'utf8'))
const scope=fixture.scope
async function poll(fn,label,limit=240){for(let i=0;i<limit;i++){if(await fn())return;await new Promise(r=>setTimeout(r,250))}throw Error('Timeout: '+label)}
const details=()=>page.evaluate(scope=>globalThis.__real.lucilleManage({operation:'details',scope,project:'atlas'}),scope)
async function settings(){await page.evaluate(()=>globalThis.__real.useUIStore.getState().openSettingsAt({tab:'agent'}));const load=page.getByRole('button',{name:'Load identity and scoped records',exact:true});if(!await load.isVisible())await page.getByRole('button',{name:'Memory',exact:true}).click();await load.click()}
async function chat(){await page.evaluate(()=>globalThis.__real.useUIStore.getState().setView('chat'));await page.locator('textarea').first().waitFor()}
async function conversation(mode){const id=await page.evaluate(({mode,scope})=>{const p=globalThis.__real;p.useModelStore.getState().setActiveModel('openai::lucille-real-gemma');p.useCodexStore.getState().setChatMode(mode==='code'?'codex':'lu');const id=p.useChatStore.getState().createConversation('openai::lucille-real-gemma','',mode==='code'?'codex':'lu');p.useChatStore.getState().setConversationLucille(id,{profile:'lucille',scope,project:'atlas',tokenBudget:8192});if(mode==='agent')p.useAgentModeStore.getState().setAgentModeActive(id,true);p.useUIStore.getState().setView('chat');return id},{mode,scope});fixture.conversations.push(id);return id}
async function send(text){
 const expectedJobs=(await page.evaluate(scope=>globalThis.__real.lucilleManage({operation:'learning_jobs',scope,project:'atlas'}),scope)).length+(text.includes('?')?0:1)
 const before=await page.evaluate(()=>globalThis.__real.useChatStore.getState().conversations.find(c=>c.id===globalThis.__real.useChatStore.getState().activeConversationId).messages.map(m=>m.id))
 await chat();await page.locator('textarea').first().fill(text);await page.getByRole('button',{name:'Send message',exact:true}).click()
 await poll(async()=>page.evaluate(async({before,scope,expectedJobs})=>{const p=globalThis.__real;const c=p.useChatStore.getState().conversations.find(c=>c.id===p.useChatStore.getState().activeConversationId);const done=c.messages.some(m=>m.role==='assistant'&&!before.includes(m.id)&&m.content.length>0&&(m.finishReason||m.usage)&&!m.content.includes('[SIMULATED:'));const jobs=await p.lucilleManage({operation:'learning_jobs',scope,project:'atlas'});return done&&!Object.keys(p.useGenerationStore.getState().aborters).length&&jobs.length>=expectedJobs&&jobs.every(j=>['done','cancelled'].includes(j.state))},{before,scope,expectedJobs}),'real generation and durable extraction')
 return page.evaluate(()=>{const p=globalThis.__real,c=p.useChatStore.getState().conversations.find(c=>c.id===p.useChatStore.getState().activeConversationId),m=c.messages.findLast(m=>m.role==='user');return {conversation_id:c.id,message_id:m.lucilleInteractionId??m.id,user_text:m.content}})
}
async function approve(kind,state='candidate'){await settings();await page.getByLabel('Filter Lucille category').selectOption(kind);await page.getByLabel('Filter Lucille state').selectOption(state);await page.getByRole('button',{name:state==='conflict'?'Approve replacement':'approve',exact:true}).first().click();await new Promise(r=>setTimeout(r,350))}

const calls=()=>readFileSync(join(root,'reports','real-broker-calls.ndjson'),'utf8').trim().split('\n').map(s=>JSON.parse(s))
function assertGrounded(question,value,forbidden){
 const c=calls().findLast(c=>c.kind==='gemma'&&!c.request.messages?.[0]?.content?.startsWith('LUCILLE_ATOMIC_EXTRACTION_V2')&&c.request.messages?.some(m=>typeof m.content==='string'&&m.content.includes(question)))
 assert(c,'actual conversation inference captured')
 const context=c.request.messages.filter(m=>typeof m.content==='string'&&m.content.startsWith('Retrieved contextual data')).map(m=>m.content).join('\n')
 assert(context.includes(value),context);if(forbidden)assert(!context.includes(forbidden),context)
 const answer=c.response.choices[0].message.content
 assert(!/minhas mem[oó]rias|na mem[oó]ria|mem[oó]ria salva|lembro que|I remember|stored preference/i.test(answer),answer)
 fixture.replies??=[];fixture.replies.push({question,answer,context})
 return answer
}
async function index(){await settings();await page.getByRole('button',{name:'Build real Nomic test index',exact:true}).click();await page.getByText(/REAL Nomic index: \d+ current vectors/).waitFor({timeout:30000})}
if(['stage','development'].includes(stage)){
 await page.evaluate(async()=>{const p=globalThis.__real,old=await p.lucilleManage({operation:'identity'});await p.lucilleManage({operation:'configure_identity',expected_version:old.version,profile:{name:'Lucille Synthetic Real QA',description:'Fictional test identity only',personality:'Concise',instructions:'Answer briefly in the language of the question. Use relevant contextual data quietly. Do not invoke tools for these synthetic memory checks.'}},true)})
 await conversation('chat');await settings();await page.getByLabel('Lucille extraction mode').selectOption('llm');const enabled=page.getByLabel('Enable Lucille automatic learning');if(!await enabled.isChecked())await enabled.check();await new Promise(r=>setTimeout(r,400))
 await send('Eu prefiro café sem açúcar.');assert((await details()).some(d=>d.memory.state==='candidate'&&d.quality?.value==='without_sugar'));await approve('preference');await index()
 await conversation('chat');const q='Would I choose a sugar-free espresso?';await send(q);assert(/without sugar|sugar.free|unsweetened|yes/i.test(assertGrounded(q,'sem açúcar')))
 if(stage==='development'){writeFileSync(join(root,'reports','real-gui-development.json'),JSON.stringify({passed:true,scope,replies:fixture.replies},null,2));console.log('PASS development: real GUI V2 candidate -> approval -> Nomic v3 -> new-conversation paraphrase -> actual grounded reply.')}
 else {
 await conversation('agent');fixture.correction=await send('Na verdade, agora prefiro café com açúcar.');await approve('preference','conflict');await index()
 await conversation('code');const q='Should I put sugar in my espresso?';await send(q);assert(/sugar|açúcar/i.test(assertGrounded(q,'com açúcar','sem açúcar')))
 await settings();await page.getByLabel('Filter Lucille category').selectOption('preference');await page.getByLabel('Filter Lucille state').selectOption('active');await page.getByRole('button',{name:'delete',exact:true}).click();await new Promise(r=>setTimeout(r,350))
 await page.evaluate(async({correction,scope})=>globalThis.__TAURI_INTERNALS__.invoke('lucille_learn',{interaction:{...correction,scope,project:'atlas',mode:'agent',role:'user'}}),{correction:fixture.correction,scope})
 await send('Eu prefiro café com açúcar.');assert(!(await details()).some(d=>d.memory.state==='active'&&d.memory.input.kind==='preference'))
 await conversation('agent');await send('Meu notebook do trabalho tem 12 GB de RAM.');await approve('hardware')
 await conversation('code');await send('Meu computador principal tem 20 GB de RAM.');await approve('hardware')
 await send('Atualizei o computador principal para 40 GB.');await approve('hardware','conflict');await index()
 const q2='How much memory is installed in my main desktop?';await send(q2);assert(/40\s*GB/i.test(assertGrounded(q2,'40 GB','20 GB')))
 fixture.identity=(await page.evaluate(()=>globalThis.__real.lucilleManage({operation:'identity'}))).version
 fixture.active=(await details()).filter(d=>d.memory.state==='active').map(d=>({id:d.memory.input.id,version:d.memory.version,value:d.quality?.value}))
 fixture.real_index=await page.evaluate(()=>globalThis.__real.lucilleManage({operation:'index_status'}));assert.equal(fixture.real_index.space.index_version,3)
 writeFileSync(stateFile,JSON.stringify(fixture,null,2));await page.screenshot({path:join(root,'reports','real-gui-stage.png'),fullPage:true});console.log('PASS: real V2 Chat/Agent/Code, paraphrase/answer, correction/reindex, deletion/replay, separate devices; staged reopen.')
 }
}else if(stage==='verify'){
 const result=await page.evaluate(async({conversations})=>({identity:(await globalThis.__real.lucilleManage({operation:'identity'})).version,index:await globalThis.__real.lucilleManage({operation:'index_status'}),bindings:globalThis.__real.useChatStore.getState().conversations.filter(c=>conversations.includes(c.id)).map(c=>c.lucille)}),fixture)
 assert.equal(result.identity,fixture.identity);assert.equal(result.bindings.length,fixture.conversations.length);assert(result.bindings.every(b=>b.scope===scope));assert.equal(result.index.indexed,fixture.real_index.indexed)
 assert.deepEqual((await details()).filter(d=>d.memory.state==='active').map(d=>({id:d.memory.input.id,version:d.memory.version,value:d.quality?.value})),fixture.active)
 assert(!(await details()).some(d=>d.memory.input.kind==='preference'&&d.memory.state==='active'))
 await page.evaluate(id=>globalThis.__real.useChatStore.getState().setActiveConversation(id),fixture.conversations.at(-1));const q='Qual é a capacidade de RAM do computador principal?';await send(q);assert(/40\s*GB/i.test(assertGrounded(q,'40 GB','20 GB')))
 fixture.passed=true;fixture.real_native_reopen=true;fixture.deleted_replay_fenced=true;fixture.real_gemma=true;fixture.real_nomic=true;fixture.short_naturalness_only=true
 writeFileSync(join(root,'reports','real-gui-results.json'),JSON.stringify(fixture,null,2));console.log('PASS: restarted native GUI retained index/identity/bindings/correction/deletion and answered from current memory.')
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
 const calls=readFileSync(join(root,'reports','real-broker-calls.ndjson'),'utf8').trim().split('\n').map(s=>JSON.parse(s));const real=calls.findLast(c=>c.kind==='gemma'&&c.request.messages?.[0]?.content?.startsWith('LUCILLE_ATOMIC_EXTRACTION_V2'))
 assert.equal(JSON.parse(real.request.messages.at(-1).content).user_text,'Prefiro café com açúcar.')
 writeFileSync(join(root,'reports','real-gui-cancellation.json'),JSON.stringify({passed:true,real_inference_completed:true,native_job_cancelled:true,late_commit_fenced:true,source_purged:true,note:'Bounded broker call can finish after native cancellation; cancellation fences writes, not immediate GPU preemption.'},null,2));console.log('PASS: real in-flight extraction cancelled; late model output wrote nothing and source was purged.')
}else if(stage==='fallback'){
 const before=await page.evaluate(scope=>globalThis.__real.lucilleManage({operation:'learning_jobs',scope,project:'atlas'}),scope)
 await page.evaluate(id=>globalThis.__real.useChatStore.getState().setActiveConversation(id),fixture.conversations[0])
 await send('Quanto é 7 vezes 9?',before.length)
 const calls=readFileSync(join(root,'reports','real-broker-calls.ndjson'),'utf8').trim().split('\n').map(s=>JSON.parse(s));const last=calls.findLast(c=>c.kind==='gemma'&&!c.request.messages?.[0]?.content?.startsWith('LUCILLE_ATOMIC_EXTRACTION_V2'))
 assert(!JSON.stringify(last.request.messages).includes('Retrieved contextual data'))
 await settings();await page.getByText(/lexical fallback; embedding endpoint unavailable or unverified/).waitFor();await page.getByRole('button',{name:'Build real Nomic test index',exact:true}).scrollIntoViewIfNeeded();await page.screenshot({path:join(root,'reports','real-gui-fallback.png'),fullPage:true})
 writeFileSync(join(root,'reports','real-gui-fallback.json'),JSON.stringify({passed:true,real_gemma_answer:true,nomic_owned_instance_stopped:true,lexical_fallback_visible:true,irrelevant_question_context_empty:true},null,2));console.log('PASS: unavailable owned Nomic -> visible lexical fallback; real Gemma answered irrelevant question with empty retrieved context.')
}else throw Error('stage, development, verify, cancel or fallback required')
await browser.close()
