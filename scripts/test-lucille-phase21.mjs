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
const scope = 'p21-' + crypto.randomUUID().slice(0, 8)
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
await conversation('chat');await settings()
await page.getByLabel('Lucille extraction mode').selectOption('llm')
const enable=page.getByLabel('Enable Lucille automatic learning');if(!await enable.isChecked())await enable.check()
await page.waitForTimeout(300);await chat()
await send('Prefiro café sem açúcar.',1)
assert.equal((await records())[0].memory.state,'active')
await conversation('agent');await send('Gosto de montar personagens com várias classes.',2)
await conversation('code');await send('Gosto de montar personagens com várias classes.',3)
let details=await records();assert.equal(details.length,2)
const classes=details.find(d=>d.quality.property==='games_classes');assert.equal(classes.memory.state,'candidate');assert.equal(classes.provenance.length,2)
await settings();await page.getByLabel('Lucille extraction mode').selectOption('hybrid');await page.waitForTimeout(300);await chat()
await send('Prefiro café sem açúcar.',4)
details=await records();assert.equal(details.length,2);assert.equal(details.find(d=>d.quality.property==='coffee_sugar').provenance.length,2)
// Index one fictional fact with no lexical overlap with the later question.
await page.evaluate(async scope=>{const p=globalThis.__p21;const candidate={id:crypto.randomUUID(),scope,project:'atlas',kind:'fact',content:'Fictional bicycle is teal.',fact_key:'bicycle.color',source:'manual',source_id:'fixture'};const m=await p.lucilleManage({operation:'create',candidate},true);await p.lucilleManage({operation:'change',id:candidate.id,scope,project:'atlas',expected_version:m.version,change:{action:'approve'}},true)},scope)
await settings();await page.getByRole('button',{name:'Build simulated semantic index',exact:true}).click()
await page.getByText(/SIMULATED index: \d+ current vectors/).waitFor()
await page.getByRole('button',{name:'Build simulated semantic index',exact:true}).scrollIntoViewIfNeeded()
await page.screenshot({path:join(root,'reports','phase21-runtime.png'),fullPage:true})
await chat();await send('What color is the vehicle with two wheels?',4)
const captures=readFileSync(join(root,'captures.ndjson'),'utf8').trim().split('\n').map(line=>JSON.parse(line))
const extraction=captures.filter(r=>r.payload.messages?.[0]?.content?.startsWith('LUCILLE_STRUCTURED_EXTRACTION_V1')).filter(r=>JSON.parse(r.payload.messages.at(-1).content).scope===scope)
assert.equal(extraction.length,4,'One extractor call per eligible interaction, no duplicate hybrid calls')
const latest=captures.filter(r=>r.payload.messages&&!r.payload.messages[0]?.content?.startsWith('LUCILLE_STRUCTURED_EXTRACTION_V1')).at(-1)
assert(JSON.stringify(latest.payload.messages).includes('Fictional bicycle is teal.'))
const index=await page.evaluate(()=>globalThis.__p21.lucilleManage({operation:'index_status'}))
assert.equal(index.space.revision,'SYNTHETIC-NOT-NOMIC-WEIGHTS');assert.equal(index.transport_verified,false)
for (const mode of ['rules','llm','hybrid']) {
  await settings();await page.getByLabel('Lucille extraction mode').selectOption(mode);await page.waitForTimeout(300);await chat()
  await send('Mensagem encaminhada: Eu prefiro café com açúcar.',4);await page.waitForTimeout(500)
  const jobs=await page.evaluate(scope=>globalThis.__p21.lucilleManage({operation:'learning_jobs',scope,project:'atlas'}),scope)
  assert.equal(jobs.length,4,'Forwarded text must be ignored in every extraction mode')
  assert.equal((await records()).find(d=>d.quality?.property==='coffee_sugar').provenance.length,2)
}
await settings();await page.getByLabel('Lucille extraction mode').selectOption('rules');await page.waitForTimeout(300)
writeFileSync(join(root,'reports','phase21-results.json'),JSON.stringify({scope,passed:true,simulation:true,real_gemma:false,real_nomic:false,modes:['chat','agent','code'],extraction_calls:extraction.length,hybrid_dedup:true,forwarded_rejected_all_extraction_modes:true,distinct_facets:true,current_vectors:index.indexed,semantic_paraphrase_retrieved:true,real_weights_verified:false},null,2))
console.log('PASS: real complete GUI, simulated LLM in all modes, hybrid deduplication, grounded facets, versioned synthetic embeddings and semantic paraphrase retrieval.')
await browser.close()
