import {expect,it,vi} from 'vitest'
import {registerAnswerSnapshot,memoryAbsenceFields} from '../lucille-fidelity'
import {validateLucilleCompletion} from '../lucille-answer'
import type {ChatMessage} from '../../api/providers/types'
import {verifyMemoryAnswer,type AnswerSnapshot,type MemoryLearningReceipt} from '../lucille-fidelity'
it('does not assign the historical generic game-feature reply to USER preferences',()=>{
 const s=base();s.query='Entre os jogos, os de estratégia por turnos são os meus favoritos.';s.currentStatements=[]
 for(const text of ['Estratégia por turnos permite uma análise cuidadosa de cada movimento. Você prefere jogos com grandes escalas épicas ou algo mais focado em pequenos grupos de personagens?','Os jogos de estratégia por turnos permitem um planejamento cuidadoso e profundo. Você prefere jogos com grandes mapas e muitos personagens ou algo mais focado em pequenos grupos e batalhas intensas?'])expect(verifyMemoryAnswer(text,s).violations,text).toEqual([])
 expect(verifyMemoryAnswer('Você prefere estratégia por turnos, que permite planejar cuidadosamente.',s).violations.some(v=>v.critical)).toBe(true)
 expect(verifyMemoryAnswer('Estratégia por turnos.',s).violations.some(v=>v.critical)).toBe(true)
})
const base=():AnswerSnapshot=>({scope:'qa',project:'candeia',query:'Quero voltar a registrar minha preferência: eu prefiro café com açúcar.',facts:[],currentStatements:[{id:'current-turn-0',version:0,entity:'user',property:'coffee_sugar',value:'with_sugar',modality:'preference',validity:'current_user_statement',scope:'qa',project:'candeia',review:'original',facets:[],evidence:[{role:'current_user_statement',text:'Eu prefiro café com açúcar.'}]}],sourceIdentity:{conversation_id:'c',source_message_id:'u'}})
const receipt=():MemoryLearningReceipt=>({id:'a'.repeat(64),action:'learn',state:'done',scope:'qa',project:'candeia',conversation_id:'c',source_message_id:'u',records:[{id:'m',version:1,state:'active',entity:'user',property:'coffee_sugar',value:'with_sugar'}]})
it('rejects completion before commit, including elliptical historical Registrado, but allows natural acknowledgment',()=>{
 for(const text of ['Registrado: você prefere café com açúcar.','Anotado! Você prefere café com açúcar.','Saved: you prefer coffee with sugar.','Registrei sua preferência por café com açúcar.','Registrado.','Atualizei sua memória com essa preferência.'])expect(verifyMemoryAnswer(text,base()).violations.map(v=>v.code),text).toContain('unconfirmed_memory_mutation')
 expect(verifyMemoryAnswer('Você prefere café com açúcar.',base()).violations).toEqual([])
})
it('requires the same durable source and distinguishes candidate persistence from active memory',()=>{
 const s=base();s.learningReceipt=receipt()
 expect(verifyMemoryAnswer('Registrei sua preferência por café com açúcar.',s).violations).toEqual([])
 s.learningReceipt.records[0].state='candidate'
 expect(verifyMemoryAnswer('Registrei sua preferência por café com açúcar.',s).violations.map(v=>v.code)).toContain('unconfirmed_memory_mutation')
 expect(verifyMemoryAnswer('Registrei uma candidata com sua preferência por café com açúcar, aguardando revisão.',s).violations).toEqual([])
 s.learningReceipt=receipt();s.learningReceipt.source_message_id='older-source'
 expect(verifyMemoryAnswer('Registrei sua preferência por café com açúcar.',s).violations.map(v=>v.code)).toContain('unconfirmed_memory_mutation')
})

