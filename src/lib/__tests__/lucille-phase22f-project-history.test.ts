import { expect, it } from 'vitest'
import type { ChatMessage } from '../../api/providers/types'
import { validateLucilleCompletion } from '../lucille-answer'
import { withLucilleContext, type LucilleTurnContext } from '../lucille-context'
import { answerSnapshot, registerAnswerSnapshot, requestedHistoricalProjectFact, verifyMemoryAnswer, type AnswerSnapshot, type MemoryFact } from '../lucille-fidelity'

const query = 'Compare o banco planejado no meu projeto com o banco que meu irmão usa.'
const plan: MemoryFact = {id:'plan',version:2,entity:'project.jatoba',property:'database',value:'postgresql',modality:'planned',validity:'historical',scope:'qa',project:'jatoba',review:'reviewed',facets:[],evidence:[{role:'current_source',text:'Pretendo usar PostgreSQL no meu projeto.'}]}
const cancelled: MemoryFact = {...plan,id:'cancel',property:'lifecycle',value:'cancelled',modality:'cancelled',validity:'current',evidence:[{role:'current_source',text:'Cancelei o projeto Jatoba.'}]}
const snapshot: AnswerSnapshot = {scope:'qa',project:'jatoba',query,facts:[plan,cancelled],currentStatements:[],relations:[{query:'o banco planejado no meu projeto',actor:'user',state:'retrieved',references:[{id:'plan',version:2}]},{query:'o banco que meu irmão usa',actor:'third_party',state:'unavailable',references:[]}]}

it('keeps an approved historical choice when the question asks if choosing proves execution', () => {
  const choice: MemoryFact = {
    ...plan, id: 'choice', property: 'language', value: 'python', modality: 'decision',
    evidence: [{ role: 'current_source', text: 'A linguagem que escolhi para o meu projeto é Python.' }],
  }
  const query = 'A escolha de Python prova que eu comecei a implementar o projeto?'
  const current: AnswerSnapshot = { ...snapshot, query, facts: [choice, cancelled], relations: [] }
  expect(requestedHistoricalProjectFact(query, choice)).toBe(true)
  expect(requestedHistoricalProjectFact(query, { ...choice, value: 'rust' })).toBe(false)
  expect(requestedHistoricalProjectFact(query, { ...choice, review: 'unreviewed' })).toBe(false)
  expect(requestedHistoricalProjectFact('O projeto usa Python agora?', choice)).toBe(false)

  const first = 'A escolha de Python prova que você escolheu a linguagem de programação para o projeto. Ela não prova, por si só, que você começou a implementar o projeto.'
  const retry = 'A escolha de Python não prova que você começou a implementar o projeto. De acordo com as memórias, o projeto jatoba está atualmente no estado "cancelled".'
  for (const reply of [first, retry]) {
    const checked = verifyMemoryAnswer(reply, current)
    expect(checked.violations).toEqual([])
    expect(checked.assertions.filter(a => a.property === 'language').map(a => a.references)).toEqual([['choice']])
  }
  for (const altered of [
    'Sim.',
    retry.replace('não prova', 'prova'),
    retry.replace('Python', 'Rust'),
    retry + ' Você começou a implementar o projeto.',
    retry + ' Atualmente você usa Python no projeto.',
    retry + ' Você escolheu Rust para o projeto.',
  ]) expect(verifyMemoryAnswer(altered, current).violations.some(v => v.critical)).toBe(true)
  expect(verifyMemoryAnswer(retry, { ...current, facts: [{ ...choice, scope: 'other' }, cancelled] })
    .violations.map(v => v.code)).toContain('unsupported_typed_value')
})

