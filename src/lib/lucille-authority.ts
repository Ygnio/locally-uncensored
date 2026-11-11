import type { AnswerSnapshot, MemoryFact } from './lucille-fidelity'

/** Linguistic recognition proposes a claim; only this evidence relation grants support.
 * Executor observations never become project facts merely by sharing a conversation.
 * No writer, tool dispatch or model is reachable from this module.
 */
export type AuthorityVerdict = 'supported' | 'contradicted' | 'insufficient' | 'out_of_scope' | 'historical' | 'hypothetical' | 'tool_dependent' | 'non_memory'
export type EvidenceOrigin = 'original_user' | 'approved_memory' | 'canonical_history' | 'executor'
export interface AuthorityClaim {
  entity: string; property: string; relation: 'state' | 'transition' | 'observation' | 'consequence'
  value: string; span: string; verdict: AuthorityVerdict; references: string[]; origins: EvidenceOrigin[]
  reason: string; critical: boolean
}
export const normalizeEvidenceText = (text: string) => text.normalize('NFD').replace(/[\u0300-\u036f]/g, '').toLowerCase()
/** Denying an implication is not asserting poor (or good) actual performance.
 * A later independent positive predicate must still be checked.
 */
/** Keep explicit contrasting subjects separate; relative game features are not
 * new speakers. This is response attribution, never permission to learn. */
export function answerRelationClauses(text:string):string[]{
 return text.split(/(?<=[.!?;\n])\s+|\b(?:mas|por[eé]m|however|but)\b|,?\s+(?:enquanto|whereas|while)\s+(?=n[aã]o (?:h[aá]|tenho|encontrei)(?![\p{L}\p{N}_])|there (?:is|are) no\b|i (?:do not|don.t) (?:know|have)\b)|,?\s+(?=(?:enquanto|whereas|while)\s+(?:(?:a|o|the)\s+)?(?:personage(?:m|ns)|characters?|narrador(?:a)?|narrators?|ferramenta|tool|voc[eê]|you)(?![\p{L}\p{N}_]))|\s+(?=(?:e|and)\s+(?:(?:a|o|the)\s+)?(?:personagem|character|narradora?|narrator|ferramenta|tool|(?:(?:voc[eê]|you)\s+)?(?:decidiu|escolheu|planeja|pretende|usa|utiliza|uses|plans|decided|chose|intends))\b)|\s+(?=(?:embora|although)\b)/iu).slice(0,128)
}

/** Query form that asks for a plan or choice, including a nominal past
 * predicate. Scope and the temporal validity of the record are supplied by SQLite. */
export function requestsProjectPlan(query: string, fact: MemoryFact): boolean {
  if (!['database','language'].includes(fact.property) || !['planned','decision'].includes(fact.modality)) return false
  const q = normalizeEvidenceText(query)
  if (/\b(?:agora|atualmente|current(?:ly)?|now|still|ainda)\b/.test(q)) return false
  const field = fact.property === 'database' ? /\b(?:banco|database)\b/ : /\b(?:linguagem|language)\b/
  const relation = fact.modality === 'planned' ? /\b(?:planejad[ao]|planejava|pretendia|planejei|planned|had intended|had planned)\b/ : /\b(?:escolhid[ao]|decidid[ao]|escolhi|decidi|chosen|chose|selected|decided)\b/
  return answerRelationClauses(q).some(clause => field.test(clause) && relation.test(clause) && !/\b(?:irmao|irma|brother|sister|colleague|colega)\b/.test(clause.split(/\b(?:com|versus|vs\.?|against|with|to)\b/u,1)[0]))
}

export function historicalProjectWording(span: string): boolean {
  return /\b(?:planejava|pretendia|tinha (?:planejado|escolhido|decidido)|havia (?:planejado|escolhido|decidido)|had (?:planned|intended|chosen|selected|decided)|used to (?:plan|intend))\b|\b(?:planejad[ao]|escolhid[ao]|decidid[ao]|planned|chosen|selected)\b[^,;.!?]{0,65}\b(?:era|foi|was)\b|\b(?:era|foi|was)\b[^,;.!?]{0,65}\b(?:planejad[ao]|escolhid[ao]|decidid[ao]|planned|chosen|selected)\b/.test(normalizeEvidenceText(span))
}
/** An explicitly named fictional/document/tool subject is not the USER.
 * A later personal subject remains a personal claim and cannot borrow this exemption. */
