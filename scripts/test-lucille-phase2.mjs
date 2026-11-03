// Complete isolated application regression. No inference and no operational data.
import { createRequire } from 'node:module'
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs'
import { resolve, join } from 'node:path'
import assert from 'node:assert/strict'
const { chromium } = createRequire(import.meta.url)('@playwright/test')
const root = resolve('.lucille-qa-runtime/lucille-synthetic-full')
assert.equal(readFileSync(join(root, '.lucille-isolated'), 'utf8'), 'synthetic-data-only\n')
const browser = await chromium.connectOverCDP('http://127.0.0.1:15287')
let pages = []
for (let attempt = 0; attempt < 50 && pages.length === 0; attempt++) {
  pages = browser.contexts().flatMap(context => context.pages()).filter(page => page.url() === 'http://127.0.0.1:15283/')
  if (!pages.length) await new Promise(resolve => setTimeout(resolve, 200))
}
assert.equal(pages.length, 1)
const page = pages[0]
const scope = 'phase2-' + crypto.randomUUID().slice(0, 8)
const reports = join(root, 'reports')
mkdirSync(reports, { recursive: true })
try { await page.getByRole('button', { name: 'Settings', exact: true }).waitFor({ timeout: 60000 }) }
catch { await page.reload(); await page.getByRole('button', { name: 'Settings', exact: true }).waitFor({ timeout: 60000 }) }
await page.evaluate(async () => {
  const modules = await Promise.all([import('/src/stores/chatStore.ts'), import('/src/stores/codexStore.ts'), import('/src/stores/agentModeStore.ts'), import('/src/stores/modelStore.ts'), import('/src/stores/uiStore.ts'), import('/src/stores/generationStore.ts'), import('/src/stores/memoryStore.ts'), import('/src/lib/lucille-context.ts')])
  globalThis.__p2 = Object.assign({}, ...modules)
  const p = globalThis.__p2
  let version = 0
  try { version = (await p.lucilleManage({ operation: 'identity' })).version } catch { /* fresh test DB */ }
  await p.lucilleManage({ operation: 'configure_identity', expected_version: version, profile: { name: 'Aurora QA', description: 'Fictional', personality: 'Concise', instructions: 'Synthetic validation only.' } }, true)
  // Keep native extraction ON: the Lucille fences must bypass it.
  p.useMemoryStore.getState().updateMemorySettings({ autoExtractEnabled: true, autoExtractInAllModes: true })
})
async function conversation(mode) {
  const id = await page.evaluate(({ mode, scope }) => {
    const p = globalThis.__p2
    p.useCodexStore.getState().setChatMode(mode === 'code' ? 'codex' : 'lu')
    p.useModelStore.getState().setActiveModel('openai::aurora-qa')
    const id = p.useChatStore.getState().createConversation('openai::aurora-qa', '', mode === 'code' ? 'codex' : 'lu')
    p.useChatStore.getState().setConversationLucille(id, { profile: 'lucille', scope, project: 'atlas', tokenBudget: 8192 })
    if (mode === 'agent') p.useAgentModeStore.getState().setAgentModeActive(id, true)
    p.useUIStore.getState().setView('chat')
    return id
  }, { mode, scope })
  await page.waitForTimeout(300)
  return id
}
async function settings() {
  await page.evaluate(() => globalThis.__p2.useUIStore.getState().openSettingsAt({ tab: 'agent' }))
  const load = page.getByRole('button', { name: 'Load identity and scoped records', exact: true })
  if (!await load.isVisible()) await page.getByRole('button', { name: 'Memory', exact: true }).click()
  await load.click()
  await page.getByLabel('Search Lucille memories').waitFor()
}
async function chat() { await page.evaluate(() => globalThis.__p2.useUIStore.getState().setView('chat')); await page.locator('textarea').first().waitFor() }
async function records() { return page.evaluate(scope => globalThis.__p2.lucilleManage({ operation: 'details', scope, project: 'atlas' }), scope) }
async function settled() {
  await page.waitForFunction(() => !Object.keys(globalThis.__p2.useGenerationStore.getState().aborters).length)
  await page.waitForFunction(async scope => {
    const jobs = await globalThis.__p2.lucilleManage({ operation: 'learning_jobs', scope, project: 'atlas' })
    return jobs.length > 0 && jobs.every(job => job.state === 'done')
  }, scope, { timeout: 20000 })
  await page.waitForTimeout(600)
}
async function send(text) {
  const before = await page.evaluate(() => globalThis.__p2.useChatStore.getState().conversations.find(c => c.id === globalThis.__p2.useChatStore.getState().activeConversationId).messages.map(m => m.id))
  await page.locator('textarea').first().fill(text)
  await page.getByRole('button', { name: 'Send message', exact: true }).click()
  await page.waitForFunction(ids => globalThis.__p2.useChatStore.getState().conversations.find(c => c.id === globalThis.__p2.useChatStore.getState().activeConversationId)?.messages.some(m => m.role === 'assistant' && !ids.includes(m.id) && m.content.includes('[SIMULATED: no inference]')), before, { timeout: 45000 })
  await settled()
}
function captureMemories() {
  const record = JSON.parse(readFileSync(join(root, 'captures.ndjson'), 'utf8').trim().split('\n').at(-1))
  return record.payload.messages.flatMap(message => (message.content ?? '').split('\n').flatMap(line => { try { const v = JSON.parse(line); return Array.isArray(v) ? v.filter(item => item?.id && item?.version) : [] } catch { return [] } }))
}
const conversations = []
conversations.push(await conversation('chat'))
await settings()
const enable = page.getByLabel('Enable Lucille automatic learning')
if (!await enable.isChecked()) await enable.check()
await page.waitForTimeout(300)
await chat()
const byMode = []
for (const [mode, text] of [['chat', 'Eu gosto de RPG tático.'], ['agent', 'Prefiro jogos de estratégia por turnos com classes.'], ['code', 'Curto RPGs táticos com evolução de personagem.']]) {
  if (mode !== 'chat') conversations.push(await conversation(mode))
  await send(text)
  const details = await records()
  assert.equal(details.length, 1)
  assert.equal(details[0].memory.state, 'active')
  byMode.push({ mode, sourceCount: details[0].provenance.length })
}
assert.deepEqual(byMode.map(item => item.sourceCount), [1, 2, 3])
// Real Chat regeneration must reuse the canonical source, not create a job
// for the regenerated assistant or its new user-message UUID.
await page.evaluate(id => { const p=globalThis.__p2; p.useChatStore.getState().setActiveConversation(id);p.useCodexStore.getState().setChatMode('lu'); }, conversations[0])
const beforeRegenerate = (await records())[0].provenance.length
await page.getByRole('button', { name: 'Regenerate response', exact: true }).last().click()
await page.waitForTimeout(700)
await settled()
assert.equal((await records())[0].provenance.length, beforeRegenerate)
await settings()
await page.getByLabel('Search Lucille memories').fill('táticos')
await page.getByLabel('Filter Lucille category').selectOption('preference')
await page.getByLabel('Filter Lucille state').selectOption('active')
await page.getByText('Evidence and revision history', { exact: true }).click()
assert.equal(await page.locator('blockquote').count(), 3)
await page.getByRole('button', { name: 'Restore before version 1', exact: true }).scrollIntoViewIfNeeded()
await page.screenshot({ path: join(reports, 'phase2-consolidation.png'), fullPage: true })
await page.getByRole('button', { name: 'Restore before version 1', exact: true }).click()
await page.waitForTimeout(300)
assert.equal((await records())[0].memory.input.content.includes('classes'), false)
await chat()
await send('Qual é a capital da França?')
assert.equal(captureMemories().length, 0)
await send('Recomende jogos para meus gostos')
assert.equal(captureMemories().length, 1)
await send('Meu PC tem 16 GB de RAM.')
await settings()
await page.getByLabel('Search Lucille memories').fill('')
await page.getByLabel('Filter Lucille category').selectOption('hardware')
await page.getByLabel('Filter Lucille state').selectOption('candidate')
await page.getByRole('button', { name: 'approve', exact: true }).click()
await page.waitForTimeout(300)
assert.equal((await records()).find(d => d.memory.input.kind === 'hardware').memory.state, 'active')
await chat()
await send('Corrigindo: agora instalei 32 GB de RAM.')
assert(!captureMemories().some(m => m.content.includes('16 GB')))
await send('Quanta RAM tem meu computador?')
assert.equal(captureMemories().filter(m => m.kind === 'hardware').length, 0)
await settings()
await page.getByLabel('Filter Lucille category').selectOption('hardware')
await page.getByLabel('Filter Lucille state').selectOption('conflict')
await page.getByRole('button', { name: 'Approve replacement', exact: true }).scrollIntoViewIfNeeded()
await page.screenshot({ path: join(reports, 'phase2-conflict.png'), fullPage: true })
await page.getByRole('button', { name: 'Approve replacement', exact: true }).click()
await page.waitForTimeout(300)
const corrected = (await records()).filter(d => d.memory.input.kind === 'hardware')
assert.equal(corrected.filter(d => d.memory.state === 'active').length, 1)
assert.equal(corrected.filter(d => d.memory.state === 'obsolete').length, 1)
await chat()
await send('Quanta RAM tem meu computador?')
assert(captureMemories().some(m => m.content.includes('32 GB')))
assert(!captureMemories().some(m => m.content.includes('16 GB')))
await send('Talvez eu compre 64 GB de RAM.')
await settings()
await page.getByLabel('Filter Lucille category').selectOption('hypothesis')
await page.getByLabel('Filter Lucille state').selectOption('candidate')
await page.getByRole('button', { name: 'reject', exact: true }).click()
await page.waitForTimeout(300)
assert.equal((await records()).find(d => d.memory.input.kind === 'hypothesis').memory.state, 'obsolete')
await page.getByLabel('Filter Lucille category').selectOption('preference')
await page.getByLabel('Filter Lucille state').selectOption('active')
await page.getByRole('button', { name: 'delete', exact: true }).click()
await page.waitForTimeout(300)
await page.getByLabel('Filter Lucille state').selectOption('deleted')
assert(await page.getByText(/preference · deleted/).isVisible())
assert(!(await records()).some(d => d.memory.input.kind === 'preference'))
await chat()
await send('Estou com 8 GB de RAM livres agora.')
await settings()
await page.getByLabel('Filter Lucille category').selectOption('fact')
await page.getByLabel('Filter Lucille state').selectOption('candidate')
assert(await page.getByText(/^Valid until:/).isVisible())
const native = await page.evaluate(() => ({ entries: globalThis.__p2.useMemoryStore.getState().entries.length, localEntries: globalThis.__p2.useMemoryStore.getState().localEntries.length }))
assert.deepEqual(native, { entries: 0, localEntries: 0 })
await settings()
await page.getByLabel('Filter Lucille category').selectOption('all')
await page.getByLabel('Filter Lucille state').selectOption('all')
await page.getByLabel('Enable Lucille automatic learning').scrollIntoViewIfNeeded()
await page.screenshot({ path: join(reports, 'phase2-management.png'), fullPage: true })
writeFileSync(join(reports, 'phase2-results.json'), JSON.stringify({ scope, conversations, real_gui: true, real_sqlite: true, real_inference: false, byMode, native, corrected: corrected.map(d => ({ state: d.memory.state, content: d.memory.input.content })), checks: ['user-only all modes', 'native writers bypassed while enabled', 'deduplication + provenance', 'Chat regenerate idempotence', 'search/category/state', 'reversible consolidation', 'irrelevance abstention', 'relevant retrieval', 'candidate review', 'current-message correction precedence', 'conflict suspension', 'replacement approval', 'obsolete exclusion', 'reject hypothesis', 'delete and erased-content label', 'temporary expiry label'] }, null, 2))
console.log('PASS: Phase2 real complete GUI, Chat/Agent/Code learning, review, conflict, consolidation and retrieval.')
await browser.close()