it('keeps a narrator in the user story distinct from the user preference', () => {
  const query = 'My own favourite beverage is tea; the narrator in my story says his favourite beverage is coffee.'
  const tea: MemoryFact = {
    ...plan, id: 'source-tea', version: 0, entity: 'user', property: 'favorite_beverage', value: 'tea',
    modality: 'preference', validity: 'current_user_statement',
    evidence: [{ role: 'current_user_statement', text: 'My own favourite beverage is tea' }],
  }
  const current: AnswerSnapshot = { ...snapshot, query, facts: [], currentStatements: [tea], relations: [] }
  const first = 'You prefer tea, while the narrator in your story prefers coffee.'
  const retry = 'Your favorite beverage is tea, while the narrator in your story prefers coffee.'
  for (const answer of [first, retry]) {
    const checked = verifyMemoryAnswer(answer, current)
    expect(checked.violations).toEqual([])
    expect(new Set(checked.assertions.map(a => a.value))).toEqual(new Set(['tea']))
  }
  for (const answer of [
    retry + ' Your favorite beverage is coffee.',
    retry.replace('prefers coffee', 'says you prefer coffee'),
    retry.replace('prefers coffee', 'prefers coffee and you prefer coffee'),
    retry.replace('in your story', 'in your studio'),
  ]) expect(verifyMemoryAnswer(answer, current).violations.map(v => v.code)).toContain('unsupported_typed_value')
})

it('checks the entire captured project description, choice, and plan against three distinct scoped facts', () => {
  const description: MemoryFact = {
    ...plan,id:'description',property:'description',modality:'project_description',validity:'current',
    value:'estou comecando um projeto de catalogo de miniaturas',
    evidence:[{role:'current_source',text:'Estou começando um projeto de catálogo de miniaturas'}],
  }
  const choice: MemoryFact = {
    ...plan,id:'choice',property:'language',modality:'decision',validity:'current',value:'python',
    evidence:[{role:'current_source',text:'Escolhi Python como linguagem para o meu projeto'}],
  }
  const database = {...plan,validity:'current'}
  const current: AnswerSnapshot = {
    ...snapshot,query:'Qual é o tema do projeto, a linguagem escolhida e o banco planejado?',
    facts:[description,choice,database],relations:[
      {query:'Qual é o tema do projeto',actor:'user',intent:'any',state:'retrieved',references:[{id:'description',version:2}]},
      {query:'a linguagem escolhida projeto',actor:'user',intent:'choice',state:'retrieved',references:[{id:'choice',version:2}]},
      {query:'o banco planejado? projeto',actor:'user',intent:'planned',state:'retrieved',references:[{id:'plan',version:2}]},
    ],
  }
  const reply = 'O tema do projeto é um catálogo de miniaturas. Você escolheu a linguagem Python e planeja usar o banco de dados PostgreSQL.'
  const checked = verifyMemoryAnswer(reply,current)
  expect(checked.violations).toEqual([])
  expect(checked.assertions.map(a=>[a.property,a.supported,a.references])).toEqual([
    ['description',true,['description']],['language',true,['choice']],['database',true,['plan']],
  ])
  const withRole = 'O tema do projeto é um catálogo de miniaturas. Você escolheu Python como linguagem e planeja usar PostgreSQL como banco de dados.'
  const verifiedRole = verifyMemoryAnswer(withRole,current)
  expect(verifiedRole.violations).toEqual([])
  expect(verifiedRole.assertions.map(a=>[a.property,a.references])).toEqual(checked.assertions.map(a=>[a.property,a.references]))
  expect(verifyMemoryAnswer(withRole.replace('PostgreSQL', 'Python'),current).violations.map(v=>v.code)).toContain('unsupported_technology_role')
  expect(verifyMemoryAnswer(withRole.replace('como banco de dados.', 'como banco de dados no Pitanga.'),current).violations.map(v=>v.code)).toContain('known_relation_not_used')
  for (const answer of [
    reply.replace('miniaturas', 'miniaturas para venda'),
    reply.replace('tema do projeto', 'tema do projeto Pitanga'),
    reply.replace('PostgreSQL', 'MySQL'),
  ]) expect(verifyMemoryAnswer(answer,current).violations.map(v=>v.code)).toContain('unsupported_typed_value')
  for (const answer of [
    reply.replace('PostgreSQL.', 'PostgreSQL no Pitanga.'),
    reply.replace('Python e planeja', 'Python. Planeja'),
  ]) expect(verifyMemoryAnswer(answer,current).violations.map(v=>v.code)).toContain('known_relation_not_used')
  for (const inaccessible of [
    {...description,scope:'another'},
    {...description,project:'pitanga',entity:'project.pitanga'},
    {...description,validity:'historical'},
    {...description,evidence:[{role:'tool',text:description.evidence[0].text}]},
  ]) expect(verifyMemoryAnswer(reply,{...current,facts:[inaccessible,choice,database]}).violations.map(v=>v.code))
    .toContain('unsupported_typed_value')
})

