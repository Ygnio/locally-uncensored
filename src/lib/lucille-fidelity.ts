import { authorityClaims as evaluateAuthority, type AuthorityClaim, technologyRoles, requestedMemoryProperties, deniesCapacityGuarantee, answerRelationClauses, externalAssertion, historicalProjectWording, requestsProjectPlan, projectChoiceEntailmentQuery, deniesProjectChoiceEntailment } from './lucille-authority'
/** Bounded independent checks for typed memory assertions, never a general truth oracle.
 * No model, persistence writer, tools or permissions are reachable in this module.
 */
import type { ChatMessage } from '../api/providers/types'
import { verifyOperationClaim, type MemoryOperationBinding, type MemoryOperationState } from './lucille-operations'

export interface MemoryFact {
  id: string; version: number; entity: string; property: string; value: string
  modality: string; validity: string; scope: string; project: string; review: string
  facets: string[]; evidence: { role: string; text: string }[]
}
export interface AnswerSnapshot {
  scope: string; project: string; query: string; facts: MemoryFact[]; currentStatements: MemoryFact[]
  directiveReceipt?:MemoryDirectiveReceipt | null
  learningReceipt?:MemoryLearningReceipt | null
  relations?:MemoryQueryRelation[]
  sourceAuthority?:SourceSpeakerSummary | null
  sourceIdentity?: { conversation_id:string;source_message_id:string }
  operationBinding?: MemoryOperationBinding
  sourceOperation?: MemoryOperationState | null
  hasScopedRecords?:boolean
  refresh?: (claims?: { entity: string; property: string; value: string }[]) => Promise<{ facts: MemoryFact[]; current_statements: MemoryFact[];directive_receipt?:MemoryDirectiveReceipt | null;has_scoped_records?:boolean;learning_receipt?:MemoryLearningReceipt | null;source_operation?:MemoryOperationState | null }>
  memoryOnly?: boolean
  toolObservations?: readonly import('./verified-environment').ToolEvidence[]
  reload?: () => Promise<ChatMessage[]>
}
export interface SourceSpeakerSummary { message_author:'user';personal:number;external:number;quoted:number;uncertain:number }
export interface MemoryQueryRelation {
  query:string;actor:string;intent?:string;outside_project?:string;state:string;references:{id:string;version:number}[]
}
export interface MemoryLearningReceipt {
  id:string;action:'learn';state:'done';scope:string;project:string;conversation_id:string;source_message_id:string
  profile?:'lucille';identity_version?:number;mode?:MemoryOperationState['mode']
  records:{id:string;version:number;state:'candidate'|'active';entity:string;property:string;value:string;effect?:'create'|'update'}[]
}
export interface MemoryDirectiveReceipt {
  id:string;action:'forget';property:string;state:'done';had_records:boolean;scope:string;project:string;conversation_id:string;source_message_id:string
  profile?:'lucille';identity_version?:number;mode?:MemoryOperationState['mode']
  records?:{id:string;version:number;state:'deleted';effect:'delete'}[]
}
export interface MemoryAssertion {
  entity: string; property: string; value: string; span: string; supported: boolean; references: string[]
}
export interface FidelityViolation { code: string; span: string; critical: boolean }
export interface FidelityCheck {
  assertions: MemoryAssertion[]; violations: FidelityViolation[]; uncertain: boolean
  supportedAssertions: number; unsupportedAssertions: number; elapsedMs: number
  authority?: AuthorityClaim[]
}
export function requestedHistoricalProjectFact(
  query: string, fact: MemoryFact, relations?: MemoryQueryRelation[],
): boolean {
  return requestsProjectPlan(query, fact) || !!relations?.some(relation =>
    relation.actor === 'user' && relation.state === 'retrieved' &&
    relation.intent === (fact.modality === 'decision' ? 'choice' : 'planned') &&
    relation.references.some(ref => ref.id === fact.id && ref.version === fact.version) &&
    requestsProjectPlan(relation.query, fact))
}
const snapshots = new WeakMap<object, AnswerSnapshot>()
export function registerAnswerSnapshot(identity: ChatMessage, snapshot: AnswerSnapshot) { snapshots.set(identity, snapshot) }
export function answerSnapshot(messages: ChatMessage[]) { return messages.map(m => snapshots.get(m)).find(Boolean) }
const fold = (text: string) => text.normalize('NFD').replace(/[\u0300-\u036f]/g, '').toLowerCase()
const uncertain = (text: string) => /nao (?:tenho|ha|encontrei|possuo|consigo)|nao sei|not (?:have|enough|available)|no (?:record|information)|don.t (?:have|know)|cannot determine|can.t determine|insufficient information|not recorded|informac(?:ao|oes).*insuficiente/.test(fold(text))
const unsupportedAbsence = (text: string) => /(?:voce|you)\s+(?:(?:ainda|nunca|have|has|still)\s+)*(?:nao|not|never|haven.t)\s+(?:me\s+)?(?:decidiu|escolheu|definiu|disse|informou|forneceu|compartilhou|mencionou|relatou|declarou|decided|chosen|selected|told|provided|shared|mentioned|reported|stated)\b|(?:nao foi|nao esta|ainda nao foi)\s+(?:definid|escolhid|decidid)|(?:has|have) not been (?:defined|chosen|decided)|(?:linguagem|banco|language|database|escolha|decisao).*ainda nao.*(?:definid|decidid|escolhid)/.test(fold(text))
/** Categorical absence is a property claim, independent of initial retrieval hits.
 * An empty canonical lookup still does not establish that the user never said it.
 */
export function memoryAbsenceFields(text:string,snapshot:AnswerSnapshot):{entity:string;property:string;value:string}[] {
  if(!unsupportedAbsence(text) && !/nao recebi|haven.t received|have not received/.test(fold(text)))return []
  const explicit=requestedMemoryProperties(text)
  const properties=explicit.length ? explicit : requestedMemoryProperties(snapshot.query)
  return [...new Set(properties)].filter(property=>!['lifecycle','description'].includes(property)).map(property=>({property,value:'',entity:['database','language','lifecycle'].includes(property)?'project.'+snapshot.project:['ram_capacity','free_ram','gpu_model'].includes(property)?entityIn(text)??entityIn(snapshot.query)??'device.primary_computer':'user'}))
}
const execution = (text: string) => /\besta (?:sendo )?(?:desenvolvid|desenvolvendo|implementad|implementando|usando|utilizando|estudando)|\bja (?:usa|utiliza|implementou|iniciou|comecou|esta)\b|(?:is|are) (?:already |currently )?(?:being developed|developing|implemented|using|building|studying)|already (?:implemented|started|using)|em (?:execucao|andamento)|in progress|\bunderway\b|(?:foi |is )?(?:escrito em|written in|built with)|\b(?:usa|utiliza)(?!-)\b|\b(?:runs on|roda em)\b|(?:projeto|project).*(?: e em | is in |foi feito em|based on)|(?:desenvolvido|implemented|developed) (?:em|in|with)/.test(fold(text))
const completion = (text: string) => /\b(?:concluiu|conclui|terminei|finalizei|terminou|finalizou|concluido|concluida|completed|finished)\b|ja (?:concluido|pronto|finalizado)|(?:is|esta|foi|was) (?:already |ja )?(?:complete|finished|pronto|concluido)|parabens.{0,40}(?:concluir|conclusao)|congratulations.{0,40}(?:completing|finishing)/.test(fold(text))
const planned = (text: string) => /planej|pretend|\bplan(?:ned|ning|s)?\b|\bplanos?\b|\b(?:usaria|utilizaria)\b|intend|want to|quero |vai (?:usar|fazer|desenvolver)|will (?:use|develop)|would use/.test(fold(text))
const past = (text: string) => /\b(?:antes|anterior(?:es)?|anteriormente|antigamente|previous|previously|formerly|historical|historico)\b|used to/.test(fold(text))
function entityIn(text: string): string | undefined {
  const q = fold(text), found: { index: number; entity: string }[] = []
  for (const [pattern, entity] of [
    [/computador principal|primary computer|main (?:desktop|computer|pc)/g, 'device.primary_computer'],
    [/notebook de trabalho|work (?:laptop|notebook)|laptop de trabalho|notebook que (?:eu )?(?:uso|utilizo) para trabalhar|notebook que voce (?:usa|utiliza) para trabalhar/g, 'device.work_laptop'],
    [/outro computador|other computer|secondary (?:computer|pc)/g, 'device.other_computer'],
  ] as const) for (const m of q.matchAll(pattern)) found.push({ index: m.index, entity })
  found.sort((a, b) => b.index - a.index)
  return found[0]?.entity
}
function aliases(value: string) {
  return ({ without_sugar: ['sem acucar', 'without sugar', 'unsweetened', 'no sugar'], with_sugar: ['com acucar', 'with sugar'],
    light: ['modo claro', 'tema claro', 'light theme', 'light mode', 'claro', 'light'], dark: ['modo escuro', 'tema escuro', 'dark theme', 'dark mode', 'escuro', 'dark'],
    concise: ['curtas', 'curtos', 'concisas', 'conciso', 'concise', 'short responses'], detailed: ['detalhad', 'detailed'],
    pt: ['portugues', 'portuguese'], en: ['ingles', 'english'], es: ['espanhol', 'spanish'],
    tea: ['cha', 'tea'], coffee: ['cafe', 'coffee'], objective: ['objetiv', 'objective'],
    acoustic_guitar: ['violao acustico', 'acoustic guitar'], adventure: ['aventura', 'adventure'],
    short_trails: ['trilhas curtas', 'short trails'], glossy: ['brilhante', 'glossy'], wood: ['madeira', 'wood'],
  } as Record<string, string[]>)[value] ?? [value.replaceAll('_', ' ')]
}
function mentions(text: string, value: string) {
  const q = fold(text)
  return aliases(value).some(v => v.length < 4 ? q.split(/[^a-z0-9]+/).includes(v) : q.includes(v))
}

