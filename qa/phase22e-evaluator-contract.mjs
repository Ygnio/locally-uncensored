// Evaluation mechanics only. No model call, memory writer or product repair.
export function selectKnownRecords(details,selectors){
 return selectors.map(selector=>{
  const matches=details.filter(d=>d.quality && d.quality.subject===selector.entity && d.quality.property===selector.property && d.quality.value===selector.value && d.memory.state===(selector.state??'active'))
  if(matches.length!==1)throw Error('Selector must identify one actual canonical record: '+JSON.stringify({selector,matches:matches.length,available:details.filter(d=>d.quality?.subject===selector.entity && d.quality?.property===selector.property).map(d=>({state:d.memory.state,value:d.quality.value,temporal:d.quality.temporal}))}))
  return matches[0].memory
 })
}
export function verifyKnownProjection(facts,references,selectors){
 if(facts.length!==references.length)throw Error('Native reference/actor/temporal validation rejected known context')
 selectors.forEach((selector,index)=>{
  const ref=references[index],fact=facts.find(f=>f.id===ref.id && f.version===ref.version)
  if(!fact || selector.validity && fact.validity!==selector.validity || selector.modality && fact.modality!==selector.modality)throw Error('Canonical projection validity/modality differs from the selector: '+JSON.stringify(selector))
 })
}
export function summarizeReviewedCases(planned,reviews){
 const known=new Map(planned.map(p=>[p.id,p])),seen=new Set()
 if(known.size!==planned.length)throw Error('Duplicate planned case')
 for(const r of reviews){if(!known.has(r.id)||seen.has(r.id))throw Error('Unknown or duplicate review');seen.add(r.id)}
 let useful=0,retained=0,expected=0,claims=0,supported=0,critical=0,executed=0
 for(const p of planned){
  const atoms=new Set(p.atoms);if(atoms.size!==p.atoms.length)throw Error('Duplicate planned atom')
  expected+=atoms.size
  const r=reviews.find(r=>r.id===p.id)
  if(!r)continue
  if(r.executed)executed++
  if(!r.delivered)continue
  if(r.useful)useful++
  retained+=[...new Set(r.retained_atoms??[])].filter(a=>atoms.has(a)).length
  for(const c of r.claims??[]){claims++;if(c.supported)supported++;else if(c.critical)critical++}
 }
 return {planned:planned.length,executed,not_executed:planned.length-executed,useful,expected_atoms:expected,retained_atoms:retained,delivered_claims:claims,supported_claims:supported,critical_delivered:critical,fidelity:claims?supported/claims:null,availability:planned.length?useful/planned.length:null,completeness:expected?retained/expected:null}
}
