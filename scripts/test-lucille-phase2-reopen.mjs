// Seeds a verified, scheduled synthetic queue entry through the transactional
// backup API, then checks native process reopening separately from UI hooks.
import { createRequire } from 'node:module'
import { createHash, randomUUID } from 'node:crypto'
import { readFileSync, writeFileSync } from 'node:fs'
import { resolve, join } from 'node:path'
import assert from 'node:assert/strict'
const root = resolve('.lucille-qa-runtime/lucille-synthetic-full')
assert.equal(readFileSync(join(root, '.lucille-isolated'), 'utf8'), 'synthetic-data-only\n')
const { chromium } = createRequire(import.meta.url)('@playwright/test')
const browser = await chromium.connectOverCDP('http://127.0.0.1:15287')
let page
for (let attempt=0;attempt<50&&!page;attempt++) {
  page=browser.contexts().flatMap(c => c.pages()).find(p => p.url() === 'http://127.0.0.1:15283/')
  if (!page) await new Promise(resolve => setTimeout(resolve,200))
}
assert(page)
try { await page.getByRole('button', { name: 'Settings', exact: true }).waitFor({ timeout: 60000 }) }
catch { await page.reload(); await page.getByRole('button', { name: 'Settings', exact: true }).waitFor({ timeout: 60000 }) }
const fixturePath = join(root, 'reports', 'phase2-reopen-fixture.json')
if (process.argv[2] === 'stage') {
  const { scope, conversations } = JSON.parse(readFileSync(join(root, 'reports', 'phase2-results.json'), 'utf8'))
  const message = randomUUID()
  const sha = value => createHash('sha256').update(value).digest('hex')
  const id = sha(JSON.stringify([conversations[0], message, scope, 'atlas', sha('Prefiro café sem açúcar.')]))
  const fixture = { id, scope, conversation: conversations[0], identity: null, before_requests: readFileSync(join(root, 'captures.ndjson'), 'utf8').trim().split('\n').length }
  const state = await page.evaluate(async ({ id, scope, message, conversation }) => {
    const { lucilleManage } = await import('/src/lib/lucille-context.ts')
    const backup = JSON.parse(await lucilleManage({ operation: 'export' }, true))
    const tables = backup.intelligent.tables
    const now = Math.floor(Date.now() / 1000)
    const claim = { kind: 'preference', content: 'Prefere café sem açúcar.', subject: 'user', property: 'coffee_sugar', value: 'without_sugar', confidence: 0.98, relevance: 0.9, temporal: 'preference', ttl_seconds: null, facets: [], evidence: 'Prefiro café sem açúcar', rule: 'explicit_coffee_preference', project_hint: null, correction: false }
    tables.extraction_jobs.push([id, conversation, message, scope, 'atlas', 'chat', 'queued', 0, JSON.stringify([claim]), now, now, now + 20, null, null, null])
    await lucilleManage({ operation: 'restore', backup: JSON.stringify(backup) }, true)
    const jobs = await lucilleManage({ operation: 'learning_jobs', scope, project: 'atlas' })
    return { job: jobs.find(j => j.id === id), identity: await lucilleManage({ operation: 'identity' }) }
  }, { id, scope, message, conversation: fixture.conversation })
  assert.equal(state.job.state, 'queued')
  fixture.identity = state.identity.version
  writeFileSync(fixturePath, JSON.stringify(fixture, null, 2))
  console.log('PASS: verified synthetic queue entry scheduled and durably queued before native shutdown.')
} else if (process.argv[2] === 'verify') {
  const fixture = JSON.parse(readFileSync(fixturePath, 'utf8'))
  let complete=false
  for(let attempt=0;attempt<240;attempt++) {
    const state=await page.evaluate(async ({id,scope})=>{const {lucilleManage}=await import('/src/lib/lucille-context.ts');return (await lucilleManage({operation:'learning_jobs',scope,project:'atlas'})).find(j=>j.id===id)?.state},fixture)
    if(state==='done'){complete=true;break}
    await page.waitForTimeout(250)
  }
  assert(complete,'Persisted job must finish after reopening')
  const result = await page.evaluate(async ({ id, scope, conversation }) => {
    const [{ lucilleManage }, { useChatStore }] = await Promise.all([import('/src/lib/lucille-context.ts'), import('/src/stores/chatStore.ts')])
    const records = await lucilleManage({ operation: 'details', scope, project: 'atlas' })
    const backup = JSON.parse(await lucilleManage({ operation: 'export' }, true))
    return { identity: (await lucilleManage({ operation: 'identity' })).version, binding: useChatStore.getState().conversations.find(c => c.id === conversation)?.lucille, matching: records.filter(d => d.provenance.some(p => p.interaction_id === id)), payload: backup.intelligent.tables.extraction_jobs.find(j => j[0] === id)[8] }
  }, fixture)
  assert.equal(result.identity, fixture.identity)
  assert.equal(result.binding.scope, fixture.scope)
  assert.equal(result.matching.length, 1)
  assert.equal(result.matching[0].memory.state, 'active')
  assert.equal(result.payload, '[]')
  assert.equal(readFileSync(join(root, 'captures.ndjson'), 'utf8').trim().split('\n').length, fixture.before_requests)
  writeFileSync(join(root, 'reports', 'phase2-reopen-results.json'), JSON.stringify({ passed: true, real_native_restart: true, queued_before_shutdown: true, completed_on_hydrated_reopen: true, no_inference_request: true, identity_version: result.identity, binding: result.binding, terminal_payload_purged: true }, null, 2))
  console.log('PASS: persisted pending queue resumed on native reopening, identity/binding retained, payload purged, no inference request.')
} else throw new Error('stage or verify required')
await browser.close()