function historicalQuestion(query: string): boolean {
  const normalized = fold(query)
  const aside = /^antes de ([a-z]+(?:ar|er|ir)\b[^,?!.]{0,90}),\s*(me (?:lembra|diga)\b.*)$/u.exec(normalized)
  if (aside && !/\b(?:troc|mud|migr|alter|substitu|atualiz|cancel|escolh|decid|inici|implement)|\b(?:projeto|banco|linguagem|computador|memoria)\b/u.test(aside[1])) {
    return past(aside[2])
  }
  return past(query)
}

function descriptionValue(text: string) {
  return text.trim().replace(/[.!]+$/u, '').replace(/^(?:um|uma|a|an|the)\s+/u, '').replace(/\s+/gu, ' ')
}

function hypotheticalCapacityPredicate(suffix: string, terminal: boolean): boolean {
  const copula = /^\s*(?:(?:de ram|of ram)\s+)?(?:e|is|sao)\s+(?:(?:apenas|somente|so|only|just)\s+)?/u.exec(suffix)
  if (!copula) return false
  const complement = suffix.slice(copula[0].length)
  const falseHypothesis = /^(?:uma hipotese (?:falsa|incorreta)|a false hypothesis|an incorrect hypothesis)\b/u.exec(complement)
  if (!falseHypothesis) return terminal && /^uma hipotese e nao o dado confirmado\s*[.!?]?$/u.test(complement)
  const tail = complement.slice(falseHypothesis[0].length).trim()
  return terminal ? /^[.!?]?$/u.test(tail) : /^(?:[.,;!?]|$)/u.test(tail)
}

