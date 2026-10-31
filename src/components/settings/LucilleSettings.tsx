import { useGenerationStore } from "../../stores/generationStore"
import { useState } from 'react'
import type { Conversation } from '../../types/chat'
import { useChatStore } from '../../stores/chatStore'
import { isLucille, lucilleManage, type LucilleMemory, type LucilleProfile } from '../../lib/lucille-context'

const kinds = ['fact', 'preference', 'hardware', 'project', 'decision', 'solution', 'goal', 'task', 'constraint', 'summary', 'hypothesis']

/** Explicit isolated QA configuration. No automatic profile import or discovery. */
export function LucilleSettings({ conversation }: { conversation: Conversation }) {
  const [project, setProject] = useState(conversation.lucille?.project ?? 'synthetic-project')
  const [scope, setScope] = useState(conversation.lucille?.scope ?? 'lucille')
  const [budget, setBudget] = useState(conversation.lucille?.tokenBudget ?? 4096)
  const [identity, setIdentity] = useState<{ profile: LucilleProfile; version: number } | null>(null)
  const [profileJson, setProfileJson] = useState('')
  const [entries, setEntries] = useState<LucilleMemory[]>([])
  const [content, setContent] = useState('')
  const [factKey, setFactKey] = useState('synthetic.fact')
  const [kind, setKind] = useState('fact')
  const [backup, setBackup] = useState('')
  const [notice, setNotice] = useState('')
  const [busy, setBusy] = useState(false)
  const generating = useGenerationStore(state => !!state.aborters[conversation.id])
  const enabled = isLucille(conversation)
  async function run(action: () => Promise<void>) {
    setBusy(true)
    setNotice('')
    try { await action() } catch (error) { setNotice(String(error)) } finally { setBusy(false) }
  }
  async function load() {
    const current = await lucilleManage<{ profile: LucilleProfile; version: number }>({ operation: 'identity' })
    setIdentity(current)
    setProfileJson(JSON.stringify(current.profile, null, 2))
    setEntries(await lucilleManage<LucilleMemory[]>({ operation: 'list', scope, project }))
  }
  async function change(entry: LucilleMemory, action: string) {
    await lucilleManage({ operation: 'change', id: entry.input.id, scope, project, expected_version: entry.version,
      change: action === 'update' ? { action, content, fact_key: factKey } : { action } }, true)
    await load()
  }
  return <fieldset disabled={busy || generating} className="space-y-2 rounded border border-purple-500 p-3 text-xs">
    <legend>Lucille — isolated phase 1 QA</legend>
    <p>Use fictional data only. This profile requires the isolated debug application. Automatic extraction is disabled for bound conversations.</p>
    <label className="block">Scope <input aria-label="Lucille scope" value={scope} disabled={enabled} onChange={event => setScope(event.target.value)} className="rounded border bg-transparent p-1" /></label>
    <label className="block">Project <input aria-label="Lucille project" value={project} disabled={enabled} onChange={event => setProject(event.target.value)} className="rounded border bg-transparent p-1" /></label>
    <label className="block">Context token ceiling <input type="number" min={512} max={16384} value={budget} disabled={enabled} onChange={event => setBudget(Number(event.target.value))} className="rounded border bg-transparent p-1" /></label>
    <button type="button" className="rounded border p-1" onClick={() => void run(async () => {
      const current = await lucilleManage<{ version: number }>({ operation: 'reset_fixture', expected_version: identity?.version ?? 0 }, true)
      setIdentity(null)
      setNotice(`Fictional identity saved at version ${current.version}`)
      await load()
    })}>Explicitly initialize/reset fictional identity</button>
    <button type="button" className="ml-2 rounded border p-1" onClick={() => void run(load)}>Load identity and scoped records</button>
    {identity && <>
      <label className="block">Identity profile (version {identity.version})<textarea aria-label="Lucille identity" value={profileJson} onChange={event => setProfileJson(event.target.value)} className="block min-h-32 w-full rounded border bg-transparent p-2" /></label>
      <button type="button" className="rounded border p-1" onClick={() => void run(async () => {
        await lucilleManage({ operation: 'configure_identity', profile: JSON.parse(profileJson), expected_version: identity.version }, true)
        await load()
      })}>Explicitly save identity configuration</button>
    </>}
    <button type="button" disabled={!enabled && (!identity || !!conversation.groupModels?.length || !!conversation.mode && conversation.mode !== 'lu')}
      className="block rounded border p-1" onClick={() => void run(async () => {
        if (!enabled) await load()
        useChatStore.getState().setConversationLucille(conversation.id, enabled ? undefined : { profile: 'lucille', scope, project, tokenBudget: budget })
      })}>{enabled ? 'Use native memory for this conversation' : 'Bind this conversation to the Lucille profile'}</button>
    <label className="block">Category <select value={kind} onChange={event => setKind(event.target.value)} className="rounded border bg-transparent p-1">{kinds.map(value => <option key={value}>{value}</option>)}</select></label>
    <label className="block">Factual key <input aria-label="Lucille factual key" value={factKey} onChange={event => setFactKey(event.target.value)} className="rounded border bg-transparent p-1" /></label>
    <label className="block">Candidate content / replacement text<textarea aria-label="Lucille candidate content" value={content} onChange={event => setContent(event.target.value)} className="block w-full rounded border bg-transparent p-2" /></label>
    <button type="button" className="rounded border p-1" onClick={() => void run(async () => {
      await lucilleManage({ operation: 'create', candidate: { id: crypto.randomUUID(), scope, project, kind, content, fact_key: factKey, source: 'manual', source_id: crypto.randomUUID() } }, true)
      await load()
    })}>Create candidate for review</button>
    <ul>{entries.map(entry => <li key={entry.input.id} className="my-2 border-t p-2">
      <p>{entry.input.kind} · {entry.state} · v{entry.version} · {entry.input.fact_key}</p><p className="whitespace-pre-wrap">{entry.input.content}</p>
      {['approve', 'reject', 'obsolete', 'update', 'delete'].map(action => <button type="button" key={action} className="mr-1 rounded border p-1"
        onClick={() => void run(() => change(entry, action))}>{action === 'update' ? 'Replace with editor text (requires reapproval)' : action}</button>)}
      <button type="button" className="rounded border p-1" onClick={() => void run(async () => setNotice(JSON.stringify(await lucilleManage({ operation: 'history', id: entry.input.id }))))}>History</button>
    </li>)}</ul>
    <label className="block">Synthetic backup<textarea aria-label="Lucille synthetic backup" value={backup} onChange={event => setBackup(event.target.value)} className="block w-full rounded border bg-transparent p-2" /></label>
    <button type="button" className="rounded border p-1" onClick={() => void run(async () => setBackup(await lucilleManage<string>({ operation: 'export' }, true)))}>Export synthetic backup</button>
    <button type="button" className="ml-2 rounded border p-1" onClick={() => void run(async () => {
      await lucilleManage({ operation: 'restore', backup }, true)
      await load()
    })}>Replace isolated database from backup</button>
    {notice && <p role="status" className="whitespace-pre-wrap">{notice}</p>}
  </fieldset>
}
