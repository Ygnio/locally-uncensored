import type { ChatMessage, ChatStreamChunk, ToolCall } from '../api/providers/types'
import { settleThinking } from './thinking-stripper'
import { answerSnapshot, verifyMemoryAnswer, type AnswerSnapshot, type FidelityCheck } from './lucille-fidelity'

export interface MemoryCompletion {
  content: string; thinking?: string; toolCalls?: ToolCall[]; promptEvalCount?: number; evalCount?: number; finishReason?: string
}
export interface FidelityAttempt {
  rawAnswer: string; check: FidelityCheck; verificationReferences?: { id: string; version: number }[]; promptTokens?: number; completionTokens?: number
}
export interface AnswerFidelityReport {
  status: 'checked' | 'regenerated' | 'blocked'; attempts: FidelityAttempt[]; refreshMs: number
  verificationMs: number; extraGenerationMs: number
  additionalUsage: { promptTokens?: number; completionTokens?: number }
}
export class MemoryAnswerBlocked extends Error {
  readonly report: AnswerFidelityReport
  constructor(report: AnswerFidelityReport) {
    super('Não foi possível confirmar as afirmações de memória desta resposta. A resposta não verificada foi retida; tente novamente ou esclareça a informação.')
    this.name = 'MemoryAnswerBlocked'
    this.report = report
  }
}
const abortIfNeeded = (signal?: AbortSignal) => { if (signal?.aborted) throw new DOMException('Turn cancelled', 'AbortError') }

export async function collectMemoryCompletion(stream: AsyncGenerator<ChatStreamChunk>): Promise<MemoryCompletion> {
  let content = '', thinking = '', last: ChatStreamChunk = { content: '', done: false }
  for await (const chunk of stream) {
    content += chunk.content; thinking += chunk.thinking ?? ''; last = { ...last, ...chunk }
  }
  return { content, thinking, toolCalls: last.toolCalls, promptEvalCount: last.promptEvalCount,
    evalCount: last.evalCount, finishReason: last.finishReason ?? (last.done ? 'stop' : 'disconnect') }
}

/** At most one additional REAL generation. No scripted replacement answer.
 * A failed second check is an observable non-answer, never a quality success.
 */
