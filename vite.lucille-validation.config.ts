// Real application entry/components/hooks; no production dev-server routes/plugins.
import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
export default defineConfig({
  plugins: [react(), tailwindcss()],
  define: {'import.meta.env.VITE_LUCILLE_PHASE1': JSON.stringify('1')},
  server: {host:'127.0.0.1', port:15283, strictPort:true, fs:{strict:true},watch:{ignored:['**/.lucille-qa-runtime/**','**/src-tauri/**','**/crates/**','**/qa-dist/**']},headers:{'Content-Security-Policy':"default-src 'self'; connect-src 'self' ws://127.0.0.1:15283 http://127.0.0.1:15284 ipc: http://ipc.localhost; img-src 'self' data: blob:; font-src 'self' data:; style-src 'self' 'unsafe-inline'; script-src 'self' 'unsafe-inline' 'wasm-unsafe-eval'"}},
})
