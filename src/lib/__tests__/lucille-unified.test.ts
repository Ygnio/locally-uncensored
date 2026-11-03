import { describe, it, expect } from 'vitest'
import { CODEX_SYSTEM_PROMPT, CODEX_REVIEW_SYSTEM_PROMPT, CODEX_SYSTEM_PROMPT_LEAN } from '../codex-prompts'
import { CODEX_PLAN_SYSTEM_PROMPT } from '../codex-plan-prompt'
import { withLucilleContext, type LucilleTurnContext } from '../lucille-context'
import { operationalCodePrompt, verifiedEnvironment, executorEvidence } from '../verified-environment'
import { executeParallel } from '../../api/agents/tool-executor'
import { mkdtempSync, writeFileSync, readFileSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { resolve, join, relative, isAbsolute } from 'node:path'

const binding = { id: 'synthetic-turn', lucille: { profile: 'lucille' as const, scope: 'lucille', project: 'synthetic-a', tokenBudget: 8192 } }
const profile = { name: 'Synthetic Aurora', description: 'Fictional', personality: 'Concise', instructions: 'Use synthetic facts.' }
const context = (request: Record<string, unknown>): LucilleTurnContext => ({ ...request,
  identity: { profile, version: 4, created_at: 1, updated_at: 2 }, memories: [], references: [], used_upper_bound: 1500, retrieval_state: 'lexical_fresh_snapshot',
} as unknown as LucilleTurnContext)

describe('unified identity and verified environment', () => {
  it('reproduces the legacy Code self-label and preserves default behavior when unbound', async () => {
    expect(CODEX_SYSTEM_PROMPT).toMatch(/^You are the Coding Agent,/)
    expect(CODEX_SYSTEM_PROMPT).not.toContain('Synthetic Aurora')
    const messages = [{ role: 'system' as const, content: CODEX_SYSTEM_PROMPT }]
    expect(await withLucilleContext({ id: 'native' }, 'm', 'q', 'code', messages)).toBe(messages)
  })
  it.each([CODEX_SYSTEM_PROMPT, CODEX_REVIEW_SYSTEM_PROMPT, CODEX_PLAN_SYSTEM_PROMPT, CODEX_SYSTEM_PROMPT_LEAN])('retains operational rules without replacing the configured name', async prompt => {
    const messages = await withLucilleContext(binding, 'm', 'q', 'code', [{ role: 'system', content: prompt }], undefined, async req => context(req), { backend: 'synthetic-provider', model: 'synthetic-effective-model', availableTools: ['file_read'] })
    expect(messages[0].content).toContain('Synthetic Aurora')
    expect(messages[0].content).toContain('"version":4')
    expect(messages.some(message => message.content.startsWith('You are the Coding Agent'))).toBe(false)
    expect(messages.at(-1)!.content.split('\n').slice(1).join('\n')).toBe(prompt.split('\n').slice(1).join('\n'))
    expect(operationalCodePrompt(prompt)).toContain('Operational mode: Code')
  })
  it.each(['chat', 'agent', 'code'] as const)('applies the same persistent identity and current verified route in %s', async mode => {
    const result = await withLucilleContext(binding, 'm', 'q', mode, [{ role: 'user', content: 'Current date?' }], undefined, async req => context(req), { backend: 'synthetic-backend', model: 'effective-model', availableTools: ['file_read', 'web_search'] })
    const env = result.find(message => message.content.includes('local_host_clock_and_executor'))!
    expect(env.content).toContain('effective-model')
    expect(env.content).toContain(`"mode":"${mode}"`)
    expect(env.content).toContain('Tool availability is not execution')
    expect(env.content).toContain('"execution_receipts":[]')
    expect(result.at(-1)!.content).toBe('Current date?')
  })
  it('uses a fresh local clock and a verified timezone across a day boundary, with no hardware assumptions', () => {
    const runtime = { backend: 'fixture', model: 'actual-route', availableTools: [] }
    const first = verifiedEnvironment('code', runtime, new Date('2026-11-01T02:59:59Z'), 'America/Sao_Paulo')
    const second = verifiedEnvironment('code', runtime, new Date('2026-11-01T03:00:01Z'), 'America/Sao_Paulo')
    expect(first.local_datetime).toBe('2026-10-31 23:59:59')
    expect(second.local_datetime).toBe('2026-11-01 00:00:01')
    expect(second.timezone).toBe('America/Sao_Paulo')
    expect(JSON.stringify(second)).not.toMatch(/hardware|workspace|C:\\|GPU/)
  })
  it('confirms real filesystem execution, preserves approval and rejects escape/error without inventing success', async () => {
    const root = mkdtempSync(join(tmpdir(), 'lucille-unified-'))
    const pathFor = (path: string) => {
      const target = resolve(root, path); const inside = relative(root, target)
      if (!inside || inside.startsWith('..') || isAbsolute(inside)) throw new Error('Outside workspace')
      return target
    }
    try {
      const runtime = { getTool: (name: string) => ['file_write', 'file_read', 'file_edit'].includes(name) ? { name } : undefined,
        awaitApproval: async () => true,
        execute: async (name: string, args: Record<string, unknown>) => {
          const path = pathFor(String(args.path))
          if (name === 'file_write') { writeFileSync(path, String(args.content)); return 'created' }
          if (name === 'file_edit') { writeFileSync(path, readFileSync(path, 'utf8').replace('blue', 'green')); return 'edited' }
          return readFileSync(path, 'utf8')
        } }
      const call = async (toolName: string, path: string, content?: string) => (await executeParallel([{ id: crypto.randomUUID(), toolName, args: { path, content } }], runtime))[0]
      const created = await call('file_write', 'fixture.txt', 'blue telescope')
      const read = await call('file_read', 'fixture.txt')
      expect(read.result).toBe('blue telescope')
      await call('file_edit', 'fixture.txt')
      expect((await call('file_read', 'fixture.txt')).result).toBe('green telescope')
      expect(executorEvidence(created)).toMatchObject({ requested: true, executed: true, result_confirmed: true })
      expect(executorEvidence(await call('file_read', '../outside.txt'))).toMatchObject({ executed: true, result_confirmed: false, outcome: 'failed' })
      expect(executorEvidence(await call('unknown_tool', 'fixture.txt'))).toMatchObject({ executed: false, result_confirmed: false })
      const rejected = (await executeParallel([{ id: 'denied', toolName: 'file_write', args: { path: 'denied.txt', content: 'fixture' } }], { ...runtime, awaitApproval: async () => false }))[0]
      expect(executorEvidence(rejected)).toMatchObject({ executed: false, result_confirmed: false, outcome: 'rejected' })
    } finally { clean(root) }
  })
})
function clean(root: string) {
  if (!root.startsWith(join(tmpdir(), 'lucille-unified-'))) throw new Error('Unexpected QA directory')
  rmSync(root, { recursive: true, force: true })
}
