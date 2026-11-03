// Synthetic model protocol only. The complete application uses its real hooks,
// real Rust HTTP proxy, real SQLite and real filesystem/tool executor.
import http from 'node:http'
import { syntheticSpace, syntheticVector, syntheticExtraction } from './lucille-phase21-fixtures.mjs'
import {readFileSync,appendFileSync} from 'node:fs'
import {resolve,join} from 'node:path'
let embeddingsAvailable=true
const root=resolve('.lucille-qa-runtime/lucille-synthetic-full')
if(readFileSync(join(root,'.lucille-isolated'),'utf8')!=='synthetic-data-only\n')throw new Error('Synthetic root required')
const parse=message=>(message.content??'').split('\n').flatMap(line=>{try{return [JSON.parse(line)]}catch{return []}})
http.createServer((req,res)=>{
  res.setHeader('Access-Control-Allow-Origin','http://127.0.0.1:15283')
  res.setHeader('Access-Control-Allow-Headers','Content-Type,Authorization')
  res.setHeader('Access-Control-Allow-Methods','GET,POST,OPTIONS')
  if(req.method==='OPTIONS'){res.writeHead(204).end();return}
  const json=value=>res.writeHead(200,{'Content-Type':'application/json'}).end(JSON.stringify(value))
  if(req.method==='GET'){
    if(req.url==='/lucille/test/embeddings-disable'){embeddingsAvailable=false;return json({synthetic:true,embeddings_available:false})}
    if(req.url==='/lucille/test/embeddings-enable'){embeddingsAvailable=true;return json({synthetic:true,embeddings_available:true})}
    if(req.url==='/lucille/embedding-manifest'&&!embeddingsAvailable){res.writeHead(503).end();return}
    if(req.url==='/lucille/embedding-manifest')return json({simulation:true,space:syntheticSpace})
    if(req.url==='/v1/models')return json({object:'list',data:['aurora-qa','aurora-qa-alt'].map(id=>({id,object:'model',owned_by:'synthetic',context_length:16384,supports_function_calling:true}))})
    if(req.url==='/props')return json({default_generation_settings:{n_ctx:16384},chat_template_caps:{supports_tools:true,supports_tool_calls:true},model_alias:'aurora-qa',chat_template:'synthetic payload validation'})
    if(req.url==='/health')return json({status:'ok',synthetic:true})
    res.writeHead(404).end();return
  }
  if(req.method!=='POST'||!['/v1/chat/completions','/v1/embeddings'].includes(req.url)){res.writeHead(404).end();return}
  let raw='';req.on('data',chunk=>{raw+=chunk;if(raw.length>4_000_000)req.destroy()})
  req.on('end',()=>{try{
    const payload=JSON.parse(raw),messages=payload.messages??[],values=messages.flatMap(parse)
    appendFileSync(join(root,'captures.ndjson'),JSON.stringify({observed_at:new Date().toISOString(),payload})+'\n')
    if(req.url==='/v1/embeddings')return json({object:'list',model:syntheticSpace.model,data:[{index:0,embedding:syntheticVector(payload.input)}],usage:{prompt_tokens:16,total_tokens:16},simulation:true})
    if(messages[0]?.content?.startsWith('LUCILLE_STRUCTURED_EXTRACTION_V1'))return json({id:'synthetic-extractor',choices:[{index:0,message:{role:'assistant',content:syntheticExtraction(JSON.parse(messages.at(-1).content))},finish_reason:'stop'}],usage:{prompt_tokens:128,completion_tokens:64}})
    const identity=values.find(value=>value?.profile_id==='lucille'),environment=values.find(value=>value?.source==='local_host_clock_and_executor')
    const memories=values.filter(Array.isArray).flat().filter(value=>value?.id&&value?.version)
    // The real provider can merge adjacent user data/request messages.
    const latest=messages.findLastIndex(message=>message.role==='user')
    const instruction=messages[latest]?.content??'',steps=messages.slice(latest+1).flatMap(message=>message.tool_calls??[])
    let call
    if(instruction.includes('QA_TOOL_SEQUENCE')){
      const plan=[['file_write',{path:'aurora.txt',content:'blue'}],['file_read',{path:'aurora.txt'}],['file_edit',{path:'aurora.txt',old_string:'blue',new_string:'green'}],['file_read',{path:'aurora.txt'}],['file_list',{path:'.'}]]
      if(steps.length<plan.length)call=plan[steps.length]
    }
    if(instruction.includes('QA_TOOL_ESCAPE')&&!steps.length)call=['file_read',{path:'../outside.txt'}]
    if(instruction.includes('QA_TOOL_FAILURE')&&!steps.length)call=['file_read',{path:'missing-synthetic.txt'}]
    if(instruction.includes('QA_TOOL_VERIFY')&&!steps.length)call=['file_read',{path:'aurora.txt'}]
    if(instruction.includes('QA_TOOL_DESTRUCTIVE')&&!steps.length)call=['shell_execute',{command:'Remove-Item aurora.txt',cwd:'.'}]
    if(call&&!payload.tools?.some(tool=>tool.function?.name===call[0]))call=undefined
    const text=`[SIMULATED: no inference] identity=${identity?.name??'default'}; version=${identity?.version??0}; mode=${environment?.mode??'default'}; memories=${JSON.stringify(memories)}; date=${environment?.local_datetime??'none'}; timezone=${environment?.timezone??'none'}; backend=${environment?.backend??'none'}; model=${environment?.model??payload.model}; receipts=${JSON.stringify(environment?.execution_receipts??[])}.`
    const tool_calls=call?[{index:0,id:'qa-'+steps.length,type:'function',function:{name:call[0],arguments:JSON.stringify(call[1])}}]:undefined
    if(!payload.stream)return json({id:'synthetic',object:'chat.completion',choices:[{index:0,message:{role:'assistant',content:tool_calls?null:text,...(tool_calls?{tool_calls}: {})},finish_reason:tool_calls?'tool_calls':'stop'}],usage:{prompt_tokens:1024,completion_tokens:32}})
    res.writeHead(200,{'Content-Type':'text/event-stream','Cache-Control':'no-cache'})
    for(const delta of [{role:'assistant'},tool_calls?{tool_calls}:{content:text}])res.write('data: '+JSON.stringify({id:'synthetic',object:'chat.completion.chunk',choices:[{index:0,delta,finish_reason:null}]})+'\n\n')
    res.end('data: '+JSON.stringify({id:'synthetic',choices:[{index:0,delta:{},finish_reason:tool_calls?'tool_calls':'stop'}],usage:{prompt_tokens:1024,completion_tokens:32}})+'\n\ndata: [DONE]\n\n')
  }catch{res.writeHead(400).end('Invalid synthetic payload')}})
}).listen(15284,'127.0.0.1',()=>console.log('Synthetic full-app model endpoint: 127.0.0.1:15284'))
