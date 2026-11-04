"""Sanitize only metadata paths; retain synthetic assertions and negative outcomes."""
import hashlib,json,re
from pathlib import Path
repo=Path(__file__).resolve().parents[2];root=repo/'.lucille-qa-runtime/lucille-synthetic-real22';reports=root/'reports';dest=repo/'docs/evidence/phase22';dest.mkdir(parents=True,exist_ok=True)
replacements=[(str(repo),'<QA_CHECKOUT>'),(str(repo).replace('\\','/'),'<QA_CHECKOUT>'),(str(repo.parent),'<QA_WORK>'),(str(repo.parent).replace('\\','/'),'<QA_WORK>'),(str(Path.home()),'<USER_HOME>'),(str(Path.home()).replace('\\','/'),'<USER_HOME>')]
replacements.append(('America/Sao_Paulo','<LOCAL_TIMEZONE>'))
replacements=[(a.replace('\\','\\\\'),b) for a,b in replacements if '\\' in a]+replacements
def clean(x):
 if isinstance(x,dict):return {clean(k):clean(v) for k,v in x.items()}
 if isinstance(x,list):return [clean(v) for v in x]
 if isinstance(x,str):
  for a,b in replacements:x=x.replace(a,b)
  return x
 return x
mapping=[]
def copy(path,name=None):
 if not path.exists():return
 name=name or path.name;target=dest/name;raw=path.read_bytes();text=raw.decode('utf-8-sig')
 if path.suffix=='.ndjson':out=''.join(json.dumps(clean(json.loads(s)),ensure_ascii=False)+'\n' for s in text.splitlines() if s.strip()).encode()
 else:out=(json.dumps(clean(json.loads(text)),indent=2,ensure_ascii=False)+'\n').encode()
 for secret_path in root.glob('*.api-key'):
  secret=secret_path.read_text(encoding='ascii').strip()
  assert not secret or secret.encode() not in out,'Actual QA key must never be published'
 assert not re.search(rb'C:[\\/]+Users[\\/]+'+re.escape(Path.home().name.encode()),out,re.I),'Unredacted personal home path'
 target.write_bytes(out);mapping.append({'public_file':name,'original_sha256':hashlib.sha256(raw).hexdigest(),'public_sha256':hashlib.sha256(out).hexdigest(),'redaction':'Known absolute QA/work/user-home paths and local timezone metadata; JSON whitespace normalized. Synthetic facts and metric values unchanged.'})
for name in ['final-v1-extraction.json','final-v1-extraction-progress.ndjson','challenge-v1-extraction.json','challenge-v1-extraction-progress.ndjson','phase22-retrieval-final.json','final-v2-extraction.json','final-v2-extraction-progress.ndjson','v1-series-interrupted-by-critical-finding.json','v2-to-v3-development-reason.json','resource-instrumentation-change.json','qa-report-route-security.json','v3-measurement-recovery.json','v3-bridge-rejections.ndjson','final-v3-extraction.json','final-v3-extraction-progress.ndjson','v3-long-series-summary.json','near-context-final.json']:
 copy(reports/name)
copy(root/'evaluation-v3/lucille-synthetic-evaluation-v3/reports/phase22-retrieval-final.json','retrieval-v3-final.json')
for path in reports.glob('driver-*-final.json'):copy(path)
for path in reports.glob('heldout-v3-*-scored.json'):copy(path)
for path in reports.glob('v3-*-scored.json'):copy(path)
for path in reports.glob('v3-*-review.json'):copy(path)
for path in reports.glob('v3-*-fault.json'):copy(path)
for path in reports.glob('*-capture.json'):copy(path)
for path in [repo.parent/'qa/phase22-initial-audit.json',repo.parent/'qa/phase22-initial-resources.json']:copy(path)
for name in ['attested-contract.json','gemma-process.json','nomic-process.json','gui-processes.json']:
 copy(root/name)
for name in ['system-resources-final.ndjson','system-resources-v2.ndjson','driver-heldout-long20-progress.json','v3-forget-inline-scoring-input.json','v3-fresh-probe-payloads.json']:
 copy(reports/name)
for directory in [reports,*sorted(reports.glob('epoch-*'))]:
 for kind in ['gemma','nomic']:
  copy(directory/(kind+'-resources.ndjson'),'resources-'+directory.name+'-'+kind+'.ndjson')
  if directory!=reports:copy(directory/(kind+'-process.json'),'manifest-'+directory.name+'-'+kind+'.json')
for name in ['phase22-core-v3-final.log','phase22-typescript-memory-final.log','phase22-native-parity-v3-final.log','phase22-typecheck-development2.log','phase22-qa-typecheck.log','phase22-qa-config-typecheck-final.log','phase22-eslint-v3-final.log','phase22-schema-v3-final.log','phase22-native-build-v3-final.log','phase22-build-v3-final.log','phase22-extraction-final-v3.log','phase22-retrieval-final-v2.log']:
 path=repo.parent/'logs'/name
 if not path.exists():continue
 raw=path.read_bytes();encoding='utf-8-sig'
 try:decoded=raw.decode(encoding)
 except UnicodeDecodeError:
  encoding='utf-16' if raw.startswith((b'\xff\xfe',b'\xfe\xff')) else 'cp1252';decoded=raw.decode(encoding)
  if encoding=='cp1252':assert decoded.encode(encoding)==raw
 out=clean(decoded).replace('\r\n','\n').encode('utf-8');(dest/name).write_bytes(out);mapping.append({'public_file':name,'original_sha256':hashlib.sha256(raw).hexdigest(),'public_sha256':hashlib.sha256(out).hexdigest(),'input_encoding':encoding,'redaction':'Known metadata paths/timezone removed; text decoded strictly in recorded encoding, newlines normalized to LF, published as UTF-8. No replacement-character decoding; original hash preserved.'})
(dest/'redactions-and-hashes.json').write_bytes((json.dumps({'files':mapping,'private_data_excluded':['SQLite files','model weights','binaries','API-key files','unreviewed operational/profile data'],'note':'Only isolated synthetic QA selected; original hashes are not hashes of the redacted public representation.'},indent=2)+'\n').encode('utf-8'))
print('Packaged',len(mapping),'UTF-8 evidence files')