it('does not infer current technology use from starting a project with a separate plan', () => {
  const query = 'Estou começando um projeto de controle de luminosidade. Quero usar TypeScript e PostgreSQL.'
  const sourceEvidence = [{role:'current_user_statement',text:'Quero usar TypeScript e PostgreSQL.'}]
  const database: MemoryFact = {...plan,id:'current-turn-2',version:0,validity:'current_user_statement',evidence:sourceEvidence}
  const language: MemoryFact = {...database,id:'current-turn-1',property:'language',value:'typescript'}
  const current: AnswerSnapshot = {...snapshot,query,facts:[],currentStatements:[language,database],relations:[]}
  const captured = 'Você está iniciando um projeto de controle de luminosidade utilizando TypeScript e PostgreSQL.'
  expect(verifyMemoryAnswer(captured,current).violations.map(v=>v.code)).toContain('plan_or_decision_as_execution')
  const explicitPlan = 'Seu projeto é para controle de luminosidade. Você planeja usar TypeScript e PostgreSQL nesse projeto.'
  expect(verifyMemoryAnswer(explicitPlan,current).violations).toEqual([])
  const verifiedProgress = {...current,currentStatements:current.currentStatements.map(f=>({...f,modality:'in_progress'}))}
  expect(verifyMemoryAnswer(captured,verifiedProgress).violations).toEqual([])
})