export async function validateLucilleCompletion<T extends MemoryCompletion>(
  messages: ChatMessage[], initial: T,
  regenerate: (messages: ChatMessage[]) => Promise<T>, signal?: AbortSignal,
): Promise<{ completion: T; report: AnswerFidelityReport }> {
  let snapshot = answerSnapshot(messages)
  if (!snapshot) throw new Error('Lucille answer has no trusted context snapshot')
  const report: AnswerFidelityReport = { status: 'checked', attempts: [], refreshMs: 0, verificationMs: 0, extraGenerationMs: 0, additionalUsage: { promptTokens: 0, completionTokens: 0 } }
  let completion = initial
  for (let attempt = 0; attempt < 2; attempt++) {
    abortIfNeeded(signal)
    const refreshStart = performance.now()
    let evidence: AnswerSnapshot = snapshot
    if (snapshot.refresh) {
      const fresh = await snapshot.refresh()
      evidence = { ...snapshot, facts: fresh.facts, currentStatements: fresh.current_statements,directiveReceipt:fresh.directive_receipt,hasScopedRecords:fresh.has_scoped_records }
    }
    abortIfNeeded(signal); report.refreshMs += performance.now() - refreshStart
    const settled = settleThinking(completion.content, completion.thinking ?? '', true)
    let check = verifyMemoryAnswer(settled.content, evidence)
    const verificationReferences: { id: string; version: number }[] = []
    if (snapshot.refresh && check.violations.some(v => v.code === 'unsupported_typed_value')) {
      const fields = [...new Map(check.assertions.filter(a => !a.supported && a.entity !== 'unspecified' && !['ram_capacity_ratio','goal_target'].includes(a.property))
        .map(a => [a.entity + '|' + a.property + '|' + a.value, { entity:a.entity, property:a.property, value:a.value }])).values()].slice(0,16)
      if (fields.length) {
        const began = performance.now(), confirmed = await snapshot.refresh(fields)
        abortIfNeeded(signal); report.refreshMs += performance.now()-began
        verificationReferences.push(...confirmed.facts.filter(f => !evidence.facts.some(old => old.id===f.id && old.version===f.version)).map(f => ({id:f.id,version:f.version})))
        // The second native read also fences deletion of the original references.
        evidence = { ...snapshot, facts:confirmed.facts, currentStatements:confirmed.current_statements,directiveReceipt:confirmed.directive_receipt,hasScopedRecords:confirmed.has_scoped_records }
        report.verificationMs += check.elapsedMs
        check = verifyMemoryAnswer(settled.content,evidence)
      }
    }
    if (!settled.content.trim() && !completion.toolCalls?.length) check.violations.push({ code: 'empty_answer', span: '', critical: false })
    if (completion.finishReason && completion.finishReason !== 'stop' && !completion.toolCalls?.length) check.violations.push({ code: 'incomplete_answer', span: '', critical: false })
    report.verificationMs += check.elapsedMs
    report.attempts.push({ rawAnswer: completion.content, check, verificationReferences, promptTokens: completion.promptEvalCount, completionTokens: completion.evalCount })
    if (attempt > 0) report.additionalUsage = { promptTokens: completion.promptEvalCount, completionTokens: completion.evalCount }
    if (check.violations.length === 0) {
      report.status = attempt ? 'regenerated' : 'checked'
      return { completion: { ...completion, content: settled.content, thinking: settled.thinking }, report }
    }
    if (attempt === 1) break
    // Refresh the actual canonical context, not just the old output. A deleted
    // or superseded fact must not reappear in the correction request.
    const current = snapshot.reload ? await snapshot.reload() : messages
    snapshot = answerSnapshot(current)
    if (!snapshot) throw new Error('Lucille refreshed context snapshot unavailable')
    const codes = [...new Set(check.violations.map(v => v.code))].join(', ')
    const repairs:Record<string,string>={
      affirmed_unproven_project_execution:'The question asks whether implementation is already occurring. The available evidence only supports a plan or choice. Do not affirm or deny actual execution; state that it is unconfirmed, then distinguish the supported future plan.',
      denied_unproven_project_execution:'A plan also does not prove that implementation is absent. Do not give a categorical yes or no; distinguish unconfirmed current execution from the supported future plan.',
      unproven_negative_execution:'There is no proof that current execution is absent. State uncertainty about execution instead of claiming it has not started.',
      unconfirmed_memory_mutation:'No completed native operation supports the claimed save or reactivation. Acknowledge the content without saying you saved, noted, registered or reactivated it.',
      unconfirmed_memory_promise:'Do not promise automatic inclusion, updates or recording. Review or persistence has not completed. Acknowledge the original content; describe pending review only if relevant.',
      unproven_personal_performance:'Capacity alone cannot guarantee this person smooth performance or absence of bottlenecks. Acknowledge the stated capacity without a personal performance guarantee.',
      global_absence_without_evidence:'An empty retrieval is not proof that no memories are saved. Limit uncertainty to the relevant information that is unavailable.',
      wrong_forget_actor:'The user requested deletion from the assistant memory store; this does not mean the user mentally forgot their preference.',
      wrong_decision_actor:'The decision or unknown status belongs to the user and their project. Do not claim in first person that you made or did not make the decision.',
    }
    const guidance=[...new Set(check.violations.map(v=>repairs[v.code]).filter(Boolean))].join(' ')
    const correction: ChatMessage = { role: 'system', content: 'Memory answer fidelity retry (one attempt only). Previous attempt failed independent checks: ' + codes + '. ' + guidance + ' Produce a new, direct answer to the same current USER question in its language using the refreshed scoped data. Preserve plans, decisions, actual execution, history, uncertainty and negation. Missing retrieval is not proof of an unmade decision. Mention memory operations only when the user asks and a native receipt supports them. Do not announce this verification. Never invent a replacement fact or repeat the rejected error.' }
    const generationStart = performance.now()
    try {
      const lastUser=current.findLastIndex(message=>message.role==='user')
      const at=lastUser<0 ? current.length : lastUser
      completion = await regenerate([...current.slice(0,at),correction,...current.slice(at)])
    } catch {
      abortIfNeeded(signal)
      report.extraGenerationMs += performance.now()-generationStart
      report.status = 'blocked'
      // No second answer or usage is invented when the real retry fails.
      throw new MemoryAnswerBlocked(report)
    }
    report.extraGenerationMs += performance.now() - generationStart
  }
  report.status = 'blocked'
  throw new MemoryAnswerBlocked(report)
}

/** Nothing, including a partial invalid sentence, is yielded before validation.
 * The underlying transport may be native streaming or the documented QA SSE adapter.
 */
export async function* validateLucilleStream(
  stream: AsyncGenerator<ChatStreamChunk>, wire: () => ChatMessage[] | undefined,
  regenerate: (messages: ChatMessage[]) => Promise<MemoryCompletion>,
  onReport: (report: AnswerFidelityReport) => void, signal?: AbortSignal,
): AsyncGenerator<ChatStreamChunk> {
  let content = '', thinking = '', last: ChatStreamChunk = { content: '', done: false }
  for await (const chunk of stream) {
    abortIfNeeded(signal); content += chunk.content; thinking += chunk.thinking ?? ''; last = { ...last, ...chunk }
  }
  const messages = wire()
  if (!messages) throw new Error('Lucille generated response without prepared context')
  const initial = { content, thinking, toolCalls: last.toolCalls, promptEvalCount: last.promptEvalCount, evalCount: last.evalCount, finishReason: last.finishReason ?? (last.done ? 'stop' : 'disconnect') }
  try {
    const { completion, report } = await validateLucilleCompletion(messages, initial, regenerate, signal)
    onReport(report); abortIfNeeded(signal)
    yield { ...last, content: completion.content, thinking: completion.thinking, toolCalls: completion.toolCalls,
      promptEvalCount: completion.promptEvalCount, evalCount: completion.evalCount, finishReason: completion.finishReason ?? 'stop', done: true }
  } catch (error) {
    if (error instanceof MemoryAnswerBlocked) onReport(error.report)
    throw error
  }
}
