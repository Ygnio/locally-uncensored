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
const stage=process.argv[2]??'development'
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
try{
 await page.evaluate(async()=>{const p=globalThis.__p22;p.useSettingsStore.getState().updateSettings({thinkingEnabled:false,maxTokens:192,temperature:0});let old;try{old=await p.lucilleManage({operation:'identity'})}catch{old={version:0}}await p.lucilleManage({operation:'configure_identity',expected_version:old.version,profile:{name:'Lucille Synthetic Phase22',description:'Fictional QA identity',personality:'Concise',instructions:'Answer in at most two short sentences in the question language. Use relevant contextual data quietly. Do not invoke tools for these synthetic memory tests.'}},true)})
 await conversation();await settings()
 await page.getByLabel('Lucille extraction mode').selectOption('llm');await new Promise(r=>setTimeout(r,300))
 for(const label of ['Enable Lucille automatic learning','Enable safe autonomous preferences','Automatically update Lucille index']){const c=page.getByLabel(label,{exact:true});if(!await c.isChecked())await c.check();await new Promise(r=>setTimeout(r,350))}
 fixture.policy=await manage({operation:'learning_policy'})
 await send('Para conversar comigo, prefiro respostas curtas e diretas.')
 fixture.after_preference=await details();assert(fixture.after_preference.some(d=>d.memory.state==='active'),'safe preference auto-promoted')
 await poll(async()=>{const m=await metrics();return m.pending_vectors===0},'automatic Nomic index')
 await conversation('agent');await send('Como seria melhor explicar um algoritmo para mim?')
 await conversation('code');await send('Estou começando um projeto de automação residencial. Quero usar Python e SQLite.')
 fixture.after_project=await details()
 assert(fixture.after_project.some(d=>d.memory.state==='candidate'&&d.memory.input.kind==='decision'),'project remains review')
 fixture.final_metrics=await metrics();fixture.finished=new Date().toISOString();fixture.passed=true
 await page.screenshot({path:join(root,'reports',`gui-${stage}.png`),fullPage:true})
 writeFileSync(stateFile,JSON.stringify(fixture,null,2));console.log('PASS actual Chat/Agent/Code development smoke')
}catch(error){fixture.error=String(error);writeFileSync(join(root,'reports',`gui-${stage}-failure-${Date.now()}.json`),JSON.stringify(fixture,null,2));throw error}finally{await browser.close()}
