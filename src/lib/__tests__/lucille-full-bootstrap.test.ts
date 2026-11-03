import { describe, expect, it } from 'vitest'
import { readFileSync } from 'node:fs'
import { runInNewContext } from 'node:vm'
import { useSettingsStore } from '../../stores/settingsStore'
import { DEFAULT_SETTINGS } from '../constants'

const bootstrap = readFileSync(new URL('../../../qa/full-bootstrap.js', import.meta.url), 'utf8')
function storage(initial: Record<string, string> = {}) {
  const values = new Map(Object.entries(initial))
  return { getItem: (key: string) => values.get(key) ?? null, setItem: (key: string, value: string) => values.set(key, value), values }
}
describe('full application validation fixture', () => {
  it('hydrates all real settings defaults before the real Code composer reads engine context', async () => {
    const localStorage = storage()
    runInNewContext(bootstrap, { localStorage })
    const saved = JSON.parse(localStorage.getItem('chat-settings')!)
    const migrated = await useSettingsStore.persist.getOptions().migrate!(saved.state, saved.version) as ReturnType<typeof useSettingsStore.getState>
    expect(migrated.settings.builtinEngine.ctx).toBe(DEFAULT_SETTINGS.builtinEngine.ctx)
    expect(migrated.settings.onboardingDone).toBe(true)
    expect(migrated.settings.defaultWorkspace).toEqual({ kind: 'sandbox' })
    const providers = JSON.parse(localStorage.getItem('lu-providers')!).state.providers
    expect(providers.openai).toMatchObject({ baseUrl: 'http://127.0.0.1:15284/v1', managed: false })
  })
  it('never overwrites a previously seeded exclusive profile on restart', () => {
    const localStorage = storage({ 'lucille-phase16-seeded': 'synthetic-only', 'chat-settings': 'preserved-synthetic-fixture' })
    runInNewContext(bootstrap, { localStorage })
    expect(localStorage.getItem('chat-settings')).toBe('preserved-synthetic-fixture')
    expect(localStorage.values.size).toBe(2)
  })
})
