import {test} from 'node:test'
import assert from 'node:assert/strict'
import {selectKnownRecords,verifyKnownProjection,summarizeReviewedCases} from './phase22e-evaluator-contract.mjs'
const details=[{memory:{input:{id:'language'},version:2,state:'active'},quality:{subject:'project.qa',property:'language',value:'typescript',temporal:'in_progress'}}]
const selector={entity:'project.qa',property:'language',value:'typescript',state:'active',validity:'historical',modality:'in_progress'}
test('cancelled project fields can remain active while their native projection is historical',()=>{
 const selected=selectKnownRecords(details,[selector]);assert.equal(selected[0].input.id,'language')
 verifyKnownProjection([{id:'language',version:2,validity:'historical',modality:'in_progress'}],[{id:'language',version:2}],[selector])
})
test('obsolete assumptions and wrong projected modality fail explicitly instead of substituting a fixture',()=>{
 assert.throws(()=>selectKnownRecords(details,[{...selector,state:'obsolete'}]),/actual canonical/)
 assert.throws(()=>verifyKnownProjection([{id:'language',version:2,validity:'current',modality:'in_progress'}],[{id:'language',version:2}],[selector]),/validity/)
})
test('missing, blocked and critical cases stay in the planned denominators',()=>{
 const planned=[{id:'a',atoms:['a1','a2']},{id:'b',atoms:['b1']},{id:'c',atoms:['c1','c2','c3']}]
 const reviews=[{id:'a',executed:true,delivered:true,useful:true,retained_atoms:['a1','a1','a2'],claims:[{supported:true},{supported:false,critical:true}]},{id:'b',executed:true,delivered:false,useful:false,retained_atoms:[],claims:[]}]
 assert.deepEqual(summarizeReviewedCases(planned,reviews),{planned:3,executed:2,not_executed:1,useful:1,expected_atoms:6,retained_atoms:2,delivered_claims:2,supported_claims:1,critical_delivered:1,fidelity:.5,availability:1/3,completeness:1/3})
 assert.throws(()=>summarizeReviewedCases(planned,[...reviews,reviews[0]]),/duplicate/)
})
