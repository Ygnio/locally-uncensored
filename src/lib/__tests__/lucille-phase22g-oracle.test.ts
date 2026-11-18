import { readFileSync, writeFileSync } from 'node:fs'
import { it, expect } from 'vitest'
import type { ChatMessage } from '../../api/providers/types'
import { validateLucilleCompletion, validateLucilleStream, MemoryAnswerBlocked } from '../lucille-answer'
import { verifyMemoryAnswer, registerAnswerSnapshot, type MemoryFact } from '../lucille-fidelity'

const dir = 'docs/evidence/phase22g'
const cases = readFileSync(dir+'/benchmark.jsonl','utf8').trim().split('\n').map(line => JSON.parse(line))
const fact = (id:string, entity:string, property:string, value:string, modality:string, validity:string, text:string): MemoryFact => ({
  id, entity, property, value, modality, validity, version:1, scope:'qa', project:'lume', review:'reviewed', facets:[],
  evidence:[{role:'current_source',text}],
})
const facts = [
  fact('old','device.primary_computer','ram_capacity','48_gb','current_fact','historical','O desktop de Lume tinha 48 GB.'),
  fact('now','device.primary_computer','ram_capacity','64_gb','current_fact','current','O desktop de Lume tem 64 GB.'),
  fact('note','device.work_laptop','ram_capacity','12_gb','current_fact','current','O notebook de Lume tem 12 GB.'),
  fact('choice','project.lume','database','sqlite','decision','current','Lume escolheu SQLite.'),
]

it('replays oracle generations through the production completion verifier', async () => {
  const results = []
  for (const row of cases.filter(r => /^(basic|explicit|variant)/.test(r.id))) {
    const query = row.user.split('\n').at(-1)!
    const system: ChatMessage = {role:'system',content:row.system}
    registerAnswerSnapshot(system,{scope:'qa',project:'lume',query,facts,currentStatements:[],hasScopedRecords:true})
    const messages: ChatMessage[] = [system,{role:'user',content:row.user}]
    let retry = false
    try {
      const checked = await validateLucilleCompletion(messages,{content:row.answer},async () => {
        retry = true
        throw new Error('First-pass diagnostic ends at regeneration')
      })
      results.push({id:row.id,delivered:checked.completion.content,repair:checked.report.localizedRepair?.method ?? null})
    } catch (error) {
      if (!(error instanceof MemoryAnswerBlocked) || !retry) throw error
      results.push({id:row.id,delivered:null,retryRequested:true,
        codes:error.report.attempts[0].check.violations.map(v=>v.code)})
    }
  }
  if (process.env.LUCILLE_REPLAY_OUTPUT) {
    writeFileSync(process.env.LUCILLE_REPLAY_OUTPUT,JSON.stringify(results,null,2)+'\n',{flag:'wx'})
  } else {
    expect(results).toEqual(JSON.parse(readFileSync(dir+'/consolidation/verifier.json','utf8')))
  }
})


