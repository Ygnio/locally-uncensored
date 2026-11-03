// Drives the complete application UI, not qa/lucille.html. Fixture setup uses
// the same stores/IPC as Settings; every send uses the actual composer/hooks.
import { createRequire } from 'node:module'
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs'
import { resolve, join } from 'node:path'
import assert from 'node:assert/strict'
const { chromium } = createRequire(import.meta.url)('@playwright/test')
const root = resolve('.lucille-qa-runtime/lucille-synthetic-full')
assert.equal(readFileSync(join(root, '.lucille-isolated'), 'utf8'), 'synthetic-data-only\n')
const browser = await chromium.connectOverCDP('http://127.0.0.1:15287')
const pages = browser.contexts().flatMap(context => context.pages()).filter(page => page.url() === 'http://127.0.0.1:15283/')
assert.equal(pages.length, 1, 'Exactly one dedicated full-application WebView required')
const page = pages[0]
const evidence = []
const scope = 'qa-' + globalThis.crypto.randomUUID().slice(0, 8)
await page.getByRole('button', { name: 'Settings', exact: true }).waitFor()
const fixture = await page.evaluate(async scope => {
  const [{ useChatStore }, { useCodexStore }, { useAgentModeStore }, { useModelStore }, { useUIStore }, { useGenerationStore }, { useMemoryStore }, { lucilleManage }] = await Promise.all([
    import('/src/stores/chatStore.ts'), import('/src/stores/codexStore.ts'), import('/src/stores/agentModeStore.ts'), import('/src/stores/modelStore.ts'), import('/src/stores/uiStore.ts'), import('/src/stores/generationStore.ts'), import('/src/stores/memoryStore.ts'), import('/src/lib/lucille-context.ts'),
  ])
  globalThis.__fullQA = { useChatStore, useCodexStore, useAgentModeStore, useModelStore, useUIStore, useGenerationStore, useMemoryStore, lucilleManage }
  useMemoryStore.getState().updateMemorySettings({ autoExtractEnabled: false, autoExtractInAllModes: false })
  let version = 0
  try { version = (await lucilleManage({ operation: 'identity' })).version } catch { /* fresh synthetic DB */ }
  const identity = await lucilleManage({ operation: 'configure_identity', expected_version: version, profile: { name: 'Aurora QA', description: 'Fictional validation assistant', personality: 'Objective and consistent', instructions: 'Identify yourself as Aurora QA in every mode.' } }, true)
  const create = async (project, content, key) => {
    const candidate = { id: globalThis.crypto.randomUUID(), scope, project, kind: 'project', content, fact_key: key, source: 'manual', source_id: globalThis.crypto.randomUUID() }
    const record = await lucilleManage({ operation: 'create', candidate }, true)
    return await lucilleManage({ operation: 'change', id: candidate.id, scope, project, expected_version: record.version, change: { action: 'approve' } }, true)
  }
  const atlas = await create('atlas', 'The fictional project Atlas uses SQLite.', 'database')
  const boreal = await create('boreal', 'The fictional project Boreal uses PostgreSQL.', 'database')
  return { identity, atlas, boreal }
}, scope)

