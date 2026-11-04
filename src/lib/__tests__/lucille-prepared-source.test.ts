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

it.each(['chat','agent','code'] as const)('publishes a durable prepared USER independently of retained/empty/transport-failed ASSISTANT in %s',async mode=>{
  for (const outcome of ['retained','transport'] as const) {
    const c=conversation('independent-'+mode+'-'+outcome);state.conversations=[c];state.invoke.mockReset()
    state.invoke.mockImplementation(async(name,args)=>name==='lucille_manage' && args.request.operation==='prepare_user_turn' ? {proof:{interaction:args.request.input,job_id:'d'.repeat(64)},job:{state:'queued'},current_statements:[]} : {state:'queued'})
    const abort=new AbortController()
    await prepareLucilleSource(c.id,'answer-'+c.id,mode,c.lucille!,abort.signal)
    c.messages[1].content=''
    c.messages[1].error={kind:'generic',detail:outcome,at:0}
    if(outcome==='retained') c.messages[1].lucilleFidelity=[{status:'blocked',attempts:[],refreshMs:0,verificationMs:0,extraGenerationMs:0,additionalUsage:{}}]
    await queueLucilleLearning(c.id,'answer-'+c.id,c.messages[0].content,mode,abort.signal)
    const writes=state.invoke.mock.calls.filter(([name])=>name==='lucille_learn')
    expect(writes).toHaveLength(1)
    expect(writes[0][1].interaction).toMatchObject({role:'user',user_text:'Prefiro café sem açúcar.',mode})
    expect(writes[0][1].interaction.user_text).not.toContain(outcome)
    expect(writes[0][1]).not.toHaveProperty('assistant')
  }
})
it.each(['chat','agent','code'] as const)('revokes an interrupted prepared source even without an aborted signal in %s',async mode=>{
  const c=conversation('explicit-stop-'+mode);state.conversations=[c];state.invoke.mockReset()
  state.invoke.mockImplementation(async(name,args)=>name==='lucille_manage' && args.request.operation==='prepare_user_turn' ? {proof:{interaction:args.request.input,job_id:'e'.repeat(64)},job:{state:'queued'},current_statements:[]} : {cancelled:true})
  const abort=new AbortController()
  await prepareLucilleSource(c.id,'answer-'+c.id,mode,c.lucille!,abort.signal)
  c.messages[1].error={kind:'interrupted',detail:'',at:0}
  await queueLucilleLearning(c.id,'answer-'+c.id,c.messages[0].content,mode,abort.signal)
  expect(state.invoke.mock.calls.filter(([name])=>name==='lucille_learn')).toHaveLength(0)
  expect(state.invoke.mock.calls.some(([,args])=>args.request?.operation==='cancel_user_turn')).toBe(true)
})
