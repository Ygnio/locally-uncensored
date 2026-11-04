// Only the dedicated QA Vite configuration can embed this independently pinned
// file contract. An advertised server name or arbitrary runtime manifest is insufficient.
import { localFetch } from '../api/backend'
type ModelPin = { model: string; revision: string; fingerprint: string; context_tokens: number; index_version?: number }
interface Pin { memory_protocol?: number; simulation: false; runtime_sha256: string; models: { gemma: ModelPin; nomic: ModelPin } }
export const realValidationPin: Pin | null = (() => {
  const raw: unknown = import.meta.env.VITE_LUCILLE_REAL_VALIDATION_PIN
  if (typeof raw !== 'string' || !raw) return null
  try {
    const pin = JSON.parse(raw) as Pin
    if (![2, 3].includes(pin.memory_protocol ?? 2) || (pin.memory_protocol === 3 && ![4, 5].includes(pin.models.nomic.index_version ?? 0)) || pin.simulation !== false || !/^[a-f0-9]{64}$/.test(pin.runtime_sha256) || pin.models.gemma.context_tokens !== 65536 || pin.models.nomic.model !== 'nomic-ai/nomic-embed-text-v1.5' || !Object.values(pin.models).every(m => /^[a-f0-9]{64}$/.test(m.fingerprint) && m.revision)) return null
    return pin
  } catch { return null }
})()
const stable = (value: unknown): string => JSON.stringify(value, (_key, v) => v && typeof v === 'object' && !Array.isArray(v) ? Object.fromEntries(Object.entries(v).sort(([a], [b]) => a.localeCompare(b))) : v)
export function validateRealManifest(value: unknown, kind: 'gemma' | 'nomic') {
  const data = value as Pin
  if (!realValidationPin || !data || data.simulation !== false || (data.memory_protocol ?? 2) !== (realValidationPin.memory_protocol ?? 2) || data.runtime_sha256 !== realValidationPin.runtime_sha256 || stable(data.models?.[kind]) !== stable(realValidationPin.models[kind])) throw new Error('Real test attestation differs from independently pinned contract')
  return data
}
export async function probeRealGemma() {
  if (!realValidationPin) return false
  const response = await localFetch('http://127.0.0.1:15284/lucille/extraction-manifest', { timeoutMs: 3000 })
  if (!response.ok) return false
  validateRealManifest(await response.json(), 'gemma')
  return true
}
