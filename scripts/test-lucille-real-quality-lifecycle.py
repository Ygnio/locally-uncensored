"""Canonical version/space lifecycle using previously measured real Nomic vectors."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import sqlite3
import subprocess
import sys
import uuid
sys.dont_write_bytecode=True
spec=importlib.util.spec_from_file_location('helper',Path(__file__).with_name('test-lucille-real-extraction.py'));h=importlib.util.module_from_spec(spec);spec.loader.exec_module(h)
def main(a):
 root=Path(a.root).resolve(strict=True);assert (root/'.lucille-isolated').read_bytes()==b'synthetic-data-only\n'
 evidence=Path(a.evidence);data=json.loads(evidence.read_text(encoding='utf-8'));space=data['space'];assert space['dimension']==768
 old=next(x for x in data['seed_evidence'] if x['event']['label']=='coffee');new=next(x for x in data['seed_evidence'] if x['event']['label']=='primary');store=root/'regression/lucille-synthetic-vector-lifecycle'
 def native(op):return h.bridge(Path(a.bridge).resolve(),store,op)
 native({'operation':'learn','mode':'rules','text':'Synthetic neutral initialization only.','message_id':'init'})
 native({'operation':'configure_embedding','space':space});m=native({'operation':'create','candidate':{'id':str(uuid.uuid5(uuid.NAMESPACE_URL,'phase21d-actual-vector-lifecycle')),'scope':'lucille','project':'atlas','kind':'fact','content':old['representation']['text'],'fact_key':'lifecycle','source':'manual','source_id':'previous-actual-Nomic-evidence'}});m=native({'operation':'change','id':m['input']['id'],'version':m['version'],'action':{'action':'approve'}})
 native({'operation':'put_vector','id':m['input']['id'],'version':m['version'],'space':space,'vector':old['vector']})
 def count():
  with sqlite3.connect((store/'synthetic.sqlite3').as_uri()+'?mode=ro',uri=True) as db:return db.execute("SELECT count(*) FROM memory_vectors v JOIN memories m ON m.id=v.memory_id WHERE m.state='active' AND m.version=v.memory_version").fetchone()[0]
 assert count()==1;old_version=m['version'];m=native({'operation':'change','id':m['input']['id'],'version':m['version'],'action':{'action':'update','content':new['representation']['text'],'fact_key':'lifecycle'}});assert count()==0;m=native({'operation':'change','id':m['input']['id'],'version':m['version'],'action':{'action':'approve'}});assert count()==0
 def rejected(spec,version):
  try:native({'operation':'put_vector','id':m['input']['id'],'version':version,'space':spec,'vector':old['vector']})
  except subprocess.CalledProcessError as error:return 'Stale memory version or embedding space' in error.stderr
  return False
 assert rejected(space,old_version)
 native({'operation':'put_vector','id':m['input']['id'],'version':m['version'],'space':space,'vector':new['vector']});assert count()==1
 changed={**space,'index_version':space['index_version']+1};native({'operation':'configure_embedding','space':changed});assert count()==0;assert rejected(space,m['version'])
 native({'operation':'put_vector','id':m['input']['id'],'version':m['version'],'space':changed,'vector':new['vector']});assert count()==1
 native({'operation':'policy','lexical_weight':0,'minimum_similarity':.6})
 request={'conversation_id':'lifecycle','message_id':'fresh','scope':'lucille','project':'atlas','mode':'code','query':'What memory is in my primary computer?','token_budget':8192}
 result=native({'operation':'context','request':request,'query_vector':{'space':changed,'vector':new['vector']}});assert [x['input']['id'] for x in result['memories']]==[m['input']['id']]
 native({'operation':'change','id':m['input']['id'],'version':m['version'],'action':{'action':'delete'}});assert count()==0
 assert not native({'operation':'context','request':{**request,'message_id':'deleted'},'query_vector':{'space':changed,'vector':new['vector']}})['memories']
 report={'passed':True,'actual_cached_nomic_vectors':True,'new_live_inference':False,'different_weights_tested':False,'space_change':'Controlled index-version namespace change with the same verified weights, not another real model download.','source_evidence_sha256':hashlib.sha256(evidence.read_bytes()).hexdigest(),'edit_invalidated_current_vector':True,'reapproval_did_not_restore_old_vector':True,'old_version_write_rejected':True,'old_space_write_rejected':True,'space_change_cleared_vectors':True,'explicit_reindex_restored_current_context':True,'delete_removed_current_vector_and_context':True,'each_native_request_reopens_store':True,'only_canonical_store_wrote':True}
 (root/'reports/real-quality-vector-lifecycle.json').write_text(json.dumps(report,indent=2),encoding='utf-8');print(json.dumps(report))
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--root',required=True);p.add_argument('--bridge',required=True);p.add_argument('--evidence',required=True);main(p.parse_args())
