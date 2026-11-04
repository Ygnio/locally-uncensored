// Real application entry/components/hooks; no production dev-server routes/plugins.
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import { ABSENT_PACKAGES } from './src/dev/absent-packages'
import { readFileSync, existsSync, writeFileSync } from 'node:fs'
import { resolve, sep } from 'node:path'
const contract = process.env.LUCILLE_REAL_VALIDATION_CONTRACT
let publicPin = ''
let phase22Root = ''
let phase22b = false
let phase22c = false
if (contract) {
  const path = resolve(contract)
  const root = ['21c', '21d', '22', '22b', '22c'].map(phase => resolve(`.lucille-qa-runtime/lucille-synthetic-real${phase}`)).find(candidate => path.startsWith(candidate + sep))
  if (!root || readFileSync(resolve(root, '.lucille-isolated'), 'utf8') !== 'synthetic-data-only\n') throw new Error('Real pin must come from the dedicated marked QA root')
  const data = JSON.parse(readFileSync(path, 'utf8'))
  if (data.memory_protocol === 3 && /lucille-synthetic-real22[bc]?$/.test(root)) { phase22Root = root; phase22b = root.endsWith('real22b'); phase22c = root.endsWith('real22c') }
  publicPin = JSON.stringify({ simulation: false, ...(data.memory_protocol ? { memory_protocol: data.memory_protocol } : {}), models: data.models, runtime_sha256: data.runtime_sha256 })
}
// A QA-only in-page driver exercises the real composer/hooks when the Windows
// runtime cannot expose CDP. Only synthetic control/reports; no canonical writer.
const configureControlServer = (server: Pick<import('vite').ViteDevServer, 'middlewares'>) => {
    if (!phase22Root) return
    server.middlewares.use('/__lucille_phase22', (req, res) => {
      if (req.headers.host !== '127.0.0.1:15283' || (req.headers.origin && req.headers.origin !== 'http://127.0.0.1:15283')) { res.statusCode = 403; res.end(); return }
      if (req.method === 'GET') {
        const file = resolve(phase22Root, 'reports/control.json')
        res.setHeader('Content-Type', 'application/json'); res.setHeader('Cache-Control', 'no-store')
        res.end(existsSync(file) ? readFileSync(file) : '{}'); return
      }
      if (req.method !== 'POST') { res.statusCode = 405; res.end(); return }
      let body = ''; let rejected = false
      req.setEncoding('utf8')
      req.on('data', (chunk: string) => { body += chunk; if (Buffer.byteLength(body) > 8 * 1024 * 1024) { rejected = true; req.destroy() } })
      req.on('end', () => {
        try {
          if (rejected) return
          const { name, report, final } = JSON.parse(body)
          if (typeof name !== 'string' || !/^driver-[a-z0-9-]{1,73}\.json$/.test(name)) throw new Error('Report name')
          if (final !== true && !name.endsWith('-progress.json')) throw new Error('Only progress may be updated')
          const target = resolve(phase22Root, 'reports', name)
          if (final && existsSync(target)) throw new Error('Completed evidence is immutable')
          writeFileSync(target, JSON.stringify(report, null, 2) + '\n', { encoding: 'utf8', flag: final ? 'wx' : 'w' })
          res.end('recorded')
        } catch { res.statusCode = 400; res.end('Rejected synthetic report') }
      })
    })
}
const phase22Driver = {
  name: 'lucille-phase22-owned-driver',
  transformIndexHtml: { order:'pre' as const, handler() { return phase22Root ? [{ tag: 'script', attrs: { type: 'module', src: phase22c ? '/qa/phase22c-driver.js' : phase22b ? '/qa/phase22b-driver.js' : '/qa/phase22-driver.js' }, injectTo: 'body' as const }] : [] } },
  configureServer: configureControlServer,
  configurePreviewServer: configureControlServer,
}
const validationHeaders = {'Content-Security-Policy':"default-src 'self'; connect-src 'self' ws://127.0.0.1:15283 http://127.0.0.1:15284 ipc: http://ipc.localhost; img-src 'self' data: blob:; font-src 'self' data:; style-src 'self' 'unsafe-inline'; script-src 'self' 'unsafe-inline' 'wasm-unsafe-eval'"}
export default defineConfig({
  plugins: [react(), tailwindcss(), phase22Driver],
  build: phase22b || phase22c ? { outDir:phase22c ? 'qa-dist-22c' : 'qa-dist-22b', emptyOutDir:true } : undefined,
  preview: { host:'127.0.0.1',port:15283,strictPort:true,headers:validationHeaders },
  resolve: { alias: ABSENT_PACKAGES },
  define: {'import.meta.env.VITE_LUCILLE_PHASE1': JSON.stringify('1'), 'import.meta.env.VITE_LUCILLE_REAL_VALIDATION_PIN': JSON.stringify(publicPin)},
  server: {host:'127.0.0.1', port:15283, strictPort:true, fs:{strict:true},watch:{ignored:['**/.lucille-qa-runtime/**','**/src-tauri/**','**/crates/**','**/qa-dist/**']},headers: validationHeaders},
})
