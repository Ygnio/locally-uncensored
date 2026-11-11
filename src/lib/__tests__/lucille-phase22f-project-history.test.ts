import { expect, it } from 'vitest'
import type { ChatMessage } from '../../api/providers/types'
import { withLucilleContext, type LucilleTurnContext } from '../lucille-context'
import { answerSnapshot, verifyMemoryAnswer, type AnswerSnapshot, type MemoryFact } from '../lucille-fidelity'

const query = 'Compare o banco planejado no meu projeto com o banco que meu irmão usa.'
const plan: MemoryFact = {id:'plan',version:2,entity:'project.jatoba',property:'database',value:'postgresql',modality:'planned',validity:'historical',scope:'qa',project:'jatoba',review:'reviewed',facets:[],evidence:[{role:'current_source',text:'Pretendo usar PostgreSQL no meu projeto.'}]}
const cancelled: MemoryFact = {...plan,id:'cancel',property:'lifecycle',value:'cancelled',modality:'cancelled',validity:'current',evidence:[{role:'current_source',text:'Cancelei o projeto Jatoba.'}]}
const snapshot: AnswerSnapshot = {scope:'qa',project:'jatoba',query,facts:[plan,cancelled],currentStatements:[],relations:[{query:'o banco planejado no meu projeto',actor:'user',state:'retrieved',references:[{id:'plan',version:2}]},{query:'o banco que meu irmão usa',actor:'third_party',state:'unavailable',references:[]}]}

it('treats the exact prior Code output as a current-plan error without attaching the brother’s verb to the own plan', () => {
  const first = 'O banco planejado para o seu projeto Jatoba é o PostgreSQL, enquanto não há registro disponível sobre o banco que seu irmão usa.'
  const second = 'Você planeja usar o PostgreSQL no seu projeto, mas não há registro sobre qual banco de dados o seu irmão usa.'
  for (const answer of [first,second]) {
    const codes = verifyMemoryAnswer(answer,snapshot).violations.map(v=>v.code)
    expect(codes).toContain('terminal_project_plan_as_current')
    expect(codes).not.toContain('plan_or_decision_as_execution')
  }
  const oldProjection={...snapshot,facts:[{...plan,validity:'current'},cancelled]}
  expect(verifyMemoryAnswer(second,oldProjection).violations.map(v=>v.code)).toContain('terminal_project_plan_as_current')
})

it('preserves the earlier own plan and the separate scoped uncertainty about the brother', () => {
  for (const answer of [
    'O banco planejado para o seu projeto Jatoba era o PostgreSQL, enquanto não há registro disponível sobre o banco que seu irmão usa.',
    'Você planejava usar PostgreSQL no seu projeto antes do cancelamento. Não há registro sobre qual banco seu irmão usa.',
  ]) {
    const check = verifyMemoryAnswer(answer,snapshot)
    expect(check.violations,answer).toEqual([])
    expect(check.assertions.some(a=>a.supported && a.references.includes('plan'))).toBe(true)
  }
  const english={...snapshot,query:'Compare the database I had planned for my project with the database my brother uses.'}
  expect(verifyMemoryAnswer('The database you had planned for your project was PostgreSQL, whereas there is no record of which database your brother uses.',english).violations).toEqual([])
  for (const answer of ['Seu projeto usa PostgreSQL.', 'Seu irmão usa PostgreSQL.', 'O banco planejado para o projeto Pitanga era PostgreSQL.']) {
    expect(verifyMemoryAnswer(answer,snapshot).violations.some(v=>v.critical),answer).toBe(true)
  }
  expect(verifyMemoryAnswer('Não há registro disponível sobre o banco do seu irmão.',snapshot).violations.map(v=>v.code)).toContain('known_relation_not_used')
})

it('injects a historical typed plan for the requested relation and never reintroduces it as untyped current text', async () => {
  const conversation = {id:'qa-conversation',lucille:{profile:'lucille' as const,scope:'qa',project:'jatoba',tokenBudget:4096}}
  const context: LucilleTurnContext = {conversation_id:conversation.id,message_id:'answer',scope:'qa',project:'jatoba',mode:'code',token_budget:4096,used_upper_bound:600,retrieval_state:'known',identity:{profile:{name:'QA',description:'',personality:'',instructions:''},version:1,created_at:1,updated_at:1},references:[{id:'plan',version:2}],memories:[{input:{id:'plan',scope:'qa',project:'jatoba',kind:'decision',content:'Raw PostgreSQL text',fact_key:'project.jatoba.database',source:'user',source_id:'planning'},state:'active',version:2,created_at:1,updated_at:1}],facts:[plan]}
  const load = async () => context
  const request = async (text:string) => {
    const original:ChatMessage[] = [{role:'user',content:text}]
    return withLucilleContext(conversation,'answer',text,'code',original,undefined,load)
  }
  const history=await request(query)
  expect(answerSnapshot(history)?.facts).toEqual([plan])
  expect(history.some(m=>m.content.includes('"validity":"historical"'))).toBe(true)
  const current=await request('Qual banco meu projeto usa atualmente?')
  expect(answerSnapshot(current)?.facts).toEqual([])
  expect(current.some(m=>m.content.includes('Raw PostgreSQL text'))).toBe(false)
})
