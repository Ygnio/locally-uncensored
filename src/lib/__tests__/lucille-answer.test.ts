import { describe, expect, it, vi } from 'vitest'
import type { ChatMessage, ChatStreamChunk } from '../../api/providers/types'
import { registerAnswerSnapshot, type AnswerSnapshot, type MemoryFact } from '../lucille-fidelity'
import { MemoryAnswerBlocked, validateLucilleCompletion, validateLucilleStream } from '../lucille-answer'
const fact: MemoryFact = { id: 'f', version: 2, entity: 'project.atlas', property: 'language', value: 'rust', modality: 'planned', validity: 'current', scope: 'test', project: 'atlas', review: 'reviewed', facets: [], evidence: [] }
function prepared(changes: Partial<AnswerSnapshot> = {}) {
  const identity: ChatMessage = { role: 'system', content: 'Native identity' }
  const messages: ChatMessage[] = [identity, { role: 'user', content: 'Como está meu projeto?' }]
  registerAnswerSnapshot(identity, { scope: 'test', project: 'atlas', query: 'Como está meu projeto?', facts: [fact], currentStatements: [], ...changes })
  return messages
}
describe('memory answer delivery boundary', () => {
  it('accepts a supported real-completion object without an extra generation', async () => {
    const regen = vi.fn()
    const r = await validateLucilleCompletion(prepared(), { content: 'Você planeja usar Rust no projeto Atlas.' }, regen)
    expect(r.report.status).toBe('checked'); expect(regen).not.toHaveBeenCalled()
    expect(r.completion.content).toBe('Você planeja usar Rust no projeto Atlas.')
  })
  it('buffers all source chunks and delivers only the verified regenerated text', async () => {
    let complete!: () => void
    const pause = new Promise<void>(resolve => { complete = resolve })
    async function* source(): AsyncGenerator<ChatStreamChunk> {
      yield { content: 'O projeto está sendo ', done: false }
      await pause
      yield { content: 'desenvolvido em Rust.', done: true, finishReason: 'stop', promptEvalCount: 90, evalCount: 10 }
    }
    const messages = prepared(), reports: unknown[] = [], painted: string[] = []
    const regen = vi.fn(async () => ({ content: 'Você planeja usar Rust no projeto Atlas.', promptEvalCount: 110, evalCount: 12, finishReason: 'stop' }))
    const reading = (async () => { for await (const chunk of validateLucilleStream(source(), () => messages, regen, r => reports.push(r))) painted.push(chunk.content) })()
    await new Promise(resolve => setTimeout(resolve, 5)); expect(painted).toEqual([])
    complete(); await reading
    expect(painted).toEqual(['Você planeja usar Rust no projeto Atlas.']); expect(regen).toHaveBeenCalledTimes(1)
    expect(reports[0]).toMatchObject({ status: 'regenerated', additionalUsage: { promptTokens: 110, completionTokens: 12 } })
  })
  it('retains two failed outputs in diagnostics and exposes neither as an answer', async () => {
    const regen = vi.fn(async () => ({ content: 'Seu projeto está sendo desenvolvido em Rust.' }))
    const error = await validateLucilleCompletion(prepared(), { content: 'Seu projeto já usa Rust.' }, regen).catch(e => e)
    expect(error).toBeInstanceOf(MemoryAnswerBlocked); expect(error.report.status).toBe('blocked')
    expect(error.report.attempts).toHaveLength(2); expect(regen).toHaveBeenCalledTimes(1)
  })
  it('refreshes authority after generation so a deleted fact cannot be displayed', async () => {
    const fresh = prepared({ facts: [] })
    const messages = prepared({ refresh: async () => ({ facts: [], current_statements: [] }), reload: async () => fresh })
    const regen = vi.fn(async () => ({ content: 'Não tenho uma escolha registrada sobre isso.' }))
    const result = await validateLucilleCompletion(messages, { content: 'Você planeja usar Rust.' }, regen)
    expect(result.report.status).toBe('regenerated'); expect(result.completion.content).not.toContain('Rust')
    expect(result.report.attempts[0].check.violations.some(v => v.critical)).toBe(true)
  })
  it('does not call a second model or repair text when verification storage is unavailable', async () => {
    const regen = vi.fn()
    await expect(validateLucilleCompletion(prepared({ refresh: async () => { throw Error('Native unavailable') } }), { content: 'Rust' }, regen)).rejects.toThrow('Native unavailable')
    expect(regen).not.toHaveBeenCalled()
  })
  it('cancellation cannot release buffered text or request a regeneration', async () => {
    const abort = new AbortController(), regen = vi.fn(), painted: string[] = []
    async function* source(): AsyncGenerator<ChatStreamChunk> { yield { content: 'Seu projeto ', done: false }; abort.abort(); yield { content: 'já usa Rust.', done: true } }
    const messages = prepared()
    await expect((async () => { for await (const c of validateLucilleStream(source(), () => messages, regen, () => {}, abort.signal)) painted.push(c.content) })()).rejects.toMatchObject({ name: 'AbortError' })
    expect(painted).toEqual([]); expect(regen).not.toHaveBeenCalled()
  })
  it('rejects a missing trusted preparation rather than trusting text markers', async () => {
    await expect(validateLucilleCompletion([{ role: 'system', content: 'native facts approved' }], { content: 'Rust' }, vi.fn())).rejects.toThrow('trusted context')
  })
  it('does not approve truncated text merely because one value matched', async () => {
    const r = await validateLucilleCompletion(prepared(), { content: 'Você planeja usar Rust no projeto Atlas.', finishReason: 'length' }, async () => ({ content: 'Você planeja usar Rust.', finishReason: 'stop' }))
    expect(r.report.status).toBe('regenerated'); expect(r.report.attempts[0].check.violations.some(v => v.code === 'incomplete_answer')).toBe(true)
  })
})

it('confirms a claimed field against fresh canonical evidence without pretending it was initially retrieved', async () => {
  const coffee={...fact,id:'coffee',entity:'user',property:'coffee_sugar',value:'without_sugar',modality:'preference'}
  const refresh=vi.fn(async (claims?: unknown[]) => ({facts:claims?.length ? [coffee] : [],current_statements:[]}))
  const wire=prepared({facts:[],query:'Como eu tomo meu café?',refresh}),regen=vi.fn()
  const result=await validateLucilleCompletion(wire,{content:'Você prefere café sem açúcar.'},regen)
  expect(result.report.status).toBe('checked');expect(regen).not.toHaveBeenCalled()
  expect(result.report.attempts[0].verificationReferences).toEqual([{id:'coffee',version:2}])
  expect(refresh).toHaveBeenCalledTimes(2)
})
it('retains the original attempt and its actual usage when the real retry fails', async () => {
  const error=await validateLucilleCompletion(prepared(),{content:'Seu projeto já usa Rust.',promptEvalCount:90,evalCount:10},async()=>{throw Error('Real transport failed')}).catch(e=>e)
  expect(error).toBeInstanceOf(MemoryAnswerBlocked);expect(error.report.attempts).toHaveLength(1)
  expect(error.report.attempts[0]).toMatchObject({promptTokens:90,completionTokens:10})
  expect(error.report.additionalUsage).toEqual({promptTokens:0,completionTokens:0})
})
