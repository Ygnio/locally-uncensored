import { afterEach, beforeEach, expect, it, vi } from 'vitest'
const mock = vi.hoisted(() => ({ manage: vi.fn(), encode: vi.fn(), manifest: vi.fn(), pin: { memory_protocol: 3 as number | undefined }, generation: { aborters: {} as Record<string, unknown> } }))
vi.mock('../lucille-context', () => ({ lucilleManage: mock.manage }))
vi.mock('../lucille-real-validation', () => ({ realValidationPin: mock.pin }))
vi.mock('../lucille-semantic', () => ({ manifest: mock.manifest, encodeIsolated: mock.encode, semanticStatus: {} }))
vi.mock('../../stores/generationStore', () => ({ useGenerationStore: { getState: () => mock.generation, subscribe: () => () => {} } }))
const space = { model: 'nomic-ai/nomic-embed-text-v1.5', revision: 'synthetic', fingerprint: 'f'.repeat(64), dimension: 3, normalization: 'l2', pooling: 'mean', document_prefix: 'search_document: ', query_prefix: 'search_query: ', context_tokens: 2048, index_version: 4 }
beforeEach(() => {
  vi.resetModules(); vi.clearAllMocks(); mock.pin.memory_protocol = 3; mock.generation.aborters = {}
  mock.manifest.mockResolvedValue(space); mock.encode.mockResolvedValue([1, 0, 0])
  mock.manage.mockImplementation(async (r: { operation: string }) => {
    if (r.operation === 'learning_policy') return { enabled: true, automatic_indexing: true }
    if (r.operation === 'index_status') return { space, indexed: 0 }
    if (r.operation === 'pending_embeddings') return [{ id: 'current', version: 4 }]
    if (r.operation === 'embedding_text') return { id: 'current', version: 5, text: 'Current canonical description, after concurrent revision.' }
    return null
  })
})
afterEach(() => { vi.restoreAllMocks() })
it('indexes the current canonical description/version rather than an earlier pending snapshot', async () => {
  const { syncLucilleIndex } = await import('../lucille-index-worker'); await syncLucilleIndex('scope', 'project')
  expect(mock.encode).toHaveBeenCalledWith(space, 'Current canonical description, after concurrent revision.', true, undefined)
  expect(mock.manage).toHaveBeenCalledWith({ operation: 'put_vector', id: 'current', version: 5, space, vector: [1, 0, 0] }, true)
  expect(mock.manage.mock.calls.some(([r]) => ['create', 'approve', 'change', 'save_learning_policy'].includes(r.operation))).toBe(false)
})
it('does no network or index work without explicit indexing approval', async () => {
  mock.manage.mockResolvedValue({ enabled: true, automatic_indexing: false })
  const { syncLucilleIndex } = await import('../lucille-index-worker'); await syncLucilleIndex('s', 'p')
  expect(mock.manifest).not.toHaveBeenCalled(); expect(mock.encode).not.toHaveBeenCalled()
})
it('revokes a late index write when the user disables indexing during encoding', async () => {
  mock.encode.mockImplementation(async () => { mock.manage.mockImplementation(async () => ({ enabled: false, automatic_indexing: false })); return [1, 0, 0] })
  const { syncLucilleIndex } = await import('../lucille-index-worker'); await syncLucilleIndex('s', 'p')
  expect(mock.manage.mock.calls.some(([r]) => r.operation === 'put_vector')).toBe(false)
})
it('does not commit an embedding after cancellation and serializes later recovery', async () => {
  const abort = new AbortController(); mock.encode.mockImplementationOnce(async () => { abort.abort(); return [1, 0, 0] })
  const { syncLucilleIndex } = await import('../lucille-index-worker')
  await expect(syncLucilleIndex('s', 'p', abort.signal)).rejects.toThrow('Cancelled')
  expect(mock.manage.mock.calls.some(([r]) => r.operation === 'put_vector')).toBe(false)
  await syncLucilleIndex('s', 'p'); expect(mock.manage.mock.calls.some(([r]) => r.operation === 'put_vector')).toBe(true)
})
it('keeps legacy builds idle and gives primary generation priority', async () => {
  const interval = vi.spyOn(globalThis, 'setInterval'); mock.pin.memory_protocol = undefined
  const { startLucilleIndex, tickLucilleIndex } = await import('../lucille-index-worker'); startLucilleIndex(); expect(interval).not.toHaveBeenCalled()
  mock.pin.memory_protocol = 3; mock.generation.aborters = { primary: {} }; await tickLucilleIndex(); expect(mock.manage).not.toHaveBeenCalled()
})