it('recognizes the original false bank nonreceipt and keeps a bounded GPU uncertainty independent of known RAM',()=>{
 const s=base();s.query='Compare o banco que planejei para meu projeto com o banco do meu irmão.'
 const answer='Como você ainda não me forneceu os detalhes específicos do banco que planejou nem do banco do seu irmão, descreva ambos.'
 expect(verifyMemoryAnswer(answer,s).violations.map(v=>v.code)).toContain('absence_without_evidence')
 expect(memoryAbsenceFields(answer,s)).toEqual([{entity:'project.candeia',property:'database',value:''}])
 s.query='Qual é minha GPU?';s.facts=[{...s.currentStatements[0],id:'ram',version:1,entity:'device.primary_computer',property:'ram_capacity',value:'32_gb',validity:'current',modality:'current_fact'}]
 expect(verifyMemoryAnswer('Não há registro disponível sobre qual GPU você utiliza.',s).violations).toEqual([])
})
it('performs a property-specific canonical read before a real retry, without treating no hit as proof of never provided',async()=>{
 const s=base();s.query='Qual banco planejei para meu projeto?';s.currentStatements=[]
 const db={...base().currentStatements[0],id:'db',version:2,entity:'project.candeia',property:'database',value:'sqlite',validity:'current',modality:'planned'}
 const identity:ChatMessage={role:'system',content:'Native identity'};registerAnswerSnapshot(identity,s)
 const retry=vi.fn(async (messages:ChatMessage[])=>{expect(messages.some(m=>m.role==='user' && m.content.includes('Additional native verification context') && m.content.includes('sqlite'))).toBe(true);return {content:'Você planeja usar SQLite no seu projeto.'}})
 // Refresh on the retry must continue to return the native record, as the real context reload does.
 s.refresh=vi.fn(async()=>({facts:[db],current_statements:[]}))
 const result=await validateLucilleCompletion([identity,{role:'user',content:s.query}],{content:'Você ainda não me forneceu os detalhes do banco.'},retry)
 expect(retry).toHaveBeenCalledTimes(1)
 expect(s.refresh).toHaveBeenCalledWith([{entity:'project.candeia',property:'database',value:''}])
 expect(result.report.status).toBe('regenerated')
})

it('binds a RAM change amount to its own predicate and one native device, without borrowing another capacity',()=>{
 const s=base();s.query='Não tenho mais 32 GB, agora são 56 GB.'
 const ram={...s.currentStatements[0],entity:'device.primary_computer',property:'ram_capacity',value:'56_gb',modality:'current_fact',evidence:[{role:'current_user_statement',text:s.query}]}
 s.currentStatements=[ram];s.facts=[{...ram,id:'work',version:2,entity:'device.work_laptop',value:'24_gb',validity:'current',evidence:[]}]
 expect(verifyMemoryAnswer('Você aumentou sua capacidade em 24 GB.',s).violations).toEqual([])
 expect(verifyMemoryAnswer('Você aumentou sua capacidade em 20 GB.',s).violations.some(v=>v.critical)).toBe(true)
 expect(verifyMemoryAnswer('Seu computador principal tem 24 GB de RAM.',s).violations.some(v=>v.critical)).toBe(true)
 expect(verifyMemoryAnswer('Você aumentou sua RAM em 24 GB para 56 GB.',s).violations).toEqual([])
 s.currentStatements=[ram,{...ram,id:'other',entity:'device.other_computer'}]
 expect(verifyMemoryAnswer('Você aumentou sua capacidade em 24 GB.',s).violations.some(v=>v.critical)).toBe(true)
})

it('keeps known own information useful in a brother comparison without granting it to the brother',()=>{
 const s=base();s.query='Compare o banco que planejei com o banco do meu irmão.';s.currentStatements=[]
 s.facts=[{...base().currentStatements[0],id:'db',version:2,entity:'project.candeia',property:'database',value:'sqlite',modality:'planned',validity:'current'}]
 s.relations=[{query:'Compare o banco que planejei',actor:'user',state:'retrieved',references:[{id:'db',version:2}]},{query:'o banco do meu irmão',actor:'third_party',state:'unavailable',references:[]}]
 expect(verifyMemoryAnswer('Você planeja usar SQLite no seu projeto. Não há informação disponível sobre o banco do seu irmão.',s).violations).toEqual([])
 expect(verifyMemoryAnswer('Não tenho informação suficiente para comparar.',s).violations.map(v=>v.code)).toContain('known_relation_not_used')
 expect(verifyMemoryAnswer('Seu irmão usa SQLite.',s).violations.map(v=>v.code)).toContain('unsupported_statement_actor')
})

