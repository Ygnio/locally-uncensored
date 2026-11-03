// Protocol fixtures only: neither Gemma inference nor Nomic embeddings.
import { createHash } from 'node:crypto'
export const syntheticSpace = { model: 'nomic-ai/nomic-embed-text-v1.5', revision: 'SYNTHETIC-NOT-NOMIC-WEIGHTS', fingerprint: 'f'.repeat(64), dimension: 64, normalization: 'l2', pooling: 'mean', document_prefix: 'search_document: ', query_prefix: 'search_query: ', context_tokens: 8192, index_version: 1 }
export function syntheticVector(input) {
  const text = input.toLowerCase().normalize('NFD').replace(/\p{M}/gu, '')
  const groups = [['bicycle', 'bicicleta', 'two wheels', 'duas rodas'], ['cafe', 'coffee', 'bebida', 'drink'], ['rpg', 'tatico', 'taticos', 'classes', 'jogos'], ['ram', 'computador', 'computer'], ['sqlite', 'database', 'banco'], ['rust', 'aprender', 'learn']]
  const vector = Array(64).fill(0)
  groups.forEach((words, i) => { if (words.some(word => text.includes(word))) vector[i] = 1 })
  if (!vector.some(Boolean)) { const bytes = createHash('sha256').update(text).digest(); for (let i = 0; i < 16; i++) vector[8 + i] = (bytes[i] - 128) / 128 }
  const norm = Math.sqrt(vector.reduce((n, v) => n + v * v, 0))
  return vector.map(v => v / norm)
}
export function syntheticExtraction(input) {
  const evidence = input.user_text.replace(/[.!]$/, '').trim()
  let property, value, content = evidence
  if (evidence === 'Gosto de montar personagens com várias classes') { property = 'games_classes'; value = 'várias classes' }
  if (evidence === 'Prefiro café sem açúcar') { property = 'coffee_sugar'; value = 'without_sugar'; content = 'Prefere café sem açúcar.' }
  const proposals = property ? [{ kind: 'preference', content, entity: 'user', property, value, scope: input.scope, project: input.project, temporal: 'preference', evidence, correction: false, hypothesis: false, confidence: 0.99, relevance: 0.95 }] : []
  return JSON.stringify({ proposals })
}
