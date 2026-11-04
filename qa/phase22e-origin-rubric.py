"""Independent literal-origin rubric, not the application's attribution parser."""
import hashlib,json

def audit_origin(details,case):
    violations=[]
    source=case['text'].encode('utf-8')
    for d in details:
        q=d['quality'];m=d['memory']
        gold=[g for g in case['expected'] if g['kind']==m['input']['kind'] and g['subject']==q['subject'] and g.get('property',q['property'])==q['property'] and ('property_prefix' not in g or q['property'].startswith(g['property_prefix']))]
        allowed={span for g in gold for span in g.get('allowed_evidence',[])}
        if not allowed:
            violations.append({'id':m['input']['id'],'reason':'No independently allowed USER evidence for this canonical claim'})
            continue
        provenance=d.get('provenance',[])
        if not provenance or any(p['evidence'] not in allowed for p in provenance):
            violations.append({'id':m['input']['id'],'reason':'Persisted provenance is outside the literal own-source gold'})
            continue
        try:
            proof=json.loads(q['evidence_rule'].split('|native_source_authority_v1:',1)[1])
            extent=source[proof['start']:proof['end']].decode('utf-8')
            valid=proof['message_author']=='user' and proof['statement_author']=='user' and proof['subject']==q['subject'] and proof['origin']=='personal_declaration' and proof['source_digest']==hashlib.sha256(source).hexdigest() and extent in allowed and proof['evidence_digest']==hashlib.sha256(extent.encode()).hexdigest()
        except (KeyError,ValueError,IndexError,TypeError,UnicodeError):valid=False
        if not valid:violations.append({'id':m['input']['id'],'reason':'Missing or inconsistent source/range/author proof against independent literal gold'})
    return violations

if __name__=='__main__':
    import unittest,copy
    class RubricTest(unittest.TestCase):
        def test_correct_value_cannot_hide_wrong_origin(self):
            own='Eu prefiro café';foreign='a personagem prefere café'
            case={'text':own+', mas '+foreign+'.','expected':[{'kind':'preference','subject':'user','property':'beverage_preference','value':'coffee','temporal':'preference','allowed_evidence':[own]}]}
            proof={'message_author':'user','statement_author':'user','subject':'user','origin':'personal_declaration','source_digest':hashlib.sha256(case['text'].encode()).hexdigest(),'start':0,'end':len(own.encode()),'evidence_digest':hashlib.sha256(own.encode()).hexdigest()}
            d={'memory':{'input':{'id':'m','kind':'preference'}},'quality':{'subject':'user','property':'beverage_preference','value':'coffee','temporal':'preference','evidence_rule':'native|native_source_authority_v1:'+json.dumps(proof)},'provenance':[{'evidence':own}]}
            self.assertEqual(audit_origin([d],case),[])
            bad=copy.deepcopy(d);bad['provenance'][0]['evidence']=foreign
            self.assertEqual(len(audit_origin([bad],case)),1)
            # Even model/native metadata claiming USER cannot override literal gold.
            bad=copy.deepcopy(d);start=case['text'].encode().index(foreign.encode());proof.update(start=start,end=start+len(foreign.encode()),evidence_digest=hashlib.sha256(foreign.encode()).hexdigest());bad['quality']['evidence_rule']='native|native_source_authority_v1:'+json.dumps(proof)
            self.assertEqual(len(audit_origin([bad],case)),1)
    unittest.main()