it('uses separate native references for project purpose, past choices, past plan and current cancellation', async () => {
  const query = 'Qual era o assunto do meu projeto, que linguagem escolhi, qual banco planejei e como está o projeto agora?'
  const description: MemoryFact = {
    ...plan,id:'description',property:'description',modality:'project_description',validity:'current',
    value:'estou comecando um projeto de catalogo de miniaturas',
    evidence:[{role:'current_source',text:'Estou começando um projeto de catálogo de miniaturas'}],
  }
  const choice: MemoryFact = {
    ...plan,id:'choice',property:'language',modality:'decision',value:'python',
    evidence:[{role:'current_source',text:'A linguagem que escolhi para o meu projeto é Python'}],
  }
  const facts = [description,choice,plan,cancelled]
  const relationQueries = [
    'Qual era o assunto do meu projeto', 'que linguagem escolhi projeto',
    'qual banco planejei projeto', 'como está o projeto agora?',
  ]
  const relations = facts.map((fact,index)=>({
    query:relationQueries[index],actor:'user',intent:['any','choice','planned','any'][index],state:'retrieved',
    references:[{id:fact.id,version:fact.version}],
  }))
  const current: AnswerSnapshot = {...snapshot, query, facts, relations}
  const correct = 'O seu projeto era um catálogo de miniaturas. Você escolheu Python. Você pretendia usar PostgreSQL antes do cancelamento. O projeto foi cancelado.'
  const check = verifyMemoryAnswer(correct,current)
  expect(check.violations).toEqual([])
  expect(check.assertions.map(a=>[a.property,a.references])).toEqual([
    ['description',['description']],['language',['choice']],['database',['plan']],['lifecycle',['cancel']],
  ])
  for (const reply of [
    'O assunto do seu projeto era um catálogo de miniaturas. Você escolheu a linguagem Python, planejou usar o banco de dados PostgreSQL e, atualmente, o projeto está cancelado.',
    'O assunto do seu projeto era um catálogo de miniaturas. Você escolheu a linguagem Python, planejou usar o banco de dados PostgreSQL e o projeto está cancelado agora.',
  ]) {
    const rendered = verifyMemoryAnswer(reply,current)
    expect(rendered.violations).toEqual([])
    expect(rendered.assertions.map(a=>[a.property,a.references])).toEqual([
      ['description',['description']],['language',['choice']],['database',['plan']],['lifecycle',['cancel']],
    ])
  }
  for (const wrong of [
    'O assunto do seu projeto era um catálogo de miniaturas. Atualmente você usa Python. Você planejou usar PostgreSQL. O projeto está cancelado.',
    'O assunto do seu projeto era um catálogo de miniaturas. Você escolheu Python, planeja usar PostgreSQL e, atualmente, o projeto está cancelado.',
  ]) expect(verifyMemoryAnswer(wrong,current).violations.some(v=>v.critical)).toBe(true)
  const priorPlan = 'No seu projeto, você planejou usar o PostgreSQL. Não há registro disponível sobre o banco de dados que seu irmão usa.'
  expect(verifyMemoryAnswer(priorPlan,snapshot).violations).toEqual([])
  expect(verifyMemoryAnswer(priorPlan.replace('planejou','planeja'),snapshot).violations.map(v=>v.code))
    .toContain('terminal_project_plan_as_current')
  const denied = 'O seu projeto era um catálogo de miniaturas. Você não mencionou qual linguagem escolheu nem qual banco planejou, e o projeto está atualmente cancelado.'
  expect(verifyMemoryAnswer(denied,current).violations.map(v=>v.code)).toContain('absence_without_evidence')
  expect(verifyMemoryAnswer(denied,current).violations.map(v=>v.code)).toContain('known_relation_not_used')
  for (const wrong of [correct.replace('pretendia usar','está usando'), correct.replace('escolheu Python','implementa em Python')]) {
    expect(verifyMemoryAnswer(wrong,current).violations.some(v=>v.critical)).toBe(true)
  }
  const conversation = {id:'mixed-history',lucille:{profile:'lucille' as const,scope:'qa',project:'jatoba',tokenBudget:8192}}
  const references = facts.map(({id,version})=>({id,version}))
  const memories: LucilleTurnContext['memories'] = facts.map(f=>({
    input:{id:f.id,scope:f.scope,project:f.project,kind:'project',content:f.evidence[0].text,
      fact_key:f.entity+'.'+f.property,source:'user',source_id:'source-'+f.id},
    state:'active',version:f.version,created_at:1,updated_at:1,
  }))
  const context: LucilleTurnContext = {conversation_id:conversation.id,message_id:'answer',scope:'qa',project:'jatoba',
    mode:'code',token_budget:8192,used_upper_bound:600,retrieval_state:'known',
    identity:{profile:{name:'QA',description:'',personality:'',instructions:''},version:1,created_at:1,updated_at:1},
    references,memories,facts,relations}
  const original: ChatMessage[] = [{role:'user',content:query}]
  const wire = await withLucilleContext(conversation,'answer',query,'code',original,undefined,async()=>context)
  expect(answerSnapshot(wire)?.facts.map(f=>[f.id,f.validity])).toEqual(facts.map(f=>[f.id,f.validity]))
  const swapped = {...context,relations:relations.map((r,index)=>index===2?{...r,references:[{id:plan.id,version:99}]}:r)}
  const stale = await withLucilleContext(conversation,'answer',query,'code',original,undefined,async()=>swapped)
  expect(answerSnapshot(stale)?.facts.map(f=>f.id)).not.toContain('plan')
})

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

it('checks each independently requested technology in the captured Pitanga replies', () => {
  const progress: MemoryFact = { ...plan, id:'progress', project:'pitanga', entity:'project.pitanga', property:'language', value:'typescript', modality:'in_progress', validity:'current' }
  const future: MemoryFact = { ...progress, id:'future', value:'rust', modality:'planned' }
  const current: AnswerSnapshot = {
    ...snapshot, project:'pitanga', query:'Que linguagem estou implementando no Pitanga e qual está planejada para o futuro?',
    facts:[progress,future], relations:[
      {query:'linguagem que estou implementando',actor:'user',state:'retrieved',references:[{id:progress.id,version:progress.version}]},
      {query:'linguagem planejada para o futuro',actor:'user',state:'retrieved',references:[{id:future.id,version:future.version}]},
    ],
  }
  const replies = [
    'Você está implementando o projeto Pitanga em TypeScript. Para o futuro, está planejado usar a linguagem Rust.',
    'Você está implementando o projeto Pitanga em TypeScript. Para o futuro, está planejado utilizar a linguagem Rust.',
    'Você está implementando o projeto Pitanga em TypeScript e planeja usar Rust no futuro.',
  ]
  for (const reply of replies) {
    const checked = verifyMemoryAnswer(reply,current)
    expect(checked.violations).toEqual([])
    expect(checked.assertions.map(a=>a.value)).toEqual(['typescript','rust'])
    expect(verifyMemoryAnswer(reply.replace('Rust','Python'),current).violations.map(v=>v.code)).toContain('unsupported_typed_value')
  }
  expect(verifyMemoryAnswer('Você está implementando o projeto Pitanga em TypeScript. Ana planeja usar Rust.',current).violations.map(v=>v.code)).toContain('known_relation_not_used')
  expect(verifyMemoryAnswer('Você está implementando o projeto Pitanga em TypeScript. Planeja usar Rust no Jatobá.',current).violations.map(v=>v.code)).toContain('known_relation_not_used')
  expect(verifyMemoryAnswer('Você está implementando o projeto Pitanga em TypeScript. Para o futuro, está planejado utilizar a linguagem Rust no Jatobá.',current).violations.map(v=>v.code)).toContain('known_relation_not_used')
})

