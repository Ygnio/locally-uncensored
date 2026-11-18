import { readFileSync, writeFileSync } from 'node:fs'
import { it, expect } from 'vitest'
import type { ChatMessage } from '../../api/providers/types'
import { validateLucilleCompletion, MemoryAnswerBlocked } from '../lucille-answer'
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

