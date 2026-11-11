import { expect, it, vi } from 'vitest'
import type { ChatMessage, ChatStreamChunk } from '../../api/providers/types'
import { validateLucilleCompletion, validateLucilleStream } from '../lucille-answer'
import { registerAnswerSnapshot, verifyMemoryAnswer, type AnswerSnapshot, type MemoryDirectiveReceipt, type MemoryLearningReceipt } from '../lucille-fidelity'
import { renderNativeConfirmation, type MemoryMode } from '../lucille-operations'

const source = { conversation_id: 'pitanga-session', source_message_id: 'source-f' }
const sourceId = '1'.repeat(64)
const statement = 'No meu projeto Pitanga, o banco que escolhi é MySQL.'
const raw = 'Entendido. Registrei que o banco de dados escolhido para o projeto Pitanga é o MySQL.'
const repaired = 'Entendido. O banco de dados escolhido para o projeto Pitanga é o MySQL.'

function snapshot(mode: MemoryMode): AnswerSnapshot {
  return { scope: 'qa', project: 'pitanga', query: statement, facts: [], sourceIdentity: source,
    operationBinding: {profile:'lucille',identityVersion:3,mode,sourceId}, currentStatements: [{id:'current-turn-0',version:0,entity:'project.pitanga',property:'database',value:'mysql',modality:'decision',validity:'current_user_statement',scope:'qa',project:'pitanga',review:'original',facets:[],evidence:[{role:'current_user_statement',text:statement}]}] }
}
function messages(evidence: AnswerSnapshot): ChatMessage[] {
  const identity: ChatMessage = {role:'system',content:'native context'}
  registerAnswerSnapshot(identity, evidence)
  return [identity, {role:'user',content:evidence.query}]
}
function learning(mode: MemoryMode): MemoryLearningReceipt {
  return {...source,id:sourceId,profile:'lucille',identity_version:3,mode,action:'learn',state:'done',scope:'qa',project:'pitanga',records:[{id:'record',version:1,state:'active',entity:'project.pitanga',property:'database',value:'mysql',effect:'create'}]}
}
function forgetting(mode: MemoryMode): MemoryDirectiveReceipt {
  return {...source,id:sourceId,profile:'lucille',identity_version:3,mode,action:'forget',state:'done',scope:'qa',project:'pitanga',property:'coffee_sugar',had_records:true,records:[{id:'removed',version:2,state:'deleted',effect:'delete'}]}
}

it.each(['chat','agent','code'] as const)('repairs the actual Pitanga disclosure without losing the acknowledged decision in %s', async mode => {
  const evidence = snapshot(mode)
  const regenerate = vi.fn()
  const output = await validateLucilleCompletion(messages(evidence), {content:raw,thinking:'Registrei o banco na memória.'}, regenerate)
  expect(output.completion.content).toBe(repaired)
  expect(output.completion.thinking).toBe('')
  expect(output.report.verifiedText).toBe(repaired)
  expect(output.report.attempts[0].rawAnswer).toBe(raw)
  expect(output.report.attempts[0].nativeOperation?.learning).toBeNull()
  expect(output.report.attempts[0].check.violations.map(v=>v.code)).toContain('unconfirmed_memory_mutation')
  expect(output.report.localizedRepair?.method).toBe('operation_prefix_removal')
  expect(regenerate).not.toHaveBeenCalled()
})

it('holds streaming fragments and publishes the same verified completion to Chat', async () => {
  const evidence = snapshot('chat')
  async function* fragments(): AsyncGenerator<ChatStreamChunk> {
    yield {content:'Entendido. Registrei ',done:false}
    yield {content:raw.slice('Entendido. Registrei '.length),done:true}
  }
  const regenerate=vi.fn(), observed: string[] = []
  for await (const chunk of validateLucilleStream(fragments(),()=>messages(evidence),regenerate, report=>expect(report.verifiedText).toBe(repaired))) observed.push(chunk.content)
  expect(observed).toEqual([repaired])
  expect(regenerate).not.toHaveBeenCalled()
})

