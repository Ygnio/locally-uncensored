import { invoke } from '@tauri-apps/api/core'
import type { Conversation } from '../types/chat'
import type { LucilleMode } from './verified-environment'
import { isLucille, lucilleManage } from './lucille-context'
import type { MemoryFact, MemoryDirectiveReceipt, SourceSpeakerSummary } from './lucille-fidelity'

export function userSourceInteraction(conversation: Conversation | undefined, assistantId: string, mode: LucilleMode) {
  if (!isLucille(conversation) || !conversation || conversation.groupModels?.length || conversation.mode === 'remote') return null
  const index=conversation.messages.findIndex(message=>message.id===assistantId)
  if(index<0 || conversation.messages[index].role!=='assistant' || conversation.messages[index].hidden)return null
  const user=conversation.messages.slice(0,index).findLast(message=>message.role==='user')
  if(!user || user.hidden || user.notice || (user.displayContent!==undefined && user.displayContent!==user.content) || !user.content.trim() || user.content.length>16000 || new TextEncoder().encode(user.content).length>32000)return null
  return {conversation_id:conversation.id,message_id:user.lucilleInteractionId ?? user.id,scope:conversation.lucille!.scope,project:conversation.lucille!.project,mode,role:'user',user_text:user.content}
}
export interface LucilleSourceProof { interaction: NonNullable<ReturnType<typeof userSourceInteraction>>; job_id: string | null }
/** Called only after the host has durably committed the turn. Assistant quality
 * is not USER authority. An explicit interruption still revokes the source;
 * transport failure and verifier retention preserve the durable original USER.
 * The native proof, not mutable UI project selection, owns its captured scope.
 */
export function durablePreparedInteraction(conversation: Conversation | undefined, assistantId: string, userText: string, mode: LucilleMode, proof: LucilleSourceProof, aborted: boolean) {
  if (aborted || !proof.job_id) return null
  const actual = userSourceInteraction(conversation, assistantId, mode)
  const answer = conversation?.messages.find(message => message.id === assistantId)
  const original = proof.interaction
  if (!actual || answer?.error?.kind === 'interrupted' || actual.user_text.trim().length < 12
    || actual.conversation_id !== original.conversation_id || actual.message_id !== original.message_id
    || actual.user_text !== original.user_text || userText !== original.user_text
    || actual.mode !== original.mode || original.role !== 'user') return null
  return original
}
interface PreparedSource {
  source_authority?:SourceSpeakerSummary | null; proof:LucilleSourceProof; job:{state:string}; current_statements:MemoryFact[]; directive_receipt?:MemoryDirectiveReceipt | null
}
const preparedSources=new Map<string,{source:PreparedSource|null;off:()=>void}>()
export async function prepareLucilleSource(conversationId:string,assistantId:string,mode:LucilleMode,binding:{scope:string;project:string},signal?:AbortSignal):Promise<PreparedSource|null>{
  if(signal?.aborted)throw new DOMException('Turn cancelled','AbortError')
  const {useChatStore}=await import('../stores/chatStore')
  const conversation=useChatStore.getState().conversations.find(c=>c.id===conversationId)
  const input=userSourceInteraction(conversation,assistantId,mode)
  preparedSources.get(assistantId)?.off()
  preparedSources.set(assistantId,{source:null,off:()=>{}})
  if(!input || input.scope!==binding.scope || input.project!==binding.project)return null
  const cancel=()=>{void lucilleManage({operation:'cancel_user_turn',source:{interaction:input,job_id:preparedSources.get(assistantId)?.source?.proof.job_id ?? null}}).catch(()=>{})}
  signal?.addEventListener('abort',cancel,{once:true})
  try{
    const source=await lucilleManage<PreparedSource>({operation:'prepare_user_turn',input})
    if(JSON.stringify(source.proof.interaction)!==JSON.stringify(input)){
      // Object key ordering is not an authority check; compare each bound scalar.
      if(Object.entries(input).some(([k,v])=>source.proof.interaction[k as keyof typeof input]!==v))throw new Error('Invalid original user source')
    }
    preparedSources.set(assistantId,{source,off:()=>signal?.removeEventListener('abort',cancel)})
    if(signal?.aborted){cancel();throw new DOMException('Turn cancelled','AbortError')}
    return source
  }catch(error){cancel();throw error}
}
export function cancelLucilleSource(assistantId:string){
  const pending=preparedSources.get(assistantId);pending?.off();preparedSources.delete(assistantId)
  if(pending?.source)void lucilleManage({operation:'cancel_user_turn',source:pending.source.proof}).catch(()=>{})
}

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
  if (signal.aborted) { cancelLucilleSource(assistantId); return }
  const { useChatStore } = await import('../stores/chatStore')
  const conversation = useChatStore.getState().conversations.find(value => value.id === conversationId)
  const interaction = learningInteraction(conversation, assistantId, userText, mode, signal.aborted)
  const prepared=preparedSources.get(assistantId)
  try{
    if(prepared){
      const source=prepared.source
      if(!source || !durablePreparedInteraction(conversation,assistantId,userText,mode,source.proof,signal.aborted)){cancelLucilleSource(assistantId);return}
      await invoke('lucille_learn',{interaction:source.proof.interaction,source:source.proof})
    }else if(interaction){await invoke('lucille_learn',{interaction})}else return
    const { startLucilleLlm } = await import('./lucille-llm'); startLucilleLlm(); const { startLucilleIndex } = await import('./lucille-index-worker'); startLucilleIndex()
  }finally{prepared?.off();preparedSources.delete(assistantId)}
}
export async function resumeLucilleLearning() {
  const { useChatStore } = await import('../stores/chatStore')
  if (!useChatStore.persist.hasHydrated()) await new Promise<void>(resolve => {
    const off = useChatStore.persist.onFinishHydration(() => { off(); resolve() })
    if (useChatStore.persist.hasHydrated()) { off(); resolve() }
  })
  if (useChatStore.getState().conversations.some(isLucille)) { await invoke('lucille_learning_wake'); const { startLucilleLlm } = await import('./lucille-llm'); startLucilleLlm(); const { startLucilleIndex } = await import('./lucille-index-worker'); startLucilleIndex() }
}
