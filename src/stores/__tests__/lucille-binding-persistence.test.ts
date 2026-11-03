import { beforeEach, expect, it, vi } from 'vitest'
import { useChatStore, flushChatPersist } from '../chatStore'
import { useGenerationStore } from '../generationStore'
import { hasLucilleBinding } from '../../lib/lucille-bindings'
import type { LucilleBinding } from '../../types/chat'

const storage = vi.hoisted(() => new Map<string, string>())
vi.mock('../../lib/idbStorage', () => ({ idbStorage: {
  getItem: async (name: string) => storage.get(name) ?? null,
  setItem: async (name: string, value: string) => { storage.set(name, value) },
  removeItem: async (name: string) => { storage.delete(name) },
} }))
const binding: LucilleBinding = { profile: 'lucille', scope: 'lucille', project: 'synthetic-a', tokenBudget: 4096 }

beforeEach(async () => {
  useGenerationStore.setState({ aborters: {} })
  await useChatStore.persist.rehydrate()
  useChatStore.setState({ conversations: [], activeConversationId: null })
})

it('persists only the routing binding and restores the native writer fence after reopen', async () => {
  const id = useChatStore.getState().createConversation('synthetic-model', '', 'lu')
  expect(hasLucilleBinding(id)).toBe(false)
  useChatStore.getState().setConversationLucille(id, binding)
  expect(hasLucilleBinding(id)).toBe(true)
  await flushChatPersist()
  const name = useChatStore.persist.getOptions().name!
  const saved = storage.get(name)!
  expect(saved).toContain('"profile":"lucille"')
  expect(saved).not.toContain('identity_profile')
  useChatStore.setState({ conversations: [] })
  expect(hasLucilleBinding(id)).toBe(false)
  storage.set(name, saved)
  await useChatStore.persist.rehydrate()
  expect(useChatStore.getState().conversations.find(conversation => conversation.id === id)?.lucille).toEqual(binding)
  expect(hasLucilleBinding(id)).toBe(true)
  useChatStore.getState().setConversationLucille(id, undefined)
  expect(hasLucilleBinding(id)).toBe(false)
})

it('allows Code and rejects invalid scopes, remote/groups and binding changes during a generation', () => {
  const id = useChatStore.getState().createConversation('synthetic-model', '', 'lu')
  useChatStore.getState().setConversationLucille(id, { ...binding, scope: '../operational' })
  expect(hasLucilleBinding(id)).toBe(false)
  useGenerationStore.setState({ aborters: { [id]: () => {} } })
  useChatStore.getState().setConversationLucille(id, binding)
  expect(hasLucilleBinding(id)).toBe(false)
  useGenerationStore.setState({ aborters: {} })
  const code = useChatStore.getState().createConversation('synthetic-model', '', 'codex')
  useChatStore.getState().setConversationLucille(code, binding)
  expect(hasLucilleBinding(code)).toBe(true)
  const remote = useChatStore.getState().createConversation("synthetic-model", "", "remote")
  useChatStore.getState().setConversationLucille(remote, binding)
  expect(hasLucilleBinding(remote)).toBe(false)
  useChatStore.getState().setGroupModels(id, ['synthetic-a', 'synthetic-b'])
  useChatStore.getState().setConversationLucille(id, binding)
  expect(hasLucilleBinding(id)).toBe(false)
})
