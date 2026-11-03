import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
export default defineConfig({
  plugins: [react()],
  build: { outDir: 'qa-dist', emptyOutDir: true, rolldownOptions: { input: 'qa/lucille.html' } },
})
