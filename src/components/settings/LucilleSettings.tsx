import { useGenerationStore } from "../../stores/generationStore"
import { useState } from 'react'
import type { Conversation } from '../../types/chat'
import { useChatStore } from '../../stores/chatStore'
import { isLucille, lucilleManage, type LucilleMemory, type LucilleProfile } from '../../lib/lucille-context'
import { LucilleLearningPanel } from './LucilleLearningPanel'

interface Quality { subject: string; property: string; temporal: string; expires_at?: number; replaces_id?: string; evidence_rule: string }
interface Detail { memory: LucilleMemory; quality?: Quality; provenance: { evidence: string; at: number }[]; revisions: number[] }

const kinds = ['fact', 'preference', 'hardware', 'project', 'decision', 'solution', 'goal', 'task', 'constraint', 'summary', 'hypothesis']

/** Explicit isolated QA configuration. No automatic profile import or discovery. */
export function LucilleSettings({ conversation }: { conversation: Conversation }) {
  const [project, setProject] = useState(conversation.lucille?.project ?? 'synthetic-project')
  const [scope, setScope] = useState(conversation.lucille?.scope ?? 'lucille')
  const [budget, setBudget] = useState(conversation.lucille?.tokenBudget ?? 4096)
  const [identity, setIdentity] = useState<{ profile: LucilleProfile; version: number } | null>(null)
  const [profileJson, setProfileJson] = useState('')
  const [entries, setEntries] = useState<LucilleMemory[]>([])
  const [details, setDetails] = useState<Record<string, Detail>>({})
  const [deleted, setDeleted] = useState<{ id: string; kind: string; deleted_at: number }[]>([])
  const [search, setSearch] = useState('')
  const [category, setCategory] = useState('all')
  const [status, setStatus] = useState('all')
  const [content, setContent] = useState('')
  const [factKey, setFactKey] = useState('')
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
    const records = await lucilleManage<Detail[]>({ operation: 'details', scope, project })
    setDetails(Object.fromEntries(records.map(record => [record.memory.input.id, record])))
    setDeleted(await lucilleManage({ operation: 'deleted', scope, project }))
  }
  async function change(entry: LucilleMemory, action: string) {
    const prior = entries.find(value => value.input.id === details[entry.input.id]?.quality?.replaces_id)
    if (action === 'approve' && prior) {
      await lucilleManage({ operation: 'resolve_conflict', id: entry.input.id, scope, project, expected_version: entry.version, prior_version: prior.version }, true)
      await load()
      return
    }
    await lucilleManage({ operation: 'change', id: entry.input.id, scope, project, expected_version: entry.version,
      change: action === 'update' ? { action, content, fact_key: factKey || entry.input.fact_key } : { action } }, true)
    await load()
  }
  return <fieldset disabled={busy || generating} className="space-y-2 rounded border border-purple-500 p-3 text-xs">
    <legend>Lucille — isolated QA</legend>
    <p>Use fictional data only. This profile requires the isolated debug application. Native extraction is bypassed for bound conversations; Lucille learning is controlled below.</p>
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
    <button type="button" disabled={!enabled && (!identity || !!conversation.groupModels?.length || !!conversation.mode && conversation.mode !== 'lu' && conversation.mode !== 'codex')}
      className="block rounded border p-1" onClick={() => void run(async () => {
        if (!enabled) await load()
        useChatStore.getState().setConversationLucille(conversation.id, enabled ? undefined : { profile: 'lucille', scope, project, tokenBudget: budget })
      })}>{enabled ? 'Use native memory for this conversation' : 'Bind this conversation to the Lucille profile'}</button>
    <LucilleLearningPanel scope={scope} project={project} bound={enabled} />
    <label className="block">Search memories <input aria-label="Search Lucille memories" value={search} onChange={event => setSearch(event.target.value)} className="rounded border bg-transparent p-1" /></label>
    <label>Category <select aria-label="Filter Lucille category" value={category} onChange={event => setCategory(event.target.value)}><option value="all">All categories</option>{kinds.map(kind => <option key={kind}>{kind}</option>)}</select></label>
    <label className="ml-2">State <select aria-label="Filter Lucille state" value={status} onChange={event => setStatus(event.target.value)}>{['all', 'active', 'candidate', 'conflict', 'obsolete', 'expired', 'deleted'].map(value => <option key={value}>{value}</option>)}</select></label>
    <button type="button" className="ml-2 rounded border p-1" onClick={() => void run(load)}>Refresh memories</button>
    <label className="block">Category <select value={kind} onChange={event => setKind(event.target.value)} className="rounded border bg-transparent p-1">{kinds.map(value => <option key={value}>{value}</option>)}</select></label>
    <label className="block">Topic identifier (optional) <input aria-label="Lucille factual key" value={factKey} onChange={event => setFactKey(event.target.value)} className="rounded border bg-transparent p-1" /></label>
    <label className="block">Candidate content / replacement text<textarea aria-label="Lucille candidate content" value={content} onChange={event => setContent(event.target.value)} className="block w-full rounded border bg-transparent p-2" /></label>
    <button type="button" className="rounded border p-1" onClick={() => void run(async () => {
      await lucilleManage({ operation: 'create', candidate: { id: crypto.randomUUID(), scope, project, kind, content, fact_key: factKey || `manual.${crypto.randomUUID()}`, source: 'manual', source_id: crypto.randomUUID() } }, true)
      await load()
    })}>Create candidate for review</button>
    <ul>{entries.filter(entry => {
      const expired = !!details[entry.input.id]?.quality?.expires_at && details[entry.input.id].quality!.expires_at! <= Date.now() / 1000
      return (category === 'all' || entry.input.kind === category) && (status === 'all' || (expired ? 'expired' : entry.state) === status) && entry.input.content.toLowerCase().includes(search.toLowerCase())
    }).map(entry => <li key={entry.input.id} data-lucille-memory-id={entry.input.id} className="my-2 border-t p-2">
      <p>{entry.input.kind} · {entry.state} · v{entry.version} · {entry.input.fact_key}</p><p className="whitespace-pre-wrap">{entry.input.content}</p>
      <p>Source: {entry.input.source} · {new Date(entry.created_at * 1000).toLocaleString()}</p>
      {details[entry.input.id]?.quality?.expires_at && <p>{details[entry.input.id].quality!.expires_at! <= Date.now() / 1000 ? 'Expired' : 'Valid until'}: {new Date(details[entry.input.id].quality!.expires_at! * 1000).toLocaleString()}</p>}
      {details[entry.input.id]?.quality?.replaces_id && <p className="text-amber-400">Possible replacement. The earlier fact is suspended from new context until this is reviewed.</p>}
      {details[entry.input.id]?.quality?.replaces_id && <p>Previous: {entries.find(value => value.input.id === details[entry.input.id].quality?.replaces_id)?.input.content ?? 'Earlier record is no longer current.'}</p>}
      <details><summary>Evidence and revision history</summary>{details[entry.input.id]?.provenance.map((source, index) => <blockquote key={index}>{source.evidence}</blockquote>)}
        {details[entry.input.id]?.revisions.map(revision => <button type="button" key={revision} className="mr-2 rounded border p-1" onClick={() => void run(async () => { await lucilleManage({ operation: 'undo_consolidation', id: entry.input.id, scope, project, expected_version: entry.version, revision }, true); await load() })}>Restore before version {revision}</button>)}
      </details>
      {['approve', 'reject', 'obsolete', 'update', 'delete'].map(action => <button type="button" key={action} className="mr-1 rounded border p-1"
        onClick={() => void run(() => change(entry, action))}>{action === 'update' ? 'Replace with editor text (requires reapproval)' : action === 'approve' && details[entry.input.id]?.quality?.replaces_id ? 'Approve replacement' : action}</button>)}
      <button type="button" className="rounded border p-1" onClick={() => void run(async () => setNotice(JSON.stringify(await lucilleManage({ operation: 'history', id: entry.input.id }))))}>History</button>
    </li>)}</ul>
    {(status === 'all' || status === 'deleted') && deleted.filter(entry => category === 'all' || entry.kind === category).map(entry => <p key={entry.id}>{entry.kind} · deleted · {new Date(entry.deleted_at * 1000).toLocaleString()} · content erased</p>)}
    <label className="block">Synthetic backup<textarea aria-label="Lucille synthetic backup" value={backup} onChange={event => setBackup(event.target.value)} className="block w-full rounded border bg-transparent p-2" /></label>
    <button type="button" className="rounded border p-1" onClick={() => void run(async () => setBackup(await lucilleManage<string>({ operation: 'export' }, true)))}>Export synthetic backup</button>
    <button type="button" className="ml-2 rounded border p-1" onClick={() => void run(async () => {
      await lucilleManage({ operation: 'restore', backup }, true)
      await load()
    })}>Replace isolated database from backup</button>
    {notice && <p role="status" className="whitespace-pre-wrap">{notice}</p>}
  </fieldset>
}