export function externalAssertion(span:string):boolean{
 const q=normalizeEvidenceText(span).trim().replace(/^(?:e|and|enquanto|while|whereas)\s+/,'')
 const anchor=q.match(/^(?:(?:a|o|the)\s+)?(?:voz narrativa|narrative voice|narrador(?:a)?|narrator|personagem|character|ferramenta|tool|documento|document|log|texto|text)\b/)
 return !!anchor && !/\b(?:voce|seu|sua|seus|suas|you|your|usuario|user)\b/.test(q.slice(anchor[0].length))
}
export function deniesCapacityGuarantee(text: string): boolean {
  const q=normalizeEvidenceText(text)
  const denial=q.match(/\b(?:nao (?:garante|garantem|assegura|asseguram|comprova|comprovam|implica|implicam)|(?:does not|doesn.t|cannot|can.t) (?:guarantee|ensure|prove|imply))\b/)
  if(!denial)return false
  const tail=q.slice(denial.index!+denial[0].length)
  return !/\b(?:mas|porem|but|however|e|and)\b.{0,80}\b(?:oferece|garante|assegura|proporciona|tem|e|is|offers|provides|guarantees|ensures|suficiente|sufficient|excelente|excellent|ideal)\b/.test(tail)
}
export function technologyRoles(text:string): {value:string;role:'origin'|'destination'}[] {
  const q=normalizeEvidenceText(text)
  const instead=q.search(/\bem vez de\b|\binstead of\b/)
  const verb=q.match(/\b(?:trocar|trocou|troquei|substituir|substituiu|replace|replaced|swap|swapped|switch|switched)\b/)
  const tail=verb ? q.slice(verb.index!+verb[0].length):''
  const boundary=tail.search(/\b(?:por|pelo|pela|with|for|to)\b/)
  if (instead<0 && boundary<0) return []
  const at=instead>=0 ? instead : verb!.index!+verb![0].length+boundary
  return [...q.matchAll(/\b(?:typescript|javascript|python|rust|sqlite|postgresql|mysql)\b/g)].map(m=>({value:m[0],role:(instead>=0 ? m.index!>at : m.index!<at)?'origin':'destination'}))
}
export function memoryQuestionNeedsNoTools(query: string): boolean {
  const q = normalizeEvidenceText(query)
  // Explicit actions and advice requests retain the ordinary executable roster.
  if (/\b(?:list|read|open|inspect|check|search|write|create|run|execute|send|email|notify|liste|listar|leia|ler|abra|abrir|inspecione|verifique|pesquise|escreva|crie|execute|rode|envie|mande|notifique)\b|\b(?:arquivos?|files?|diretorio|directory|folder|pasta|repository|repositorio)\b/.test(q)) return false
  if (/\b(?:devo|should|recommend|recomenda|melhor|best|how to)\b/.test(q))return false
  // Native memory directives are not filesystem actions. Keep the ordinary
  // roster only when an external action/resource was explicitly requested above.
  const memoryDirective=/^(?:por favor[, ]+|please[, ]+)?(?:esqueca|forget|memorize|memorise|guarde|registre|lembre|volte a (?:guardar|memorizar|registrar)|quero voltar a registrar|i want to remember|you can remember)\b/.test(q.trim())
  if (memoryDirective && /preferenc|preference|memori|memory/.test(q)) return true
  const recalled=/\b(?:lembra|recorda|remember|recall)\b/.test(q)
  const personal=/\b(?:meu|minha|meus|minhas|my|our|aquele|aquela|that)\b/.test(q)
  const slots=/linguagem|language|banco|database|\bram\b|notebook|laptop|prefer|objetivo|goal|tema|theme|bebida|drink/.test(q)
  const question=/\b(?:qual|quais|what|which|how much|quanto)\b/.test(q)
  const choice=/\b(?:escolhi|decidi|chose|choose|decided|planned|planejad)\b/.test(q)
  const status=/^(?:como esta|how is|what is the status of).{0,70}(?:projeto|project)\b/.test(q.trim())
  return recalled || (slots && question && (personal || choice)) || (personal && status)
}