async function conversation(mode, project = 'atlas', bound = true) {
  await page.waitForFunction(() => !Object.keys(globalThis.__fullQA.useGenerationStore.getState().aborters).length)
  const id = await page.evaluate(({ mode, project, bound, scope }) => {
    const p = globalThis.__fullQA
    p.useCodexStore.getState().setChatMode(mode === 'code' ? 'codex' : 'lu')
    p.useModelStore.getState().setActiveModel('openai::aurora-qa')
    const id = p.useChatStore.getState().createConversation('openai::aurora-qa', '', mode === 'code' ? 'codex' : 'lu')
    if (bound) p.useChatStore.getState().setConversationLucille(id, { profile: 'lucille', scope, project, tokenBudget: 4096 })
    if (mode === 'agent') p.useAgentModeStore.getState().setAgentModeActive(id, true)
    p.useUIStore.getState().setView('chat')
    return id
  }, { mode, project, bound, scope })
  // The composer restores its draft after conversation/mode changes.
  await page.waitForTimeout(300)
  return id
}
function capture() {
  const record = JSON.parse(readFileSync(join(root, 'captures.ndjson'), 'utf8').trim().split('\n').at(-1))
  const values = record.payload.messages.flatMap(message => (message.content ?? '').split('\n').flatMap(line => { try { return [JSON.parse(line)] } catch { return [] } }))
  return { model: record.payload.model, identities: values.filter(value => value?.profile_id === 'lucille'), environment: values.find(value => value?.source === 'local_host_clock_and_executor'), memories: values.filter(Array.isArray).flat().filter(value => value?.id && value?.version), generic_code_intro: record.payload.messages.some(message => message.content?.startsWith('You are the Coding Agent')) }
}
async function send(text, test) {
  const before = await page.evaluate(() => globalThis.__fullQA.useChatStore.getState().conversations.find(c => c.id === globalThis.__fullQA.useChatStore.getState().activeConversationId).messages.map(m => m.id))
  await page.locator('textarea').first().fill(text)
  await page.getByRole('button', { name: 'Send message', exact: true }).click()
  await page.waitForFunction(ids => globalThis.__fullQA.useChatStore.getState().conversations.find(c => c.id === globalThis.__fullQA.useChatStore.getState().activeConversationId)?.messages.some(m => m.role === 'assistant' && !ids.includes(m.id) && m.content.includes('[SIMULATED: no inference]')), before, { timeout: 45000 })
  await page.waitForFunction(() => !Object.keys(globalThis.__fullQA.useGenerationStore.getState().aborters).length)
  const result = capture()
  evidence.push({ test, result })
  return result
}
for (const mode of ['chat', 'agent', 'code']) {
  await conversation(mode)
  const result = await send('QA_FULL_IDENTITY: Identify yourself and recall Atlas SQLite Boreal PostgreSQL.', mode)
  assert.equal(result.identities.length, 1)
  assert.equal(result.identities[0].name, 'Aurora QA')
  assert.equal(result.environment.mode, mode)
  assert.equal(result.environment.model, 'aurora-qa')
  assert.deepEqual(result.memories.map(memory => memory.id), [fixture.atlas.input.id])
  assert.equal(result.generic_code_intro, false)
}
const reports = join(root, 'reports')
mkdirSync(reports, { recursive: true })
await page.screenshot({ path: join(reports, 'full-code.png') })
const staged = await send('QA_TOOL_SEQUENCE: Create aurora.txt with blue, read it, edit blue to green, read again and list files. Only the synthetic workspace is authorized.', 'staged-sequence')
assert.equal(staged.environment.execution_receipts.filter(receipt => receipt.outcome === 'staged' && !receipt.result_confirmed).length, 4)
const disk = async () => page.evaluate(async () => {
  const { resolveChatWorkspaceSlug } = await import('/src/api/workspace-slug.ts')
  const slug = await resolveChatWorkspaceSlug(globalThis.__fullQA.useChatStore.getState().activeConversationId)
  try { return await globalThis.__TAURI_INTERNALS__.invoke('fs_read', { path: 'aurora.txt', chatId: slug }) } catch { return null }
})
assert.equal(await disk(), null, 'Staged file must not exist on disk before approval')
await page.getByRole('button', { name: 'Apply all', exact: true }).click()
assert.equal((await disk()).content, 'green')
const confirmed = await send('QA_TOOL_VERIFY: Read aurora.txt from the synthetic workspace.', 'approved-disk-read')
assert.equal(confirmed.environment.execution_receipts[0].result_confirmed, true)
for (const marker of ['QA_TOOL_ESCAPE', 'QA_TOOL_FAILURE']) {
  const result = await send(marker + ': Safety test in the synthetic workspace.', marker)
  assert.equal(result.environment.execution_receipts[0].result_confirmed, false)
}
const pending = send('QA_TOOL_DESTRUCTIVE: Safety test only. Never authorize a destructive shell operation.', 'rejected-shell')
await page.getByRole('button', { name: 'No', exact: true }).waitFor()
await page.getByRole('button', { name: 'No', exact: true }).click()
const rejected = await pending
assert.equal(rejected.environment.execution_receipts[0].executed, false)
assert.equal((await disk()).content, 'green')
await page.screenshot({ path: join(reports, 'full-permissions.png') })
await conversation('chat', 'boreal')
assert.deepEqual((await send('QA_BOREAL: Recall Atlas SQLite Boreal PostgreSQL.', 'boreal-isolation')).memories.map(memory => memory.id), [fixture.boreal.input.id])
await page.evaluate(async ({ fixture, scope }) => {
  await globalThis.__fullQA.lucilleManage({ operation: 'change', id: fixture.atlas.input.id, scope, project: 'atlas', expected_version: fixture.atlas.version, change: { action: 'delete' } }, true)
}, { fixture, scope })
for (const mode of ['chat', 'agent', 'code']) {
  await conversation(mode)
  assert.equal((await send('QA_FULL_DELETED: Recall Atlas SQLite Boreal PostgreSQL.', 'deleted-' + mode)).memories.length, 0)
}
await conversation('code', 'atlas', false)
const disabled = await send('QA_FULL_DISABLED: Identify the default operational mode.', 'disabled')
assert.equal(disabled.identities.length, 0)
assert.equal(disabled.generic_code_intro, true)
const native = await page.evaluate(() => ({ entries: globalThis.__fullQA.useMemoryStore.getState().entries.length, localEntries: globalThis.__fullQA.useMemoryStore.getState().localEntries.length }))
assert.deepEqual(native, { entries: 0, localEntries: 0 })
writeFileSync(join(reports, 'full-results.json'), JSON.stringify({ captured_at: new Date().toISOString(), real_gui: true, real_sqlite: true, real_inference: false, native, evidence }, null, 2))
console.log('PASS: complete GUI real hooks, identity/memory across modes, real approved filesystem operations, staging evidence, scope/approval/error boundaries and native fallback.')
await browser.close()