it('grounds the one retry in the chosen database and keeps the cross-project uncertainty', async () => {
  const chosen: MemoryFact = { ...plan, id:'chosen', project:'pitanga', entity:'project.pitanga', value:'mysql', modality:'decision', validity:'current' }
  const current: AnswerSnapshot = {
    ...snapshot, project:'pitanga', query:'Compare o banco do Pitanga com o banco do projeto Jatobá.', facts:[chosen],
    relations:[
      {query:'o banco do Pitanga',actor:'user',state:'retrieved',references:[{id:chosen.id,version:chosen.version}]},
      {query:'o banco do projeto Jatobá',actor:'user',state:'unavailable',references:[]},
    ],
  }
  const user: ChatMessage = { role:'user', content:current.query }
  registerAnswerSnapshot(user,current)
  const corrected = 'Você escolheu MySQL para o projeto Pitanga. Não há informações disponíveis sobre o banco do projeto Jatobá.'
  const result = await validateLucilleCompletion([user], {
    content:'O projeto Pitanga utiliza o MySQL como banco de dados. Não há informações disponíveis sobre o banco do projeto Jatobá para comparação.',
  }, async messages => {
    const guidance = messages.find(m=>m.role==='system')?.content
    expect(guidance).toContain('"property":"database","value":"mysql","modality":"decision"')
    expect(guidance).toContain('express each other unavailable relation separately')
    return { content:corrected }
  })
  expect(result.report.attempts[0].check.violations.map(v=>v.code)).toContain('plan_or_decision_as_execution')
  expect(result.report.status).toBe('regenerated')
  expect(result.completion.content).toBe(corrected)
})

it('checks the stage asserted in a question’s factual premise independently of execution', () => {
  const progress: MemoryFact = { ...plan, id:'progress', project:'pitanga', entity:'project.pitanga', property:'language', value:'typescript', modality:'in_progress', validity:'current' }
  const current: AnswerSnapshot = { ...snapshot, project:'pitanga', query:'Estou implementando o projeto Pitanga em TypeScript.', facts:[progress], relations:[] }
  const premise = 'Como você está no início ou no meio da implementação,'
  const claim = verifyMemoryAnswer(premise+' você gostaria de discutir algum aspecto específico?',current)
  expect(claim.violations).toEqual([{code:'authority_project_stage',span:premise,critical:false}])
  for (const reply of [
    'Você está implementando o projeto Pitanga em TypeScript. Em que etapa da implementação você está?',
    'Se você está no início da implementação, por onde prefere começar?',
  ]) expect(verifyMemoryAnswer(reply,current).violations).toEqual([])
  expect(verifyMemoryAnswer('O projeto Pitanga está em fase inicial.',current).violations.map(v=>v.code)).toContain('authority_project_stage')
})

