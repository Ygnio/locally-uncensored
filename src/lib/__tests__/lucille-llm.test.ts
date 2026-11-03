import { beforeEach, describe, expect, it, vi } from 'vitest'
import { isolatedLlmRoute, tickLucilleLlm } from '../lucille-llm'
const mock = vi.hoisted(() => ({ invoke: vi.fn(), chat: vi.fn(), aborters: {} as Record<string, unknown>, listener: null as null | ((state: { aborters: Record<string, unknown> }) => void), config: { enabled: true, isLocal: true, managed: false, baseUrl: 'http://127.0.0.1:15284/v1' } }))
vi.mock('@tauri-apps/api/core', () => ({ invoke: mock.invoke }))
vi.mock('../../api/providers/registry', () => ({ openaiEntryOfModel: () => null, getProviderForModel: () => ({ provider: { chatWithTools: mock.chat }, modelId: 'synthetic' }) }))
vi.mock('../../stores/modelStore', () => ({ useModelStore: { getState: () => ({ activeModel: 'openai::synthetic' }) } }))
vi.mock('../../stores/providerStore', () => ({ useProviderStore: { getState: () => ({ providers: { openai: mock.config }, openaiEntries: [] }) } }))
vi.mock('../../stores/generationStore', () => ({ useGenerationStore: { getState: () => ({ aborters: mock.aborters }), subscribe: (callback: typeof mock.listener) => { mock.listener = callback; return () => { mock.listener = null } } } }))
beforeEach(() => {
  vi.clearAllMocks(); mock.aborters = {}; mock.config = { enabled: true, isLocal: true, managed: false, baseUrl: 'http://127.0.0.1:15284/v1' }
  mock.invoke.mockImplementation(async (_command: string, { request }: { request: { operation: string } }) => {
    if (request.operation === 'learning_policy') return { enabled: true, extraction_mode: 'hybrid' }
    if (request.operation === 'claim_llm') return { lease: { id: 'lease', scope: 'lucille', project: 'atlas' }, input: ['Prefiro café sem açúcar.', 'hybrid'] }
    return { completed: true }
  })
  mock.chat.mockResolvedValue({ content: '{"proposals":[]}', toolCalls: [] })
})
describe('background extraction route and scheduling', () => {
  it('refuses managed, cloud and operational routes before dispatch', () => {
    mock.config.baseUrl = 'http://127.0.0.1:8080/v1'; expect(isolatedLlmRoute).toThrow()
    mock.config.baseUrl = 'https://example.com/v1'; expect(isolatedLlmRoute).toThrow()
    mock.config.baseUrl = 'http://127.0.0.1:15284/v1'; mock.config.managed = true; expect(isolatedLlmRoute).toThrow()
  })
  it('does not claim work or infer while a primary response is active', async () => {
    mock.aborters = { primary: {} }; await tickLucilleLlm(); expect(mock.invoke).not.toHaveBeenCalled(); expect(mock.chat).not.toHaveBeenCalled()
  })
  it('makes one bounded call with minimum user input and no tools', async () => {
    await tickLucilleLlm(); expect(mock.chat).toHaveBeenCalledTimes(1)
    const [, messages, tools, options] = mock.chat.mock.calls[0]
    expect(messages).toHaveLength(2); expect(JSON.parse(messages[1].content)).toEqual({ scope: 'lucille', project: 'atlas', user_text: 'Prefiro café sem açúcar.' })
    expect(tools).toEqual([]); expect(options.maxTokens).toBe(768); expect(options).not.toHaveProperty('contextWindow')
  })
  it('aborts extraction when primary generation starts and fences completion', async () => {
    mock.chat.mockImplementation(async (_model, _messages, _tools, options) => new Promise((_resolve, reject) => options.signal.addEventListener('abort', () => reject(new Error('cancelled')))))
    const pending = tickLucilleLlm()
    await vi.waitFor(() => expect(mock.chat).toHaveBeenCalledTimes(1))
    mock.listener!({ aborters: { primary: {} } }); await pending
    const operations = mock.invoke.mock.calls.map(call => call[1].request.operation)
    expect(operations).toContain('fail_llm'); expect(operations).not.toContain('complete_llm')
  })
  it('uses verified rule-only fallback after hybrid transport failure', async () => {
    mock.chat.mockRejectedValue(new Error('endpoint failed')); await tickLucilleLlm()
    expect(mock.invoke.mock.calls.find(call => call[1].request.operation === 'complete_llm')?.[1].request.output).toBe('{"proposals":[]}')
  })
})
