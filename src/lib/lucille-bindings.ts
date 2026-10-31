import type { Conversation } from '../types/chat'

// An in-process routing fence, derived from the chat store. Contains IDs only;
// identity and memory records remain solely in Rust/SQLite. Keeping this tiny
// module independent avoids loading chat/remote stores in the native extractor.
let bound = new Set<string>()
export function syncLucilleBindings(conversations: Pick<Conversation, 'id' | 'lucille'>[]): void {
  bound = new Set(conversations.filter(conversation => conversation.lucille?.profile === 'lucille').map(conversation => conversation.id))
}
export const hasLucilleBinding = (conversationId: string): boolean => bound.has(conversationId)