it('uses the property and temporal track of independently reviewed evidence', () => {
  const snapshot = { scope:'qa',project:'lume',query:'O banco escolhido já está em uso?',facts,currentStatements:[] }
  const codes=(answer:string, data=facts)=>verifyMemoryAnswer(answer,{...snapshot,facts:data}).violations.map(v=>v.code)
  expect(codes('A decisão sobre o banco implica que Lume já o utiliza.')).toContain('affirmed_unproven_project_execution')
  const initial = JSON.parse(readFileSync(dir+'/consolidation/c2-native.json','utf8')).queries.find((q:{id:string})=>q.id==='f2q-10')
  const generated = JSON.parse(readFileSync(dir+'/consolidation/c2-answers.json','utf8')).find((q:{id:string})=>q.id===initial.id)
  const raw = generated.generations[0].response.choices[0].message.content
  const observed = {scope:initial.context.facts[0].scope, project:initial.context.facts[0].project,
    facts:initial.context.facts, query:initial.query, currentStatements:[]}
  expect(verifyMemoryAnswer(raw,observed).violations.map(v=>v.code)).toContain('unsupported_typed_value')
  const purpose = {...snapshot, query:'Que finalidade devo descrever para o projeto Lume?'}
  expect(verifyMemoryAnswer('Você deve descrever a finalidade de organizar um coral.',purpose).violations.map(v=>v.code)).toContain('unsupported_typed_value')
  const description = fact('purpose','project.lume','description','organizar um coral','project_description','current','Iniciei um projeto de organizar um coral.')
  expect(verifyMemoryAnswer('Você deve descrever a finalidade de organizar um coral.',{...purpose,facts:[description]}).violations).toEqual([])
  const usage = fact('used','project.lume','database','sqlite','current_fact','current','Lume utiliza SQLite em produção.')
  expect(codes('Lume utiliza SQLite.',[...facts,usage])).not.toContain('affirmed_unproven_project_execution')
  expect(codes('O plano não prova que Lume já utiliza o banco.')).not.toContain('affirmed_unproven_project_execution')
  expect(codes('Não temos registros sobre a RAM anterior do computador principal.')).toContain('absence_without_evidence')
  expect(codes('Não temos registros sobre a RAM anterior do notebook de trabalho.')).not.toContain('absence_without_evidence')
})


it('requires a scoped explanation for the inaccessible project in a comparison', () => {
  const snapshot = {scope:'qa',project:'lume',query:'Compare o banco de Lume com o de Cedro.',
    currentStatements:[],facts,relations:[{actor:'user',query:'o banco escolhido em Cedro',
      intent:'choice',outside_project:'Cedro',state:'unavailable',references:[]}]}
  const codes = (answer:string)=>verifyMemoryAnswer(answer,snapshot).violations.map(v=>v.code)
  expect(codes('O registro de Cedro não está disponível.')).toContain('missing_project_access_boundary')
  expect(codes('Tenho acesso apenas ao histórico de Lume. Não há registro em Cedro.')).toContain('missing_project_access_boundary')
  expect(codes('Cedro está fora do escopo desta conversa.')).not.toContain('missing_project_access_boundary')
  for (const name of ['regeneration','corrected-first-pass']) {
    const row = JSON.parse(readFileSync(dir+'/fidelity/'+name+'.json','utf8'))
    const context = row.contexts[0]
    expect(verifyMemoryAnswer(row.delivered,{...snapshot,scope:context.facts[0].scope,
      project:context.facts[0].project,query:row.generations[0].request.messages.findLast((m:{role:string})=>m.role==='user')?.content,
      facts:context.facts,relations:context.relations}).violations.map(v=>v.code)).not.toContain('missing_project_access_boundary')
  }
})

it('publishes only the verified replacement through the streaming response path', async () => {
  const fixed = JSON.parse(readFileSync(dir+'/fidelity/corrected-first-pass.json','utf8'))
  const retry = JSON.parse(readFileSync(dir+'/fidelity/regeneration.json','utf8'))
  const first = JSON.parse(readFileSync(dir+'/fidelity/first-pass.json','utf8')).find((r:{id:string})=>r.id==='f2q-17')
  const messages:ChatMessage[] = fixed.generations[0].request.messages
  const context = fixed.contexts[0]
  registerAnswerSnapshot(messages[0],{scope:context.facts[0].scope,project:context.facts[0].project,
    query:messages[messages.length-1].content as string,facts:context.facts,relations:context.relations,currentStatements:[]})
  async function* incoming() { yield {content:first.delivered,done:true} }
  const chunks = []
  let retries = 0
  for await (const chunk of validateLucilleStream(incoming(),()=>messages,async () => {
    retries++
    return {content:retry.delivered,finishReason:'stop'}
  }, report => expect(report.status).toBe('regenerated'))) chunks.push(chunk.content)
  expect(retries).toBe(1)
  expect(chunks).toEqual([retry.delivered])
})


