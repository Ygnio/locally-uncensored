import { lucilleManage } from './lucille-context'
import { manifest, encodeIsolated, semanticStatus, type IndexStatus } from './lucille-semantic'
import { realValidationPin } from './lucille-real-validation'
import { useGenerationStore } from '../stores/generationStore'
let tail: Promise<void> = Promise.resolve()
let timer: ReturnType<typeof setInterval> | undefined
let busy = false
let cursor = 0
let active: AbortController | undefined
interface IndexPolicy { enabled: boolean; automatic_indexing: boolean }
const stopped = (signal?: AbortSignal) => { if (signal?.aborted) throw new DOMException('Cancelled index synchronization', 'AbortError') }
async function synchronize(scope: string, project: string, signal?: AbortSignal) {
  if (realValidationPin?.memory_protocol !== 3) return
  stopped(signal)
  const policy = await lucilleManage<IndexPolicy>({ operation: 'learning_policy' })
  if (!policy.enabled || !policy.automatic_indexing) return
  const space = await manifest(signal)
  const current = await lucilleManage<IndexStatus>({ operation: 'index_status' })
  if (JSON.stringify(current.space) !== JSON.stringify(space)) {
    // Native serde order can differ. Compare key/value sets instead.
    const same = current.space && Object.entries(space).every(([key, value]) => (current.space as unknown as Record<string, unknown>)[key] === value)
    if (!same) await lucilleManage({ operation: 'configure_embedding', space }, true)
  }
  const pending = await lucilleManage<{ id: string; version: number }[]>({ operation: 'pending_embeddings', scope, project, limit: 8 })
  for (const record of pending) {
    stopped(signal)
    const description = await lucilleManage<{ id: string; version: number; text: string }>({ operation: 'embedding_text', id: record.id, scope, project })
    const vector = await encodeIsolated(space, description.text, true, signal)
    stopped(signal)
    const latest = await lucilleManage<IndexPolicy>({ operation: 'learning_policy' })
    if (!latest.enabled || !latest.automatic_indexing) return
    // The canonical writer rejects deleted/edited/obsolete versions atomically.
    await lucilleManage({ operation: 'put_vector', id: description.id, version: description.version, space, vector }, true)
  }
  semanticStatus.state = 'REAL Nomic automatic index synchronized; canonical versions checked'
  semanticStatus.model = space.model; semanticStatus.simulation = false
}
export function syncLucilleIndex(scope: string, project: string, signal?: AbortSignal) {
  const next = tail.catch(() => {}).then(() => synchronize(scope, project, signal))
  tail = next
  return next
}
export async function tickLucilleIndex() {
  if (busy || realValidationPin?.memory_protocol !== 3 || Object.keys(useGenerationStore.getState().aborters).length) return
  busy = true; const abort = new AbortController(); active = abort
  const off = useGenerationStore.subscribe(s => { if (Object.keys(s.aborters).length) abort.abort() })
  try {
    const { useChatStore } = await import('../stores/chatStore')
    const bindings = [...new Map(useChatStore.getState().conversations.flatMap(c => c.lucille?.profile === 'lucille' ? [[`${c.lucille.scope}::${c.lucille.project}`, c.lucille] as const] : [])).values()]
    for (let i = 0; i < Math.min(4, bindings.length); i++) {
      stopped(abort.signal); const binding = bindings[cursor++ % bindings.length]
      await syncLucilleIndex(binding.scope, binding.project, abort.signal)
    }
  } catch { if (!abort.signal.aborted) semanticStatus.state = 'lexical fallback; automatic indexing waits for the verified endpoint' }
  finally { off(); busy = false; if (active === abort) active = undefined }
}
export function startLucilleIndex() {
  if (realValidationPin?.memory_protocol !== 3) return
  if (!timer) timer = setInterval(() => { void tickLucilleIndex() }, 3000)
  void tickLucilleIndex()
}
if (import.meta.hot) import.meta.hot.dispose(() => { if (timer) clearInterval(timer); active?.abort() })
