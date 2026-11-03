// Loopback-only deterministic payload inspector. No model, credentials or external requests.
import http from 'node:http'
import { appendFileSync, readFileSync } from 'node:fs'
import { resolve, join, relative, isAbsolute } from 'node:path'
const root = resolve(process.argv[2] ?? '')
const allowed = resolve('.lucille-qa-runtime')
const rel = relative(allowed, root)
if (!rel || rel.startsWith('..') || isAbsolute(rel) || readFileSync(join(root, '.lucille-isolated'), 'utf8') !== 'synthetic-data-only\n') throw new Error('Explicit synthetic QA root required')
const parse = message => {
  for (const line of message?.content?.split('\n') ?? []) {
    if (!line.startsWith('{') && !line.startsWith('[')) continue
    try { return JSON.parse(line) } catch { /* ordinary operational text */ }
  }
  return null
}
http.createServer((request, response) => {
  response.setHeader('Access-Control-Allow-Origin', '*')
  response.setHeader('Access-Control-Allow-Headers', 'Content-Type')
  response.setHeader('Access-Control-Allow-Methods', 'POST,OPTIONS')
  if (request.method === 'OPTIONS') { response.writeHead(204).end(); return }
  if (request.method !== 'POST' || request.url !== '/v1/chat/completions') { response.writeHead(404).end(); return }
  let data = ''
  request.on('data', chunk => { data += chunk; if (data.length > 1_000_000) request.destroy() })
  request.on('end', () => {
    try {
      const payload = JSON.parse(data)
      const messages = payload.messages ?? []
      const identity = messages.map(parse).find(value => value?.profile_id === 'lucille')
      const environment = messages.map(parse).find(value => value?.source === 'local_host_clock_and_executor')
      const memories = messages.map(parse).filter(Array.isArray).flat()
      appendFileSync(join(root, 'captures.ndjson'), JSON.stringify({ captured_at: new Date().toISOString(), payload }) + '\n')
      const name = identity?.name ?? (payload.mode === 'code' ? 'Coding Agent (default)' : 'Default assistant')
      const content = `[SIMULATED: no inference] ${name}; mode=${payload.mode}; date=${environment?.local_datetime ?? 'not supplied'}; timezone=${environment?.timezone ?? 'not supplied'}; memories=${memories.map(memory => memory.content).join(' | ') || 'none'}; confirmed tools=${environment?.execution_receipts?.filter(receipt => receipt.result_confirmed).length ?? 0}.`
      response.writeHead(200, { 'Content-Type': 'application/json' }).end(JSON.stringify({ choices: [{ message: { role: 'assistant', content } }] }))
    } catch { response.writeHead(400).end('Invalid synthetic payload') }
  })
}).listen(15274, '127.0.0.1', () => process.stdout.write('Synthetic payload inspector listening on 127.0.0.1:15274\n'))
