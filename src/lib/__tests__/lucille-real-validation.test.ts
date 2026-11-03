import { afterEach, expect, it, vi } from 'vitest'
vi.mock('../../api/backend', () => ({ localFetch: vi.fn() }))
afterEach(() => { vi.unstubAllEnvs(); vi.resetModules() })
const pin = { simulation: false, runtime_sha256: 'c'.repeat(64), models: {
  gemma: { model: 'fictional-gemma-fixture', revision: 'g1', fingerprint: 'a'.repeat(64), context_tokens: 65536 },
  nomic: { model: 'nomic-ai/nomic-embed-text-v1.5', revision: 'n1', fingerprint: 'b'.repeat(64), context_tokens: 2048 },
} }
it('cannot turn an advertised server name into real verification without a pinned QA contract', async () => {
  vi.stubEnv('VITE_LUCILLE_REAL_VALIDATION_PIN', '')
  const { validateRealManifest } = await import('../lucille-real-validation')
  expect(() => validateRealManifest(pin, 'nomic')).toThrow()
})
it('rejects changed weights, revisions and simulation despite a matching model name', async () => {
  vi.stubEnv('VITE_LUCILLE_REAL_VALIDATION_PIN', JSON.stringify(pin))
  const { validateRealManifest } = await import('../lucille-real-validation')
  expect(validateRealManifest(pin, 'nomic')).toEqual(pin)
  expect(() => validateRealManifest({ ...pin, simulation: true }, 'nomic')).toThrow()
  for (const patch of [{ fingerprint: 'd'.repeat(64) }, { revision: 'different' }]) {
    expect(() => validateRealManifest({ ...pin, models: { ...pin.models, nomic: { ...pin.models.nomic, ...patch } } }, 'nomic')).toThrow()
  }
})
