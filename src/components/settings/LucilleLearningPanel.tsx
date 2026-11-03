import { useCallback, useEffect, useState } from 'react'
import { initializeSimulatedIndex, semanticStatus } from '../../lib/lucille-semantic'
import { invoke } from '@tauri-apps/api/core'
import { extractionStatus, startLucilleLlm } from '../../lib/lucille-llm'
import { lucilleManage } from '../../lib/lucille-context'
import { startLucilleIndex } from '../../lib/lucille-index-worker'
import { probeRealGemma, realValidationPin } from '../../lib/lucille-real-validation'

interface Policy { autonomous_preferences?: boolean; automatic_indexing?: boolean; enabled: boolean; extraction_mode: 'rules' | 'llm' | 'hybrid'; auto_kinds: string[]; minimum_confidence: number; minimum_relevance: number; max_pending: number; lexical_weight: number; semantic_weight: number; minimum_similarity: number }
interface Job { id: string; state: string; attempts: number; error_code?: string }
export function LucilleLearningPanel({ scope, project, bound }: { scope: string; project: string; bound: boolean }) {
  const [policy, setPolicy] = useState<Policy | null>(null)
  const [jobs, setJobs] = useState<Job[]>([])
  const [notice, setNotice] = useState('')
  const [counts, setCounts] = useState({ candidates: 0, conflicts: 0 })
  const [saving, setSaving] = useState(false)
  const [realGemma, setRealGemma] = useState(false)
  useEffect(() => {
    if (!realValidationPin) return
    let live = true
    const probe = () => { void probeRealGemma().then(value => { if (live) setRealGemma(value) }).catch(() => { if (live) setRealGemma(false) }) }
    probe(); const timer = setInterval(probe, 3000)
    return () => { live = false; clearInterval(timer) }
  }, [])
  const refresh = useCallback(async () => {
    setJobs(await lucilleManage<Job[]>({ operation: 'learning_jobs', scope, project }))
    const records = await lucilleManage<{ state: string }[]>({ operation: 'list', scope, project })
    setCounts({ candidates: records.filter(r => r.state === 'candidate').length, conflicts: records.filter(r => r.state === 'conflict').length })
  }, [scope, project])
  useEffect(() => {
    let live = true
    void lucilleManage<Policy>({ operation: 'learning_policy' }).then(value => { if (live) setPolicy(value) }).catch(() => { if (live) setNotice('Learning settings unavailable.') })
    const update = () => { if (live) void refresh().catch(() => setNotice('Processing status unavailable.')) }
    update()
    const timer = setInterval(update, 1500)
    return () => { live = false; clearInterval(timer) }
  }, [refresh])
  async function save(next: Policy) {
    setSaving(true); setNotice('')
    try {
      await lucilleManage({ operation: 'save_learning_policy', policy: next }, true)
      setPolicy(next)
      if (next.enabled) { await invoke('lucille_learning_wake'); startLucilleLlm(); startLucilleIndex() }
      await refresh()
    } catch { setNotice('Could not save learning settings.') } finally { setSaving(false) }
  }
  return <section className="space-y-2 border-y p-2" aria-label="Lucille automatic learning">
    <h3>Automatic learning</h3>
    <p>Only eligible messages you wrote in explicitly bound conversations are considered. Unsupported, sensitive, assistant and tool content is skipped. {realValidationPin ? 'Isolated real-model validation; no model is loaded by this panel. Transport verification does not imply quality approval or production readiness.' : 'No additional model is loaded. Real Gemma and Nomic validation is pending.'}</p>
    {realValidationPin && <p>Gemma test endpoint: {realGemma ? 'REAL weights/runtime verified; 65536 context' : 'UNAVAILABLE or verification failed'}. Extraction quality is evaluated separately.</p>}
    <p>LLM: {extractionStatus.state} {extractionStatus.model} {extractionStatus.simulation ? '(SIMULATED, no Gemma inference)' : ''}</p>
    {policy && <>
      <label className="block">Extraction mode <select aria-label="Lucille extraction mode" value={policy.extraction_mode} disabled={saving} onChange={event => void save({ ...policy, extraction_mode: event.target.value as Policy['extraction_mode'] })}><option value="rules">Rules</option><option value="llm">LLM (requires isolated local endpoint)</option><option value="hybrid">Hybrid (rules + one LLM call)</option></select></label>
      <label className="block"><input type="checkbox" aria-label="Enable Lucille automatic learning" disabled={saving || !bound} checked={policy.enabled} onChange={event => void save({ ...policy, enabled: event.target.checked })} /> Learn from eligible user messages</label>
      <label className="block"><input type="checkbox" aria-label="Automatically approve simple preferences" disabled={saving} checked={policy.auto_kinds.includes('preference')} onChange={event => void save({ ...policy, auto_kinds: event.target.checked ? ['preference'] : [] })} /> Automatically approve clear, non-sensitive preferences without conflicts</label>
      {realValidationPin?.memory_protocol === 3 && <>
        <label className="block"><input type="checkbox" aria-label="Enable safe autonomous preferences" checked={policy.autonomous_preferences ?? false} disabled={saving || !bound} onChange={event => void save({ ...policy, autonomous_preferences: event.target.checked })} /> Learn clear preferences without asking for review</label>
        <label className="block"><input type="checkbox" aria-label="Automatically update Lucille index" checked={policy.automatic_indexing ?? false} disabled={saving || !bound} onChange={event => void save({ ...policy, automatic_indexing: event.target.checked, ...(event.target.checked ? { minimum_similarity: 0.6, lexical_weight: 0.5, semantic_weight: 0.5 } : {}) })} /> Keep remembered information searchable automatically</label>
        <p>Only independently verified preferences qualify. Projects, equipment and unclear changes stay in review. Explicit preference corrections keep history. Forgetting a topic blocks old replay; a fresh request to remember the same preference again needs review.</p>
      </>}
      <p>Hardware, decisions, goals, tasks, hypotheses and ambiguous changes wait for review. Disabling learning cancels pending jobs.</p>
    </>}
    {!bound && <p>Bind this conversation before enabling learning.</p>}
    <p>Embeddings: {semanticStatus.state} {semanticStatus.model}</p>
    <button type="button" disabled={saving || !bound} onClick={() => { setSaving(true); void initializeSimulatedIndex(scope, project).then(value => setNotice(realValidationPin ? `REAL Nomic index: ${value.indexed} current vectors. QA only; quality measured separately.` : `SIMULATED index: ${value.indexed} current vectors. Real Nomic not validated.`)).catch(() => setNotice('Embedding endpoint unavailable; lexical fallback remains available.')).finally(() => setSaving(false)) }}>{realValidationPin ? 'Build real Nomic test index' : 'Build simulated semantic index'}</button>
    <p>{realValidationPin ? 'Pinned real weights are checked by the isolated broker against actual file hashes and owned process identity. Unsupported or unavailable endpoints fail to lexical fallback.' : 'Real Gemma/Nomic weights and performance are unverified.'} Indexing is limited to 64 current memories per explicit action.</p>
    <p>Candidates: {counts.candidates} · Conflicts: {counts.conflicts} · Failed jobs: {jobs.filter(j => j.state === 'failed').length}</p>
    <h4>Processing for this project</h4>
    <button type="button" onClick={() => void refresh().catch(() => setNotice('Processing status unavailable.'))}>Refresh processing</button>
    {jobs.length === 0 ? <p>No eligible interactions queued.</p> : <ul>{jobs.map(job => <li key={job.id}>{job.state} · attempt {job.attempts}/3 · {job.id.slice(0, 8)}
      {['queued', 'running', 'retry'].includes(job.state) && <button type="button" className="ml-2 rounded border p-1" onClick={() => void lucilleManage({ operation: 'cancel_learning', id: job.id }, true).then(refresh).catch(() => setNotice('Cancellation unavailable.'))}>Cancel processing</button>}
      {job.error_code && <span> · Processing failed; no unverified facts were accepted.</span>}
    </li>)}</ul>}
    {notice && <p role="status">{notice}</p>}
  </section>
}
