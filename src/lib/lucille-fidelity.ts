/** Bounded independent checks for typed memory assertions, never a general truth oracle.
 * No model, persistence writer, tools or permissions are reachable in this module.
 */
import type { ChatMessage } from '../api/providers/types'

export interface MemoryFact {
  id: string; version: number; entity: string; property: string; value: string
  modality: string; validity: string; scope: string; project: string; review: string
  facets: string[]; evidence: { role: string; text: string }[]
}
export interface AnswerSnapshot {
  scope: string; project: string; query: string; facts: MemoryFact[]; currentStatements: MemoryFact[]
  refresh?: (claims?: { entity: string; property: string; value: string }[]) => Promise<{ facts: MemoryFact[]; current_statements: MemoryFact[] }>
  reload?: () => Promise<ChatMessage[]>
}
export interface MemoryAssertion {
  entity: string; property: string; value: string; span: string; supported: boolean; references: string[]
}
export interface FidelityViolation { code: string; span: string; critical: boolean }
export interface FidelityCheck {
  assertions: MemoryAssertion[]; violations: FidelityViolation[]; uncertain: boolean
  supportedAssertions: number; unsupportedAssertions: number; elapsedMs: number
}
const snapshots = new WeakMap<object, AnswerSnapshot>()
export function registerAnswerSnapshot(identity: ChatMessage, snapshot: AnswerSnapshot) { snapshots.set(identity, snapshot) }
export function answerSnapshot(messages: ChatMessage[]) { return messages.map(m => snapshots.get(m)).find(Boolean) }
const fold = (text: string) => text.normalize('NFD').replace(/[\u0300-\u036f]/g, '').toLowerCase()
const uncertain = (text: string) => /nao (?:tenho|ha|encontrei|possuo|consigo)|nao sei|not (?:have|enough|available)|no (?:record|information)|don.t (?:have|know)|cannot determine|can.t determine|insufficient information|not recorded|informac(?:ao|oes).*insuficiente/.test(fold(text))
const unsupportedAbsence = (text: string) => /(?:voce|you)\s+(?:(?:ainda|nunca|have|has|still)\s+)*(?:nao|not|never|haven.t)\s+(?:me\s+)?(?:decidiu|escolheu|definiu|disse|informou|decided|chosen|selected|told)|(?:nao foi|nao esta|ainda nao foi)\s+(?:definid|escolhid|decidid)|(?:has|have) not been (?:defined|chosen|decided)|(?:linguagem|banco|language|database|escolha|decisao).*ainda nao.*(?:definid|decidid|escolhid)/.test(fold(text))
const execution = (text: string) => /\besta (?:sendo )?(?:desenvolvid|implementad|usando|utilizando|estudando)|\bja (?:usa|utiliza|implementou|iniciou|comecou|esta)\b|(?:is|are) (?:already |currently )?(?:being developed|developing|implemented|using|building|studying)|already (?:implemented|started|using)|em (?:execucao|andamento)|in progress|\bunderway\b|(?:foi |is )?(?:escrito em|written in|built with)|\b(?:usa|utiliza|runs on|roda em)\b|(?:projeto|project).*(?: e em | is in |foi feito em|based on)|(?:desenvolvido|implemented|developed) (?:em|in|with)/.test(fold(text))
const completion = (text: string) => /\b(?:concluiu|conclui|terminei|finalizei|terminou|finalizou|concluido|concluida|completed|finished)\b|ja (?:concluido|pronto|finalizado)|(?:is|esta|foi|was) (?:already |ja )?(?:complete|finished|pronto|concluido)/.test(fold(text))
const planned = (text: string) => /planej|pretend|\bplan\b|\bplans\b|intend|want to|quero |vai (?:usar|fazer|desenvolver)|will (?:use|develop)/.test(fold(text))
const past = (text: string) => /\b(?:antes|anteriormente|antigamente|previously|formerly|historical|historico)\b|used to/.test(fold(text))
function entityIn(text: string): string | undefined {
  const q = fold(text), found: { index: number; entity: string }[] = []
  for (const [pattern, entity] of [
    [/computador principal|primary computer|main (?:desktop|computer|pc)/g, 'device.primary_computer'],
    [/notebook de trabalho|work (?:laptop|notebook)|laptop de trabalho/g, 'device.work_laptop'],
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

export function verifyMemoryAnswer(text: string, snapshot: AnswerSnapshot): FidelityCheck {
  const began = performance.now(), assertions: MemoryAssertion[] = [], violations: FidelityViolation[] = []
  const add = (code: string, span: string, critical = true) => {
    if (!violations.some(v => v.code === code && v.span === span)) violations.push({ code, span, critical })
  }
  if (text.length > 24000) add('verification_ceiling', '', false)
  const all = [...snapshot.facts, ...snapshot.currentStatements].filter(f => f.scope === snapshot.scope && f.project === snapshot.project)
  const current = all.filter(f => f.validity !== 'historical')
  const query = fold(snapshot.query)
  const personalQuestion = /\b(?:meu|minha|meus|minhas|my|our|aquele|aquela|aqueles|aquelas|that)\b|pref|gosto|lembra|remember/.test(query) || (/\b(?:eu|i)\b/.test(query) && /tenho|tomo|bebo|gosto|escolhi|decidi|conclui|terminei|finalizei|have|drink|chose|choose|decided|like|prefer/.test(query))
  const preferenceQuestion = personalQuestion && /pref|gost|favorit|tema|theme|interface|bebida|beverage|respost|answer|repl|explain|cafe|coffee|idioma|language|jogo|game|character|personag|classe/.test(query)
  const projectQuestion = personalQuestion && /projeto|project|linguagem|language|banco|database/.test(query)
  const record = (span: string, property: string, value: string, entity?: string, negative = false) => {
    const historical = past(span) || (past(snapshot.query) && !/\b(?:agora|atualmente|now|currently|prefere|prefiro|gosta|gosto|tem|tenho|possui|usa|utiliza|is|are|have|has|prefer|prefers|like|likes|uses)\b/.test(fold(span)))
    const candidates = all.filter(f => f.property === property && (!entity || f.entity === entity))
    const matching = candidates.filter(f => negative ? f.value !== value && f.validity !== 'historical' : f.value === value && (f.validity !== 'historical' || historical))
    const assertion = { entity: entity ?? matching[0]?.entity ?? 'unspecified', property, value, span, supported: matching.length > 0, references: matching.map(f => f.id) }
    assertions.push(assertion)
    if (!assertion.supported) { add('unsupported_typed_value', span); return }
    if (historical && !matching.some(f => f.validity === 'historical')) {
      // Only a linked previous property source can support an older value.
      if (!matching.some(f => f.evidence.some(e => e.role === 'previous_property_link_not_current_value' && mentions(e.text, value)))) add('unsupported_historical_state', span)
    }
    if (/decidiu|escolheu|decided|chosen|selected/.test(fold(span)) && !matching.some(f => ['decision', 'current_fact', 'in_progress', 'completed'].includes(f.modality))) add('plan_as_decision', span)
    if (matching.every(f => f.modality === 'cancelled') && !/cancel/.test(fold(span))) add('cancelled_as_current', span)
    if (execution(span) && !matching.some(f => ['in_progress', 'current_fact'].includes(f.modality))) add('plan_or_decision_as_execution', span)
    if (!historical && execution(span) && entity === 'project.' + snapshot.project && current.some(f => f.entity === entity && f.property === 'lifecycle' && ['cancelled', 'completed'].includes(f.modality))) add('terminal_project_as_execution', span)
    if (completion(span) && !matching.some(f => f.modality === 'completed')) add('unproven_completion', span)
    if (!historical && !planned(span) && !execution(span) && matching.every(f => ['planned', 'hypothesis'].includes(f.modality)) && !planned(snapshot.query)) add('modality_omitted', span, false)
    if (matching.every(f => f.modality === 'hypothesis') && !/talvez|pode ser|might|maybe|hipot|possib/.test(fold(span))) add('hypothesis_as_fact', span)
  }
  const clauses = text.split(/(?<=[.!?;\n])\s+|\b(?:mas|porem|however|but)\b/i).slice(0, 128)
  for (const span of clauses) {
    const q = fold(span)
    if (span.trim().endsWith('?') && /^(?:qual|quais|como|quando|onde|por que|what|which|how|when|where|why)\b/.test(q.trim())) continue
    if (unsupportedAbsence(span)) {
      const domain = q + ' ' + query
      const property = /banco|database/.test(domain) ? 'database' : /linguagem|language/.test(domain) ? 'language' : undefined
      if (!property || !current.some(f => f.modality === 'explicitly_unknown' && f.property === property && f.entity === 'project.' + snapshot.project)) add('absence_without_evidence', span)
    }
    if (/poderia usar|could use|recomendo|i recommend|sugiro|i suggest/.test(q) && !/voce (?:decidiu|escolheu)|you (?:decided|chose)/.test(q)) continue
    if (uncertain(span) && !execution(span) && !completion(span)) continue
    const subject = /\b(?:voce|seus?|suas?|you|your)\b|projeto|project/.test(q)
    if (subject && /projeto|project/.test(q)) {
      const projectFacts = current.filter(f => f.entity === 'project.' + snapshot.project)
      if (/em execucao|em andamento|in progress|underway|sendo desenvolvido|being developed/.test(q) && !projectFacts.some(f => f.modality === 'in_progress')) add('unproven_project_progress', span)
      if (/(?:projeto|project).{0,70}(?:esta|foi|is|was) (?:cancelad|cancelled|canceled)/.test(q) && !/nao.*cancel|not.*cancel/.test(q) && !projectFacts.some(f => f.property === 'lifecycle' && f.modality === 'cancelled')) add('unproven_project_cancellation', span)
      if (completion(span) && !projectFacts.some(f => ['lifecycle', 'project_status'].includes(f.property) && f.modality === 'completed')) add('unproven_project_completion', span)
    }
    if (subject) {
      for (const m of q.matchAll(/\b(?:projeto|project)\s+["`']?([a-z0-9_.-]+)/g)) {
        const name = m[1] === fold(snapshot.project) ? m[1] : m[1].replace(/\.+$/, '')
        if (!['de', 'do', 'da', 'sem', 'com', 'para', 'with', 'without', 'esta', 'e', 'is', 'was', 'foi', 'sera', 'vai', 'ainda', 'has', 'tem', 'continua', 'planeja', 'agora', 'atualmente', 'hoje', 'usa', 'utiliza', 'possui', 'segue', 'parece', 'estava', 'estara', 'ja', 'nao', 'aqui', 'precisa', 'exige', 'requer', 'apresenta', 'will', 'uses', 'needs', 'requires', 'currently', 'now', 'already', 'still', 'seems', 'would', 'could', 'should', 'may', 'might', 'can'].includes(name) && name !== fold(snapshot.project)) add('unsupported_project_entity', span)
      }
    }
    if (subject || projectQuestion) {
      for (const value of ['typescript', 'javascript', 'python', 'rust', 'sqlite', 'postgresql', 'mysql']) {
        if (!new RegExp('\\b' + value + '\\b').test(q)) continue
        const property = ['sqlite', 'postgresql', 'mysql'].includes(value) ? 'database' : 'language'
        if (/\bprojeto|\bproject|linguagem|programming language|banco de dados|database/.test(q) || projectQuestion || (subject && personalQuestion && /escolh|decid|guardar|armazen|persist|store|stack/.test(query)) || (current.some(f => f.property === property && f.entity === 'project.' + snapshot.project) && !/objetivo|goal|aprender|learn|estud|study/.test(q))) record(span, property, value, 'project.' + snapshot.project)
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
    for (const match of q.matchAll(/\b(\d+)\s*(?:gb|gib)\b/g)) {
      if (!subject && !personalQuestion && !/ram|computador|computer|laptop|notebook/.test(q)) continue
      const property = /livre|disponivel|available|free/.test(q) ? 'free_ram' : /\bram\b|memoria|memory/.test(q + ' ' + query) ? 'ram_capacity' : 'capacity_update'
      const entity = entityIn(q.slice(0, match.index)) ?? entityIn(query)
      record(span, property, match[1] + '_gb', entity, /nao (?:tem|possui|sao|e)|does not have|not \d/.test(q.slice(0, match.index)))
    }
    if ((subject && /pref|gost|likes|favorit|adora|curt|toma|bebe|drink|idioma|lingua|tema|theme|comunica|communication/.test(q)) || (preferenceQuestion && span.length<160 && !/\b(?:pode|poderia|opcao|alternativa|could|might|option|recommend|sugiro|depende)\b/.test(q))) {
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
          if (!/favorit|prefere|prefer/.test(q) && !/favorit|bebida|beverage/.test(query)) continue
          const positions=[...new Set(values)].flatMap(value=>aliases(value).flatMap(alias=>[...q.matchAll(new RegExp('\\b'+alias+'\\b','g'))].map(m=>({value,index:m.index}))))
            .filter(m=>!/\b(?:nao|not|never)\s+(?:o |a |the )?$/.test(q.slice(Math.max(0,m.index-16),m.index)))
            .sort((a,b)=>a.index-b.index)
          const binding=q.match(/(?:prefere|prefers|prefer)\s+|(?:favorit[ao]|favorite)\s+(?:e|is)\s+/)
          const bound=binding ? positions.find(m=>m.index>binding.index!+binding[0].length-1) : undefined
          const target=bound?.value ?? positions[0]?.value
          if (/favoritas|favorites|both|ambas/.test(q)) {
            for (const value of new Set(positions.map(m=>m.value))) record(span,property,value,'user',negatedChoice)
          } else if (target) record(span,property,target,'user',negatedChoice)
        } else for (const value of new Set(values)) if (mentions(span, value)) record(span, property, value, 'user',negatedChoice)
      }
      const denied = /nao (?:gosta|prefere|curte)|don.t (?:like|prefer)|does not (?:like|prefer)/.test(q)
      if (/rpg/.test(q) && /tatic|tactic/.test(q)) record(span, 'games_tactics', denied ? 'negative' : 'positive', 'user')
      if (/estrategia por turnos|turn.based strategy/.test(q)) record(span, 'games_turn_strategy', denied ? 'negative' : 'positive', 'user')
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
    const goalQuestion=personalQuestion && goalDomain.test(query)
    if ((subject && goalDomain.test(q)) || (goalQuestion && span.length<200)) {
      const stop=new Set(['voce','seu','sua','seus','suas','your','you','the','was','foi','era','objetivo','goal','meta','concluiu','conclui','concluido','completed','finished','concluir','concluido','concluida','terminou','finalizou','apenas','seu','meu','minha','esta','seu','que','para','uma','um','de','do','da','em','of','to'])
      const tokens=(text:string)=>fold(text).split(/[^a-z0-9]+/).filter(t=>t.length>2 && !stop.has(t)).map(t=>({aprender:'learn',portugues:'portuguese',ingles:'english',espanhol:'spanish'} as Record<string,string>)[t] ?? t)
      const target=tokens(span), requiredCompletion=completion(span) || (goalQuestion && completion(query))
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
  const relevantKnown = current.some(f => f.validity === 'current')
  if (personalQuestion && relevantKnown && uncertain(text) && assertions.length === 0) add('known_information_not_used', text, false)
  return { assertions, violations, uncertain: uncertain(text), supportedAssertions: assertions.filter(c => c.supported).length,
    unsupportedAssertions: assertions.filter(c => !c.supported).length, elapsedMs: performance.now() - began }
}
