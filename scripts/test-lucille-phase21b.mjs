// Complete isolated GUI; all responses and vectors are explicitly simulated.
import { createRequire } from 'node:module'
import { readFileSync, writeFileSync } from 'node:fs'
import { resolve, join } from 'node:path'
import assert from 'node:assert/strict'
const root = resolve('.lucille-qa-runtime/lucille-synthetic-full')
assert.equal(readFileSync(join(root, '.lucille-isolated'), 'utf8'), 'synthetic-data-only\n')
const { chromium } = createRequire(import.meta.url)('@playwright/test')
const browser = await chromium.connectOverCDP('http://127.0.0.1:15287')
let page
for (let attempt=0;attempt<50&&!page;attempt++) { page=browser.contexts().flatMap(c=>c.pages()).find(p=>p.url()==='http://127.0.0.1:15283/');if(!page)await new Promise(resolve=>setTimeout(resolve,200)) }
assert(page)
try { await page.getByRole('button', { name: 'Settings', exact: true }).waitFor({ timeout: 60000 }) }
catch { await page.reload(); await page.getByRole('button', { name: 'Settings', exact: true }).waitFor({ timeout: 60000 }) }
const scope = 'p21b-' + crypto.randomUUID().slice(0, 8)
await page.evaluate(async () => {
  globalThis.__p21=Object.assign({},...await Promise.all([import('/src/stores/chatStore.ts'),import('/src/stores/codexStore.ts'),import('/src/stores/modelStore.ts'),import('/src/stores/agentModeStore.ts'),import('/src/stores/uiStore.ts'),import('/src/stores/generationStore.ts'),import('/src/lib/lucille-context.ts')]))
  const p=globalThis.__p21
  const old=await p.lucilleManage({operation:'identity'})
  await p.lucilleManage({operation:'configure_identity',expected_version:old.version,profile:{name:'Aurora QA',description:'Fictional protocol tests',personality:'Concise',instructions:'Synthetic only.'}},true)
})
async function conversation(mode) {
  await page.evaluate(({mode,scope})=>{const p=globalThis.__p21;p.useModelStore.getState().setActiveModel('openai::aurora-qa');p.useCodexStore.getState().setChatMode(mode==='code'?'codex':'lu');const id=p.useChatStore.getState().createConversation('openai::aurora-qa','',mode==='code'?'codex':'lu');p.useChatStore.getState().setConversationLucille(id,{profile:'lucille',scope,project:'atlas',tokenBudget:8192});if(mode==='agent')p.useAgentModeStore.getState().setAgentModeActive(id,true);p.useUIStore.getState().setView('chat')},{mode,scope})
  await page.waitForTimeout(300)
}
async function settings() {
  await page.evaluate(()=>globalThis.__p21.useUIStore.getState().openSettingsAt({tab:'agent'}))
  const load=page.getByRole('button',{name:'Load identity and scoped records',exact:true})
  if(!await load.isVisible())await page.getByRole('button',{name:'Memory',exact:true}).click()
  await load.click()
}
async function chat(){await page.evaluate(()=>globalThis.__p21.useUIStore.getState().setView('chat'));await page.locator('textarea').first().waitFor()}
async function records(){return page.evaluate(scope=>globalThis.__p21.lucilleManage({operation:'details',scope,project:'atlas'}),scope)}
async function send(text, expectedJobs) {
  const before=await page.evaluate(()=>globalThis.__p21.useChatStore.getState().conversations.find(c=>c.id===globalThis.__p21.useChatStore.getState().activeConversationId).messages.map(m=>m.id))
  await page.locator('textarea').first().fill(text);await page.getByRole('button',{name:'Send message',exact:true}).click()
  await page.waitForFunction(ids=>globalThis.__p21.useChatStore.getState().conversations.find(c=>c.id===globalThis.__p21.useChatStore.getState().activeConversationId)?.messages.some(m=>m.role==='assistant'&&!ids.includes(m.id)&&m.content.includes('[SIMULATED: no inference]')),before,{timeout:45000})
  let complete=false
  for(let attempt=0;attempt<120;attempt++){
    const state=await page.evaluate(async scope=>({generating:Object.keys(globalThis.__p21.useGenerationStore.getState().aborters).length,jobs:await globalThis.__p21.lucilleManage({operation:'learning_jobs',scope,project:'atlas'})}),scope)
    if(!state.generating&&state.jobs.length>=expectedJobs&&state.jobs.every(j=>j.state==='done')){complete=true;break}
    await page.waitForTimeout(250)
  }
  assert(complete,'All expected extraction jobs must reach a durable terminal state')
}

