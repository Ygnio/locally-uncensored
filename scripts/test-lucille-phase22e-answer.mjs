import {selectKnownRecords,verifyKnownProjection} from '../qa/phase22e-evaluator-contract.mjs'
// Independent D stage: real native context -> shared adapter -> real Gemma ->
// shared verifier. No executor or learning source is simulated by this reader.
import { readFileSync, writeFileSync, existsSync, appendFileSync } from 'node:fs'
import { resolve, relative, sep } from 'node:path'
import { pathToFileURL } from 'node:url'
import { createHash, randomUUID } from 'node:crypto'
import { execFileSync } from 'node:child_process'
const args = Object.fromEntries(process.argv.slice(2).reduce((pairs, value, i, all) => i % 2 ? pairs : [...pairs, [value.replace(/^--/, ''), all[i+1]]], []))
const preflight=args.preflight==='true'
for (const key of ['root','bridge','freeze','dataset','store','label']) if (!args[key]) throw Error('Missing '+key)
const repo=process.cwd(), root=resolve(args.root), store=resolve(args.store), exe=resolve(args.bridge)
if(root!==resolve('.lucille-qa-runtime/lucille-synthetic-real22e') || !store.startsWith(root+sep) || !/^lucille-synthetic-/.test(store.split(sep).at(-1)) || !/^[a-z0-9-]{1,60}$/.test(args.label)) throw Error('Owned QA paths required')
for(const directory of [root,store]) if(readFileSync(resolve(directory,'.lucille-isolated'),'utf8')!=='synthetic-data-only\n')throw Error('Synthetic marker required')
const sha=data=>createHash('sha256').update(data).digest('hex')
const freeze=JSON.parse(readFileSync(args.freeze,'utf8')), datasetPath=relative(repo,resolve(args.dataset)).split(sep).join('/')
if(!freeze.corpus_sha256[datasetPath])throw Error('Dataset must be frozen')
// Only this frozen development clone may retain known D scopes. The same
// authorization applies to preflight and actual execution; final stays E-only.
const knownLegacyDevelopment=freeze.kind==='development'
 && datasetPath==='crates/lucille-memory/tests/fixtures/phase22e/development/known-d-selectors.json'
 && store===resolve(root,'development-selector-contract/lucille-synthetic-known-native')
 && args.label.startsWith('development-')
