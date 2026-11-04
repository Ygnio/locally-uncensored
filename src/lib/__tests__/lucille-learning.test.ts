import { describe, it, expect, vi } from 'vitest'
import { readFileSync } from 'node:fs'
import { learningInteraction } from '../lucille-learning'
import type { Conversation } from '../../types/chat'
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }))
const fixture = () => ({ id: 'synthetic-conversation', mode: 'lu', lucille: { profile: 'lucille', scope: 'lucille', project: 'atlas' }, messages: [
  { id: 'user', role: 'user', content: 'Prefiro café sem açúcar.' },
  { id: 'answer', role: 'assistant', content: 'Synthetic answer, not evidence.' },
] } as Conversation)
describe('canonical Lucille learning boundary', () => {
  it.each(['chat', 'agent', 'code'] as const)('learns only the actual durable user message in %s', mode => {
    const input = learningInteraction(fixture(), 'answer', 'Prefiro café sem açúcar.', mode, false)
    expect(input).toMatchObject({ message_id: 'user', role: 'user', user_text: 'Prefiro café sem açúcar.', mode, project: 'atlas' })
    expect(JSON.stringify(input)).not.toContain('Synthetic answer')
  })
  it('preserves the original source across regenerated answers', () => {
    const conversation = fixture(); conversation.messages[0].lucilleInteractionId = 'original-user'
    expect(learningInteraction(conversation, 'answer', conversation.messages[0].content, 'chat', false)?.message_id).toBe('original-user')
  })
  it('refuses cancellation, hidden/generated text, tools, assistant errors and unbound sessions', () => {
    const text = fixture().messages[0].content
    expect(learningInteraction(fixture(), 'answer', text, 'chat', true)).toBeNull()
    expect(learningInteraction(fixture(), 'answer', 'Expanded attachment or generated instruction', 'chat', false)).toBeNull()
    for (const mutate of [
      (c: Conversation) => { c.lucille = undefined },
      (c: Conversation) => { c.mode = 'remote' },
      (c: Conversation) => { c.groupModels = ['synthetic'] },
      (c: Conversation) => { c.messages[0].hidden = true },
      (c: Conversation) => { c.messages[0].role = 'tool' },
      (c: Conversation) => { c.messages[1].error = { kind: 'interrupted', detail: '', at: 0 } },
      (c: Conversation) => { c.messages[1].content = '' },
    ]) { const conversation = fixture(); mutate(conversation); expect(learningInteraction(conversation, 'answer', text, 'chat', false)).toBeNull() }
  })
  it.each(['useChat', 'useAgentChat', 'useCodex'])('keeps learning and native writers fenced in real %s hook', hook => {
    const source = readFileSync(new URL(`../../hooks/${hook}.ts`, import.meta.url), 'utf8')
    expect(source).toContain('queueLucilleLearning(')
    expect(source).toContain('abort.signal')
    expect(source).toMatch(/!isLucille\(/)
    expect(source).toContain(hook === 'useChat' ? 'extractAndSave' : 'extractMemoriesFromPair')
  })
})

it('can learn only the durable user source when fidelity retains the assistant reply', () => {
  const conversation=fixture()
  conversation.messages[1].error={kind:'generic',detail:'Unverified response retained',at:0}
  conversation.messages[1].lucilleFidelity=[{status:'blocked',attempts:[],refreshMs:0,verificationMs:0,extraGenerationMs:0,additionalUsage:{}}]
  expect(learningInteraction(conversation,'answer',conversation.messages[0].content,'chat',false)?.user_text).toBe('Prefiro café sem açúcar.')
  expect(learningInteraction(conversation,'answer',conversation.messages[0].content,'chat',true)).toBeNull()
})
