// Dedicated QA injection only, loaded exclusively with the pinned V3 test contract.
// Actual DOM composer + production hooks + native Store + owned real endpoints.
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms))
const p = Object.assign({}, ...await Promise.all([
  import('/src/stores/chatStore.ts'), import('/src/stores/codexStore.ts'),
  import('/src/stores/modelStore.ts'), import('/src/stores/agentModeStore.ts'),
  import('/src/stores/uiStore.ts'), import('/src/stores/generationStore.ts'),
  import('/src/stores/settingsStore.ts'), import('/src/lib/lucille-context.ts'),
]))
if (!globalThis.__TAURI_INTERNALS__) throw Error('Actual native host required')
const manage = (request, confirm = false) => p.lucilleManage(request, confirm)
const buttons = name => [...document.querySelectorAll('button')].filter(b => b.textContent.trim() === name || b.getAttribute('aria-label') === name)
async function until(fn, label, timeout = 180000) {
  const start = Date.now()
  while (Date.now() - start < timeout) { if (await fn()) return; await sleep(250) }
  throw Error('Timeout: ' + label)
}
async function click(name) {
  await until(() => buttons(name).some(b => !b.disabled && b.getClientRects().length), name, 20000)
  buttons(name).find(b => !b.disabled && b.getClientRects().length).click(); await sleep(250)
}
async function record(name, report, final = false) {
  const response = await fetch('/__lucille_phase22', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ name, report, final }) })
  if (!response.ok) throw Error('Report persistence rejected')
}
async function select(label, value) {
  const element = document.querySelector(`select[aria-label="${label}"]`)
  if (!element) throw Error('Missing select ' + label)
  Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, 'value').set.call(element, value)
  element.dispatchEvent(new Event('change', { bubbles: true })); await sleep(500)
}
async function settings() {
  p.useUIStore.getState().openSettingsAt({ tab: 'agent' }); await sleep(500)
  if (!buttons('Load identity and scoped records').some(b => b.getClientRects().length)) await click('Memory')
  await click('Load identity and scoped records')
}
async function run(control) {
  const { plan, label } = control
  if (!/^[a-z0-9-]{1,50}$/.test(label)) throw Error('Stage label')
  const scope = control.scope ?? 'p22-' + crypto.randomUUID().slice(0, 8), project = control.project ?? 'cedro'
  const fixture = { label, scope, project, plan_id: plan.id, started: new Date().toISOString(), turns: [], probes: [], conversations: [] }
  const scoped = operation => manage({ operation, scope, project })
  async function progress() { await record(`driver-${label}-progress.json`, fixture) }
  async function conversation(mode = 'chat') {
    p.useModelStore.getState().setActiveModel('openai::lucille-real-gemma')
    p.useCodexStore.getState().setChatMode(mode === 'code' ? 'codex' : 'lu')
    const id = p.useChatStore.getState().createConversation('openai::lucille-real-gemma', '', mode === 'code' ? 'codex' : 'lu')
    p.useChatStore.getState().setConversationLucille(id, { profile: 'lucille', scope, project, tokenBudget: 8192 })
    if (mode === 'agent') p.useAgentModeStore.getState().setAgentModeActive(id, true)
    p.useUIStore.getState().setView('chat'); fixture.conversations.push(id); await sleep(400); return id
  }
  async function send(text) {
    p.useUIStore.getState().setView('chat'); await sleep(200)
    const chat = () => p.useChatStore.getState().conversations.find(c => c.id === p.useChatStore.getState().activeConversationId)
    const before = chat().messages.map(m => m.id), start = Date.now()
    const input = document.querySelector('textarea')
    if (!input) throw Error('Actual composer unavailable')
    Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, 'value').set.call(input, text)
    input.dispatchEvent(new Event('input', { bubbles: true })); await sleep(100)
    await click('Send message')
    await until(() => chat().messages.some(m => m.role === 'assistant' && !before.includes(m.id) && m.content?.length && (m.finishReason || m.usage)) && !Object.keys(p.useGenerationStore.getState().aborters).length, 'real generation')
    const generated = Date.now(); await sleep(600)
    await until(async () => (await scoped('learning_metrics')).jobs.every(j => !['queued', 'retry', 'running'].includes(j.state)), 'durable extraction')
    const latest = chat().messages.slice(-2)
    if (latest.some(m => m.error || m.content.includes('[SIMULATED:'))) throw Error('Failed or simulated reply')
    const result = { text, conversation_id: chat().id, messages: latest, generation_ms: generated - start, extraction_wait_ms: Date.now() - generated, metrics: await scoped('learning_metrics') }
    fixture.turns.push(result); await progress(); return result
  }
  async function review(preferences = false) {
    await settings()
    for (let i = 0; i < 16; i++) {
      const row = (await scoped('details')).find(d => ['candidate', 'conflict'].includes(d.memory.state) && d.quality?.temporal !== 'pending' && (preferences ? d.memory.input.kind === 'preference' : ['project', 'decision', 'hardware', 'goal'].includes(d.memory.input.kind)))
      if (!row) return
      await select('Filter Lucille category', row.memory.input.kind); await select('Filter Lucille state', row.memory.state)
      await click(row.memory.state === 'conflict' ? 'Approve replacement' : 'approve')
    }
    throw Error('Review did not drain')
  }
  try {
    await until(() => p.useChatStore.persist.hasHydrated(), 'chat hydration')
    fixture.existing_bindings = p.useChatStore.getState().conversations.filter(c => c.lucille?.scope === scope).map(c => ({ id: c.id, binding: c.lucille }))
    try { fixture.initial_identity = await manage({ operation: 'identity' }) } catch { fixture.initial_identity = null }
    fixture.initial_index = await manage({ operation: 'index_status' })
    fixture.initial_details = await scoped('details'); fixture.initial_deleted = await scoped('deleted')

    for (const button of buttons('Got it')) if (button.getClientRects().length) button.click()
    p.useSettingsStore.getState().updateSettings({ thinkingEnabled: false, maxTokens: 384, temperature: 0 })
    if (!control.resume) {
      let identity; try { identity = await manage({ operation: 'identity' }) } catch { identity = { version: 0 } }
      await manage({ operation: 'configure_identity', expected_version: identity.version, profile: { name: 'Lucille Synthetic Phase22', description: 'Fictional isolated QA identity', personality: 'Concise', instructions: 'Answer in at most two short sentences in the language of the question. Quietly use relevant contextual data. Preserve planned choices as plans, not completed implementation. Do not invent progress or personal facts; say when current information is unavailable. No tools are needed in these synthetic memory checks.' } }, true)
    }
    await conversation(); await settings()
    await select('Lucille extraction mode', 'llm')
    for (const label of ['Enable Lucille automatic learning', 'Enable safe autonomous preferences', 'Automatically update Lucille index']) {
      const input = document.querySelector(`input[aria-label="${label}"]`)
      if (!input || input.disabled) throw Error('Policy checkbox unavailable ' + label)
      if (!input.checked) { input.click(); await sleep(600) }
    }
    fixture.policy = await manage({ operation: 'learning_policy' }); fixture.initial_metrics = await scoped('learning_metrics')
    for (const turn of plan.turns ?? []) {
      if (turn.new_session) await conversation(turn.mode ?? 'chat')
      await send(turn.text)
      if (turn.review) await review()
      if (turn.review_preferences) await review(true)
      if (turn.replay_turn_index !== undefined) {
        const old = fixture.turns[turn.replay_turn_index], user = old.messages.find(m => m.role === 'user')
        const before = await scoped('details')
        const result = await globalThis.__TAURI_INTERNALS__.invoke('lucille_learn', { interaction: { conversation_id: old.conversation_id, message_id: user.lucilleInteractionId ?? user.id, scope, project, mode: 'chat', role: 'user', user_text: old.text } })
        const after = await scoped('details'); fixture.replays ??= []; fixture.replays.push({ result, canonical_unchanged: JSON.stringify(before) === JSON.stringify(after) })
      }
    }
    if (control.interrupt_source) {
      const id = crypto.randomUUID(), conversation_id = fixture.conversations.at(-1)
      const job = await globalThis.__TAURI_INTERNALS__.invoke('lucille_learn', { interaction: { conversation_id, message_id: id, scope, project, mode: 'chat', role: 'user', user_text: control.interrupt_source } })
      void import('/src/lib/lucille-llm.ts').then(m => m.tickLucilleLlm())
      await until(async () => (await scoped('learning_jobs')).find(j => j.id === job.id)?.state === 'running', 'leased actual extractor')
      fixture.interrupted_job = { id: job.id, message_id: id, conversation_id, text: control.interrupt_source, scope, project }
      await record(`driver-${label}-prepared.json`, fixture, true)
      return
    }
    fixture.after_long = await scoped('details')
    if (!control.allow_pending_index) await until(async () => (await scoped('learning_metrics')).pending_vectors === 0, 'automatic real Nomic index')
    for (const item of plan.probes ?? []) {
      await conversation(item.mode ?? 'chat'); const turn = await send(item.text)
      fixture.probes.push({ item, turn }); await progress()
    }
    fixture.final_details = await scoped('details'); fixture.final_metrics = await scoped('learning_metrics')
    fixture.identity = await manage({ operation: 'identity' }); fixture.index = await manage({ operation: 'index_status' })
    fixture.deleted = await scoped('deleted'); fixture.finished = new Date().toISOString(); fixture.execution_complete = true
    await record(`driver-${label}-final.json`, fixture, true)
  } catch (error) { fixture.error = String(error); await record(`driver-${label}-failure-${Date.now()}.json`, fixture, true); throw error }
}
let busy = false
const processed = new Set(JSON.parse(localStorage.getItem("lucille-phase22-driver-consumed") ?? "[]"))
setInterval(async () => {
  if (busy) return
  try {
    const control = await (await fetch('/__lucille_phase22', { cache: 'no-store' })).json()
    if (!control.id || processed.has(control.id)) return
    processed.add(control.id); localStorage.setItem("lucille-phase22-driver-consumed", JSON.stringify([...processed].slice(-100))); busy = true
    await run(control)
  } catch (error) { console.error('Phase22 actual GUI driver', error) } finally { busy = false }
}, 2000)
await record('driver-ready-progress.json', { native: true, href: location.href, ready: new Date().toISOString() })
