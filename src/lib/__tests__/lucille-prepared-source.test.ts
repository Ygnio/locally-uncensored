import { describe, expect, it, vi } from 'vitest'
import type { Conversation } from '../../types/chat'
const state=vi.hoisted(()=>({conversations:[] as Conversation[],invoke:vi.fn()}))
vi.mock('@tauri-apps/api/core',()=>({invoke:state.invoke}))
vi.mock('../../stores/chatStore',()=>({useChatStore:{getState:()=>state}}))
vi.mock('../lucille-llm',()=>({startLucilleLlm:vi.fn()}))
vi.mock('../lucille-index-worker',()=>({startLucilleIndex:vi.fn()}))
import { prepareLucilleSource, queueLucilleLearning, cancelLucilleSource } from '../lucille-learning'

function conversation(id:string):Conversation {
  return {id,mode:'lu',lucille:{profile:'lucille',scope:'synthetic',project:'demo',tokenBudget:8192},messages:[
    {id:'user-'+id,role:'user',content:'Prefiro café sem açúcar.'},
    {id:'answer-'+id,role:'assistant',content:'Entendido.'},
  ]} as Conversation
}
describe('original user source reservation',()=>{
  it('publishes only after a durable answer and preserves the scope captured before generation',async()=>{
    const c=conversation('publish');state.conversations=[c];state.invoke.mockReset()
    state.invoke.mockImplementation(async(name,args)=>name==='lucille_manage' && args.request.operation==='prepare_user_turn' ? {proof:{interaction:args.request.input,job_id:'a'.repeat(64)},job:{state:'queued'},current_statements:[]} : {state:'queued'})
    const abort=new AbortController()
    const prepared=await prepareLucilleSource(c.id,'answer-'+c.id,'chat',c.lucille!,abort.signal)
    expect(state.invoke.mock.calls.some(([name])=>name==='lucille_learn')).toBe(false)
    c.lucille!.project='other-project'
    await queueLucilleLearning(c.id,'answer-'+c.id,c.messages[0].content,'chat',abort.signal)
    const write=state.invoke.mock.calls.find(([name])=>name==='lucille_learn')
    expect(write?.[1].interaction.project).toBe('demo')
    expect(write?.[1].source).toEqual(prepared?.proof)
  })
  it('cancels an interrupted prepared source instead of publishing it',async()=>{
    const c=conversation('abort');state.conversations=[c];state.invoke.mockReset()
    state.invoke.mockImplementation(async(name,args)=>name==='lucille_manage' && args.request.operation==='prepare_user_turn' ? {proof:{interaction:args.request.input,job_id:'b'.repeat(64)},job:{state:'queued'},current_statements:[]} : {cancelled:true})
    const abort=new AbortController()
    await prepareLucilleSource(c.id,'answer-'+c.id,'code',c.lucille!,abort.signal)
    abort.abort()
    await queueLucilleLearning(c.id,'answer-'+c.id,c.messages[0].content,'code',abort.signal)
    expect(state.invoke.mock.calls.some(([name])=>name==='lucille_learn')).toBe(false)
    expect(state.invoke.mock.calls.some(([,args])=>args.request?.operation==='cancel_user_turn' && args.request.source.interaction.message_id==='user-abort')).toBe(true)
    cancelLucilleSource('answer-'+c.id)
  })
})
