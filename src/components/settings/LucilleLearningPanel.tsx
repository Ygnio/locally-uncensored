import { useCallback, useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { lucilleManage } from '../../lib/lucille-context'

interface Policy { enabled: boolean; auto_kinds: string[]; minimum_confidence: number; minimum_relevance: number; max_pending: number; lexical_weight: number; semantic_weight: number; minimum_similarity: number }
interface Job { id: string; state: string; attempts: number; error_code?: string }
export function LucilleLearningPanel({ scope, project, bound }: { scope: string; project: string; bound: boolean }) {
  const [policy, setPolicy] = useState<Policy | null>(null)
  const [jobs, setJobs] = useState<Job[]>([])
  const [notice, setNotice] = useState('')
  const [saving, setSaving] = useState(false)
  const refresh = useCallback(async () => {
    setJobs(await lucilleManage<Job[]>({ operation: 'learning_jobs', scope, project }))
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
      if (next.enabled) await invoke('lucille_learning_wake')
      await refresh()
    } catch { setNotice('Could not save learning settings.') } finally { setSaving(false) }
  }
  return <section className="space-y-2 border-y p-2" aria-label="Lucille automatic learning">
    <h3>Automatic learning</h3>
    <p>Only eligible messages you wrote in explicitly bound conversations are considered. Unsupported, sensitive, assistant and tool content is skipped. No additional model is loaded.</p>
    {policy && <>
      <label className="block"><input type="checkbox" aria-label="Enable Lucille automatic learning" disabled={saving || !bound} checked={policy.enabled} onChange={event => void save({ ...policy, enabled: event.target.checked })} /> Learn from eligible user messages</label>
      <label className="block"><input type="checkbox" aria-label="Automatically approve simple preferences" disabled={saving} checked={policy.auto_kinds.includes('preference')} onChange={event => void save({ ...policy, auto_kinds: event.target.checked ? ['preference'] : [] })} /> Automatically approve clear, non-sensitive preferences without conflicts</label>
      <p>Hardware, decisions, goals, tasks, hypotheses and ambiguous changes wait for review. Disabling learning cancels pending jobs.</p>
    </>}
    {!bound && <p>Bind this conversation before enabling learning.</p>}
    <h4>Processing for this project</h4>
    <button type="button" onClick={() => void refresh().catch(() => setNotice('Processing status unavailable.'))}>Refresh processing</button>
    {jobs.length === 0 ? <p>No eligible interactions queued.</p> : <ul>{jobs.map(job => <li key={job.id}>{job.state} · attempt {job.attempts}/3 · {job.id.slice(0, 8)}
      {['queued', 'running', 'retry'].includes(job.state) && <button type="button" className="ml-2 rounded border p-1" onClick={() => void lucilleManage({ operation: 'cancel_learning', id: job.id }, true).then(refresh).catch(() => setNotice('Cancellation unavailable.'))}>Cancel processing</button>}
      {job.error_code && <span> · Processing failed; no unverified facts were accepted.</span>}
    </li>)}</ul>}
    {notice && <p role="status">{notice}</p>}
  </section>
}
