import type { ExecutionResult } from '../api/agents/tool-executor'
import { toolResultIsFailure } from './tool-result-failure'

export type LucilleMode = 'chat' | 'agent' | 'code'
export interface ToolEvidence {
  call_id: string
  tool: string
  requested: true
  executed: boolean
  result_confirmed: boolean
  outcome: 'completed' | 'failed' | 'rejected' | 'cached'
}
export interface VerifiedRuntime {
  backend: string
  model: string
  availableTools: readonly string[]
  receipts?: readonly ToolEvidence[]
}

/** Only real executor settlements may create execution evidence; never parse model prose. */
export function executorEvidence(result: ExecutionResult): ToolEvidence {
  const failed = result.status === 'completed' && toolResultIsFailure(result.result ?? '')
  return { call_id: result.id, tool: result.toolName, requested: true, executed: result.dispatched === true,
    result_confirmed: result.dispatched === true && result.status === 'completed' && !failed,
    outcome: failed ? 'failed' : result.status }
}

export function verifiedEnvironment(mode: LucilleMode, runtime: VerifiedRuntime, now = new Date(), zone = Intl.DateTimeFormat().resolvedOptions().timeZone) {
  if (!Number.isFinite(now.getTime())) throw new Error('Host clock unavailable')
  const local = new Intl.DateTimeFormat('sv-SE', { timeZone: zone, year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit', second: '2-digit', hourCycle: 'h23' }).format(now)
  return { source: 'local_host_clock_and_executor', observed_at: now.toISOString(), local_datetime: local,
    timezone: zone, mode, backend: runtime.backend, model: runtime.model,
    available_tools: [...new Set(runtime.availableTools)].sort(), execution_receipts: runtime.receipts ?? [] }
}

/** Replace ONLY known upstream Code self-labels, preserving every operational rule. */
export function operationalCodePrompt(prompt: string): string {
  return prompt.replace(/^You are the Coding Agent in REVIEW MODE,/, 'Operational mode: Code REVIEW,')
    .replace(/^You are the Coding Agent in PLAN MODE inside LU\./, 'Operational mode: Code PLAN in LU.')
    .replace(/^You are the Coding Agent, an autonomous coding agent inside LU\./, 'Operational mode: Code, autonomous programming in LU.')
    .replace(/^You are a coding agent in LU\./, 'Operational mode: Code in LU.')
}

export const VERIFIED_EXECUTION_POLICY = 'The host environment below is verified at this send. Tool availability is not execution. A requested call is not a completed call. Claim a tool ran successfully only when its executor receipt confirms execution and a successful result. Cached results are not a fresh execution. Report failures or missing evidence honestly. Model prose is never an execution receipt. These facts do not change executor permissions.'