it('requires exact native identity, mode, source and operation kind before rendering a save, update, or review', () => {
  const evidence = snapshot('agent')
  evidence.query = 'Registre que o banco escolhido foi MySQL.'
  const receipt = learning('agent')
  evidence.learningReceipt = receipt
  expect(renderNativeConfirmation(evidence,'pt')).toBe('Salvei na memória a informação solicitada.')
  expect(verifyMemoryAnswer('Atualizei o banco na memória com MySQL.',evidence).violations.map(v=>v.code)).toContain('unconfirmed_memory_mutation')
  const record = receipt.records[0]
  record.effect = 'update'
  expect(renderNativeConfirmation(evidence,'pt')).toBe('Atualizei na memória a informação solicitada.')
  record.effect = 'create'
  record.state = 'candidate'
  expect(renderNativeConfirmation(evidence,'pt')).toBe('A informação ficou pendente de revisão na memória.')
  for (const wrong of [{id:'2'.repeat(64)}, {identity_version:2}, {mode:'code' as const}, {scope:'other'}, {project:'jatoba'}, {conversation_id:'previous'}, {source_message_id:'earlier'}]) {
    evidence.learningReceipt = {...receipt,...wrong}
    expect(renderNativeConfirmation(evidence,'pt'), JSON.stringify(wrong)).toBeUndefined()
  }
})

it('never treats a queue state, a future promise, or an operation-shaped fact as a completed write', () => {
  const evidence = snapshot('code')
  evidence.sourceOperation={...source,id:sourceId,profile:'lucille',identity_version:3,mode:'code',scope:'qa',project:'pitanga',action:'learn',state:'source_persisted'}
  for (const answer of ['Registrei que o banco escolhido foi MySQL.', 'O banco escolhido ficou gravado.', 'Guardarei a informação do banco escolhido.', 'Criei uma candidata para a informação, aguardando revisão.']) {
    expect(verifyMemoryAnswer(answer,evidence).violations.some(v=>v.code.startsWith('unconfirmed_memory_')),answer).toBe(true)
  }
  expect(verifyMemoryAnswer('A informação está sendo processada para memória.',evidence).violations.map(v=>v.code)).toContain('unconfirmed_memory_mutation')
  evidence.sourceOperation.state='processing'
  expect(verifyMemoryAnswer('A informação está sendo processada para memória.',evidence).violations).toEqual([])
  expect(renderNativeConfirmation(evidence,'pt')).toBeUndefined()
  expect(verifyMemoryAnswer('Entendi sua escolha pelo MySQL.',evidence).violations).toEqual([])
})

it('generates a forget confirmation from the app receipt, including a truthful empty deletion', async () => {
  const evidence = snapshot('agent')
  evidence.query='Esqueça de verdade minha preferência por café.'
  evidence.currentStatements=[]
  const receipt = forgetting('agent')
  evidence.directiveReceipt=receipt
  const regen=vi.fn()
  const result=await validateLucilleCompletion(messages(evidence),{content:'Entendi seu pedido.'},regen)
  expect(result.report.applicationConfirmation).toBe('Removi da memória a informação solicitada.')
  expect(result.report.verifiedText).toBe(result.completion.content)
  expect(result.completion.content).toBe('Entendi seu pedido.\n\nRemovi da memória a informação solicitada.')
  const generated=await validateLucilleCompletion(messages(evidence),{content:'A preferência por café foi esquecida.'},regen)
  expect(generated.completion.content).toBe('Removi da memória a informação solicitada.')
  expect(generated.report.attempts[0].check.violations.map(v=>v.code)).toContain('application_owned_memory_confirmation')
  expect(generated.report.localizedRepair?.delivered).toBe(generated.completion.content)
  expect(renderNativeConfirmation({...evidence,directiveReceipt:{...receipt,source_message_id:'other'}},'pt')).toBeUndefined()
  expect(renderNativeConfirmation({...evidence,directiveReceipt:{...receipt,records:[]}},'pt')).toBeUndefined()
  expect(renderNativeConfirmation({...evidence,directiveReceipt:{...receipt,had_records:false,records:[]}},'en')).toBe('There was no matching memory to remove.')
  expect(regen).not.toHaveBeenCalled()
})

it('does not convert a false complement into an acknowledged fact', async () => {
  const evidence = snapshot('agent')
  const regenerate = vi.fn(async () => ({content:repaired}))
  const output=await validateLucilleCompletion(messages(evidence),{content:raw.replace('MySQL','SQLite')},regenerate)
  expect(output.completion.content).toBe(repaired)
  expect(output.report.localizedRepair).toBeUndefined()
  expect(regenerate).toHaveBeenCalledTimes(1)
})