export function requestedMemoryProperties(query: string): string[] {
  const q=normalizeEvidenceText(query), properties:string[]=[]
  if (/\b(?:linguagem|language)\b/.test(q)) properties.push('language')
  if (/\b(?:banco|database)\b/.test(q)) properties.push('database')
  if (/\bram\b|\b(?:memoria|memory|gigabytes)\b/.test(q) && !/gpu|vram|storage|armazenamento/.test(q)) properties.push(/livre|free/.test(q)?'free_ram':'ram_capacity')
  if (/\b(?:projeto|project)\b/.test(q) && /como esta|how is|status|estado atual|current state/.test(q)) properties.push('lifecycle')
  if (/\bgpu\b|placa de video|graphics card/.test(q)) properties.push('gpu_model')
  if (/data de nascimento|birth ?date|birthday/.test(q)) properties.push('birth_date')
  if (/cafe|coffee/.test(q)) properties.push('coffee_sugar')
  if (/pref|gost|likes/.test(q) && /bebida|beverage|drink|cafe|coffee/.test(q) && !/favorit|favourit|acucar|sugar/.test(q)) properties.push('beverage_preference')
  if (/bebida|beverage|drink/.test(q) && !properties.includes('beverage_preference')) properties.push('favorite_beverage')
  if (/tema|theme|modo escuro|dark mode/.test(q)) properties.push('interface_theme')
  return properties
}

export function answerLanguage(query: string, facts: readonly MemoryFact[]): 'en'|'pt'|undefined {
  const q=normalizeEvidenceText(query)
  if (/responda em ingles|answer in english/.test(q)) return 'en'
  if (/responda em portugues|answer in portuguese/.test(q)) return 'pt'
  const preference=facts.find(f=>f.property==='response_language' && f.entity==='user' && f.validity!=='historical' && ['pt','en'].includes(f.value))
  if (preference) return preference.value as 'en'|'pt'
  if (/^(?:what|which|how|should|is|are|do|does|have|has|can you|could you)\b/.test(q.trim())) return 'en'
  if (/^(?:qual|quais|como|quanto|quando|onde|voce|eu|meu|minha)\b/.test(q.trim())) return 'pt'
  return undefined
}

export function violatesAnswerLanguage(text: string, language: 'en'|'pt'|undefined): boolean {
  if (!language) return false
  const words=normalizeEvidenceText(text).match(/[a-z]+/g)??[]
  const pt=words.filter(w=>['voce','seu','sua','nao','prefere','concluiu','projeto','objetivo','esta'].includes(w)).length
  const en=words.filter(w=>['you','your','not','prefer','project','goal','has','have','completed'].includes(w)).length
  return language==='en' ? pt>=2 && en===0 : en>=2 && pt===0
}

function origins(facts: MemoryFact[]): EvidenceOrigin[] {
  return [...new Set(facts.map(f => f.validity === 'historical' ? 'canonical_history' : f.validity === 'current_user_statement' ? 'original_user' : 'approved_memory') as EvidenceOrigin[])]
}
function literalEvidence(span: string, facts: MemoryFact[]) {
  const literal = normalizeEvidenceText(span).replace(/[.!?]+$/, '').trim()
  return facts.filter(f => f.evidence.some(e => e.role !== 'previous_property_link_not_current_value' && normalizeEvidenceText(e.text).replace(/[.!?]+$/, '').trim() === literal))
}

const implementationStage = /^(?:voce (?:esta|se encontra)|you are|(?:o projeto|the project)(?:\s+[\p{L}\p{N}_-]+){0,3}\s+(?:esta|is))\s+(no (?:inicio(?: ou no meio)?|meio|fim) da implementacao|em fase (?:inicial|intermediaria|final)|in the (?:early stages|middle|final stages) of implementation|in the (?:initial|intermediate|final) phase)\b/u

