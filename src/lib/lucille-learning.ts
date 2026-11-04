import { invoke } from '@tauri-apps/api/core'
import type { Conversation } from '../types/chat'
import type { LucilleMode } from './verified-environment'
import { isLucille } from './lucille-context'

/** Only a canonical USER message before the finished answer can be evidence. */
export function learningInteraction(conversation: Conversation | undefined, assistantId: string, userText: string, mode: LucilleMode, aborted: boolean) {
  if (aborted || !isLucille(conversation) || !conversation || conversation.groupModels?.length || conversation.mode === 'remote') return null
  const index = conversation.messages.findIndex(message => message.id === assistantId)
  const answer = conversation.messages[index]
  const retained = answer?.error?.kind === 'generic' && answer.lucilleFidelity?.at(-1)?.status === 'blocked'
  if (index < 0 || answer?.role !== 'assistant' || !answer.content.trim() || (answer.error && !retained) || answer.hidden) return null
  const user = conversation.messages.slice(0, index).findLast(message => message.role === 'user')
  if (!user || user.hidden || user.notice || user.content !== userText || user.content.length > 16000 || user.content.trim().length < 12) return null
  return { conversation_id: conversation.id, message_id: user.lucilleInteractionId ?? user.id,
    scope: conversation.lucille!.scope, project: conversation.lucille!.project, mode, role: 'user', user_text: user.content }
}
/** Fire-and-forget after the turn's durable commit; no generation/provider call. */
export async function queueLucilleLearning(conversationId: string, assistantId: string, userText: string, mode: LucilleMode, signal: AbortSignal) {
  if (signal.aborted) return
  const { useChatStore } = await import('../stores/chatStore')
  const conversation = useChatStore.getState().conversations.find(value => value.id === conversationId)
  const interaction = learningInteraction(conversation, assistantId, userText, mode, signal.aborted)
  if (interaction) { await invoke('lucille_learn', { interaction }); const { startLucilleLlm } = await import('./lucille-llm'); startLucilleLlm(); const { startLucilleIndex } = await import('./lucille-index-worker'); startLucilleIndex() }
}
export async function resumeLucilleLearning() {
  const { useChatStore } = await import('../stores/chatStore')
  if (!useChatStore.persist.hasHydrated()) await new Promise<void>(resolve => {
    const off = useChatStore.persist.onFinishHydration(() => { off(); resolve() })
    if (useChatStore.persist.hasHydrated()) { off(); resolve() }
  })
  if (useChatStore.getState().conversations.some(isLucille)) { await invoke('lucille_learning_wake'); const { startLucilleLlm } = await import('./lucille-llm'); startLucilleLlm(); const { startLucilleIndex } = await import('./lucille-index-worker'); startLucilleIndex() }
}
