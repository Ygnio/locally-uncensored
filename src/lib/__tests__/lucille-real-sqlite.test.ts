import { describe, it, expect } from 'vitest'
import { execFileSync } from 'node:child_process'
import { mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { randomUUID } from 'node:crypto'
import { withLucilleContext, type LucilleTurnContext } from '../lucille-context'

function cleanSyntheticRoot(root: string): void {
  if (!root.startsWith(join(tmpdir(), 'lucille-synthetic-'))) throw new Error('Unexpected synthetic test directory')
  rmSync(root, { recursive: true, force: true })
}

// CI and isolated QA build the real Rust bridge first. Ordinary unit-only runs report a skip.
describe.skipIf(!process.env.LUCILLE_QA_BRIDGE)('Chat/Agent -> shared adapter -> real Rust SQLite/FTS5', () => {
  it.each(['chat', 'agent'] as const)('%s uses persisted identity, scoped facts/preferences and fresh deletion state', async mode => {
    const root = mkdtempSync(join(tmpdir(), 'lucille-synthetic-'))
    const call = (command: unknown) => JSON.parse(execFileSync(process.env.LUCILLE_QA_BRIDGE!, [], { input: JSON.stringify(command), env: { ...process.env, LUCILLE_QA_ROOT: root }, encoding: 'utf8', windowsHide: true }))
    const id = randomUUID()
    const base = { scope: 'lucille', project: 'synthetic-a', source: 'manual', source_id: 'synthetic-manual' }
    try {
      call({ operation: 'seed', candidates: [
        { ...base, id, kind: 'fact', content: 'The synthetic telescope is blue', fact_key: 'telescope.color' },
        { ...base, id: randomUUID(), kind: 'preference', content: 'Prefer concise answers', fact_key: 'style' },
        { ...base, id: randomUUID(), project: 'synthetic-b', kind: 'fact', content: 'Other project telescope-private-fixture', fact_key: 'telescope.color' },
      ] })
      const conversation = { id: 'synthetic-conversation', lucille: { profile: 'lucille' as const, scope: 'lucille', project: 'synthetic-a', tokenBudget: 8192 } }
      const loader = async (request: Record<string, unknown>): Promise<LucilleTurnContext> => call({ operation: 'context', request })
      const initial = [{ role: 'user' as const, content: 'telescope' }]
      const first = await withLucilleContext(conversation, 'synthetic-message', 'telescope', mode, initial, undefined, loader)
      expect(first[0].content).toContain('Lucille Test Fixture')
      expect(JSON.stringify(first)).toContain('telescope is blue')
      expect(JSON.stringify(first)).toContain('Prefer concise answers')
      expect(JSON.stringify(first)).not.toContain('telescope-private-fixture')
      call({ operation: 'delete', id })
      const next = await withLucilleContext(conversation, 'synthetic-message', 'telescope', mode, first, undefined, loader)
      expect(JSON.stringify(next)).not.toContain('telescope is blue')
      expect(next.filter(message => message.content.includes('persistent identity'))).toHaveLength(1)
    } finally {
      cleanSyntheticRoot(root)
    }
  })
})
