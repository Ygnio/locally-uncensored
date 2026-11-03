import { beforeEach, describe, expect, it, vi } from 'vitest'
import { encodeIsolated, queryVector, validateSpace, validateVector, type EmbeddingSpace } from '../lucille-semantic'
const mock = vi.hoisted(() => ({ fetch: vi.fn(), invoke: vi.fn() }))
vi.mock('../../api/backend', () => ({ localFetch: mock.fetch }))
vi.mock('@tauri-apps/api/core', () => ({ invoke: mock.invoke }))
const space: EmbeddingSpace = { model: 'nomic-ai/nomic-embed-text-v1.5', revision: 'SYNTHETIC', fingerprint: 'a'.repeat(64), dimension: 64, normalization: 'l2', pooling: 'mean', document_prefix: 'search_document: ', query_prefix: 'search_query: ', context_tokens: 8192, index_version: 1 }
const vector = [1, ...Array(63).fill(0)]
const response = (data: unknown) => new Response(JSON.stringify(data), { status: 200 })
beforeEach(() => {
  vi.clearAllMocks()
  mock.invoke.mockResolvedValue({ space, indexed: 1 })
  mock.fetch.mockImplementation(async (url: string) => url.endsWith('embedding-manifest') ? response({ simulation: true, space }) : response({ model: space.model, data: [{ index: 0, embedding: vector }] }))
})
describe('isolated semantic transport', () => {
  it('uses distinct prefixes, verifies manifest twice and validates effective dimension', async () => {
    expect(await encodeIsolated(space, 'fictional document', true)).toEqual(vector)
    expect(JSON.parse(mock.fetch.mock.calls[1][1].body).input).toBe('search_document: fictional document')
    expect(mock.fetch).toHaveBeenCalledTimes(3)
    await encodeIsolated(space, 'fictional query', false)
    expect(JSON.parse(mock.fetch.mock.calls[4][1].body).input).toBe('search_query: fictional query')
  })
  it('rejects advertised real identity without weight attestation', async () => {
    mock.fetch.mockResolvedValue(response({ simulation: false, space }))
    await expect(encodeIsolated(space, 'fictional', false)).rejects.toThrow('attestation')
  })
  it('rejects model drift between encoding and final verification', async () => {
    let manifests = 0
    mock.fetch.mockImplementation(async (url: string) => url.endsWith('embedding-manifest') ? response({ simulation: true, space: { ...space, revision: ++manifests === 1 ? space.revision : 'changed' } }) : response({ model: space.model, data: [{ index: 0, embedding: vector }] }))
    await expect(encodeIsolated(space, 'fictional', false)).rejects.toThrow('changed')
  })
  it('fails back to lexical on endpoint/shape failure and never writes vectors on retrieval', async () => {
    mock.fetch.mockRejectedValue(new Error('endpoint unavailable'))
    expect(await queryVector('fictional')).toBeNull()
    expect(mock.invoke.mock.calls.every(call => call[1].request.operation === 'index_status')).toBe(true)
  })
  it('rejects invalid spaces, nonfinite vectors, wrong dimension and context overflow', async () => {
    expect(() => validateSpace({ ...space, pooling: 'cls' })).toThrow()
    expect(() => validateSpace({ ...space, permission: 'admin' })).toThrow()
    expect(() => validateVector([NaN], 1)).toThrow()
    expect(() => validateVector([2], 1)).toThrow()
    expect(() => validateVector([], 64)).toThrow()
    await expect(encodeIsolated(space, 'x'.repeat(8192), false)).rejects.toThrow('ceiling')
  })
})