it('uses the original transition endpoints without claiming prior execution or accepting a reversed choice',()=>{
 const s=base();s.query='Decidi trocar TypeScript por Rust.'
 s.currentStatements=[{...s.currentStatements[0],entity:'project.candeia',property:'language',value:'rust',modality:'decision',evidence:[{role:'current_user_statement',text:s.query}]}]
 expect(verifyMemoryAnswer('Entendido, você decidiu trocar TypeScript por Rust.',s).violations).toEqual([])
 for(const text of ['Você trocou TypeScript por Rust.','Antes seu projeto usava TypeScript.','Você decidiu trocar Rust por TypeScript.','Você decidiu trocar JavaScript por Rust.'])expect(verifyMemoryAnswer(text,s).violations.some(v=>v.critical),text).toBe(true)
})
it('preserves the correct negative capacity in the original localized repair example',()=>{
 const s=base();s.query='Não estou dizendo que meu computador tem 128 GB de RAM.';s.currentStatements=[]
 s.facts=[{...base().currentStatements[0],id:'ram',version:2,entity:'device.primary_computer',property:'ram_capacity',value:'48_gb',modality:'current_fact',validity:'current'}]
 const text='Entendido, você está apenas corrigindo o valor atual para 48 GB. Isso deixa claro que o foco é o aumento recente, não uma capacidade total de 128 GB.'
 expect(verifyMemoryAnswer(text,s).violations).toEqual([])
 expect(verifyMemoryAnswer('Seu computador possui uma capacidade total de 128 GB.',s).violations.some(v=>v.critical)).toBe(true)
 s.facts.push({...s.facts[0],id:'work',entity:'device.work_laptop',value:'128_gb'})
 expect(verifyMemoryAnswer('Não uma capacidade total de 128 GB.',s).violations.some(v=>v.critical)).toBe(true)
})

it('does not infer workload headroom from capacity alone, while preserving a denial of the guarantee',()=>{
 const s=base();s.query='Meu computador principal tem 26 GB de RAM.';s.currentStatements=[{...s.currentStatements[0],entity:'device.primary_computer',property:'ram_capacity',value:'26_gb',modality:'current_fact',evidence:[{role:'current_user_statement',text:s.query}]}]
 for(const text of ['26 GB de RAM oferecem uma excelente margem para multitarefa e projetos complexos.','26 GB de RAM é uma quantidade sólida para produtividade e desenvolvimento fluido.'])expect(verifyMemoryAnswer(text,s).violations.map(v=>v.code)).toContain('authority_workload_performance')
 expect(verifyMemoryAnswer('26 GB de RAM não garantem desenvolvimento fluido.',s).violations).toEqual([])
 expect(verifyMemoryAnswer('26 GB de RAM podem oferecer uma excelente margem para multitarefa, dependendo da carga.',s).violations).toEqual([])
})

it('a plain coffee preference does not prove sugar preparation or a favorite, and cannot support tea instead',()=>{
 const s=base();s.query='Qual bebida eu prefiro?';s.currentStatements=[]
 s.facts=[{...base().currentStatements[0],id:'coffee',version:1,property:'beverage_preference',value:'coffee',validity:'current'}]
 expect(verifyMemoryAnswer('Você prefere café.',s).violations).toEqual([])
 expect(verifyMemoryAnswer('Café.',s).violations).toEqual([])
 for(const text of ['Você prefere chá.','Sua bebida favorita é café.','Você prefere café sem açúcar.'])expect(verifyMemoryAnswer(text,s).violations.some(v=>v.critical),text).toBe(true)
})

it('keeps speakers local in the actual narrator and mixed-source GUI replies',()=>{
 const s=base();s.query='Eu prefiro café, mas a personagem do jogo prefere chá.'
 s.currentStatements=[{...s.currentStatements[0],property:'beverage_preference',value:'coffee'}]
 for(const text of ['Entendido. Você prefere café, enquanto a personagem do jogo prefere chá.','A personagem do jogo prefere chá, enquanto você prefere café.','Você prefere café e a personagem prefere chá.'])expect(verifyMemoryAnswer(text,s).violations,text).toEqual([])
 expect(verifyMemoryAnswer('Você prefere chá, enquanto a personagem prefere café.',s).violations.some(v=>v.critical)).toBe(true)
 s.currentStatements=[];s.query='No diálogo que inventei, a narradora diz eu gosto de café sem açúcar.'
 expect(verifyMemoryAnswer('A narradora afirma que gosta de café sem açúcar.',s).violations).toEqual([])
 expect(verifyMemoryAnswer('A narradora afirma que prefere café sem açúcar. Isso contrasta com a sua preferência por chá mencionada anteriormente.',s).violations.some(v=>v.critical)).toBe(true)
 expect(verifyMemoryAnswer('A ferramenta informa que você prefere chá como sua bebida favorita.',s).violations.some(v=>v.critical)).toBe(true)
})