/** Finite high-risk relations, separate from surface value extraction. */
export function authorityClaims(text: string, snapshot: AnswerSnapshot): AuthorityClaim[] {
  const facts = [...snapshot.facts, ...snapshot.currentStatements].filter(f => f.scope === snapshot.scope && f.project === snapshot.project)
  const current = facts.filter(f => f.validity !== 'historical')
  const result: AuthorityClaim[] = []
  const add = (span: string, entity: string, property: string, value: string, relation: AuthorityClaim['relation'], proof: MemoryFact[], reason: string, critical = true, tool = false) => {
    result.push({ span, entity, property, value, relation, verdict: proof.length ? 'supported' : tool ? 'tool_dependent' : 'insufficient', references: proof.map(f => f.id), origins: origins(proof), reason, critical })
  }
  for (const span of answerRelationClauses(text).filter(s=>s.trim())) {
    if(externalAssertion(span))continue
    const q = normalizeEvidenceText(span)
    // A factive lead-in makes a claim even when the main clause asks a question.
    const presupposition = q.match(/^\s*(?:como|since|given that)\s+([^,]+),/u)
    const stage = (presupposition?.[1] ?? (span.trim().endsWith('?') ? '' : q.trim())).match(implementationStage)?.[1]
    if (stage) {
      const entity = 'project.' + snapshot.project
      const proof = current.filter(f => f.entity === entity && f.property === 'project_stage' && f.value === stage)
      add(presupposition ? span.slice(0,presupposition[0].length) : span,entity,'project_stage',stage,'state',proof,'Evidence of implementation does not establish its stage.',false)
    }
    if (span.trim().endsWith('?')) continue
    const epistemic = /nao (?:ha|tenho).{0,35}(?:confirmacao|evidencia)|no (?:confirmation|evidence)|(?:not|isn.t|nao).{0,18}(?:confirmed|confirmado)|cannot confirm|nao (?:implica|significa|comprova)|does not (?:mean|imply|prove)|doesn.t (?:mean|imply|prove)/.test(q)
    const hypothetical = (predicate:RegExp) => {
      const at=q.search(predicate)
      // Modality binds before this predicate, never to a later relative clause
      // such as “games where you can choose a class”.
      return /^(?:se|if|talvez|maybe|hipoteticamente|hypothetically)\b/.test(q.trim())
        || (at>=0 && /\b(?:might|could|may|pode|podem|podera|poderao|poderia|poderiam|should|estimativa|estimate)\b/.test(q.slice(0,at)))
    }
    const project = /\b(?:projeto|project)\b/.test(q)
    if (/\b(?:trocou|migrou|substituiu|swapped|migrated|switched|replaced)\b/.test(q) && !/\b(?:decidiu|decidi|decided|planeja|plans?|pretende|intends?)\b/.test(q) && !epistemic && !hypothetical(/\b(?:trocou|migrou|substituiu|swapped|migrated|switched|replaced)\b/)) {
      for (const target of technologyRoles(span).filter(v=>v.role==='destination')) {
        const property=['mysql','postgresql','sqlite'].includes(target.value)?'database':'language'
        const proof=current.filter(f=>f.entity==='project.'+snapshot.project && f.property===property && f.value===target.value && ['in_progress','current_fact'].includes(f.modality) && f.evidence.some(e=>/\b(?:troquei|migrei|substitui|switched|migrated|replaced)\b/.test(normalizeEvidenceText(e.text))))
        add(span,'project.'+snapshot.project,'stack_transition',target.value,'transition',proof,'A decision to replace technology is not an executed replacement.')
      }
    }
    if (project && /em desenvolvimento|em andamento|em execucao|esta (?:desenvolvendo|implementando|avancando)|is progressing|in development|in progress|under development|underway|being developed|is (?:still |currently )?developing/.test(q) && !epistemic && !hypothetical(/em desenvolvimento|em andamento|em execucao|avancando|progress|development|developed|developing|underway/)) {
      const proof = current.filter(f => f.entity === 'project.' + snapshot.project && ['in_progress','current_fact'].includes(f.modality) && ['lifecycle','project_status','language'].includes(f.property))
      const terminal = current.some(f => f.entity === 'project.' + snapshot.project && f.property === 'lifecycle' && ['cancelled','completed'].includes(f.modality))
      const negative=/nao (?:esta|segue)|is not|isn.t|not in/.test(q)
      const established=negative ? current.filter(f=>f.entity==='project.'+snapshot.project && f.property==='lifecycle' && ['cancelled','completed'].includes(f.modality)) : terminal ? [] : proof
      add(span,'project.'+snapshot.project,'execution',negative?'not_in_progress':'in_progress','state',established,'A technology choice or project description is not proof of execution.')
      if (terminal && !negative) result[result.length-1].verdict='contradicted'
    }
    if (/^(?:a pasta (?:consultada|atual)|o diretorio (?:consultado|atual)|the queried folder|the consulted directory|the current (?:folder|directory)|the folder)\b/.test(q) && /vazi|empty/.test(q) && !hypothetical(/vazi|empty/)) {
      const currentDirectory=/pasta atual|diretorio atual|current (?:folder|directory)/.test(q)
      // A later failed read cannot borrow an earlier successful empty result;
      // an observation of another path cannot prove the current directory.
      const observations=(snapshot.toolObservations??[]).filter(o=>o.tool==='file_list').slice(-1).filter(o=>o.executed && o.result_confirmed && o.outcome==='completed' && o.observation?.resource && (!currentDirectory || ['.','./','.\\'].includes(o.observation.resource)) && ['The folder is empty.','A pasta está vazia.'].includes(o.observation.result.trim()))
      result.push({entity:observations.at(-1)?.observation?.resource??'unbound_directory',property:'directory_entries',value:'empty',relation:'observation',span,verdict:observations.length?'supported':'tool_dependent',references:observations.map(o=>o.call_id),origins:observations.length?['executor']:[],reason:'Only the requested directory was observed; no project-wide coverage follows.',critical:true})
    }
    if (/\b(?:voce|you)\b/.test(q) && /voltou a gostar|voltou a preferir|retomou (?:seu |o )?gosto|deixou de gostar|nao gosta mais|stopped liking|like.{0,100}again|again.{0,25}likes?|returned to liking|resumed liking|no longer likes/.test(q) && !epistemic && !hypothetical(/voltou|retomou|deixou|nao gosta|stopped|likes?|again|returned|resumed/)) {
      add(span,'user','preference_transition',/deixou|stopped|no longer|nao gosta mais/.test(q)?'stopped':'resumed','transition',literalEvidence(span,current),'A memory-store operation establishes no transition in the user’s personal liking.')
    }
    if (project && /(?:nao (?:tem|possui|ha)|sem).{0,25}arquivos|(?:has no|does not have|doesn.t have|without).{0,20}files|(?:projeto|project).{0,40}(?:esta vazio|esta vazia|is empty)/.test(q) && !epistemic && !hypothetical(/nao tem|nao possui|nao ha|sem|has no|does not|doesn.t|without|vazio|vazia|empty/)) {
      add(span,'project.'+snapshot.project,'file_inventory','empty','observation',literalEvidence(span,current),'An observed directory is not an exhaustive project inventory. A resource/entity/coverage binding is required.',true,true)
    }
    if (/\bram\b|\b\d+\s*(?:gb|gib)\b|computador|computer|desktop|notebook|laptop|(?:capacidade|how much|quanto).{0,20}(?:memoria|memory)/.test(normalizeEvidenceText(text+' '+snapshot.query)) && /\b(?:voce|you|seu|sua|your|meu|minha|my)\b/.test(q+' '+normalizeEvidenceText(snapshot.query)) && /(?:excelente|ideal|suficiente|excellent|ideal|sufficient).{0,40}(?:capacidade|capacity|rodar|run|desempenho|performance)|(?:capacidade|capacity).{0,25}(?:excelente|excellent)|\bgarant.{0,45}(?:rodar|run|fluidez|desempenho)|(?:excelente|ideal|suficiente|excellent|sufficient)\s+(?:margem|folga|headroom)|(?:quantidade|amount)\s+(?:solida|adequada|suficiente|solid|adequate|sufficient).{0,60}(?:produtividade|desenvolvimento|multitarefa|tarefas|productivity|development|multitask|tasks)|(?:excelente|ideal|suficiente|solid[ao]|adequad[ao]|excellent|sufficient|solid|adequate).{0,40}\b(?:para|for)\b.{0,35}(?:multitarefa|produtividade|edicao|navegacao|desenvolvimento|tarefas|multitask|productivity|editing|browsing|development|tasks)/.test(q) && !deniesCapacityGuarantee(span) && !hypothetical(/excelente|ideal|suficiente|solid[ao]|adequad[ao]|excellent|sufficient|solid|adequate|garant/)) {
      add(span,'user','workload_performance','adequate','consequence',literalEvidence(span,current),'RAM quantity alone cannot attest workload suitability or performance.',false)
    }
  }
  return result
}