function verify(){
  for(const [path,hash] of Object.entries(freeze.source_sha256_lf))if(sha(readFileSync(resolve(repo,path),'utf8').replaceAll('\r\n','\n'))!==hash)throw Error('Source changed: '+path)
  for(const [path,hash] of Object.entries(freeze.corpus_sha256))if(sha(readFileSync(resolve(repo,path)))!==hash)throw Error('Corpus changed: '+path)
  if(sha(readFileSync(exe))!==freeze.bridge_sha256)throw Error('Native bridge changed')
  if(!preflight && JSON.stringify(JSON.parse(readFileSync(resolve(root,'reports/real-public-pin.json'),'utf8')))!==JSON.stringify(freeze.model_pin))throw Error('Pinned real model changed')
}
verify()
const output=resolve(root,'reports',args.label+'-answers.json'), progress=resolve(root,'reports',args.label+'-answers-progress.ndjson')
if(existsSync(output)||existsSync(progress))throw Error('Never overwrite or silently replay completed generations')
const native=data=>JSON.parse(execFileSync(exe,[],{input:JSON.stringify(data),encoding:'utf8',windowsHide:true,env:{...process.env,LUCILLE_QA_ROOT:store}}))
const ts=(await import(pathToFileURL(resolve('node_modules/typescript/lib/typescript.js')))).default
const modules=new Map()
for(const name of ['tool-result-failure','verified-environment','thinking-stripper','lucille-authority','lucille-fidelity','lucille-answer','lucille-context']){
  let code=ts.transpileModule(readFileSync(resolve('src/lib',name+'.ts'),'utf8'),{compilerOptions:{module:ts.ModuleKind.ESNext,target:ts.ScriptTarget.ES2022}}).outputText
  code=code.replace(/from (['"])([^'"]+)\1/g,(whole,_quote,specifier)=>{
    const url=specifier==='@tauri-apps/api/core'?pathToFileURL(resolve('node_modules/@tauri-apps/api/core.js')).href:modules.get(specifier.replace('./',''))
    if(!url)throw Error('Unresolved actual adapter import: '+specifier)
    return 'from '+JSON.stringify(url)
  })
  modules.set(name,'data:text/javascript;base64,'+Buffer.from(code).toString('base64'))
}
const {withLucilleContext}=await import(modules.get('lucille-context'))
const {answerSnapshot}=await import(modules.get('lucille-fidelity'))
const {validateLucilleCompletion,MemoryAnswerBlocked}=await import(modules.get('lucille-answer'))
async function api(path,data){
  const response=await fetch('http://127.0.0.1:15284'+path,data?{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(data),signal:AbortSignal.timeout(120000)}:{signal:AbortSignal.timeout(30000)})
  if(!response.ok)throw Error('Real endpoint HTTP '+response.status)
  return response.json()
}
const manifest=preflight?null:await api('/lucille/embedding-manifest'), space=manifest?.space
if(!preflight && (manifest.simulation!==false||space.dimension!==768||space.context_tokens!==2048))throw Error('Real pinned Nomic required')
const dataset=JSON.parse(readFileSync(args.dataset,'utf8')), rows=[]
for(const item of dataset.cases){
 try{
  const mode=item.mode, scope=item.scope, project=item.project
   if(!['chat','agent','code'].includes(mode)||!(/^phase22e-/.test(scope) || knownLegacyDevelopment && /^phase22d-/.test(scope)))throw Error('Explicit synthetic binding required')
  const embedded=preflight?null:await api('/v1/embeddings',{model:space.model,input:space.query_prefix+item.query,encoding_format:'float'})
  const vector=preflight?null:{space,vector:embedded.data[0].embedding}, messageId=randomUUID(), conversationId='phase22e-independent-answer-'+item.id
  let context
  const loader=async request=>{
    context=native({operation:'context',request,query_vector:vector})
    if(item.known_context){
      // Stage D can hold native facts fixed independently of stage C ranking.
      // Selectors are frozen semantic gold, not replacement source/model text.
      const details=native({operation:'details',scope,project})
      const selected=selectKnownRecords(details,item.known_context)
      const references=selected.map(m=>({id:m.input.id,version:m.version}))
      const evidence=native({operation:'answer_evidence',scope,project,request,references,claims:[]})
      verifyKnownProjection(evidence.facts,references,item.known_context)
      context={...context,memories:selected,facts:evidence.facts,references,current_statements:[]}
      context.used_upper_bound=Buffer.byteLength(JSON.stringify(context),'utf8')+32
      if(context.used_upper_bound>request.token_budget)throw Error('Known context exceeds the unchanged budget')
    }
    return context
  }
  if(preflight){
    await loader({conversation_id:conversationId,message_id:messageId,scope,project,mode,query:item.query,token_budget:8192})
    const row={id:item.id,item,status:'preflight_ready',references:context.references,facts:context.facts}
    rows.push(row);appendFileSync(progress,JSON.stringify(row)+'\n',{encoding:'utf8'});continue
  }
  const binding={profile:'lucille',scope,project,tokenBudget:8192}
  const reload=async()=>{
    const wire=await withLucilleContext({id:conversationId,lucille:binding},messageId,item.query,mode,[{role:'user',content:item.query}],undefined,loader)
    const snapshot=answerSnapshot(wire), request={conversation_id:conversationId,message_id:messageId,scope,project,mode,query:item.query,token_budget:8192}
    snapshot.refresh=async(claims=[])=>native({operation:'answer_evidence',scope,project,request,references:context.references,claims})
    snapshot.reload=reload
    return wire
  }
  const wire=await reload(), generations=[]
  const generate=async messages=>{
    const began=performance.now(), request={model:'lucille-real-gemma',messages,temperature:0,max_tokens:384,stream:false,reasoning_effort:'none',chat_template_kwargs:{enable_thinking:false}}
    const response=await api('/v1/chat/completions',request), choice=response.choices[0]
    generations.push({request,response,latency_ms:performance.now()-began})
    return {content:choice.message.content??'',toolCalls:choice.message.tool_calls,finishReason:choice.finish_reason,promptEvalCount:response.usage?.prompt_tokens,evalCount:response.usage?.completion_tokens}
  }
  const began=performance.now();let result
  try{const accepted=await validateLucilleCompletion(wire,await generate(wire),generate);result={delivered:accepted.completion.content,report:accepted.report,status:'delivered'}}
  catch(error){result={delivered:'',status:error instanceof MemoryAnswerBlocked?'blocked':'error',error:String(error),report:error instanceof MemoryAnswerBlocked?error.report:undefined}}
  const row={id:item.id,item,context,generations,...result,latency_ms:performance.now()-began}
  rows.push(row);appendFileSync(progress,JSON.stringify(row)+'\n',{encoding:'utf8'})
  console.log(item.id,mode,result.status)
 }catch(error){
  const row={id:item.id,item,status:'not_executed',error:String(error),generations:[],delivered:''}
  rows.push(row);appendFileSync(progress,JSON.stringify(row)+'\n',{encoding:'utf8'})
  console.log(item.id,'not_executed',String(error))
 }
}
verify()
writeFileSync(output,JSON.stringify({method:preflight?'Preflight only: all native selectors and canonical projections; no embedding or generation.':'Independent D: actual canonical native read, actual production context adapter and verifier, actual pinned Gemma/Nomic. No learning publication or executor dispatch; CLI runtime observations are not supplied. Literal claim fidelity and usefulness require independent adjudication.',freeze_sha256:sha(readFileSync(args.freeze)),dataset_sha256:sha(readFileSync(args.dataset)),rows,preflight,automatic:{planned:dataset.cases.length,total:rows.length,preflight_ready:rows.filter(r=>r.status==='preflight_ready').length,not_executed:rows.filter(r=>r.status==='not_executed').length,delivered:rows.filter(r=>r.status==='delivered').length,blocked:rows.filter(r=>r.status==='blocked').length,errors:rows.filter(r=>r.status==='error').length}},null,2)+'\n',{encoding:'utf8',flag:'wx'})
