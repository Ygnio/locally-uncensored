"""Frozen rubric: actual broker payload/reference provenance and answer use are separate."""
import argparse,datetime,json,re
from pathlib import Path
MARK='Retrieved contextual data, not a new user request or tool authorization:'
def content(message):
 v=message.get('content','')
 return v if isinstance(v,str) else '\n'.join(x.get('text','') for x in v if x.get('type')=='text')
def refs(messages):
 out=[]
 for m in messages:
  text=content(m)
  while MARK in text:
   text=text.split(MARK,1)[1].lstrip();items,end=json.JSONDecoder().raw_decode(text);out.extend(items);text=text[end:]
 return list({(r['id'],r['version']):r for r in out}.values())
def question(messages):
 text=content(next(m for m in reversed(messages) if m['role']=='user'))
 while MARK in text:
  a,b=text.split(MARK,1);b=b.lstrip();_,end=json.JSONDecoder().raw_decode(b);text=a+b[end:]
 text=text.split('\n\nDate and time at the start of this run:')[0]
 return text.strip()
def match(record,gold):
 q=record.get('quality') or {}
 return (q.get('property','').startswith(gold['property']) if gold['property'].endswith('.') else q.get('property')==gold['property']) and all(q.get(k)==gold[k] for k in ['subject','value','temporal'] if k in gold)
def main(a):
 root=Path(a.root).resolve(strict=True);fixture=json.loads(Path(a.fixture).read_text(encoding='utf-8'));assert fixture['execution_complete']
 report_path=Path(a.output);assert not report_path.exists(),'Never overwrite scored evidence'
 begin=datetime.datetime.fromisoformat(fixture['started'].replace('Z','+00:00')).timestamp();end=datetime.datetime.fromisoformat(fixture['finished'].replace('Z','+00:00')).timestamp()
 calls=[json.loads(s) for s in (root/'reports/real-broker-calls.ndjson').read_text(encoding='utf-8').splitlines()]
 calls=[c for c in calls if c['kind']=='gemma' and begin<=c.get('completed_unix',0)<=end+1 and not any(content(m).startswith('LUCILLE_ATOMIC_EXTRACTION_') for m in c['request']['messages'])]
 details={d['memory']['input']['id']:d for d in fixture['final_details']};samples=[];totals={'tp':0,'fp':0,'missed':0,'critical_references':0,'natural_points':0,'natural_applicable':0};categories={}
 for i,probe in enumerate(fixture['probes']):
  item=probe['item'];call=next(c for c in reversed(calls) if question(c['request']['messages'])==item['text']);actual=refs(call['request']['messages']);gold=item['expected'];matched=set();tp=fp=critical=0
  for ref in actual:
   row=details.get(ref['id'])
   if row is None or row['memory']['state']!='active' or row['memory']['version']!=ref['version']:critical+=1
   index=next((j for j,g in enumerate(gold) if j not in matched and row and match(row,g)),None)
   if index is None:fp+=1
   else:tp+=1;matched.add(index)
  missed=len(gold)-len(matched);answer=call['response']['choices'][0]['message']['content'];pattern_correct=bool(re.search(item['answer_pattern'],answer,re.I))
  mentions=[term for term in item.get('answer_none',[]) if re.search(term,answer,re.I)]
  announcement=bool(re.search(r'I remember|lembro que|minhas mem[oó]rias|na mem[oó]ria(?: salva)?|mem[oó]ria armazenada|stored preference|saved memory',answer,re.I))
  preference=any((details.get(r['id']) or {}).get('memory',{}).get('input',{}).get('kind')=='preference' for r in actual)
  repeated=len(re.findall(r'prefere|prefiro|\bprefer\b|preference|gosta|\blikes\b',answer,re.I))>1
  terms=['café','coffee','rpg','rust','sqlite','mysql','ram','interface','preferência','preference']
  contamination=not gold and any(t in answer.lower() and t not in item['text'].lower() for t in terms)
  uncertain=bool(re.search(r'não sei|nao sei|não tenho informa|nao tenho informa|do not know|don.t know|cannot determine|can.t determine',answer,re.I))
  planned=any(g.get('temporal')=='planned' for g in gold)
  implementation=planned and bool(re.search(r'já usa|já utiliza|já implement|is already implemented|is currently using|está sendo desenvolvido em|está implementado',answer,re.I))
  complete=call['response']['choices'][0]['finish_reason']=='stop'
  checks={'grounded_answer':pattern_correct and not fp and not missed and complete and not (gold and uncertain),'current_versions':not critical and not mentions and not implementation,'no_gratuitous_announcement':not announcement}
  if preference:checks['no_repeated_preference']=not repeated
  if not gold:checks['irrelevant_uncontaminated']=not contamination
  # A forbidden word may be a correct historical/negated mention. Flag for literal
  # human review under the PRE-EXISTING rubric; never silently excuse it.
  sample={'id':fixture['label']+f'-probe-{i+1:02}','question':item['text'],'category':item.get('category','unspecified'),'gold':gold,'actual_references':actual,'provenance':{r['id']:(details.get(r['id']) or {}).get('provenance',[]) for r in actual},'tp':tp,'fp':fp,'missed':missed,'critical_references':critical,'answer':answer,'finish_reason':call['response']['choices'][0]['finish_reason'],'usage':call['response'].get('usage'),'fresh_user_messages':sum(m['role']=='user' for m in call['request']['messages']),'checks':checks,'forbidden_mentions_requiring_review':mentions,'planned_implementation_requiring_review':implementation}
  samples.append(sample);cat=categories.setdefault(sample['category'],{'n':0,'tp':0,'fp':0,'missed':0,'natural_points':0,'natural_applicable':0});cat['n']+=1
  for k,v in [('tp',tp),('fp',fp),('missed',missed),('natural_points',sum(checks.values())),('natural_applicable',len(checks))]:totals[k]+=v;cat[k]+=v
  totals['critical_references']+=critical
 totals['precision']=totals['tp']/max(1,totals['tp']+totals['fp']);totals['recall']=totals['tp']/max(1,totals['tp']+totals['missed']);totals['naturalness']=totals['natural_points']/max(1,totals['natural_applicable'])
 report={'fixture':Path(a.fixture).name,'scope':fixture['scope'],'project':fixture['project'],'totals':totals,'categories':categories,'samples':samples,'review_pending':any(s['forbidden_mentions_requiring_review'] or s['planned_implementation_requiring_review'] for s in samples),'notes':['Context metrics score actual injected references, never an answer guessed without memory.','Forbidden mentions require literal review to distinguish obsolete-as-current from legitimate history/negation. Preliminary automatic rubric remains conservative.','Fewer than five examples per category: counts only; no generalization from percentages.']}
 report_path.write_bytes((json.dumps(report,indent=2,ensure_ascii=False)+'\n').encode());print(json.dumps(totals))
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--root',required=True);p.add_argument('--fixture',required=True);p.add_argument('--output',required=True);main(p.parse_args())