it('checks a named project purpose separately from its choice and an introductory aside', () => {
  const purpose=fact('purpose','project.lume','description','catalogar cadernos','project_description','current','Criei o projeto Lume para catalogar cadernos.')
  const snap={scope:'qa',project:'lume',query:'Antes de pintar a moldura, me lembra para que serve o projeto e qual banco eu escolhi?',facts:[...facts,purpose],currentStatements:[]}
  const correct='O projeto Lume serve para catalogar cadernos. Você escolheu SQLite como banco de dados.'
  expect(verifyMemoryAnswer(correct,snap).violations).toEqual([])
  expect(verifyMemoryAnswer('A finalidade do Lume é guardar recibos.',snap).violations.map(v=>v.code)).toContain('unsupported_typed_value')
  expect(verifyMemoryAnswer(correct,{...snap,facts}).violations.map(v=>v.code)).toContain('unsupported_typed_value')
  expect(verifyMemoryAnswer('Você escolheu SQLite como banco de dados.',
    {...snap,query:'Antes de trocar o banco, me lembra qual foi a minha escolha?'}).violations.map(v=>v.code)).toContain('unsupported_historical_state')
})


it('keeps a reviewed fictional theme bound to the named character in a comparison', () => {
  const own = fact('own', 'user', 'interface_theme', 'light', 'preference', 'current', 'Prefiro o tema claro.')
  const story = fact('story', 'character.iris', 'fictional_theme', 'sepia', 'fictional_preference', 'current', 'No meu conto, defini que a personagem Íris prefere o tema sépia.')
  const snapshot = {
    scope: 'qa', project: 'lume', query: 'Qual tema eu prefiro e qual é o da Íris no conto?',
    facts: [own, story], currentStatements: [],
    relations: [own, story].map(f => ({query: 'tema', actor: f.entity, state: 'retrieved', references: [{id:f.id, version:1}]})),
  }
  expect(verifyMemoryAnswer('Você prefere o tema claro, enquanto Íris prefere o tema sépia.', snapshot).violations).toEqual([])
  for (const answer of ['Você prefere o tema sépia. Íris prefere o tema claro.', 'Você prefere o tema claro.', 'Você prefere o tema claro. Íris prefere o tema escuro.']) {
    expect(verifyMemoryAnswer(answer, snapshot).violations.length).toBeGreaterThan(0)
  }
})

it('binds the origin of a revised decision to the same first-person correction', () => {
  const choice = fact('current-turn-0','project.lume','database','postgresql','decision','current_user_statement','agora escolhi PostgreSQL')
  choice.version = 0
  choice.evidence[0].role = 'current_user_statement'
  choice.evidence.push({role:'adjacent_personal_clause', text:'No Lume, antes eu tinha escolhido SQLite'})
  const speaker = {message_author:'user' as const,personal:2,quoted:0,external:0,uncertain:0}
  const snapshot = {scope:'qa',project:'lume',query:'No Lume, antes eu tinha escolhido SQLite, mas agora escolhi PostgreSQL.',facts:[],currentStatements:[choice],sourceAuthority:speaker,sourceIdentity:{conversation_id:'c',source_message_id:'s'}}
  const answer = 'Você decidiu substituir o SQLite pelo PostgreSQL no projeto Lume.'
  expect(verifyMemoryAnswer(answer,snapshot).violations).toEqual([])
  expect(verifyMemoryAnswer(answer.replace('SQLite','MySQL'),snapshot).violations.length).toBeGreaterThan(0)
  expect(verifyMemoryAnswer(answer,{...snapshot,currentStatements:[{...choice,evidence:[choice.evidence[0]]}]}).violations.length).toBeGreaterThan(0)
  expect(verifyMemoryAnswer('Você trocou o SQLite pelo PostgreSQL no projeto Lume.',snapshot).violations.length).toBeGreaterThan(0)
})

it('rechecks both observed decision-update generations with the native source projection', () => {
  const replay = JSON.parse(readFileSync(dir+'/persistence-retest/decision-replay.json', 'utf8'))
  for (const attempt of replay.attempts) {
    const check = verifyMemoryAnswer(attempt.raw, replay.snapshot)
    expect(check.violations.length === 0 ? 'accepted' : 'blocked').toBe(attempt.expected_after)
  }
})
