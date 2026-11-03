import { invoke } from '@tauri-apps/api/core'
import { localFetch } from '../api/backend'
export interface EmbeddingSpace { model: string; revision: string; fingerprint: string; dimension: number; normalization: string; pooling: string; document_prefix: string; query_prefix: string; context_tokens: number; index_version: number }
export interface IndexStatus { space: EmbeddingSpace | null; indexed: number; transport_verified: boolean }
export const semanticStatus = { state: 'lexical fallback; no transport verified', model: '', simulation: false }
const manage = <T>(request: object, explicit = false): Promise<T> => invoke('lucille_manage', { request, ...(explicit ? { confirmation: 'CONFIRM LUCILLE CONFIGURATION' } : {}) })
const origin = 'http://127.0.0.1:15284'
const signature = (space: EmbeddingSpace) => JSON.stringify(Object.entries(space).sort(([a], [b]) => a.localeCompare(b)))
export function validateSpace(value: unknown): EmbeddingSpace {
  if (!value || typeof value !== 'object') throw new Error('Missing embedding manifest')
  const s = value as EmbeddingSpace
  const keys = ['model', 'revision', 'fingerprint', 'dimension', 'normalization', 'pooling', 'document_prefix', 'query_prefix', 'context_tokens', 'index_version']
  if (Object.keys(s).length !== keys.length || keys.some(k => !(k in s)) || s.model !== 'nomic-ai/nomic-embed-text-v1.5' || typeof s.revision !== 'string' || !s.revision || s.revision.length > 512 || !/^[0-9a-f]{64}$/.test(s.fingerprint) || ![64, 128, 256, 512, 768].includes(s.dimension) || s.pooling !== 'mean' || !['l2', 'layer_norm_then_l2'].includes(s.normalization) || s.document_prefix !== 'search_document: ' || s.query_prefix !== 'search_query: ' || !Number.isInteger(s.context_tokens) || s.context_tokens < 1 || s.context_tokens > 8192 || !Number.isInteger(s.index_version) || s.index_version < 1) throw new Error('Incompatible Nomic manifest')
  return s
}
export function validateVector(vector: unknown, dimension: number): number[] {
  if (!Array.isArray(vector) || vector.length !== dimension || !vector.every(v => typeof v === 'number' && Number.isFinite(v)) || Math.abs(Math.sqrt(vector.reduce((n, v) => n + v * v, 0)) - 1) > 0.001) throw new Error('Invalid normalized embedding')
  return vector
}
async function manifest(signal?: AbortSignal) {
  const response = await localFetch(origin + '/lucille/embedding-manifest', { signal, timeoutMs: 3000 })
  if (!response.ok) throw new Error('Embedding endpoint unavailable')
  const raw = await response.text()
  if (raw.length > 4096) throw new Error('Oversized manifest')
  const data = JSON.parse(raw)
  // Advertised names are not weight attestation. This stage permits the
  // explicitly marked simulated test contract only, never claims real Nomic.
  if (data.simulation !== true) throw new Error('Real weight attestation not established')
  return validateSpace(data.space)
}
export async function encodeIsolated(space: EmbeddingSpace, text: string, document: boolean, signal?: AbortSignal) {
  if (signature(await manifest(signal)) !== signature(space)) throw new Error('Embedding space changed')
  const input = (document ? space.document_prefix : space.query_prefix) + text
  if (new TextEncoder().encode(input).length > space.context_tokens) throw new Error('Embedding input ceiling')
  const response = await localFetch(origin + '/v1/embeddings', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ model: space.model, input, encoding_format: 'float' }), signal, timeoutMs: 3000 })
  if (!response.ok) throw new Error('Embedding request failed')
  const raw = await response.text()
  if (raw.length > 100000) throw new Error('Oversized vector response')
  const data = JSON.parse(raw)
  if (data.model !== space.model || !Array.isArray(data.data) || data.data.length !== 1 || data.data[0].index !== 0 || signature(await manifest(signal)) !== signature(space)) throw new Error('Embedding identity or response changed')
  return validateVector(data.data[0].embedding, space.dimension)
}
export async function initializeSimulatedIndex(scope: string, project: string) {
  const space = await manifest()
  await manage({ operation: 'configure_embedding', space }, true)
  const records = await manage<{ memory: { input: { id: string; content: string }; state: string; version: number } }[]>({ operation: 'details', scope, project })
  for (const { memory } of records.filter(d => d.memory.state === 'active').slice(0, 64)) {
    const vector = await encodeIsolated(space, memory.input.content, true)
    await manage({ operation: 'put_vector', id: memory.input.id, version: memory.version, space, vector }, true)
  }
  semanticStatus.state = 'SIMULATED index ready; real Nomic unverified'; semanticStatus.model = space.model; semanticStatus.simulation = true
  return manage<IndexStatus>({ operation: 'index_status' })
}
export async function queryVector(query: string, signal?: AbortSignal) {
  try {
    const status = await manage<IndexStatus>({ operation: 'index_status' })
    if (!status.space) return null
    const space = validateSpace(status.space)
    const vector = await encodeIsolated(space, query, false, signal)
    semanticStatus.state = 'SIMULATED hybrid retrieval; real Nomic unverified'; semanticStatus.simulation = true; semanticStatus.model = space.model
    return { space, vector }
  } catch { semanticStatus.state = 'lexical fallback; embedding endpoint unavailable or unverified'; return null }
}
