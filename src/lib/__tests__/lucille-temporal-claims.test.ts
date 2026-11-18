import { readFileSync, writeFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { verifyMemoryAnswer, type AnswerSnapshot, type MemoryFact } from '../lucille-fidelity'

const folder = 'docs/evidence/phase22g'
const read = (file: string) => JSON.parse(readFileSync(folder + '/' + file, 'utf8'))
const query = 'Qual era minha escolha anterior de linguagem e qual vale agora para o projeto Urze?'
const decision = (id: string, value: string, validity: string): MemoryFact => ({
  id, value, validity, entity:'project.urze', property:'language', modality:'decision',
  scope:'qa', project:'urze', version:validity === 'historical' ? 3 : 2, review:'reviewed', facets:[],
  evidence:[{role:'current_source',text:'Escolhi '+value+' como linguagem.'}],
})
const snapshot: AnswerSnapshot = {scope:'qa',project:'urze',query,
  facts:[decision('old','python','historical'),decision('active','rust','current')],currentStatements:[]}

it('binds the time and modality of each project choice before matching its value', () => {
  const rows: {answer:string;correct:boolean;accepted:boolean;codes:string[];label:string;context:AnswerSnapshot}[] = []
  function check(label:string, answer:string, correct:boolean, context=snapshot) {
    const result = verifyMemoryAnswer(answer,context)
    rows.push({label,answer,correct,context,accepted:result.violations.length===0,codes:result.violations.map(v=>v.code)})
    return result
  }
  for (const answer of [
    'Sua escolha anterior era Python, mas você escolheu Rust para o projeto Urze.',
    'Você escolheu Rust. Sua escolha anterior era Python.',
    'Antes escolheu Python e escolheu Rust para o projeto Urze.',
    'A anterior era Python e a atual é Rust.',
    'A escolha era Python, mas passou a ser Rust.',
    'Antes, Python. Hoje, Rust.',
    'Python era a escolha anterior e Rust é a atual.',
  ]) {
    const result=check('contraste',answer,true)
    expect(result.assertions.filter(a=>a.property==='language').map(a=>[a.value,a.temporality,a.references]).sort(),answer)
      .toEqual([['python','historical',['old']],['rust','current',['active']]])
  }
  check('pergunta somente histórica','Minha escolha anterior era Python.',true,{...snapshot,query:'Qual era minha escolha anterior de linguagem?'})
  for(const answer of [
    'Sua escolha anterior era Rust, mas você escolheu Python para o projeto Urze.',
    'A escolha atual é Python.',
    'Você escolheu Rust.',
    'Sua escolha anterior era Python, mas o projeto Urze já está implementado em Rust.',
    'O projeto Urze migrou de Python para Rust.',
    'Sua escolha anterior era Python e a atual é Rust. A migração foi executada.',
    'Sua escolha anterior era JavaScript, mas você escolheu Rust.',
    'Sua escolha anterior era Python, mas você escolheu Rust para o projeto Amora.',
  ]) check('contraprova',answer,false)
  check('outro escopo','Sua escolha anterior era Python, mas você escolheu Rust.',false,
    {...snapshot,facts:snapshot.facts.map(f=>({...f,scope:'outro'}))})
  check('outra entidade','Sua escolha anterior era Python, mas você escolheu Rust.',false,
    {...snapshot,facts:snapshot.facts.map(f=>({...f,entity:'project.amora'}))})
  check('modalidade de plano','Sua escolha anterior era Python, mas você escolheu Rust.',false,
    {...snapshot,facts:[snapshot.facts[0],{...snapshot.facts[1],modality:'planned'}]})
  const completed = {...snapshot.facts[1], id:'migration',property:'project_status',modality:'current_fact',value:'migration_completed',evidence:[{role:'current_source',text:'A migração foi executada.'}]}
  check('execução com prova','Sua escolha anterior era Python e a atual é Rust. A migração foi executada.',true,{...snapshot,facts:[...snapshot.facts,completed]})
  const replay=read('persistence-retest/decision-replay.json')
  for(const a of replay.attempts) check('atualização arquivada',a.raw,a.expected_after==='accepted',replay.snapshot)
  const perf=read('gui-trace.json')['fv1-main-t03']
  for(const a of perf.attempts) check('explicação geral arquivada',a.text,true,{...snapshot,query:'Dá para garantir que o meu computador principal ficará livre de travamentos só pela quantidade de RAM?',facts:[]})
  if(process.env.LUCILLE_TEMPORAL_MATRIX==='1') writeFileSync(folder+'/temporal-claims/unit-matrix.json',JSON.stringify(rows,null,2)+'\n')
  expect(rows.filter(r=>r.correct!==r.accepted)).toEqual([])
})
