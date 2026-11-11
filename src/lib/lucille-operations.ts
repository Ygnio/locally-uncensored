import { answerRelationClauses, externalAssertion, normalizeEvidenceText as fold } from './lucille-authority'
import type { AnswerSnapshot, MemoryDirectiveReceipt, MemoryLearningReceipt } from './lucille-fidelity'

export type MemoryMode = 'chat' | 'agent' | 'code'
export interface MemoryOperationBinding {
  profile: 'lucille'
  identityVersion: number
  mode: MemoryMode
  sourceId?: string | null
}
export interface MemoryOperationState {
  id: string
  profile: 'lucille'
  identity_version: number
  mode: MemoryMode
  scope: string
  project: string
  conversation_id: string
  source_message_id: string
  action: 'learn' | 'forget'
  state: 'received' | 'source_persisted' | 'processing' | 'completed' | 'failed' | 'cancelled'
}

type OperationClaim = 'create' | 'candidate' | 'update' | 'forget' | 'stored' | 'promise' | 'processing'
const propertyPatterns: [RegExp, string][] = [
  [/cafe|coffee/, 'coffee_sugar'], [/tema|theme/, 'interface_theme'], [/idioma|language/, 'response_language'],
  [/bebida|beverage|drink/, 'favorite_beverage'], [/objetiv|objective/, 'communication_focus'], [/poluid|clutter|carregad/, 'interface_clutter'],
  [/personag|character/, 'games_character_creation'], [/classe|class/, 'games_class_selection'],
]
const persistedTarget = /\b(?:memoria|memorias|memory|memories|preferencia|preferencias|preference|preferences|personal goals|metas pessoais|informacoes de hardware|your settings|suas configuracoes)\b/
const externalTarget = /\b(?:registrei|anotei|salvei|gravei|atualizei|apaguei|removi|salve|salvar|guarde|guardar|atualize|atualizar|apagar|remover|save|saved|recorded|registered|update|updated|remove|removed|delete|deleted)\s+(?:(?:o|a|os|as|um|uma|the|a|this|that)\s+)?(?:arquivos?|files?|readme|patch|codigo|code|repositorio|repository|diretorio|directory|folder|logs?)\b|\b(?:no|na|in the|to the|into the)\s+(?:arquivo|file|readme|repositorio|repository|log|disco|disk)\b/
const copyOfSource = /\b(?:isso|essa|esse|this|it|que|that)\b/
const signalOfMutation = /\b(?:remov|apag|exclu|esquec|memor|guard|salv|registr|anot|armazen|grav|reativ|atualiz|persist|remov|delet|forgot|sav|stor|noted|record|updat)/
const firstPersonCompletion = /\b(?:removi|apaguei|exclui|esqueci|memorizei|guardei|salvei|registrei|reativei|anotei|atualizei|gravei|armazenei|persisti|adicionei|retive|incorporei)\b|\b(?:i have |i.ve |i )(?:already )?(?:removed|deleted|forgot|forgotten|saved|memorized|memorised|stored|registered|recorded|reactivated|noted|updated|persisted|added|retained)\b/
const passiveCompletion = /\b(?:foi|foram|esta|estao|ficou|ficaram|has been|have been|is|are|was|were) (?:ja |already )?(?:removid|apagad|excluid|esquecid|memorizad|guardad|anotad|armazenad|reativad|salv|registrad|gravad|persistid|atualizad|removed|deleted|forgotten|memorized|memorised|reactivated|saved|stored|recorded|registered|persisted|updated)/
const memoryResult = /\b(?:consta|constam|esta|estao|ficou|ficaram) (?:ja )?(?:na|em) (?:minha |nossa )?memoria\b|\b(?:is|are) (?:already |now )?in (?:my |our )?memory\b/
const futurePredicate = /\b(?:sera|serao|will be)\s+(?:incluid|registrad|salv|armazenad|atualizad|guardad|memoriza|grav|esquec|remov|included|saved|stored|recorded|updated|forgotten|removed)|\b(?:atualizara|salvarei|memorizarei|registrarei|anotarei|guardarei|armazenarei|gravarei|esquecerei|removerei|vou (?:salvar|memorizar|registrar|guardar|atualizar|esquecer|gravar|remover)|i (?:will |.ll )(?:save|store|remember|record|update|forget|remove))\b/
const acknowledgmentPrefix = /^(\s*(?:(?:Entendido|Certo|Perfeito|Claro|Sure|Understood|Got it)[,!]\s+)?)(?:(?:Eu\s+)?(?:já\s+)?(?:registrei|anotei|guardei|salvei|gravei|memorizei|armazenei|retive|atualizei|vou (?:salvar|guardar|registrar)|guardarei|registrarei)(?:\s+novamente)?(?:\s+(?:a informaç[ãa]o|o fato|o dado)\s+de)?\s+que|I(?:\s+have|['’]ve)?\s+(?:noted|recorded|saved|stored|registered|memorized|memorised|updated)\s+that)\s+/iu

export const forgetRequest = (text: string): boolean => /^(?:(?:por favor|please)[, ]+)?(?:esqueca|forget)\b/.test(fold(text).trim())
const explicitLearnRequest = (text: string): boolean => {
  const query = fold(text).trim()
  return !externalTarget.test(query) && /^(?:(?:por favor|please)[, ]+)?(?:(?:quero|gostaria de|pode|could you|can you|i want you to)\s+)?(?:volte a |voltar a )?(?:guarde|guardar|salve|salvar|registre|registrar|memorize|memorizar|memorise|armazene|armazenar|remember|save|store|record|atualize|update)\b/.test(query)
}

function classify(text: string, snapshot: AnswerSnapshot): OperationClaim | undefined {
  const q = fold(text).trim(), query = fold(snapshot.query)
  if (/^(?:(?:a |the )?frase|(?:the )?phrase|(?:a |the )?expressao|(?:the )?expression)\b/.test(q)) return
  const fileOperation = externalTarget.test(q) && !persistedTarget.test(q)
  if (fileOperation) return
  const elliptical = /^(?:(?:agora|ja|already|now)\s+)?(?:(?:foi|ficou|was|has been)\s+)?(?:registrad[ao]s?|anotad[ao]s?|salv[ao]s?|memorizad[ao]s?|guardad[ao]s?|gravad[ao]s?|recorded|registered|saved|stored|noted)(?:\s*[:.!,-]|\s*$)/.test(q)
  const memoryRequest = explicitLearnRequest(query) || /\b(?:guardar|memorizar|memorize|remember|registrar|record|armazenar|store)\b/.test(query) && (snapshot.currentStatements.length > 0 || persistedTarget.test(query))
  const personalTarget = /\b(?:banco|database|prefer|gost|goal|objetivo|projeto|project|ram|tema|theme|informacao|information|details?|decisao|decision|escolha|choice|linguagem|language)\b/.test(q)
  const contextual = persistedTarget.test(q) || elliptical || forgetRequest(query) || memoryRequest || /^(?:isso|essa|esse|this|it)\b/.test(q) && snapshot.currentStatements.length > 0 || !!snapshot.operationBinding && (personalTarget || copyOfSource.test(q))
  const narratorOperation = /\b(?:registrei|anotei|guardei|salvei|memorizei|gravei|atualizei|armazenei)\b|\bi(?:\s+have|.ve)?\s+(?:recorded|registered|saved|noted|stored|memorized|updated)\b/.test(q)
  const processing = /\b(?:estou|esta|estao|is|are|am) (?:sendo |being )?(?:processando|processad[ao]|processing|processed)\b/.test(q) && /\b(?:memoria|memory|aprendizagem|learning)\b/.test(q)
  if (!(contextual || narratorOperation && (copyOfSource.test(q) || personalTarget || !!snapshot.operationBinding) || processing)) return
  // A denial of the operation itself differs from a negated fact in the proposition.
  const denial = q.match(/\b(?:nao|not|haven.t|didn.t|cannot|can.t|nunca|never)\s+(?:(?:o|a|isso|this|it|yet|been|foi|esta|is|was)\s+){0,3}/)
  const operation = q.search(signalOfMutation)
  if (denial && operation >= denial.index! && operation <= denial.index! + denial[0].length + 2) return
  const keepInMind = /\b(?:manterei|vou manter|vou levar|levarei)\s+em mente\b|\bi(?: will|.ll) keep (?:that |this |it )?in mind\b/.test(q)
  const conditionalReview = /\b(?:se|if)\b.{0,45}(?:aprovar|aprovad|confirmar|confirm|approve)|(?:apos|after).{0,25}(?:aprovacao|approval)|depende.{0,25}(?:revisao|aprovacao)/.test(q)
  if ((futurePredicate.test(q) || memoryRequest && keepInMind) && !conditionalReview) return 'promise'
  if (processing) return 'processing'
  const candidate = /\b(?:candidata?|candidate|pending review|aguardando revisao|pendente de revisao)\b/.test(q)
  if (candidate && /\b(?:criei|created|entrou|placed|awaits|aguarda|encaminhei|enviei|submitted|ficou|is now)\b/.test(q)) return 'candidate'
  if (!(elliptical || firstPersonCompletion.test(q) || passiveCompletion.test(q) || memoryResult.test(q))) return
  if (/removi|apaguei|exclui|esqueci|removid|apagad|excluid|esquecid|removed|deleted|forgot/.test(q)) return 'forget'
  if (candidate) return 'candidate'
  if (/atualizei|atualizad|updated|reativei|reativad|reactivated/.test(q)) return 'update'
  if (memoryResult.test(q) || /\b(?:esta|estao|is|are)\s+(?:salv|registrad|memorizad|guardad|armazenad|saved|stored|recorded|memorized|memorised)/.test(q)) return 'stored'
  return 'create'
}

function bound(receipt: Pick<MemoryOperationState, 'id' | 'scope' | 'project' | 'conversation_id' | 'source_message_id'> & Partial<Pick<MemoryOperationState, 'profile' | 'identity_version' | 'mode'>>, snapshot: AnswerSnapshot): boolean {
  if (receipt.scope !== snapshot.scope || receipt.project !== snapshot.project || !/^[a-f0-9]{64}$/.test(receipt.id)) return false
  const source = snapshot.sourceIdentity
  if (source && (source.conversation_id !== receipt.conversation_id || source.source_message_id !== receipt.source_message_id)) return false
  const binding = snapshot.operationBinding
  return !binding || !!source && receipt.id === binding.sourceId && receipt.profile === binding.profile && receipt.identity_version === binding.identityVersion && receipt.mode === binding.mode
}

function forgotten(receipt: MemoryDirectiveReceipt | null | undefined, snapshot: AnswerSnapshot) {
  if (!receipt || receipt.state !== 'done' || receipt.action !== 'forget' || !bound(receipt, snapshot)) return
  if (snapshot.operationBinding && (!Array.isArray(receipt.records) || receipt.had_records !== (receipt.records.length > 0) || receipt.records.some(r => r.version < 1 || r.state !== 'deleted' || r.effect !== 'delete'))) return
  return receipt
}

function learned(receipt: MemoryLearningReceipt | null | undefined, snapshot: AnswerSnapshot) {
  return receipt?.state === 'done' && receipt.action === 'learn' && snapshot.sourceIdentity && bound(receipt, snapshot) ? receipt : undefined
}

export function verifyOperationClaim(span: string, snapshot: AnswerSnapshot, mentions: (text: string, value: string) => boolean): { code?: string; completedForget?: boolean } {
  const q = fold(span)
  if (forgetRequest(snapshot.query) && /\b(?:voce esqueceu|you forgot)\b/.test(q)) return { code: 'wrong_forget_actor' }
  const claim = classify(span, snapshot)
  if (!claim) return {}
  if (claim === 'promise') return { code: 'unconfirmed_memory_promise' }
  const properties = propertyPatterns.filter(([pattern]) => pattern.test(q)).map(([,property]) => property)
  // A current, already-existing record may answer a storage status question.
  // It does not authorize saying that the current turn performed a write.
  if (claim === 'stored' && properties.length === 1 && snapshot.facts.some(f => f.scope === snapshot.scope && f.project === snapshot.project && f.validity === 'current' && f.version > 0 && f.entity === 'user' && f.property === properties[0] && (mentions(span, f.value) || mentions(snapshot.query, f.value)))) return {}
  if (claim === 'forget') {
    const receipt = forgotten(snapshot.directiveReceipt, snapshot)
    if (receipt && (receipt.had_records || !snapshot.operationBinding && forgetRequest(snapshot.query) && /\besqueci\b|\besquecid[ao]s?\b|\bforgotten\b/.test(q)) && properties.every(p => p === receipt.property)) {
      return snapshot.operationBinding ? { code: 'application_owned_memory_confirmation', completedForget: true } : { completedForget: true }
    }
  }
  if (claim === 'processing' && snapshot.sourceOperation?.state === 'processing' && bound(snapshot.sourceOperation, snapshot)) return {}
  const receipt = learned(snapshot.learningReceipt, snapshot)
  if (receipt && claim !== 'forget' && claim !== 'processing' && receipt.records.some(r => r.version > 0 &&
      (claim === 'candidate' ? r.state === 'candidate' : r.state === 'active') &&
      (claim === 'update' ? r.effect === 'update' : !snapshot.operationBinding || r.effect === 'create' || r.effect === 'update') &&
      (!properties.length || properties.includes(r.property)) &&
      (mentions(span, r.value) || mentions(snapshot.query, r.value)))) return snapshot.operationBinding ? { code: 'application_owned_memory_confirmation' } : {}
  return { code: 'unconfirmed_memory_mutation' }
}

/** Product text comes only from exact native metadata. A model's choice of
 * wording, scope or completion timing cannot select the rendered operation. */
export function renderNativeConfirmation(snapshot: AnswerSnapshot, language: 'pt' | 'en' | undefined): string | undefined {
  if (!snapshot.operationBinding) return
  const english = language === 'en' || language !== 'pt' && /^(?:please\b|forget\b|remember\b|save\b|i\b|can you\b|could you\b)/i.test(snapshot.query.trim())
  const forget = forgotten(snapshot.directiveReceipt, snapshot)
  if (forget) return forget.had_records
    ? english ? 'I removed the requested information from memory.' : 'Removi da memória a informação solicitada.'
    : english ? 'There was no matching memory to remove.' : 'Não havia memória correspondente para remover.'
  if (!explicitLearnRequest(snapshot.query)) return
  const records = learned(snapshot.learningReceipt, snapshot)?.records
  if (!records?.length) return
  if (records.every(r => r.state === 'candidate')) return english ? 'The information is pending review in memory.' : 'A informação ficou pendente de revisão na memória.'
  if (records.every(r => r.state === 'active' && r.effect === 'update')) return english ? 'I updated the requested information in memory.' : 'Atualizei na memória a informação solicitada.'
  if (records.every(r => r.state === 'active' && r.effect === 'create')) return english ? 'I saved the requested information in memory.' : 'Salvei na memória a informação solicitada.'
}

/** Remove only an operation reporting prefix and retain its literal factual
 * complement for re-verification. No paraphrase or fact synthesis. */
export function neutralOperationPrefix(span: string): string | undefined {
  const match = acknowledgmentPrefix.exec(span)
  if (!match) return
  const tail = span.slice(match[0].length)
  if (!tail || /^(?:que|that|n[aã]o\s+(?:registrei|salvei)|not\s+(?:recorded|saved))\b/iu.test(tail)) return
  return match[1] ? match[1] + tail : tail[0].toLocaleUpperCase() + tail.slice(1)
}

export function modelOperationClauses(text: string, snapshot: AnswerSnapshot): string[] {
  return answerRelationClauses(text).filter(span => !externalAssertion(span) && classify(span, snapshot) !== undefined)
}
