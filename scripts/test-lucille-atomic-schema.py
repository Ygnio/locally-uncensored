"""The actual broker and TS helper must constrain the same source without repairs."""
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import unittest
sys.dont_write_bytecode = True
repo=Path(__file__).resolve().parents[1]
spec=importlib.util.spec_from_file_location('broker',repo/'scripts/lucille-real-broker.py')
broker=importlib.util.module_from_spec(spec);spec.loader.exec_module(broker)
class AtomicGrammar(unittest.TestCase):
    def test_shared_schema_and_complete_source(self):
        source='Eu não tenho mais 18 GB, agora são 36 GB. No projeto aurora, decidi usar SQLite; Meu vizinho usa 80 GB!'
        script="import {atomicSchemaFor} from './src/lib/lucille-atomic.ts';const x=JSON.parse(process.argv[1]);process.stdout.write(JSON.stringify(atomicSchemaFor(...x)))"
        actual=json.loads(subprocess.check_output(['node','--input-type=module','-e',script,json.dumps([source,'qa','aurora'])],cwd=repo,text=True,encoding='utf-8'))
        self.assertEqual(actual,broker.atomic_schema_for(source,'qa','aurora'))
        spans=actual['properties']['claims']['items']['properties']['evidence']['enum']
        self.assertEqual(len(spans),3);self.assertIn('Meu vizinho usa 80 GB',spans)
        self.assertNotIn('agora são 36 GB',spans)
    def test_boundaries(self):
        for source in ['', 'é'*6001, ';'.join('claim'+str(i) for i in range(65))]:
            with self.assertRaises(ValueError):broker.atomic_schema_for(source,'s','p')
        self.assertEqual(len(broker.atomic_schema_for('claim;'*100,'s','p')['properties']['claims']['items']['properties']['evidence']['enum']),1)
if __name__=='__main__':unittest.main()