/** Retain exact independent sentences only. Never manufacture an uncertainty sentence.
 * Mixed or dependent clauses require a real rewrite, not substring surgery.
 */
export function projectIndependentSentences(text: string, badSpans: string[], snapshot: AnswerSnapshot): { text: string; removed: string[] } | undefined {
  if (!badSpans.length || /\b(?:ja|already|comecou|started|implementado|implemented|concluido|completed)\b/.test(normalizeEvidenceText(snapshot.query))) return undefined
  // A single matched property cannot certify completeness for two devices or
  // both old/current values. Those requests require the real rewrite path.
  if (/\b(?:e|and|ambos|both|antes|before|previous|historico|historical)\b/.test(normalizeEvidenceText(snapshot.query))) return undefined
  const parts = text.match(/[^.!?]+(?:[.!?]+|$)/g)?.map(s=>s.trim()).filter(Boolean) ?? []
  const removed = parts.filter(s=>badSpans.includes(s))
  if (!removed.length || badSpans.some(s=>!parts.includes(s))) return undefined
  const retained = parts.filter(s=>!removed.includes(s))
  if (!retained.length || retained.some(s=>/^(?:isso|essa|ele|ela|this|that|it|therefore|portanto|assim|tambem|also)\b/i.test(normalizeEvidenceText(s)))) return undefined
  return { text:retained.join(' '), removed }
}
