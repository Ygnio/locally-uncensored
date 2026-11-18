// Isolated C2 continuation with the production context adapter and verifier.
import { execFileSync } from 'node:child_process'
import { readFileSync, realpathSync } from 'node:fs'
import { resolve, sep } from 'node:path'
import { pathToFileURL } from 'node:url'
import ts from '../node_modules/typescript/lib/typescript.js'

const repo = realpathSync(process.cwd())
const root = realpathSync(resolve(repo, '.lucille-qa-runtime/lucille-synthetic-real22f'))
const bridge = realpathSync(process.argv[2])
const store = realpathSync(process.argv[3])
if (bridge !== realpathSync(resolve(repo, '../build/phase22f/real_validation_bridge.exe'))
    || !store.startsWith(root + sep)
    || !store.split(sep).at(-1).startsWith('lucille-synthetic-c2-')) throw Error('Isolated F C2 paths required')
for (const path of [root, store]) {
  if (readFileSync(resolve(path, '.lucille-isolated'), 'utf8') !== 'synthetic-data-only\n') throw Error('Missing QA marker')
}
const { request, query_vector } = JSON.parse(readFileSync(0, 'utf8'))
const native = data => JSON.parse(execFileSync(bridge, [], {
  input: JSON.stringify(data), encoding: 'utf8', windowsHide: true, env: { ...process.env, LUCILLE_QA_ROOT: store },
}))

const urls = new Map()
const loading = new Set()
function actualModule(name) {
  if (urls.has(name)) return urls.get(name)
  if (!/^[a-z0-9-]+$/.test(name) || loading.has(name)) throw Error('Unsupported production adapter import: ' + name)
  loading.add(name)
  const source = readFileSync(resolve(repo, 'src/lib', name + '.ts'), 'utf8')
  const code = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
  }).outputText.replace(/from (['"])([^'"]+)\1/g, (_whole, _quote, specifier) => {
    if (specifier === '@tauri-apps/api/core') {
      return 'from ' + JSON.stringify(pathToFileURL(resolve(repo, 'node_modules/@tauri-apps/api/core.js')).href)
    }
    if (!specifier.startsWith('./')) throw Error('Unresolved production adapter import: ' + specifier)
    return 'from ' + JSON.stringify(actualModule(specifier.slice(2)))
  })
  loading.delete(name)
  const url = 'data:text/javascript;base64,' + Buffer.from(code).toString('base64')
  urls.set(name, url)
  return url
}

const { withLucilleContext } = await import(actualModule('lucille-context'))
const { answerSnapshot } = await import(actualModule('lucille-fidelity'))
const { validateLucilleCompletion, MemoryAnswerBlocked } = await import(actualModule('lucille-answer'))
const binding = { profile: 'lucille', scope: request.scope, project: request.project, tokenBudget: request.token_budget }
const generations = []
const contexts = []
async function reload() {
  let context
  const messages = await withLucilleContext({ id: request.conversation_id, lucille: binding }, request.message_id,
    request.query, request.mode, [{ role: 'user', content: request.query }], undefined, async turn => {
      context = native({ operation: 'context', request: turn, query_vector })
      contexts.push(context)
      return context
    })
  const snapshot = answerSnapshot(messages)
  snapshot.refresh = async (claims = []) => native({ operation: 'answer_evidence', request, references: context.references, claims })
  snapshot.reload = reload
  return messages
}

async function generate(messages) {
  const started = performance.now()
  const payload = { model: 'lucille-real-gemma', messages, temperature: 0, max_tokens: 384, stream: false,
    reasoning_effort: 'none', chat_template_kwargs: { enable_thinking: false } }
  const response = await fetch('http://127.0.0.1:15284/v1/chat/completions', {
    method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(payload), signal: AbortSignal.timeout(120000),
  })
  if (!response.ok) {
    const error = new Error('Owned model HTTP ' + response.status)
    error.status = response.status >= 500 ? 'infrastructure_failure' : 'evaluator_failure'
    throw error
  }
  const data = await response.json()
  generations.push({ request: payload, response: data, latency_ms: performance.now() - started })
  const choice = data.choices[0]
  return { content: choice.message.content ?? '', toolCalls: choice.message.tool_calls, finishReason: choice.finish_reason,
    promptEvalCount: data.usage?.prompt_tokens, evalCount: data.usage?.completion_tokens }
}

const started = performance.now()
let result
try {
  const messages = await reload()
  const { completion, report } = await validateLucilleCompletion(messages, await generate(messages), generate)
  result = { status: 'delivered', delivered: completion.content, report }
} catch (error) {
  if (error instanceof MemoryAnswerBlocked) result = { status: 'blocked', delivered: '', report: error.report }
  else result = { status: error.status ?? (error.name === 'TimeoutError' || error.cause?.code === 'ECONNREFUSED'
    ? 'infrastructure_failure' : 'evaluator_failure'), delivered: '', error: String(error) }
}
console.log(JSON.stringify({ ...result, contexts, generations, latency_ms: performance.now() - started }))
