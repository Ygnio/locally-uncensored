// Real application entry/components/hooks; no production dev-server routes/plugins.
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import { readFileSync } from 'node:fs'
import { resolve, sep } from 'node:path'
const contract = process.env.LUCILLE_REAL_VALIDATION_CONTRACT
let publicPin = ''
if (contract) {
  const path = resolve(contract), root = resolve('.lucille-qa-runtime/lucille-synthetic-real21c')
  if (!path.startsWith(root + sep) || readFileSync(resolve(root, '.lucille-isolated'), 'utf8') !== 'synthetic-data-only\n') throw new Error('Real pin must come from the dedicated marked QA root')
  const data = JSON.parse(readFileSync(path, 'utf8'))
  publicPin = JSON.stringify({ simulation: false, models: data.models, runtime_sha256: data.runtime_sha256 })
}
export default defineConfig({
  plugins: [react(), tailwindcss()],
  define: {'import.meta.env.VITE_LUCILLE_PHASE1': JSON.stringify('1'), 'import.meta.env.VITE_LUCILLE_REAL_VALIDATION_PIN': JSON.stringify(publicPin)},
  server: {host:'127.0.0.1', port:15283, strictPort:true, fs:{strict:true},watch:{ignored:['**/.lucille-qa-runtime/**','**/src-tauri/**','**/crates/**','**/qa-dist/**']},headers:{'Content-Security-Policy':"default-src 'self'; connect-src 'self' ws://127.0.0.1:15283 http://127.0.0.1:15284 ipc: http://ipc.localhost; img-src 'self' data: blob:; font-src 'self' data:; style-src 'self' 'unsafe-inline'; script-src 'self' 'unsafe-inline' 'wasm-unsafe-eval'"}},
})
