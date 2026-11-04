import {expect,it,vi} from 'vitest'
import {registerAnswerSnapshot,memoryAbsenceFields} from '../lucille-fidelity'
import {validateLucilleCompletion} from '../lucille-answer'
import type {ChatMessage} from '../../api/providers/types'
import {verifyMemoryAnswer,type AnswerSnapshot,type MemoryLearningReceipt} from '../lucille-fidelity'
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
 const retry=vi.fn(async()=>({content:'Você planeja usar SQLite no seu projeto.'}))
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
