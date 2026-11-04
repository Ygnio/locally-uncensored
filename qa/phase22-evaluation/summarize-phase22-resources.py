"""Read-only bounded resource summary. No extrapolated performance claims."""
import json,statistics,datetime
from pathlib import Path
repo=Path(__file__).resolve().parents[2];root=repo/'.lucille-qa-runtime/lucille-synthetic-real22';reports=root/'reports'
def nd(path):return [json.loads(x) for x in path.read_text(encoding='utf-8').splitlines() if x.strip()]
def distribution(values):
 if not values:return {'n':0}
 v=sorted(values);return {'n':len(v),'minimum':v[0],'median':statistics.median(v),'p95_nearest_rank':v[max(0,__import__('math').ceil(.95*len(v))-1)],'maximum':v[-1]}
epochs=[];all_samples=[]
for directory in [reports,*sorted(reports.glob('epoch-*'))]:
 for kind in ['gemma','nomic']:
  resource=directory/f'{kind}-resources.ndjson';manifest=(root if directory==reports else directory)/f'{kind}-process.json'
  if not resource.exists() or not manifest.exists():continue
  m=json.loads(manifest.read_text(encoding='utf-8'));samples=nd(resource)
  if not samples:continue
  start=m['started_unix'];all_samples.extend([{**s,'sample_unix':start+s['elapsed_s'],'sample_kind':kind} for s in samples])
  epochs.append({'epoch':directory.name,'kind':kind,'pid':m['pid'],'started_unix':start,'samples':len(samples),'elapsed_last_seconds':samples[-1]['elapsed_s'],'physical_available_bytes':distribution([s['PhysicalAvailable'] for s in samples]),'commit_total_bytes':distribution([s['CommitTotal'] for s in samples]),'commit_available_bytes':distribution([s['CommitLimit']-s['CommitTotal'] for s in samples]),'own_process_working_set_bytes':distribution([s['process_working_set'] for s in samples]),'own_process_private_bytes':distribution([s['process_private_bytes'] for s in samples])})
calls=nd(reports/'real-broker-calls.ndjson');latencies={}
for c in calls:
 typ='embeddings' if c['kind']=='nomic' else 'extraction' if any(str(x.get('content','')).startswith('LUCILLE_ATOMIC_EXTRACTION_') for x in c['request'].get('messages',[])) else 'primary_generation'
 latencies.setdefault(typ,[]).append(c['latency_seconds'])
system=[]
for path in [reports/'system-resources-final.ndjson',reports/'system-resources-v2.ndjson']:
 if path.exists():system.extend(nd(path))
pages=[(s['pagefile'] if isinstance(s.get('pagefile'),list) else [s.get('pagefile')]) for s in system];pages=[sum(x['CurrentUsage'] for x in p if x) for p in pages]
gpu=[]
for s in all_samples:
 try:gpu.append(int(s['gpu_csv'].split(',')[0].strip()))
 except (ValueError,KeyError):pass
result={'generated_utc':datetime.datetime.now(datetime.timezone.utc).isoformat(),'epochs':epochs,'scenario_A_measured':{'physical_available_bytes':distribution([s['PhysicalAvailable'] for s in all_samples]),'commit_available_bytes':distribution([s['CommitLimit']-s['CommitTotal'] for s in all_samples]),'gpu_used_mib':distribution(gpu),'pagefile_current_usage_mib':distribution(pages),'pagefile_scope':'Only timestamped sampler observations. Original sampler failed during GUI transition; pagefile gap approx 19:15–19:46 local retained, not reconstructed. Model RAM/commit telemetry is separate. PeakUsage is OS lifetime, not task peak.'},'scenario_B_projection_only':{'additional_physical_available_bytes':2*1024**3,'minimum_projected_physical_available_bytes':min(s['PhysicalAvailable'] for s in all_samples)+2*1024**3,'not_measured':['latency','VRAM','commit','pagefile'],'note':'Arithmetic projection only. Existing ChatGPT and applications were not closed.'},'broker_latencies_seconds':{k:distribution(v) for k,v in latencies.items()},'notes':['Whole-run latency spans development, failed and final evaluations, not an independent final-corpus aggregate.','Model telemetry samples overlap between Gemma and Nomic; sample count is not independent observations.','Minimum is a sampled minimum; no continuous-time guarantee or leak-free certification.','Build periods in system sampler must not be described as inference measurements.']}
stages={}
for path in sorted(reports.glob('driver-heldout-v3-*-final.json')):
 f=json.loads(path.read_text(encoding='utf-8'));start=datetime.datetime.fromisoformat(f['started'].replace('Z','+00:00')).timestamp();end=datetime.datetime.fromisoformat(f['finished'].replace('Z','+00:00')).timestamp();samples=[s for s in all_samples if start<=s['sample_unix']<=end];sys_samples=[s for s in system if start<=datetime.datetime.fromisoformat(s['utc'].replace('Z','+00:00')).timestamp()<=end]
 stages[f['label']]={'duration_seconds':end-start,'body_plus_probe_turns':len(f['turns']),'fresh_probes':len(f['probes']),'physical_available_bytes':distribution([s['PhysicalAvailable'] for s in samples]),'commit_available_bytes':distribution([s['CommitLimit']-s['CommitTotal'] for s in samples]),'generation_ms':distribution([t['generation_ms'] for t in f['turns']]),'extraction_wait_ms':distribution([t['extraction_wait_ms'] for t in f['turns']]),'initial_metrics':f['initial_metrics'],'final_metrics':f['final_metrics'],'native_gui_working_set_bytes':distribution([s['own_gui']['working_set'] for s in sys_samples if s.get('own_gui')]),'pagefile_current_usage_mib':distribution([sum(x['CurrentUsage'] for x in (s['pagefile'] if isinstance(s['pagefile'],list) else [s['pagefile']])) for s in sys_samples if s.get('pagefile')]),'notes':['SQLite page bytes are global logical database size, not per-project allocation. Vector/provenance/job counters are scoped snapshots.','Native GUI PID RSS excludes WebView descendants; physical total includes all concurrent apps.','Generation/extraction waits are observed end-to-end GUI timings, including polling; not pure model kernel time.']}
result['gui_stages']=stages
gpu_free=[]
for s in all_samples:
 try:gpu_free.append(int(s['gpu_csv'].split(',')[1].strip()))
 except (ValueError,KeyError,IndexError):pass
result['scenario_A_measured']['gpu_free_mib']=distribution(gpu_free)
near=reports/'near-context-final.json'
if near.exists():
 n=json.loads(near.read_text(encoding='utf-8'));samples=[s for s in all_samples if n['started']<=s['sample_unix']<=n.get('finished',n['started'])]
 result['near_context_phase']={'gui_and_nomic_owned_processes_stopped':True,'physical_available_bytes':distribution([s['PhysicalAvailable'] for s in samples]),'commit_available_bytes':distribution([s['CommitLimit']-s['CommitTotal'] for s in samples]),'source_tokens_without_template':n.get('source_tokens_without_chat_template'),'actual_usage':n.get('response',{}).get('usage'),'elapsed_seconds':n.get('elapsed_seconds'),'marker_recall_correct':n.get('marker_recall_correct'),'note':'Separate serialized context stress. Not simultaneous full-GUI/Nomic evidence and not persistent-memory recall.'}
(reports/'v3-resources-review.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8');print(json.dumps({'epochs':len(epochs),'scenario_A':result['scenario_A_measured'],'latencies':result['broker_latencies_seconds']}))