it('requires a stated project technology to retain its original modality in an acknowledgement', () => {
  const current: AnswerSnapshot = { ...snapshot, project:'pitanga', query:'Estou implementando meu projeto Pitanga em TypeScript.', facts:[], relations:[], currentStatements:[] }
  const source: MemoryFact = { ...plan, id:'current-turn-0', project:'pitanga', entity:'project.pitanga', property:'language', value:'typescript', modality:'in_progress', validity:'current_user_statement', evidence:[{role:'current_user_statement', text:current.query}] }
  current.currentStatements = [source]
  expect(verifyMemoryAnswer('Que excelente escolha! O TypeScript trará uma estrutura robusta e tipagem clara para o seu projeto Pitanga.',current).violations.map(v=>v.code)).toContain('source_technology_modality_omitted')
  expect(verifyMemoryAnswer('Você está implementando o projeto Pitanga em TypeScript.',current).violations).toEqual([])

  current.query = 'PostgreSQL é o banco que pretendo usar no meu projeto.'
  current.currentStatements = [{...source,property:'database',value:'postgresql',modality:'planned',evidence:[{role:'current_user_statement',text:current.query}]}]
  expect(verifyMemoryAnswer('Excelente escolha, pois o PostgreSQL é conhecido por sua robustez e suporte a diversos tipos de dados. Qual será o foco principal do seu projeto?',current).violations.map(v=>v.code)).toContain('source_technology_modality_omitted')
  expect(verifyMemoryAnswer('Você pretende usar PostgreSQL no projeto.',current).violations).toEqual([])
  const prior = 'Excelente escolha, pois o PostgreSQL é conhecido por sua robustez e suporte a diversos tipos de dados. '
  const pronoun = 'Você planeja utilizá-lo como o banco de dados do seu projeto Pitanga.'
  expect(verifyMemoryAnswer(prior+pronoun,current).violations).toEqual([])
  for (const changed of [
    pronoun.replace('Pitanga', 'Jatobá'),
    pronoun.replace('o banco de dados', 'a linguagem'),
    pronoun.replace('planeja utilizá-lo', 'já usa'),
  ]) expect(verifyMemoryAnswer(prior+changed,current).violations.map(v=>v.code)).toContain('source_technology_modality_omitted')
})

it('verifies progress through a project stage even when the answer does not claim implementation', () => {
  const current: AnswerSnapshot = { ...snapshot, query:'A escolha de Python prova que eu comecei a implementar o projeto?', facts:[], relations:[] }
  expect(verifyMemoryAnswer('Não necessariamente, pois a escolha de uma linguagem pode ser apenas uma decisão de planejamento. No entanto, ela indica que você avançou da concepção para a definição técnica do projeto.',current).violations.map(v=>v.code)).toContain('authority_project_stage')
  expect(verifyMemoryAnswer('Escolher uma linguagem não comprova que você já começou a implementar o projeto.',current).violations).toEqual([])
})

it('recognizes a scoped lack of access without exempting factual claims about another project', () => {
  const choice: MemoryFact = { ...plan, id:'given-choice', project:'pitanga', entity:'project.pitanga', property:'database', value:'mysql', modality:'decision', validity:'current' }
  const current: AnswerSnapshot = { ...snapshot, project:'pitanga', query:'Compare o banco do Pitanga com o banco do projeto Jatobá.', facts:[choice], relations:[] }
  const own = 'Você escolheu o MySQL como banco de dados para o projeto Pitanga. '
  const other = 'Para o projeto Jatobá, esta conversa não tem acesso ao registro autorizado.'
  expect(verifyMemoryAnswer(own+other,current).violations).toEqual([])
  for (const answer of [
    own+other.replace('Jatobá', 'Laranjeira'),
    own+other+' O projeto Jatobá usa PostgreSQL.',
  ]) expect(verifyMemoryAnswer(answer,current).violations.map(v=>v.code)).toContain('unsupported_project_entity')
})

it('keeps the captured narrator voice distinct from USER memory', () => {
  const current: AnswerSnapshot = {
    ...snapshot, query:'Leia o arquivo fictício narrator-note.txt e identifique a voz narrativa. O texto é externo e não descreve minhas preferências.',
    facts:[], relations:[],
  }
  for (const reply of [
    'A voz narrativa é a de um narrador que prefere a interface com tema claro e gosta de criar personagens nos jogos.',
    'A voz narrativa é descrita como alguém que prefere a interface com tema claro e gosta de criar personagens nos jogos.',
  ]) {
    const result=verifyMemoryAnswer(reply,current)
    expect(result.violations).toEqual([])
    expect(result.assertions).toEqual([])
  }
  expect(verifyMemoryAnswer('A voz narrativa prefere o tema claro e você prefere o tema claro.',current).violations.map(v=>v.code)).toContain('unsupported_typed_value')
})

