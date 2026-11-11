// Phase 2.2E/F dedicated QA observer. Pinned real endpoints; no canonical writer.
// Actual DOM composer + production hooks + native Store + owned real endpoints.
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms))
const qaPhase = import.meta.env.VITE_LUCILLE_REAL_QA_PHASE === '22f' ? 'phase22f' : 'phase22e'
const ownScope = new RegExp(`^${qaPhase}-[a-z0-9-]{1,55}$`)
const faultScope = new RegExp(`^${qaPhase}-fault-[a-z0-9-]{1,50}$`)
const historicalScope = 'phase22c-final-historical-series'
const p = Object.assign({}, ...await Promise.all([
  import('/src/stores/chatStore.ts'), import('/src/stores/codexStore.ts'),
  import('/src/stores/modelStore.ts'), import('/src/stores/agentModeStore.ts'),
  import('/src/stores/uiStore.ts'), import('/src/stores/generationStore.ts'),
  import('/src/stores/settingsStore.ts'), import('/src/lib/lucille-context.ts'),
  import('/src/api/workspace-slug.ts'),
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
  const scope = control.scope ?? `${qaPhase}-` + crypto.randomUUID().slice(0, 8), project = control.project ?? 'cedro'
  if (qaPhase === 'phase22f' && !ownScope.test(scope) && scope !== historicalScope) throw Error('Isolated Phase 2.2F scope required')
  const fixture = { label, scope, project, plan_id: plan.id, started: new Date().toISOString(), turns: [], probes: [], conversations: [] }
  const scoped = operation => manage({ operation, scope, project })
  async function progress() { await record(`driver-${label}-progress.json`, fixture) }
  let currentMode='chat'
  async function conversation(mode = 'chat') {
    currentMode=mode
    p.useModelStore.getState().setActiveModel('openai::lucille-real-gemma')
    p.useCodexStore.getState().setChatMode(mode === 'code' ? 'codex' : 'lu')
    const id = p.useChatStore.getState().createConversation('openai::lucille-real-gemma', '', mode === 'code' ? 'codex' : 'lu')
    p.useChatStore.getState().setConversationLucille(id, { profile: 'lucille', scope, project, tokenBudget: 8192 })
    if (mode === 'agent') p.useAgentModeStore.getState().setAgentModeActive(id, true)
    for(const file of control.plan.fixture_files??[]){
      if(!/^[a-z0-9-]{1,40}\.txt$/.test(file.name) || typeof file.text!=='string' || file.text.length>2000)throw Error('Bounded synthetic tool fixture required')
      const title=p.useChatStore.getState().conversations.find(c=>c.id===id)?.title
      const slug=await p.resolveChatWorkspaceSlug(id,title)
      await globalThis.__TAURI_INTERNALS__.invoke('fs_write',{path:file.name,content:file.text,chatId:slug,workingDirectory:null})
      fixture.files??=[];fixture.files.push({conversation_id:id,name:file.name,synthetic_fixture:true})
    }
    p.useUIStore.getState().setView('chat'); fixture.conversations.push(id); await sleep(400); return id
  }
  async function send(text) {
    p.useUIStore.getState().setView('chat'); await sleep(200)
    const chat = () => p.useChatStore.getState().conversations.find(c => c.id === p.useChatStore.getState().activeConversationId)
    const before = chat().messages.map(m => m.id), start = Date.now()
    const paints = [], domPaints = [], notePaints = [], operationSnapshots=[], seen = new Map(), seenNotes = new Set()
    let nativeReads=Promise.resolve()
    const sampleOperation=event=>{
      // observed_at marks the trigger. Native evidence and job status are
      // separate asynchronous reads bounded by read_started/read_finished;
      // they do not establish the exact SQLite state at the paint instant.
      const original=chat().messages.findLast(m=>m.role==='user' && !m.hidden)
      if(!original)return
      const interaction={conversation_id:chat().id,message_id:original.lucilleInteractionId??original.id,scope,project,mode:currentMode,role:'user',user_text:original.content}
      const request={conversation_id:interaction.conversation_id,message_id:interaction.message_id,scope,project,mode:currentMode,query:interaction.user_text,token_budget:8192}
      const observed_at_ms=Date.now()-start
      nativeReads=nativeReads.then(async()=>{
        const read_started_ms=Date.now()-start
        try{
          const evidence=await manage({operation:'answer_evidence',request,references:[],claims:[],source:{interaction,job_id:null}})
          const sha=async text=>[...new Uint8Array(await crypto.subtle.digest('SHA-256',new TextEncoder().encode(text)))].map(v=>v.toString(16).padStart(2,'0')).join('')
          const jobId=await sha(JSON.stringify([interaction.conversation_id,interaction.message_id,scope,project,await sha(interaction.user_text)]))
          const job=(await scoped('learning_jobs')).find(j=>j.id===jobId)??null
          operationSnapshots.push({event,observed_at_ms,read_started_ms,read_finished_ms:Date.now()-start,job,learning_receipt:evidence.learning_receipt??null,directive_receipt:evidence.directive_receipt??null,...(qaPhase==='phase22f'?{source_operation:evidence.source_operation??null}:{})})
        }catch(error){operationSnapshots.push({event,observed_at_ms,read_started_ms,read_finished_ms:Date.now()-start,error:String(error)})}
      })
    }
    const transportFailure=m=>m.role==='assistant' && !m.lucilleFidelity?.some(r=>r.status==='blocked') && (!!m.error || /^\s*(?:Agent error:|Error:\s*\[network\])/.test(m.content??''))
    const observe = state => {
      for (const c of state.conversations.filter(c => c.id === chat().id)) for (const m of c.messages.filter(m => m.role === 'assistant' && !before.includes(m.id))) {
        for(const block of m.agentBlocks ?? []) {
          if(block.phase!=='reflection' || !block.content?.trim())continue
          const key=m.id+'|'+block.id+'|'+block.content
          if(seenNotes.has(key))continue
          seenNotes.add(key)
          const report=m.lucilleFidelity?.find(r=>r.id===block.fidelityReportId)
          notePaints.push({at_ms:Date.now()-start,message_id:m.id,block_id:block.id,content:block.content,report_id:block.fidelityReportId??null,verified_text_matches:!!report && report.verifiedText?.trim()===block.content.trim()})
        }
        if (seen.get(m.id) === m.content) continue
        seen.set(m.id,m.content)
        sampleOperation(m.content?.trim() ? 'assistant_content' : 'assistant_created')
        paints.push({ at_ms: Date.now()-start, id:m.id, content:m.content, fidelity_status:m.lucilleFidelity?.at(-1)?.status ?? null, error:m.error ?? null })
      }
    }
    const unsubscribe=p.useChatStore.subscribe(observe)
    const observer=new MutationObserver(() => {
      for (const m of chat().messages.filter(m => m.role === 'assistant' && !before.includes(m.id))) {
        const element=document.querySelector(`[data-message-id="${m.id}"]`)
        const codeTranscript=document.querySelector('[data-testid="codex-transcript"]')
        const text=element?.innerText ?? (currentMode==='code' ? codeTranscript?.innerText??'' : '')
        if (domPaints.at(-1)?.text !== text) domPaints.push({at_ms:Date.now()-start,id:m.id,text,fidelity_status:m.lucilleFidelity?.at(-1)?.status ?? null})
      }
    })
    try {
    observer.observe(document.body,{subtree:true,childList:true,characterData:true})
    const input = document.querySelector('textarea')
    if (!input) throw Error('Actual composer unavailable')
    Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, 'value').set.call(input, text)
    input.dispatchEvent(new Event('input', { bubbles: true })); await sleep(100)
    await click('Send message')
    await until(() => chat().messages.some(m => m.role === 'assistant' && !before.includes(m.id) && (m.error || transportFailure(m) || (m.content?.length && (m.finishReason || m.usage)))) && !Object.keys(p.useGenerationStore.getState().aborters).length, 'real generation')
    const generated = Date.now()
    const failure=chat().messages.filter(m=>!before.includes(m.id)).some(transportFailure)
    if(failure)await record(`driver-${label}-transport-progress.json`,{conversation_id:chat().id,observed:new Date().toISOString(),messages:chat().messages.filter(m=>!before.includes(m.id))})
    sampleOperation('generation_terminal'); await sleep(600)
    await until(async () => (await scoped('learning_metrics')).jobs.every(j => !['queued', 'retry', 'running'].includes(j.state)), 'durable extraction')
    const latest = chat().messages.filter(m => !before.includes(m.id))
    if (latest.some(m => m.content.includes('[SIMULATED:'))) throw Error('Simulated reply')
    if (latest.some(transportFailure) && control.expect_transport_failure!==true) throw Error('Transport failure; retained as evaluator/product failure')
    await sleep(100); unsubscribe(); observer.disconnect()
    sampleOperation('learning_terminal'); await nativeReads
    const result = { paints, domPaints, notePaints, operationSnapshots, transport_failure:failure,mode:currentMode, text, conversation_id: chat().id, messages: latest, generation_ms: generated - start, extraction_wait_ms: Date.now() - generated, metrics: await scoped('learning_metrics') }
    fixture.turns.push(result); await progress(); return result
    }finally{unsubscribe();observer.disconnect();await nativeReads}
  }
  async function review(preferences = false) {
    await settings()
    for (let i = 0; i < 16; i++) {
      const row = (await scoped('details')).find(d => ['candidate', 'conflict'].includes(d.memory.state) && d.quality?.temporal !== 'pending' && (preferences ? d.memory.input.kind === 'preference' : ['project', 'decision', 'hardware', 'goal', 'fact'].includes(d.memory.input.kind)))
      if (!row) return
      await select('Filter Lucille category', row.memory.input.kind); await select('Filter Lucille state', row.memory.state)
      const label=row.memory.state === 'conflict' ? 'Approve replacement' : 'approve'
      const target=buttons(label).find(button=>button.closest('[data-lucille-memory-id]')?.getAttribute('data-lucille-memory-id')===row.memory.input.id)
      if(!target || target.disabled)throw Error('Exact review row unavailable')
      target.click(); await sleep(500)
      fixture.reviews ??= []; fixture.reviews.push({id:row.memory.input.id,version:row.memory.version,scope,project,action:label})
    }
    throw Error('Review did not drain')
  }
  async function replay(interaction) {
    const before = await scoped('details')
    const deletedBefore = await scoped('deleted')
    const result = await globalThis.__TAURI_INTERNALS__.invoke('lucille_learn', { interaction })
    const after = await scoped('details')
    const deletedAfter = await scoped('deleted')
    fixture.replays ??= []
    fixture.replays.push({result,canonical_unchanged:JSON.stringify(before)===JSON.stringify(after),tombstones_unchanged:JSON.stringify(deletedBefore)===JSON.stringify(deletedAfter)})
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
      await manage({ operation: 'configure_identity', expected_version: identity.version, profile: { name: 'Lucille Synthetic Phase22', description: 'Fictional isolated QA identity', personality: 'Concise', instructions: 'Answer in at most two short sentences in the language of the question. Quietly use relevant contextual data. Preserve planned choices as plans, not completed implementation. Do not invent progress or personal facts; say when current information is unavailable. Use tools only when the user asks for an external observation or action; remembered facts alone need no external inspection.' } }, true)
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
        await replay({ conversation_id: old.conversation_id, message_id: user.lucilleInteractionId ?? user.id, scope, project, mode: old.mode ?? 'chat', role: 'user', user_text: old.text })
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
    if (qaPhase === 'phase22f' && plan.replay_persisted_sources) {
      for (const old of plan.replay_persisted_sources) {
        if (typeof old.text !== 'string' || !['chat','agent','code'].includes(old.mode)) throw Error('Explicit replay source required')
        const candidates = p.useChatStore.getState().conversations
          .filter(c => c.lucille?.scope === scope && c.lucille?.project === project && !fixture.conversations.includes(c.id))
          .flatMap(c => c.messages.filter(m => m.role === 'user' && !m.hidden && m.content === old.text).map(m => ({c,m})))
        if (candidates.length !== 1) throw Error('Expected one exact persisted USER source in the bound scope and project')
        const {c,m} = candidates[0]
        await replay({conversation_id:c.id,message_id:m.lucilleInteractionId??m.id,scope,project,mode:old.mode,role:'user',user_text:m.content})
      }
    }
    fixture.final_details = await scoped('details'); fixture.final_metrics = await scoped('learning_metrics')
    fixture.identity = await manage({ operation: 'identity' }); fixture.index = await manage({ operation: 'index_status' })
    fixture.deleted = await scoped('deleted'); fixture.finished = new Date().toISOString(); fixture.execution_complete = true
    await record(`driver-${label}-final.json`, fixture, true)
  } catch (error) { fixture.last_conversation = p.useChatStore.getState().conversations.find(c => c.id === p.useChatStore.getState().activeConversationId); fixture.error = String(error); await record(`driver-${label}-failure-${Date.now()}.json`, fixture, true); throw error }
}
let busy = false
// Fault probes use the same native RPC as the real settings/learning worker.
// This is bundled only in the isolated QA host, with no general script runner.
// Exercise the real copy handler. F can check its payload in the background
// without replacing the user's clipboard.
async function copyVerifiedReply(control){
 if(!/^[a-z0-9-]{1,50}$/.test(control.label))throw Error('Copy label')
 const state=p.useChatStore.getState(),c=state.conversations.find(c=>c.id===state.activeConversationId)
 if(!c || !(ownScope.test(c.lucille?.scope) || c.lucille?.scope===historicalScope))throw Error('Own synthetic conversation required')
 const m=c.messages.findLast(m=>m.role==='assistant' && m.content?.trim())
 const verified=!!m?.lucilleFidelity?.some(r=>['checked','regenerated'].includes(r.status) && r.verifiedText?.trim()===m.content.trim())
 if(!verified){await record(`driver-${control.label}-copy.json`,{status:'refused_unverified'},true);return}
 const element=document.querySelector(`[data-message-id="${m.id}"]`),button=element?.querySelector('button[aria-label="Copy message"]')
 if(!button || !navigator.clipboard?.writeText){await record(`driver-${control.label}-copy.json`,{status:'not_applicable',mode:c.mode,button:!!button},true);return}
 const original=navigator.clipboard.writeText;let captured=null,outcome=null
 try{
  navigator.clipboard.writeText=async function(text){
   captured=text
   if(qaPhase==='phase22f' && control.observe_copy_payload){outcome={status:'payload_observed',system_clipboard_written:false};return}
   try{await original.call(navigator.clipboard,text);outcome={status:'system_write_resolved'}}catch(error){outcome={status:'system_write_rejected',error:String(error)};throw error}
  }
  button.click();await until(()=>outcome!==null,'original clipboard write',10000)
 }finally{navigator.clipboard.writeText=original}
 await record(`driver-${control.label}-copy.json`,{...outcome,message_id:m.id,conversation_id:c.id,handler_payload_matches_verified:captured===m.content,verified_text:m.content,document_focused:document.hasFocus(),dom_text:element.innerText},true)
}

async function nativeFault(control) {
  if(!/^[a-z0-9-]{1,50}$/.test(control.label))throw Error('Fault label')
  const request=control.fault
  const ownScope=value=>typeof value==='string' && faultScope.test(value)
  const allowed=['learning_metrics','learning_jobs','details','index_status','cancel_learning','prepare_user_turn','cancel_user_turn','complete_llm','answer_evidence']
  const publish=request.native_command==='lucille_learn'
  if(publish) {
    const {interaction,source}=request.args??{}
    if(!ownScope(interaction?.scope) || interaction.role!=='user' || Object.keys(interaction).length!==Object.keys(source?.interaction??{}).length || Object.entries(interaction).some(([key,value])=>source?.interaction?.[key]!==value))throw Error('Bound original synthetic source required')
  } else {
    if(!allowed.includes(request.operation))throw Error('Fault operation')
    if(request.scope && !ownScope(request.scope))throw Error('Fault scope')
    for(const input of [request.input,request.source?.interaction,request.lease,request.request])if(input && !ownScope(input.scope))throw Error('Foreign nested scope')
    if(request.id) {
      if(!/^[a-f0-9]{64}$/.test(request.id) || !ownScope(request.scope))throw Error('Scoped fault ID required')
      const jobs=await manage({operation:'learning_jobs',scope:request.scope,project:request.project})
      if(!jobs.some(job=>job.id===request.id))throw Error('Foreign fault job')
    }
  }
  // Scope/project fence the harness request; CancelLearning's strict native
  // schema accepts only the ID after that ownership check.
  const nativeRequest=request.operation==='cancel_learning' ? {operation:'cancel_learning',id:request.id} : request
  const result=await globalThis.__TAURI_INTERNALS__.invoke(publish?'lucille_learn':'lucille_manage',publish?request.args:{request:nativeRequest,confirmation:['cancel_learning','complete_llm'].includes(request.operation)?'CONFIRM LUCILLE CONFIGURATION':null})
  await record(`driver-${control.label}-fault.json`,{native:true,observed:new Date().toISOString(),request,result},true)
}
const controlStorage = `lucille-${qaPhase}-driver-consumed`
const processed = new Set(JSON.parse(localStorage.getItem(controlStorage) ?? '[]'))
setInterval(async () => {
  if (busy) return
  try {
    const control = await (await fetch('/__lucille_phase22', { cache: 'no-store' })).json()
    if (!control.id || processed.has(control.id)) return
    processed.add(control.id); localStorage.setItem(controlStorage, JSON.stringify([...processed].slice(-100))); busy = true
    if (control.fault) {
      try { await nativeFault(control) } catch(error) {
        await record(`driver-${control.label}-fault.json`,{native:true,rejected:true,error:String(error)},true)
      }
    } else if(control.copy===true){await copyVerifiedReply(control)
    } else if (control.inspect === true) {
      if (!/^[a-z0-9-]{1,50}$/.test(control.label)) throw Error('Inspection label')
      await record(`driver-${control.label}-inspection.json`, {
        inspected: new Date().toISOString(),
        conversations: p.useChatStore.getState().conversations.filter(c => ownScope.test(c.lucille?.scope)),
        generation_active: Object.keys(p.useGenerationStore.getState().aborters),
      }, true)
    } else await run(control)
  } catch (error) { console.error('Phase22 actual GUI driver', error) } finally { busy = false }
}, 2000)
await record('driver-ready-progress.json', { native: true, href: location.href, ready: new Date().toISOString() })
