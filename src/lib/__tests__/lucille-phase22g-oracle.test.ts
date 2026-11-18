import { readFileSync, writeFileSync } from 'node:fs'
import { it, expect } from 'vitest'
import type { ChatMessage } from '../../api/providers/types'
import { validateLucilleCompletion, MemoryAnswerBlocked } from '../lucille-answer'
import { registerAnswerSnapshot, type MemoryFact } from '../lucille-fidelity'

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
    expect(results).toEqual(JSON.parse(readFileSync(dir+'/verifier.json','utf8')))
  }
})