function reportedCapacityDenial(clause: string, start: number, end: number): boolean {
  const prefix = clause.slice(0, start).trim()
  const suffix = clause.slice(end)
  const attributedDenial = /^(?:(?:exatamente|correto|isso mesmo|exactly|right)[,\s]+)?(?:(?:eu|voce)\s+)?nao\s+(?:afirmei|afirmou)\s+que\s+(?:(?:o|meu|seu)\s+)?(?:computador principal|pc principal|notebook de trabalho)\s+(?:tem|tenha|tinha|possui|possua|possuia)$/u.test(prefix) ||
    /^(?:(?:exactly|right)[,\s]+)?(?:i|you)\s+(?:did not|didn't)\s+(?:say|claim|assert)\s+(?:that\s+)?(?:my|your|the)\s+(?:primary computer|work laptop)\s+(?:has|had)$/u.test(prefix)
  if (attributedDenial) return /^\s*(?:de ram|of ram)?\s*(?:[.,;!?]|$)/u.test(suffix)
  return /^(?:o valor|o numero|the value|the number)\s+(?:de|of)$/u.test(prefix) &&
    hypotheticalCapacityPredicate(suffix, false)
}

/* This restatement can only refer to a denied amount on the bound source device. */
function restatedCapacityDenial(clause: string, start: number, end: number): boolean {
  const prefix = clause.slice(0, start).trim()
  const suffix = clause.slice(end)
  if (/,\s+descartando a hipotese de$/u.test(prefix)) {
    return /^\s*(?:de ram)?\s*[.!?]?$/u.test(suffix)
  }
  const nominal = /^(?:(?:exatamente|correto|isso mesmo),\s+)?(?:o valor|o numero)\s+de$/u.test(prefix)
  return (nominal || /,\s+e\s+os$/u.test(prefix)) && hypotheticalCapacityPredicate(suffix, true)
}

export function verifyMemoryAnswer(text: string, snapshot: AnswerSnapshot): FidelityCheck {
  const began = performance.now(), assertions: MemoryAssertion[] = [], violations: FidelityViolation[] = []
  const add = (code: string, span: string, critical = true) => {
    if (!violations.some(v => v.code === code && v.span === span)) violations.push({ code, span, critical })
  }
  if (text.length > 24000) add('verification_ceiling', '', false)
  const all = [...snapshot.facts, ...snapshot.currentStatements].filter(f => f.scope === snapshot.scope && f.project === snapshot.project)
  const current = all.filter(f => f.validity !== 'historical')
  const query = fold(snapshot.query)
  const askedChoice = projectChoiceEntailmentQuery(snapshot.query)
  if (askedChoice && /^\s*(?:sim|yes)\b/u.test(fold(text))) add('affirmed_unproven_project_execution', text)
  const deniedSourceAmounts = answerRelationClauses(query).flatMap(clause =>
    [...clause.matchAll(/\b(\d+)\s*(?:gb|gib)\b/g)]
      .filter(m => reportedCapacityDenial(clause, m.index!, m.index! + m[0].length))
      .map(m => ({amount: m[1], entity: entityIn(clause)})))
  const askedProperties = requestedMemoryProperties(snapshot.query)
  const descriptionRequested = /\b(?:tema|assunto|finalidade|proposito|descricao|objetivo)\s+(?:do|deste|desse|do seu|do meu)\s+projeto\b|\b(?:topic|subject|purpose|description)\s+of\s+(?:the|this|your|my)\s+project\b/.test(query)
  const personalQuestion = /\b(?:meu|minha|meus|minhas|my|our|aquele|aquela|aqueles|aquelas|that)\b|pref|gosto|lembra|remember/.test(query) || (/\b(?:eu|i)\b/.test(query) && /tenho|tomo|bebo|gosto|escolhi|decidi|conclui|terminei|finalizei|have|drink|chose|choose|decided|like|prefer/.test(query))
  const preferenceQuestion = personalQuestion && /pref|gost|favorit|tema|theme|interface|bebida|beverage|respost|answer|repl|explain|cafe|coffee|idioma|language|jogo|game|character|personag|classe/.test(query)
  const projectQuestion = personalQuestion && /projeto|project|linguagem|language|banco|database/.test(query)
  if (/^\s*(?:sim|yes)\b/.test(fold(text)) && projectQuestion && execution(query)) {
    const values = ['typescript','javascript','python','rust','sqlite','postgresql','mysql'].filter(value => new RegExp('\\b'+value+'\\b').test(query))
    const actual=current.filter(f => f.entity === 'project.'+snapshot.project && ['in_progress','current_fact'].includes(f.modality))
    const proof = (values.length ? values.every(value=>actual.some(f=>f.value===value)) : actual.length>0)
      && !current.some(f => f.entity === 'project.'+snapshot.project && f.property === 'lifecycle' && ['completed','cancelled'].includes(f.modality))
    if (!proof) add('affirmed_unproven_project_execution',text)
  }
  if (/^\s*(?:nao|no)\s*[,!.](?:\s|$)/.test(fold(text)) && projectQuestion && execution(query) && !/significa|quer dizer|\bmeans?\b|implies/.test(query) && !current.some(f=>f.entity==='project.'+snapshot.project && f.property==='lifecycle' && ['cancelled','completed'].includes(f.modality)))add('denied_unproven_project_execution',text)
  const record = (span: string, property: string, value: string, entity?: string, negative = false, choiceOrigin = false, afterRamCorrection = false) => {
    const projectTechnology = entity === 'project.' + snapshot.project && ['language','database'].includes(property)
    const candidates = all.filter(f => (f.property === property || (property==='beverage_preference' && f.property==='favorite_beverage')) && (!entity || f.entity === entity))
    const predicate = fold(span).trim()
    const startingWithTechnology = projectTechnology && !planned(span) && /\b(?:esta|estao|is|are)\s+(?:iniciando|comecando|starting)\b[^.!?;]{1,110}\b(?:usando|utilizando|using)\b/u.test(predicate)
    const claimsExecution = execution(span) || startingWithTechnology
    const recordedProjectHistory = projectTechnology && !claimsExecution &&
      !/\b(?:agora|atualmente|ainda|now|currently|still)\b/.test(predicate) &&
      candidates.some(f => f.value === value && f.validity === 'historical' &&
        requestedHistoricalProjectFact(snapshot.query, f, snapshot.relations) &&
        (f.modality === 'decision' && (/\b(?:escolheu|decidiu|chose|decided)\b/.test(predicate) ||
          askedChoice === value && deniesProjectChoiceEntailment(span, value)) ||
         f.modality === 'planned' && /^(?:(?:no seu projeto|in your project),\s+)?(?:(?:voce|you)\s+)?(?:planejou usar|planned to use)\b/.test(predicate)))
    const historical = !afterRamCorrection && (choiceOrigin || past(span) || recordedProjectHistory || projectTechnology && historicalProjectWording(span) || (historicalQuestion(snapshot.query) && !/\b(?:agora|atualmente|now|currently|prefere|prefiro|gosta|gosto|tem|tenho|possui|usa|utiliza|is|are|have|has|prefer|prefers|like|likes|uses)\b/.test(fold(span))))
    const terminalProject = projectTechnology && current.some(f=>f.entity===entity && f.property==='lifecycle' && ['cancelled','completed'].includes(f.modality))
    if (terminalProject && !historical && planned(span) && candidates.some(f=>f.value===value && ['planned','decision'].includes(f.modality))) add('terminal_project_plan_as_current',span)
    const sourceOrigin=choiceOrigin && ['language','database'].includes(property) ? all.filter(f=>f.entity===entity && f.property===property && f.validity==='current_user_statement' && ['decision','planned'].includes(f.modality) && f.evidence.some(e=>['current_user_statement','current_source'].includes(e.role) && technologyRoles(e.text).some(r=>r.role==='origin' && r.value===value) && technologyRoles(e.text).some(r=>r.role==='destination' && r.value===f.value))) : []
    const ambiguousNegative=negative && !entity && new Set(candidates.map(f=>f.entity)).size>1
    const matching = [...sourceOrigin,...candidates.filter(f => !ambiguousNegative && (negative ? f.value !== value && f.validity !== 'historical' : (f.value === value && (f.validity !== 'historical' || historical)) || (historical && property==='ram_capacity' && /^\d+_gb$/.test(value) && f.evidence.some(e=>{
      if(e.role!=='previous_property_link_not_current_value')return false
      const amounts=[...fold(e.text).matchAll(/\b(\d+)\s*(?:gb|gib)\b/g)]
      return amounts.length===1 && amounts[0][1]+'_gb'===value
    }))))]
    const assertion = { entity: entity ?? matching[0]?.entity ?? 'unspecified', property, value, span, supported: matching.length > 0, references: matching.map(f => f.id) }
    assertions.push(assertion)
    if (!assertion.supported) { add('unsupported_typed_value', span); return }
    const earlierMention=/\b(?:mencionou|disse|informou|mentioned|said|told)\b/.test(fold(span)) && !/antes de|before the/.test(fold(span)) && matching.some(f=>f.validity==='current' && f.evidence.some(e=>e.role==='current_source') && (planned(span) || !['planned','hypothesis'].includes(f.modality)))
    // A legacy projection can support a past formulation of the plan. An
    // explicit chronology still needs the native historical relation.
    const preTerminalPlan = terminalProject && !past(span) && historicalProjectWording(span) && matching.some(f => f.modality === 'planned')
    if (historical && !choiceOrigin && !earlierMention && !preTerminalPlan && !matching.some(f => f.validity === 'historical')) {
      // Only a linked previous property source can support an older value.
      if (!matching.some(f => f.evidence.some(e => e.role === 'previous_property_link_not_current_value' && mentions(e.text, value)))) add('unsupported_historical_state', span)
    }
    if (!choiceOrigin && /decidiu|escolheu|decided|chosen|selected/.test(fold(span)) && !matching.some(f => f.modality === 'decision')) add('unproven_decision', span)
    if (matching.every(f => f.modality === 'cancelled') && !/cancel/.test(fold(span))) add('cancelled_as_current', span)
    if (!choiceOrigin && entity?.startsWith('project.') && ['language','database'].includes(property) && claimsExecution && !matching.some(f => ['in_progress', 'current_fact'].includes(f.modality))) add('plan_or_decision_as_execution', span)
    if (!historical && claimsExecution && entity === 'project.' + snapshot.project && current.some(f => f.entity === entity && f.property === 'lifecycle' && ['cancelled', 'completed'].includes(f.modality))) add('terminal_project_as_execution', span)
    if (completion(span) && !matching.some(f => f.modality === 'completed')) add('unproven_completion', span)
    if (!choiceOrigin && !historical && !planned(span) && !execution(span) && matching.every(f => ['planned', 'hypothesis'].includes(f.modality)) && !planned(snapshot.query)) add('modality_omitted', span, false)
    if (matching.every(f => f.modality === 'hypothesis') && !/talvez|pode ser|might|maybe|hipot|possib/.test(fold(span))) add('hypothesis_as_fact', span)
  }
  // A coordinated predicate owns its modality. A decision about PostgreSQL
  // cannot change the preceding current Python use (or the reverse).
  const mixedProjectHistory = projectQuestion && current.some(f =>
    f.entity === 'project.' + snapshot.project && f.property === 'lifecycle' && f.modality === 'cancelled') &&
    ['decision', 'planned'].every(modality => all.some(f =>
      f.entity === 'project.' + snapshot.project && f.validity === 'historical' && f.modality === modality &&
      requestedHistoricalProjectFact(snapshot.query, f, snapshot.relations)))
  const clauses = answerRelationClauses(text).flatMap(span => {
    if (!mixedProjectHistory || !/^\s*(?:(?:voce|you)\s+)?(?:escolheu|decidiu|chose|decided)\b/.test(fold(span))) return [span]
    // A listed past choice and past plan remain separate from a concluding
    // present-tense cancellation, whose time adverb does not modify them.
    return span.split(/,\s+(?=(?:planejou usar|planned to use)\b)|(?:,\s*|\s+)(?:e|and)\s*,?\s+(?=(?:(?:atualmente|agora|currently|now)\s*,?\s+)?(?:o|seu|the|your)\s+(?:projeto|project)\b[^.!?;]{0,50}\b(?:cancelad[ao]|cancelled|canceled)\b)/iu)
  })
  let favoriteSlotBound=false
  for (const [clauseIndex, span] of clauses.entries()) {
    const q = fold(span)
    if(externalAssertion(span))continue
    if (askedChoice && /\bvoce\s+(?:ja\s+)?(?:comecou|iniciou) a implementar\b/u.test(q) &&
        !deniesProjectChoiceEntailment(span, askedChoice)) {
      const entails = /^(?:a\s+)?escolha de \S+\s+(?:prova|comprova|significa|implica|mostra|indica)\s+que\b/u.test(q.trim())
      const independentlySupported = current.some(f => f.entity === 'project.' + snapshot.project &&
        ['in_progress', 'current_fact'].includes(f.modality) && ['language', 'lifecycle', 'project_status'].includes(f.property))
      if (entails || !independentlySupported) add('affirmed_unproven_project_execution', span)
    }
    const thirdParty=/\b(?:irmao|irma|brother|sister|colega|colleague|amigo|friend)\b/.test(q)
    if(thirdParty && !uncertain(span) && !span.trim().endsWith('?') && /\b(?:typescript|javascript|python|rust|sqlite|postgresql|mysql|\d+\s*(?:gb|gib)|prefere|prefers|gosta|likes)\b/.test(q))add('unsupported_statement_actor',span)
    // Naming a fictional or quoted source does not bind its preference to
    // the USER. A positive identity claim needs an independent personal value.
    const externalPreference = /\b(?:fictici[oa]|ficcional|narrador[a]?|personagem|fictional|narrator|character)\b/.test(q)
      && /\b(?:e (?:uma |a )?(?:preferencia sua|sua preferencia)|is (?:a preference of yours|your preference))\b/.test(q)
    if (externalPreference && !/\b(?:nao|not|talvez|maybe)\b/.test(q) && !span.trim().endsWith('?') &&
        !current.some(f => f.entity === 'user' && f.modality === 'preference' && mentions(span, f.value))) {
      add('unsupported_statement_actor', span)
    }
    // A choice (or plan) does not entail execution. Check the requested
    // technology track, even when the answer uses a project name or pronoun.
    const entails = /\b(?:escolha|decisao|plano|choice|decision|plan)\b[^.!?;]{0,80}\b(?:prova|comprova|implica|significa|proves|implies|means)\b/
    const choiceQuestion = entails.test(query) && execution(query)
    if (!span.endsWith('?') && ((entails.test(q) && execution(q) && !/\b(?:nao|not|never)\b/.test(q)) ||
        (choiceQuestion && /^(?:sim|yes)\b/.test(q.trim())))) {
      const field = /\b(?:banco|database|sgbd)\b/.test(q+' '+query) ? 'database' :
        /\b(?:linguagem|language)\b/.test(q+' '+query) ? 'language' : undefined
      const choices = current.filter(f => f.entity === 'project.'+snapshot.project && f.property === field &&
        ['decision','planned'].includes(f.modality))
      if (choices.length && !choices.every(c => current.some(f => f.entity===c.entity && f.property===c.property &&
          f.value===c.value && ['in_progress','current_fact'].includes(f.modality)))) add('affirmed_unproven_project_execution',span)
    }
    if (/\b(?:nao ha|nao temos|no|no available)\s+(?:evidencias?|registros?|dados|evidence|records?|data)\b/.test(q) &&
        past(q) && /\b(?:memoria|ram|memory)\b/.test(q)) {
      const device = entityIn(span) ?? (/\bdesktop\b/.test(q) ? 'device.primary_computer' : undefined)
      if (device && all.some(f=>f.entity===device && f.property==='ram_capacity' && f.validity==='historical'))
        add('absence_without_evidence',span)
    }
    const operation = verifyOperationClaim(span, snapshot, mentions)
    if (operation.code) add(operation.code, span)
    if (operation.completedForget && !/(?:,?\s+e|\band)\s+(?:voce|seu|sua|you|your)\b/.test(q)) continue
    const exclusiveHistory=/\bvoce (?:me )?(?:informou|forneceu|disse|contou) (?:apenas|somente|so)\b|\byou (?:have )?(?:only (?:told|provided|supplied)|(?:told|provided|supplied) (?:me )?only)\b/.test(q)
      && !/contexto (?:atual|disponivel)|current context|available context/.test(q)
    const globalAbsence=exclusiveHistory || /(?:nao ha|nao tenho|nao existem).{0,20}(?:registros? de memorias?|memorias? salv[ao]s?|registros? salvos?)|(?:there are no|i have no).{0,15}(?:saved memories|saved records)/.test(q)
    if(globalAbsence && snapshot.hasScopedRecords!==false)add('global_absence_without_evidence',span)
    if(projectQuestion && /^(?:(?:ainda\s+)?nao\s+(?:defini|decidi|escolhi)\b|eu\s+(?:defini|decidi|escolhi)\b|i\s+(?:(?:have not|haven.t|never)\s+)?(?:chose|chosen|decided|selected)\b)/.test(q.trim()) && /banco|database|linguagem|language|projeto|project/.test(q))add('wrong_decision_actor',span)
    if (span.trim().endsWith('?') && /^(?:qual|quais|como|quando|onde|por que|what|which|how|when|where|why|voce (?:pretende|quer|esta|vai|usa|usaria|usou|prefere)|do you|are you|have you|is your)\b/.test(q.trim()) && !/certo|\bne\b|\bright\b|correct|isn.t it/.test(q)) continue
    if (unsupportedAbsence(span) || (personalQuestion && /\bnao recebi (?:a |essa |esta |tal )?informacao\b|\bi (?:have not|haven.t) received (?:that |this )?information\b/.test(q))) {
      const properties = memoryAbsenceFields(span,snapshot).map(field=>field.property).filter(p=>p==='database'||p==='language')
      if (!properties.length || properties.some(property => {
        const scoped = all.filter(f => f.property === property && f.entity === 'project.' + snapshot.project)
        // One explicit unknown field cannot support a denial concerning its
        // coordinated neighbor or erase an earlier documented decision.
        return scoped.some(f => f.modality !== 'explicitly_unknown') ||
          !scoped.some(f => f.modality === 'explicitly_unknown' && f.validity !== 'historical')
      })) add('absence_without_evidence',span)
    }
    if (/poderia usar|could use|recomendo|i recommend|sugiro|i suggest/.test(q) && !/voce (?:decidiu|escolheu)|you (?:decided|chose)/.test(q)) continue
    // A scoped lack-of-evidence statement does not affirm its embedded clause.
    // Keep independent assertions and "no doubt" statements outside this rule.
    const epistemic = /^(?:nao ha|nao tenho|nao encontrei|nao possuo).{0,35}(?:registros?|informac(?:ao|oes)|evidencias?).{0,40}(?:que (?:confirm|indiqu)|indicando (?:que|se)|confirmando (?:que|se)|sobre se|para (?:confirmar|afirmar))/.test(q.trim()) || /^(?:i (?:do not|don.t|cannot|can.t)|there (?:is|are) no).{0,45}(?:records?|information|evidence).{0,40}(?:confirm|indicat|whether|to say|to determine)/.test(q.trim())
    if (epistemic && !/duvid|doubt|certeza|certain|,\s*(?:e |and |seu |your |o projeto|the project|voce |you )/.test(q)) continue
    if (uncertain(span) && !execution(span) && !completion(span)) continue
    // Naming an external project to explain its access boundary is not a
    // factual claim about that project. Keep the exemption sentence-bounded.
    const scopedLimitation = q.trim().match(/^(?:para o projeto|for (?:the )?project)\s+([a-z0-9_-]+),\s+(?:esta conversa nao tem acesso ao registro autorizado|this conversation cannot access (?:its|the) authorized record)[.!]?$/u)
    if (scopedLimitation && scopedLimitation[1] !== fold(snapshot.project) &&
      query.split(/[^a-z0-9_-]+/u).includes(scopedLimitation[1])) continue
    const subject = /\b(?:voce|seus?|suas?|you|your)\b|projeto|project/.test(q)
    const description = q.trim().match(/^(?:(?:o|a)\s+)?(?:tema|assunto|finalidade|proposito|descricao|objetivo)\s+(?:do|deste|desse|do seu)\s+projeto(?:\s+([\p{L}\p{N}_.-]+))?\s+(?:e|era)\s+(.+)$|^(?:the\s+)?(?:topic|subject|purpose|description)\s+of\s+(?:the|this|your)\s+project(?:\s+([\p{L}\p{N}_.-]+))?\s+(?:is|was)\s+(.+)$/u)
    const projectDescription = descriptionRequested
      ? q.trim().match(/^(?:(?:o seu|o|seu)\s+projeto(?:\s+([\p{L}\p{N}_.-]+))?\s+era|(?:your|the)\s+project(?:\s+([\p{L}\p{N}_.-]+))?\s+was)\s+(.+)$/u)
      : null
    const describedPurpose = /\b(?:finalidade|proposito|objetivo|purpose)\b/u.test(query) && /\b(?:projeto|project)\b/u.test(query)
      ? q.trim().match(/^(?:voce (?:deve|pode) descrever (?:a finalidade|o proposito|o objetivo) de|you (?:should|can) describe the purpose (?:of|as))\s+(.+)$/u)
      : null
    const namedPurpose = q.trim().match(/^(?:a finalidade|o objetivo|o proposito) (?:do|da) ([\p{L}\p{N}_-]+) (?:e|era) (.+)$|^(?:o projeto|projeto) ([\p{L}\p{N}_-]+) serve para (.+)$/u)
    const boundPurpose = namedPurpose && (namedPurpose[1] ?? namedPurpose[3]) === fold(snapshot.project)
    if (boundPurpose || describedPurpose || (description || projectDescription) && (descriptionRequested || current.some(f => f.property === 'description' && f.entity === 'project.' + snapshot.project))) {
      const entity = 'project.' + snapshot.project
      const named = description ? description[1] ?? description[3] : projectDescription?.[1] ?? projectDescription?.[2]
      const value = descriptionValue((boundPurpose ? namedPurpose[2] ?? namedPurpose[4] : undefined) ?? describedPurpose?.[1] ?? (description ? description[2] ?? description[4] : projectDescription![3]))
      // Compare the entire claimed purpose against the literal source payload.
      // A shared substring cannot ground additions or a different project.
      const proof = current.filter(f => (!named || named === fold(snapshot.project)) &&
        f.entity === entity && f.property === 'description' && f.modality === 'project_description' &&
        (f.validity === 'current' && f.review === 'reviewed' || f.validity === 'current_user_statement') && f.evidence.some(e => {
          if (e.role !== (f.validity === 'current_user_statement' ? 'current_user_statement' : 'current_source')) return false
          const source = fold(e.text).trim().match(/^(?:estou comecando um projeto de|iniciei um projeto de|i am starting a project(?: about| on| for| to)?|(?:(?:o|meu) )?projeto continua sendo|(?:my|the) project remains)\s+(.+)$/u)
          if (source) return value === descriptionValue(source[1])
          const namedSource = fold(e.text).trim().match(/^(?:(?:comecei|iniciei|criei) o projeto ([\p{L}\p{N}_-]+) para|i (?:started|created) project ([\p{L}\p{N}_-]+) to) (.+)$/u)
          return namedSource && (namedSource[1] ?? namedSource[2]) === fold(snapshot.project)
            && value === descriptionValue(namedSource[3])
        }))
      assertions.push({entity,property:'description',value,span,supported:proof.length>0,references:proof.map(f=>f.id)})
      if (!proof.length) add('unsupported_typed_value',span)
    }
    // A general causal explanation about performance does not assert a
    // measured outcome on the USER's machine.
    const generalFactors = /^(?:outros fatores|fatores como|other factors)\b/.test(q.trim()) &&
      /\b(?:influenciam|afetam|influence|affect)\b/.test(q) &&
      !/\b(?:seu|sua|voce|your|you|garante|guarantees|sempre|always)\b/.test(q)
    if (!generalFactors && !deniesCapacityGuarantee(span) && (subject || personalQuestion) && /ram|memoria|memory|computador|computer|notebook|laptop/.test(fold(text)+' '+query) && /fluidez|(?:experiencia|desempenho|multitarefas?|navegacao|produtividade|desenvolvimento|execucao|tarefas?).{0,25}\bfluid[ao]s?\b|sem (?:grandes )?gargalos|sem travamentos|mais estavel|maior estabilidade|no bottlenecks|smooth performance|more stable|will run smoothly/.test(q) && !/\b(?:pode|poderia|talvez|depende|estimativa|might|could|may|depends|estimate)\b/.test(q)) {
      const source=current.some(f=>f.evidence.some(e=>e.role!=='previous_property_link_not_current_value' && fold(e.text).includes(q.trim())))
      if(!source)add('unproven_personal_performance',span,false)
    }
    if (subject && /projeto|project/.test(q)) {
      const projectFacts = current.filter(f => f.entity === 'project.' + snapshot.project)
      if(/nao (?:esta (?:sendo )?(?:desenvolvid|implementad|usando)|usa\b|comecou\b)|(?:is not|isn.t|has not|hasn.t).{0,12}(?:being developed|implemented|using|started)/.test(q) && !projectFacts.some(f=>f.property==='lifecycle' && ['cancelled','completed'].includes(f.modality)))add('unproven_negative_execution',span)
      if (/(?:cancelou|cancelei|cancelled|canceled)\s+(?:(?:o|seu|meu|the|your|my)\s+)*(?:projeto|project)\b/.test(q) && !/nao.{0,20}cancel|not.{0,20}cancel/.test(q)) record(span,'lifecycle','cancelled','project.'+snapshot.project)
      if (/em execucao|em andamento|in progress|underway|sendo desenvolvido|being developed/.test(q) && !projectFacts.some(f => f.modality === 'in_progress')) add('unproven_project_progress', span)
      if (/(?:projeto|project).{0,70}(?:esta|foi|is|was) (?:atualmente |currently )?(?:cancelad|cancelled|canceled)/.test(q) && !/nao.*cancel|not.*cancel/.test(q)) {
        const proof=projectFacts.filter(f=>f.property==='lifecycle' && f.modality==='cancelled')
        assertions.push({entity:'project.'+snapshot.project,property:'lifecycle',value:'cancelled',span,supported:proof.length>0,references:proof.map(f=>f.id)})
        if(!proof.length)add('unproven_project_cancellation',span)
      }
      if (completion(span) && !projectFacts.some(f => ['lifecycle', 'project_status'].includes(f.property) && f.modality === 'completed')) add('unproven_project_completion', span)
    }
    if (subject) {
      for (const m of q.matchAll(/\b(?:projeto|project)\s+(?:chamado|denominado|called|named)\s+["`']?([a-z0-9_.-]+)/g)) {
        if(m[1].replace(/\.+$/,'')!==fold(snapshot.project))add('unsupported_project_entity',span)
      }
      for (const m of q.matchAll(/\b(?:projeto|project)\s+["`']([a-z0-9_.-]+)["`']/g)) {
        if (m[1] !== fold(snapshot.project)) add('unsupported_project_entity',span)
      }
      for (const m of (snapshot.project.includes('-') ? span.replace(new RegExp(snapshot.project.split('-').map(v=>v.replace(/[.*+?^${}()|[\]\\]/g,'\\$&')).join('[ -]'), 'gi'),snapshot.project) : span).matchAll(/\b(?:projeto|project)\s+["`']?([\p{L}\p{N}_.-]+)/giu)) {
        const rawName=m[1], foldedName=fold(rawName)
        const name = foldedName === fold(snapshot.project) ? foldedName : foldedName.replace(/\.+$/, '')
        const identitySyntax=/^[\p{Lu}]/u.test(rawName) || /[0-9_.-]/.test(name) || /^\s+(?:esta|foi|era|e|usa|utiliza|tem|is|was|uses|has)\b/.test(fold(span.slice(m.index!+m[0].length)))
        if (!['de', 'do', 'da', 'que', 'that', 'which', 'sobre', 'about', 'sem', 'com', 'para', 'with', 'without', 'esta', 'esteja', 'e', 'is', 'was', 'foi', 'sera', 'vai', 'ainda', 'has', 'tem', 'continua', 'planeja', 'agora', 'atualmente', 'recentemente', 'recently', 'anteriormente', 'previously', 'atual', 'current', 'ou', 'or', 'hoje', 'usa', 'usava', 'utiliza', 'utilizava', 'possui', 'segue', 'parece', 'estava', 'estara', 'ja', 'nao', 'aqui', 'precisa', 'exige', 'requer', 'apresenta', 'will', 'uses', 'needs', 'requires', 'currently', 'now', 'already', 'still', 'seems', 'would', 'could', 'should', 'may', 'might', 'can'].includes(name) && name !== fold(snapshot.project) && identitySyntax) add('unsupported_project_entity', span)
      }
    }
    // A bare plan can inherit a project from a verified technology claim.
    // Crossing fields requires explicit coordination and a native plan relation.
    const impliedPlan = q.trim().match(/^(?:(?:para o futuro|futuramente|in the future)[,\s]+)?(?:(?:e|and)\s+)?(?:(?:esta planejad[ao]|planeja|pretende) (?:usar|utilizar)|is planned to use|plans to use|intends to use)\s+(?:(?:a linguagem|o banco(?: de dados)?|the language|the database)\s+)?[a-z0-9]+(?:\s+(?:como|as)\s+(?:(?:o|the)\s+)?(banco de dados|database))?(?:\s+(?:no futuro|futuramente|in the future))?[.!]?$/u)
    if (subject || projectQuestion || impliedPlan) {
      for (const value of ['typescript', 'javascript', 'python', 'rust', 'sqlite', 'postgresql', 'mysql']) {
        if (!new RegExp('\\b' + value + '\\b').test(q)) continue
        const property = ['sqlite', 'postgresql', 'mysql'].includes(value) ? 'database' : 'language'
        if (impliedPlan?.[1] && property !== 'database') {
          add('unsupported_technology_role', span)
          continue
        }
        if (!subject && !projectQuestion) {
          const earlier = assertions.filter(a => a.span !== span && a.supported &&
            a.entity === 'project.' + snapshot.project && !violations.some(v => v.span === a.span))
          const previousClause = clauses[clauseIndex - 1]
          const coordinated = /^(?:e|and)\s+/u.test(q.trim()) && previousClause && !/[.!?;]\s*$/u.test(previousClause) &&
            earlier.some(a => a.span === previousClause && ['language','database'].includes(a.property)) &&
            snapshot.relations?.some(r => r.actor === 'user' && r.intent === 'planned' && current.some(f =>
              f.entity === 'project.' + snapshot.project && f.property === property && f.modality === 'planned' &&
              r.references.some(ref => ref.id === f.id && ref.version === f.version)))
          if (!askedProperties.includes(property) || (!earlier.some(a => a.property === property) && !coordinated)) continue
        }
        if (/\bprojeto|\bproject|linguagem|programming language|banco de dados|database/.test(q) || projectQuestion || (subject && personalQuestion && /escolh|decid|guardar|armazen|persist|store|stack/.test(query)) || (current.some(f => f.property === property && f.entity === 'project.' + snapshot.project) && !/objetivo|goal|aprender|learn|estud|study/.test(q))) {
          const choiceOrigin=technologyRoles(span).some(role=>role.value===value && role.role==='origin')
          record(span, property, value, 'project.' + snapshot.project, false, choiceOrigin)
        }
      }
    }
    if (/dobr|triplic|doubled|tripled|twice|three times/.test(q) && /memoria|memory|ram|computador|computer|notebook|laptop/.test(q)) {
      const ratio=/triplic|tripled|three times/.test(q) ? 3 : 2
      const entity=entityIn(q) ?? entityIn(query)
      const proof=current.some(f => f.property==='ram_capacity' && (!entity || f.entity===entity) && f.value.endsWith('_gb') &&
        f.evidence.some(e => {
          if (e.role!=='previous_property_link_not_current_value') return false
          const values=[...fold(e.text).matchAll(/\b(\d+)\s*(?:gb|gib)\b/g)]
          if (values.length!==1) return false
          const previous=Number(values[0][1]), now=Number(f.value.replace('_gb',''))
          return /mais que|more than/.test(q) ? now>previous*ratio : now===previous*ratio
        }))
      assertions.push({entity:entity ?? 'unspecified',property:'ram_capacity_ratio',value:String(ratio),span,supported:proof,references:proof ? current.filter(f=>f.property==='ram_capacity').map(f=>f.id) : []})
      if (!proof) add('unsupported_hardware_ratio',span)
    }
    const correctionBoundary = /\s+(?:e|and)\s*,\s*(?:depois da correcao|apos a correcao|after the correction)\s*,\s*/u.exec(q)
    const correctionEnd = correctionBoundary ? correctionBoundary.index + correctionBoundary[0].length : Infinity
    const priorRamClause = correctionBoundary ? q.slice(0, correctionBoundary.index) : ''
    for (const match of q.matchAll(/\b(\d+)\s*(?:gb|gib)\b/g)) {
      const amountStart = match.index!
      const amountEnd = amountStart + match[0].length
      // The same original USER denial can be acknowledged. No other number in
      // this response borrows that negative or hypothetical modality.
      if (deniedSourceAmounts.some(source => {
        if (source.amount !== match[1]) return false
        const entity = entityIn(q.slice(0, amountStart))
        if (source.entity && entity && source.entity !== entity) return false
        return reportedCapacityDenial(q, amountStart, amountEnd) ||
          !!source.entity && restatedCapacityDenial(q, amountStart, amountEnd)
      })) continue
      const afterRamCorrection = amountStart >= correctionEnd && past(priorRamClause) &&
        /\b\d+\s*(?:gb|gib)\b/u.test(priorRamClause)

      if (!subject && !personalQuestion && !/ram|computador|computer|laptop|notebook/.test(q)
        && !(current.some(f=>f.property==='ram_capacity' && f.validity==='current_user_statement') && /\b\d+\s*(?:gb|gib)\b/.test(query))) continue
      // A change amount is a relation, not the device's new total capacity.
      // Derive it only from one typed device and a literal linked predecessor
      // or the native-validated current USER correction, never model prose.
      const deltaPrefix=q.slice(Math.max(0,match.index!-90),match.index)
      if (/\b(?:aumento|expansao|acrescimo|increase|expansion|reducao|decrease|reduction)\s+(?:de|em|of|by)\s*$|\b(?:aumentou|aumentei|ampliou|reduziu|reduzi|increased|decreased|expanded)\b[^,;:.!?]{0,55}\b(?:em|by)\s*$/.test(deltaPrefix)) {
        const entity=entityIn(span) ?? entityIn(query), decrease=/reducao|reduziu|reduzi|decrease|reduction/.test(deltaPrefix)
        const proofs=current.filter(f=>f.property==='ram_capacity' && f.entity.startsWith('device.') && f.entity!=='device.unspecified' && (!entity || f.entity===entity) && f.modality==='current_fact' && /^\d+_gb$/.test(f.value)).filter(f=>{
          const total=Number(f.value.replace('_gb',''))
          return f.evidence.some(e=>{
            const source=fold(e.text)
            const values=[...source.matchAll(/\b(\d+)\s*(?:gb|gib)\b/g)].map(v=>Number(v[1]))
            const old=e.role==='previous_property_link_not_current_value' && values.length===1 ? values[0] :
              ['current_user_statement','current_source'].includes(e.role) && f.validity==='current_user_statement' && /nao tenho mais|no longer have/.test(source) && /agora|now/.test(source) && values.length===2 && values[1]===total ? values[0] : undefined
            return old!==undefined && (decrease ? old-total : total-old)===Number(match[1])
          })
        })
        const unique=new Set(proofs.map(f=>f.entity+':'+f.value))
        const supported=unique.size===1
        assertions.push({entity:entity ?? (supported?proofs[0].entity:'unspecified'),property:'ram_capacity_delta',value:match[1]+'_gb',span,supported,references:supported?proofs.map(f=>f.id):[]})
        if(!supported)add('unsupported_typed_value',span)
        continue
      }
      // Bind availability and negation only to this amount's own predicate.
      const quantityText = afterRamCorrection ? q.slice(correctionEnd) : q
      const quantityIndex = afterRamCorrection ? amountStart - correctionEnd : amountStart
      const quantityClause = quantityText.slice(quantityText.lastIndexOf(',',quantityIndex)+1, quantityText.indexOf(',',quantityIndex)<0 ? undefined : quantityText.indexOf(',',quantityIndex))
      const property = /livre|disponivel|available|free/.test(quantityClause) ? 'free_ram' : /\bram\b|memoria|memory/.test(q + ' ' + query) ? 'ram_capacity' : 'capacity_update'
      const priorEntity = entityIn(priorRamClause)
      const localSubject = quantityText.slice(0,quantityIndex)
      const localEntity = entityIn(localSubject)
      if (afterRamCorrection && (!priorEntity || (localEntity ? localEntity !== priorEntity : !/^(?:ele|ela|it)\s/u.test(localSubject)))) {
        add('unsupported_ram_correction_device',span)
        continue
      }
      const entity = localEntity ?? (afterRamCorrection ? priorEntity : entityIn(query))
      record(span, property, match[1] + '_gb', entity, /nao (?:tem|possui|sao|e)|does not have|not \d|\bnao (?:uma |a |sua )?capacidade (?:[a-z]+\s+){0,3}(?:de|como)\s*$|\bnot (?:a |your )?(?:total )?capacity of\s*$/.test(localSubject), false, afterRamCorrection)
    }
    const plainPreference=/\b(?:prefere|prefiro|prefers|prefer|gosta de|like|likes|preferencia por|preference for|gosto por|liking for)\b/.test(q)
    const drinkLead=/^(?:(?:e|is|it is|it's)\s+)?(?:cha|tea|cafe|coffee)\b/.test(q.trim())
    const genericDrinkQuestion=/pref|gost|likes/.test(query) && /bebida|beverage|drink|cafe|coffee/.test(query) && !/favorit|favourit|acucar|sugar/.test(query)
    if((subject && plainPreference || drinkLead && genericDrinkQuestion) && !/favorit|favourit|acucar|sugar/.test(q)) {
      for(const value of ['coffee','tea'])if(mentions(span,value))record(span,'beverage_preference',value,'user')
    }
    const unboundFavoriteAnswer=!favoriteSlotBound && /favorit|favourit|bebida|beverage/.test(query) && /^(?:(?:e|is|it is|it's)\s+)?(?:cha|tea|cafe|coffee)\b/.test(q.trim())
    // A feature of the game is not a statement about its player's preferences.
    // Keep short elliptical answers eligible, but do not borrow USER ownership
    // from the question when a descriptive predicate has its own subject.
    const impersonalFeature=!subject && /\b(?:permite|permitem|oferece|oferecem|possibilita|possibilitam|allows?|enables?|offers?|provides?)\b/.test(q)
    if (unboundFavoriteAnswer || (subject && /pref|gost|likes|favorit|adora|curt|toma|bebe|drink|idioma|lingua|tema|theme|comunica|communication/.test(q)) || (preferenceQuestion && !impersonalFeature && span.length<160 && !/\b(?:pode|poderia|opcao|alternativa|could|might|option|recommend|sugiro|depende)\b/.test(q))) {
      for (const property of ['coffee_sugar', 'interface_theme', 'communication_style', 'response_language', 'favorite_beverage', 'communication_focus']) {
        const domain = q + ' ' + query
        if (property === 'coffee_sugar' && !/cafe|coffee|acucar|sugar/.test(domain)) continue
        if (property === 'interface_theme' && !/interface|tema|theme|modo|display|aparencia/.test(domain)) continue
        if (['communication_style', 'communication_focus'].includes(property) && !/respost|explica|communicat|comunic|answer|repl|explan/.test(domain)) continue
        if (property === 'response_language' && !/idioma|lingua|convers|speak|falar|respond in|language/.test(domain)) continue
        if (property === 'favorite_beverage' && !/bebida|beverage|drink|cha|tea/.test(domain)) continue
        const deniedLiking=/nao (?:gosta|curte)|don.t like|does not like|dislikes/.test(q)
        if (deniedLiking) {
          if (property==='coffee_sugar' && /cafe|coffee/.test(q) && !/acucar|sugar/.test(q)) record(span,property,'dislikes_coffee','user')
          else if ((property==='response_language' && /portugues|portuguese|ingles|english|espanhol|spanish/.test(q)) || (property==='favorite_beverage' && /cha|tea/.test(q)) || (property==='interface_theme' && /claro|escuro|light|dark/.test(q)) || (property==='coffee_sugar' && /acucar|sugar/.test(q)) || (property==='communication_style' && /curt|concise|short|detalhad|detailed/.test(q)) || (property==='communication_focus' && /objetiv|objective/.test(q))) add('unproven_preference_dislike',span)
          continue
        }
        const negatedChoice=/nao (?:prefere|e)|not (?:your |the )?(?:preferred|favorite)|does not prefer|don.t prefer/.test(q)
        const values = property === 'coffee_sugar' ? ['with_sugar', 'without_sugar'] : property === 'interface_theme' ? ['light', 'dark'] : property === 'communication_style' ? ['concise', 'detailed'] : property === 'response_language' ? ['pt', 'en', 'es'] : property === 'favorite_beverage' ? ['tea', 'coffee'] : ['objective']
        if (property === 'favorite_beverage') {
          if(genericDrinkQuestion && !/favorit|favourit/.test(q))continue
          const leadDrink=/^(?:(?:e|is|it is|it's)\s+)?(?:cha|tea|cafe|coffee)\b/.test(q.trim())
          const choiceVerb=/\b(?:prefere|prefiro|prefers|prefer)\b/.test(q)
          if (!/favorit|favourit/.test(q) && !(!favoriteSlotBound && (leadDrink || choiceVerb) && /favorit|favourit|bebida|beverage/.test(query))) continue
          const positions=[...new Set(values)].flatMap(value=>aliases(value).flatMap(alias=>[...q.matchAll(new RegExp('\\b'+alias+'\\b','g'))].map(m=>({value,index:m.index}))))
            .filter(m=>!/\b(?:nao|not|never)\s+(?:o |a |the )?$/.test(q.slice(Math.max(0,m.index-16),m.index)))
            .sort((a,b)=>a.index-b.index)
          const binding=q.match(/(?:prefere|prefers|prefer)\s+|(?:favorit[ao]|favorite)\s+(?:e|is)\s+/)
          const bound=binding ? positions.find(m=>m.index>binding.index!+binding[0].length-1) : undefined
          const target=bound?.value ?? positions[0]?.value
          if(target)favoriteSlotBound=true
          if (/favoritas|favorites|both|ambas/.test(q)) {
            for (const value of new Set(positions.map(m=>m.value))) record(span,property,value,'user',negatedChoice)
          } else if (target) record(span,property,target,'user',negatedChoice)
        } else for (const value of new Set(values)) if (mentions(span, value)) record(span, property, value, 'user',negatedChoice)
      }
      const denied = /nao (?:gosta|prefere|curte)|don.t (?:like|prefer)|does not (?:like|prefer)/.test(q)
      if (/rpg/.test(q) && /tatic|tactic/.test(q)) record(span, 'games_tactics', denied ? 'negative' : 'positive', 'user')
      if (/estrategias? por turnos|turn.based strateg(?:y|ies)/.test(q)) record(span, 'games_turn_strategy', denied ? 'negative' : 'positive', 'user')
      if (/criacao de personagens|criar personagens|character creation|creating characters/.test(q)) record(span, 'games_character_creation', denied ? 'negative' : 'positive', 'user')
      if (/escolh.*classe|choos.*class|class selection/.test(q)) record(span, 'games_class_selection', denied ? 'negative' : 'positive', 'user')
      if (/interface|tela|screen/.test(q) && /poluid|clutter|carregad/.test(q)) record(span, 'interface_clutter', /detest|odeia|dislike|nao (?:gosta|curte|prefere)/.test(q) ? 'negative' : 'positive', 'user')
      for (const [property, value] of [[/tocar|play/.test(q) ? 'hobby' : 'music_instrument', 'acoustic_guitar'], ['book_genre', 'adventure'], ['outing', 'short_trails'], ['photo_paper', 'glossy'], ['shelf_material', 'wood']]) {
        const domain=q+' '+query
        if (property==='book_genre' && !/romance|livro|book|novel/.test(domain)) continue
        if (property==='photo_paper' && !/foto|photo/.test(domain)) continue
        if (property==='shelf_material' && !/estante|prateleir|shelf/.test(domain)) continue
        if (mentions(span, value)) {
          if (denied) add('preference_polarity_mismatch', span)
          record(span, property, value, 'user')
        }
      }
    }
    const goalDomain=/\b(?:objetivos?|metas?|goals?)\b/
    const goalQuestion=personalQuestion && goalDomain.test(query) && (/\?/.test(query) || /\b(?:qual|quais|what|which|lembra|recorda|recall|remember)\b/.test(query))
    if ((subject && goalDomain.test(q) && !/\b(?:projeto|project)\b/.test(q)) || (goalQuestion && !projectQuestion && span.length<200)) {
      const stop=new Set(['voce','seu','sua','seus','suas','your','you','the','was','foi','era','objetivo','goal','meta','concluiu','conclui','concluido','completed','finished','concluir','concluido','concluida','terminou','finalizou','apenas','seu','meu','minha','esta','seu','que','para','uma','um','de','do','da','em','of','to','pessoais','personal','atual','atualmente','current','currently'])
      const tokens=(text:string)=>fold(text).split(/[^a-z0-9]+/).filter(t=>t.length>2 && !stop.has(t)).map(t=>({aprender:'learn',learning:'learn',portugues:'portuguese',ingles:'english',espanhol:'spanish',japones:'japanese',frances:'french',italiano:'italian',alemao:'german'} as Record<string,string>)[t] ?? t)
      const goalAnchor=fold(span).match(goalDomain)
      const target=tokens(goalAnchor ? span.slice(goalAnchor.index!+goalAnchor[0].length) : span), requiredCompletion=completion(span) || (goalQuestion && completion(query))
      const matching=all.filter(f=>f.property.startsWith('goal.') && f.validity!=='historical' && (!requiredCompletion || f.modality==='completed')).filter(f=>{
        const source=tokens(f.value+' '+f.evidence.filter(e=>e.role!=='previous_property_link_not_current_value').map(e=>e.text).join(' '))
        return target.every(token=>source.includes(token))
      })
      assertions.push({entity:'user',property:'goal_target',value:target.join('_'),span,supported:matching.length>0,references:matching.map(f=>f.id)})
      if (!matching.length) add(requiredCompletion ? 'unproven_goal_completion' : 'unsupported_goal_target',span)
      if (execution(span) && matching.every(f=>!['current_fact','in_progress'].includes(f.modality))) add('plan_or_decision_as_execution',span)
    }
    if (subject && /\b(?:20\d\d)\b/.test(q) && /projeto|project|computador|computer/.test(q) && !current.some(f => mentions(span, f.value) && /date|data|year/.test(f.property))) add('unsupported_personal_date', span)
  }
  const foreignActor = /\b(?:irmao|irma|brother|sister|colega|colleague|amigo|friend)\b/.test(query)
  const askedEntity = entityIn(snapshot.query)
  const generalProjectStatus = projectQuestion && /como esta|como anda|how is|status of/.test(query) && !/linguagem|language|banco|database/.test(query)
  const available = all.filter(f => f.validity !== 'historical' || f.entity === 'project.' + snapshot.project && requestedHistoricalProjectFact(snapshot.query,f,snapshot.relations))
  const relevantKnown = (!foreignActor || snapshot.relations?.some(r=>r.actor==='user')) && available.some(f =>
    (!askedEntity || f.entity === askedEntity) &&
    (generalProjectStatus ? f.entity === 'project.' + snapshot.project : askedProperties.length ? askedProperties.includes(f.property) :
      projectQuestion ? f.entity === 'project.' + snapshot.project :
      preferenceQuestion ? f.entity === 'user' && f.modality === 'preference' : false))
  if (personalQuestion && relevantKnown && uncertain(text) && assertions.length === 0) add('known_information_not_used', text, false)
  for(const relation of snapshot.relations??[]) {
    if(relation.actor!=='user')continue
    if (relation.outside_project && relation.state === 'unavailable') {
      const answer = fold(text)
      const boundary = /(?:fora|outside|beyond).{0,48}(?:escopo|scope|acesso|access|autoriz)|(?:\bsem|\bnao (?:tenho|tem|temos|possuo)|\bno(?=\s+access)|\bwithout|\bcannot|\bcan't|\bnot authorized).{0,32}(?:acesso|acessar|access|autoriz|permission)|(?:acesso|access|autoriz|escopo|scope|consultar).{0,45}(?:apenas|somente|restrit|limit|only)|(?:apenas|somente|so|only).{0,45}(?:acesso|acessar|consultar|access|autoriz)/u
      const project = fold(relation.outside_project)
      const onlyExternal = snapshot.relations?.filter(r => r.outside_project).length === 1
      const explainsBoundary = answer.split(/[.!?\n]+/u).some(sentence => {
        const identifiesProject = sentence.split(/[^\p{L}\p{N}_-]+/u).includes(project)
          || (onlyExternal && /\b(?:outro projeto|other project)\b/u.test(sentence))
        return identifiesProject && boundary.test(sentence)
      })
      if (!explainsBoundary) {
        add('missing_project_access_boundary', text, false)
      }
    }
    const known=available.filter(f=>relation.references.some(r=>r.id===f.id && r.version===f.version))
    if(known.length && !assertions.some(a=>a.supported && a.references.some(id=>known.some(f=>f.id===id))))add('known_relation_not_used',text,false)
  }
  // On a direct technology update, advice or generic praise does not preserve
  // whether the USER described a plan or work already underway.
  const directTechnology = current.filter(f => f.validity === 'current_user_statement' &&
    f.entity === 'project.' + snapshot.project && ['language', 'database'].includes(f.property) &&
    ['planned', 'in_progress'].includes(f.modality) && f.evidence.some(e =>
      e.role === 'current_user_statement' && descriptionValue(fold(e.text)) === descriptionValue(query)))
  for (const fact of directTechnology) {
    const predicate = fact.modality === 'planned' ? planned : execution
    const explicit = assertions.filter(a => a.supported && a.references.includes(fact.id) &&
      a.property === fact.property && a.value === fact.value)
    const pronominalPlan = fact.modality === 'planned' && explicit.some(a => {
      // A following sentence can refer back to one supported technology.
      // The field and any explicit project still have to match the source.
      if (assertions.some(other => other.span === a.span && other.property === fact.property && other.value !== fact.value)) return false
      const following = clauses[clauses.indexOf(a.span) + 1] ?? ''
      const referred = fold(following).trim().match(/^(?:voce\s+)?(?:planeja|pretende)\s+(?:usa|utiliza)-l[oa]\s+como\s+(o banco(?: de dados)?|a linguagem(?: de programacao)?)(?:\s+do\s+(?:seu\s+)?projeto(?:\s+([a-z0-9_-]+))?)?[.!]?$/u)
      return !!referred && (!referred[2] || referred[2] === fold(snapshot.project)) &&
        (referred[1].startsWith('o banco') ? 'database' : 'language') === fact.property
    })
    if (!explicit.some(a => predicate(a.span)) && !pronominalPlan) add('source_technology_modality_omitted', text, false)
  }
  const authority=evaluateAuthority(text,snapshot)
  for (const claim of authority) if (!['supported','historical','hypothetical','non_memory'].includes(claim.verdict)) add('authority_'+claim.property,claim.span,claim.critical)
  return { assertions, violations, authority, uncertain: uncertain(text), supportedAssertions: assertions.filter(c => c.supported).length,
    unsupportedAssertions: assertions.filter(c => !c.supported).length, elapsedMs: performance.now() - began }
}