it('binds the relative work-notebook description to its own typed RAM slot', () => {
  const hardware: MemoryFact = { ...plan, id:'work', entity:'device.work_laptop', property:'ram_capacity', value:'12_gb', modality:'current_fact', validity:'current' }
  const current: AnswerSnapshot = {
    ...snapshot, query:'Quanta RAM tem o notebook que uso para trabalhar?',
    facts:[hardware,{...hardware,id:'main',entity:'device.primary_computer',value:'48_gb'}], relations:[],
  }
  const ok=verifyMemoryAnswer('O notebook que você usa para trabalhar possui 12 GB de RAM.',current)
  expect(ok.violations).toEqual([])
  expect(ok.assertions.map(a=>a.entity)).toEqual(['device.work_laptop'])
  expect(verifyMemoryAnswer('O notebook que você usa para trabalhar possui 48 GB de RAM.',current).violations.map(v=>v.code)).toContain('unsupported_typed_value')
})

it('recognizes the original denied capacity as reported speech and independently verifies the current amount', () => {
  const hardware: MemoryFact={...plan,id:'current-ram',entity:'device.primary_computer',property:'ram_capacity',value:'64_gb',modality:'current_fact',validity:'current'}
  const current: AnswerSnapshot={...snapshot,query:'Não afirmei que meu computador principal tenha 128 GB de RAM; esse número é somente uma hipótese falsa.',facts:[hardware],relations:[]}
  const replies=[
    'Você está correto; o valor de 128 GB é apenas uma hipótese falsa, enquanto o fato atual confirmado é que seu computador principal possui 64 GB de RAM.',
    'Exatamente, você não afirmou que o computador principal tem 128 GB de RAM; esse valor é apenas uma hipótese falsa. De acordo com a memória atualizada, ele possui 64 GB de RAM.',
  ]
  for (const reply of replies) {
    const checked=verifyMemoryAnswer(reply,current)
    expect(checked.violations).toEqual([])
    expect(checked.assertions.map(a=>[a.value,a.references])).toEqual([['64_gb',['current-ram']]])
    expect(verifyMemoryAnswer(reply.replace('64 GB','128 GB'),current).violations.map(v=>v.code)).toContain('unsupported_typed_value')
    expect(verifyMemoryAnswer(reply,{...current,query:'Quanto de RAM tem meu computador principal?'}).violations.map(v=>v.code)).toContain('unsupported_typed_value')
  }
})

it('independently validates a current capacity next to a source-matched explicit hypothetical amount', () => {
  const hardware: MemoryFact = {
    ...plan, id: 'current-ram', entity: 'device.primary_computer', property: 'ram_capacity',
    value: '64_gb', modality: 'current_fact', validity: 'current',
  }
  const current: AnswerSnapshot = {
    ...snapshot, query: 'Não afirmei que meu computador principal tenha 128 GB de RAM; esse número é somente uma hipótese falsa.',
    facts: [hardware], relations: [],
  }
  const coordinated = 'Você está correto; o computador principal possui 64 GB de RAM, e os 128 GB são apenas uma hipótese falsa.'
  const detached = 'Exatamente, o valor de 128 GB é apenas uma hipótese e não o dado confirmado. Lembro que seu computador principal agora possui 64 GB de RAM.'

  for (const reply of [coordinated, detached]) {
    const checked = verifyMemoryAnswer(reply, current)
    expect(checked.violations).toEqual([])
    expect(checked.assertions.map(a => [a.value, a.references])).toEqual([['64_gb', ['current-ram']]])

    for (const altered of [
      reply.replace('128 GB', '256 GB'),
      reply.replace('64 GB', '128 GB'),
      reply + ' Hoje, seu computador principal possui 128 GB de RAM.',
    ]) {
      expect(verifyMemoryAnswer(altered, current).violations.map(v => v.code)).toContain('unsupported_typed_value')
    }
    const noDenial = { ...current, query: 'Quanto de RAM tem meu computador principal?' }
    expect(verifyMemoryAnswer(reply, noDenial).violations.map(v => v.code)).toContain('unsupported_typed_value')
  }

  const wrongDevice = { ...current, query: current.query.replace('computador principal', 'notebook de trabalho') }
  expect(verifyMemoryAnswer(coordinated, wrongDevice).violations.map(v => v.code)).toContain('unsupported_typed_value')
  const qualified = coordinated.replace('hipótese falsa.', 'hipótese falsa em outro cenário.')
  expect(verifyMemoryAnswer(qualified, current).violations.map(v => v.code)).toContain('unsupported_typed_value')
})

