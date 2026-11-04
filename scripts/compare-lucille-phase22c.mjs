// Known-case development comparison only. No model, tool or canonical writer.
import ts from 'typescript'
import { execFileSync } from 'node:child_process'
import { readFileSync, writeFileSync, existsSync } from 'node:fs'
import { createHash } from 'node:crypto'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
const repo=resolve(dirname(fileURLToPath(import.meta.url)),'..')
const base='97d610e7b508dbf5de9d964be56c41254f739ec7'
const sha=bytes=>createHash('sha256').update(bytes).digest('hex')
const dataModule=source=>'data:text/javascript;base64,'+Buffer.from(source).toString('base64')
const transpile=source=>ts.transpileModule(source,{compilerOptions:{module:ts.ModuleKind.ESNext,target:ts.ScriptTarget.ES2022}}).outputText
async function compile(contents) {
  const authority=transpile(readFileSync(resolve(repo,'src/lib/lucille-authority.ts'),'utf8'))
  const compiled=transpile(contents).replace(/(['"])\.\/lucille-authority\1/g,JSON.stringify(dataModule(authority)))
  return import(dataModule(compiled))
}
const original=execFileSync('git',['show',base+':src/lib/lucille-fidelity.ts'],{cwd:repo,encoding:'utf8'})
const old=await compile(original,resolve(repo,'src/lib'))
const current=await compile(readFileSync(resolve(repo,'src/lib/lucille-fidelity.ts'),'utf8'))
const inputs=readFileSync(resolve(repo,'docs/evidence/phase22c/baseline/baseline-inputs.json'))
const cases=JSON.parse(inputs)
cases.push(JSON.parse(readFileSync(resolve(repo,'docs/evidence/phase22c/baseline/baseline-historical-name.json'),'utf8')).input)
cases[cases.length-1].id='historical-name-alias'
const unsafe=new Set(['decision-execution','relearn-liking','unlinked-empty-folder','ram-performance'])
const rows=cases.map(c=>{
  const before=old.verifyMemoryAnswer(c.answer,c.snapshot)
  const after=current.verifyMemoryAnswer(c.answer,c.snapshot)
  const authority=(after.authority??[]).filter(a=>!['supported','historical','hypothetical','non_memory'].includes(a.verdict))
  return {id:c.id,original_answer:c.answer,expected_raw_intervention:unsafe.has(c.id),legacy:before,authority_only_would_intervene:!!(before.violations.length||authority.length),authority_and_predicate_roles:after}
})
const summary=predicate=>({unsafe_raw_answers_prevented:rows.filter(r=>r.expected_raw_intervention&&predicate(r)).length,
  unsafe_raw_answers_missed:rows.filter(r=>r.expected_raw_intervention&&!predicate(r)).length,
  supported_raw_answers_falsely_blocked:rows.filter(r=>!r.expected_raw_intervention&&predicate(r)).length,
  supported_raw_answers_accepted:rows.filter(r=>!r.expected_raw_intervention&&!predicate(r)).length})
const output=process.argv[2]
if(!output||existsSync(output))throw Error('New explicit output required; never overwrite a comparison')
writeFileSync(output,JSON.stringify({evaluation:'known-case diagnostic ablation, not unseen and not a real-generation availability measurement',base,
  input_sha256:sha(inputs),legacy_source_sha256:sha(original),current_source_sha256:sha(readFileSync(resolve(repo,'src/lib/lucille-fidelity.ts'))),
  variants:{legacy:summary(r=>r.legacy.violations.length>0),authority_only:summary(r=>r.authority_only_would_intervene),authority_and_predicate_roles:summary(r=>r.authority_and_predicate_roles.violations.length>0)},
  localized_repair:'Verbatim independent-sentence projection is separately unit tested. Real regeneration, availability, completeness and token costs require the full model/GUI development run; no simulated completion is counted here.',
  rows,limitations:['Known cases used for development; no new quality gate approval.','These counts distinguish detector misses and false raw blocks, not final answer availability after repair.','Semantic recognition remains finite; typed authority does not prove unrestricted natural-language understanding.']},null,2)+'\n','utf8')
console.log('Known-case comparison recorded:',output)