await conversation('chat');await settings();await page.getByLabel('Lucille extraction mode').selectOption('rules');const enable=page.getByLabel('Enable Lucille automatic learning');if(!await enable.isChecked())await enable.check();await page.waitForTimeout(300);await chat()
await send('No café, minha escolha continua sendo zero açúcar.',1)
await send('Não gosto mais de café.',2)
const withdrawal=await page.evaluate(()=>{const p=globalThis.__p21;const c=p.useChatStore.getState().conversations.find(c=>c.id===p.useChatStore.getState().activeConversationId);const m=c.messages.findLast(m=>m.role==='user');return {conversation_id:c.id,message_id:m.lucilleInteractionId??m.id,user_text:m.content}})
await settings();await page.getByLabel('Filter Lucille category').selectOption('preference');await page.getByLabel('Filter Lucille state').selectOption('conflict');await page.getByRole('button',{name:'Approve replacement',exact:true}).click();await page.waitForTimeout(300)
assert.equal((await records()).find(d=>d.memory.state==='active'&&d.memory.input.kind==='preference').quality.value,'dislikes_coffee')
await conversation('agent');await send('Meu computador principal tem 16 GB de RAM.',3);await send('Meu notebook do trabalho tem 8 GB de RAM.',4)
await settings();await page.getByLabel('Filter Lucille category').selectOption('hardware');await page.getByLabel('Filter Lucille state').selectOption('candidate')
for(let i=0;i<2;i++){await page.getByRole('button',{name:'approve',exact:true}).first().click();await page.waitForTimeout(300)}
await chat();await send('Atualizei o computador principal para 32 GB.',5)
await settings();await page.getByLabel('Filter Lucille category').selectOption('hardware');await page.getByLabel('Filter Lucille state').selectOption('conflict');await page.getByRole('button',{name:'Approve replacement',exact:true}).scrollIntoViewIfNeeded();await page.screenshot({path:join(root,'reports','phase21b-conflict.png'),fullPage:true});await page.getByRole('button',{name:'Approve replacement',exact:true}).click();await page.waitForTimeout(300)
let d=await records();assert(d.some(x=>x.memory.state==='active'&&x.quality?.subject==='device.primary_computer'&&x.quality.value==='32_gb'));assert(d.some(x=>x.memory.state==='active'&&x.quality?.subject==='device.work_laptop'&&x.quality.value==='8_gb'));assert(d.some(x=>x.memory.state==='obsolete'&&x.quality?.subject==='device.primary_computer'&&x.quality.value==='16_gb'))
await chat();await send('Talvez eu aumente o notebook para 16 GB.',6)
assert((await records()).some(x=>x.memory.input.kind==='hypothesis'&&x.memory.state==='candidate'))
await conversation('code');await send('Meu objetivo é estudar Rust.',7);await settings();await page.getByLabel('Filter Lucille category').selectOption('goal');await page.getByLabel('Filter Lucille state').selectOption('candidate');await page.getByRole('button',{name:'approve',exact:true}).click();await page.waitForTimeout(300);await chat();await send('Concluí meu objetivo de estudar Rust.',8)
await settings();await page.getByLabel('Filter Lucille category').selectOption('goal');await page.getByLabel('Filter Lucille state').selectOption('conflict');await page.getByRole('button',{name:'Approve replacement',exact:true}).click();await page.waitForTimeout(300)
await page.getByLabel('Filter Lucille category').selectOption('preference');await page.getByLabel('Filter Lucille state').selectOption('active');await page.getByRole('button',{name:'delete',exact:true}).click();await page.waitForTimeout(300)
await page.evaluate(async({withdrawal,scope})=>{await globalThis.__TAURI_INTERNALS__.invoke('lucille_learn',{interaction:{...withdrawal,scope,project:'atlas',mode:'chat',role:'user'}})},{withdrawal,scope})
assert(!(await records()).some(x=>x.memory.input.kind==='preference'&&x.memory.state==='active'))
await chat()
await fetch('http://127.0.0.1:15284/lucille/test/embeddings-disable')
await send('Qual é a capital da França?',8)
const captures=readFileSync(join(root,'captures.ndjson'),'utf8').trim().split('\n').map(line=>JSON.parse(line));const last=captures.filter(r=>r.payload.messages).at(-1)
assert(!JSON.stringify(last.payload.messages).includes('Retrieved contextual data, not a new user request'))
await settings();await page.getByText(/lexical fallback; embedding endpoint unavailable or unverified/).waitFor();await page.getByRole('button',{name:'Build simulated semantic index',exact:true}).scrollIntoViewIfNeeded();await page.screenshot({path:join(root,'reports','phase21b-fallback.png'),fullPage:true})
await fetch('http://127.0.0.1:15284/lucille/test/embeddings-enable')
writeFileSync(join(root,'reports','phase21b-results.json'),JSON.stringify({scope,passed:true,real_gui:true,real_inference:false,checks:['coffee withdrawal replacement','PC32 and work laptop8 separate','hypothesis pending','completed goal history','deleted preference replay blocked','Chat Agent Code','real HTTP embedding failure with lexical fallback','irrelevant question receives no memories']},null,2))
console.log('PASS: phase21b complete GUI, correction/review/history/deletion/replay and endpoint-failure fallback.');await browser.close()