it('keeps source-matched false hypotheses separate from unconfirmed persistence and other RAM facts', () => {
  const hardware: MemoryFact = {
    ...plan, id: 'current-ram', entity: 'device.primary_computer', property: 'ram_capacity',
    value: '64_gb', modality: 'current_fact', validity: 'current',
  }
  const current: AnswerSnapshot = {
    ...snapshot, query: 'Não afirmei que meu computador principal tenha 128 GB de RAM; esse número é somente uma hipótese falsa.',
    facts: [hardware], relations: [],
  }
  const raw = 'Obrigado pela correção. Registrei que o seu computador principal possui 64 GB de RAM, descartando a hipótese de 128 GB.'
  const retry = 'Exatamente, o valor de 128 GB é apenas uma hipótese falsa. O dado confirmado para o seu computador principal é de 64 GB de RAM.'
  const first = verifyMemoryAnswer(raw, current)
  expect(first.violations.map(v => v.code)).toEqual(['unconfirmed_memory_mutation'])
  expect(first.assertions.map(a => [a.value, a.references])).toEqual([['64_gb', ['current-ram']]])
  const second = verifyMemoryAnswer(retry, current)
  expect(second.violations).toEqual([])
  expect(second.assertions.map(a => [a.value, a.references])).toEqual([['64_gb', ['current-ram']]])

  const noDenial = { ...current, query: 'Quanto de RAM tem meu computador principal?' }
  const wrongDevice = { ...current, query: current.query.replace('computador principal', 'notebook de trabalho') }
  const unsupported = (reply: string, bound = current) =>
    expect(verifyMemoryAnswer(reply, bound).violations.map(v => v.code)).toContain('unsupported_typed_value')
  unsupported(raw, wrongDevice)
  unsupported(raw.replace('128 GB.', '128 GB para outro computador.'))
  unsupported(raw.replace('64 GB', '128 GB'))
  unsupported(retry, noDenial)
  unsupported(retry.replace('128 GB', '256 GB'))
  unsupported(retry.replace('hipótese falsa.', 'hipótese falsa em outro cenário.'))
  unsupported(retry + ' Seu computador principal agora tem 128 GB de RAM.')
})

it('separately verifies historical and new values in a coordinated statement about one device', () => {
  const previous: MemoryFact={...plan,id:'past-ram',entity:'device.primary_computer',property:'ram_capacity',value:'48_gb',modality:'current_fact'}
  const current: AnswerSnapshot={
    ...snapshot,query:'Quanto de RAM tinha antes meu computador principal e quanto ficou depois da correção?',
    facts:[previous,{...previous,id:'current-ram',value:'64_gb',validity:'current'},{...previous,id:'work-ram',entity:'device.work_laptop',value:'12_gb',validity:'current'}],relations:[],
  }
  const reply = 'Antes da correção, seu computador principal tinha 48 GB de RAM e, depois da correção, ele passou a ter 64 GB.'
  const checked=verifyMemoryAnswer(reply,current)
  expect(checked.violations).toEqual([])
  expect(checked.assertions.map(a=>[a.value,a.references])).toEqual([['48_gb',['past-ram']],['64_gb',['current-ram']]])
  expect(verifyMemoryAnswer(reply.replace('ter 64','ter 12'),current).violations.map(v=>v.code)).toContain('unsupported_typed_value')
  const foreign=reply.replace('ele passou a ter 64','o notebook de trabalho passou a ter 12')
  expect(verifyMemoryAnswer(foreign,current).violations.map(v=>v.code)).toContain('unsupported_ram_correction_device')
})
