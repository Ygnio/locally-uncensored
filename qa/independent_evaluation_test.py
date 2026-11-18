"""Offline controller rehearsal. The temporary transport never opens GUI or inference ports."""
import copy
import json
import runpy
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

ROOT=Path(__file__).resolve().parents[1]
sys.dont_write_bytecode=True
runner=runpy.run_path(str(ROOT/'scripts/lucille-independent.py'))
score=runpy.run_path(str(ROOT/'qa/independent_score.py'))['summarize']
env=runner['campaign'].__globals__
suite=env['SUITE']
read=env['read']
inputs=read(suite/'inputs.json')
gold=read(suite/'oracle.json')


class Independent(unittest.TestCase):
    def test_structure(self):
        env['validate'](inputs,gold,read(suite/'config.json'))
        poisoned=copy.deepcopy(gold)
        poisoned['cases'][0]['authorized_facts'].append('another-project')
        with self.assertRaises(RuntimeError):
            env['validate'](inputs,poisoned,read(suite/'config.json'))
        for i in inputs:
            body=env['control_for'](i,{})
            self.assertEqual(body['plan']['turns'][0]['text'],i['text'])
            self.assertNotIn('required_claims',json.dumps(body))

    def test_prompt(self):
        item={'text':'Pergunta de ensaio','scope':'x','project':'y'}
        call={'kind':'gemma','request':{'messages':[{'role':'user','content':item['text']}]}}
        self.assertTrue(env['audit_prompt'](item,[call],{'memories':[]}))
        call['request']['messages'].insert(0,{'role':'assistant','content':'history'})
        with self.assertRaises(RuntimeError): env['audit_prompt'](item,[call],{'memories':[]})

    def test_aggregation(self):
        rows=[dict(id=c['id'],status='observed',delivered=True,complete=True,useful=True,faithful=True,
                   assertions=[dict(text='fixture',basis='fixture',supported=True)],drafts=['correct_accepted'],
                   retrieval=[dict(fact=x['fact'],relevant=True,native_version='fixture:1') for x in c['required_references']],
                   evidence='fixture') for c in gold['cases']]
        ledger=dict(queries=rows,materialization=dict.fromkeys(gold['facts'],'correct'),safety_events=[],closed_cleanly=True)
        self.assertEqual(score(gold,ledger)['status'],'pass')
        for r in rows[:2]: r.update(delivered=False,complete=False,useful=False,faithful=False,assertions=[])
        self.assertEqual(score(gold,ledger)['status'],'pass')
        rows[2]['complete']=False
        self.assertEqual(score(gold,ledger)['status'],'fail')
        rows[2]['status']='pending'
        self.assertEqual(score(gold,ledger)['status'],'incomplete')
        ledger['safety_events']=[dict(kind='premature',evidence='fixture')]
        self.assertEqual(score(gold,ledger)['status'],'fail')

    def rehearse(self,stop):
        hosts=[]; services=[]
        with tempfile.TemporaryDirectory() as tmp:
            base=Path(tmp); reports=base/'reports'; reports.mkdir()
            (reports/'control.json').write_text('{}')
            def launch(p):
                hosts.append(p)
                return {'pid':len(hosts),'created':str(len(hosts))}
            def dispatch(scope,i,seen,replay,initialize):
                if replay: return {'replays':[{'canonical_unchanged':True,'tombstones_unchanged':True}]}
                env['write'](base/'run'/(i['id']+'-review.json'),dict(decision='critical' if stop else 'ok',note='fixture'))
                return {'turns':[{'text':i['text'],'mode':i['mode'],'conversation_id':i['id']}]}
            with patch.dict(env,dict(QA_ROOT=base,REPORTS=reports,RUN=base/'run',BUDGET=base/'budget',
                frozen=lambda commit:{'application_commit':'fixture'},environment=lambda:read(suite/'runtime-lock.json'),
                profile=lambda p:base/p,new_profile=lambda p:(base/p).mkdir(),gui_start=launch,gui_stop=lambda p:None,
                lifecycle=services.append,integrity=lambda:104,port_open=lambda n:False,dispatch=dispatch,
                db_state=lambda path:{'memories':[]})):
                if stop:
                    with self.assertRaisesRegex(RuntimeError,'Critical observation'):env['campaign']('fixture')
                else: env['campaign']('fixture')
                states=read(base/'run/closure.json')['states']
                self.assertEqual(list(states.values()).count('pending'),67 if stop else 0)
                self.assertEqual(len(hosts),1 if stop else 15)
                self.assertEqual(services,['start','stop'])

    def test_complete_schedule(self): self.rehearse(False)
    def test_critical_stop(self): self.rehearse(True)


if __name__=='__main__': unittest.main()
