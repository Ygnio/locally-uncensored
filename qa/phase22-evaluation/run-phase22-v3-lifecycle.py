"""Sequential real lifecycle after longitudinal series; no algorithm edits."""
import importlib.util,json,subprocess,sys,time,datetime
from pathlib import Path
here=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('series',here/'run-phase22-v3-series.py');s=importlib.util.module_from_spec(spec);spec.loader.exec_module(s)
repo=s.repo;root=s.root;reports=s.reports;plan=json.loads((s.fixtures/'lifecycle-v3.json').read_text(encoding='utf-8'))
def owned(operation):s.run([sys.executable,'scripts/run-lucille-phase22-owned.py','--root',str(root),'--operation',operation],f'phase22-v3-{operation}-{time.time_ns()}.log')
def read(name):return json.loads((reports/name).read_text(encoding='utf-8'))
def main():
 while not (reports/'v3-long-series-summary.json').exists():
  if (root/'gemma.stop').exists():raise RuntimeError('Owned model stopped before lifecycle')
  time.sleep(3)
 summary={'started':datetime.datetime.now(datetime.timezone.utc).isoformat()}
 # Only this host/Vite pair restarts. Native launcher checks PID/path/creation/command.
 summary['before_host_restart_manifest']=json.loads((root/'gui-processes.json').read_text())
 s.stop();s.start();final=s.control('heldout-v3-host-restart',plan['host_restart']);summary['host_restart_scores']=s.score(final,'heldout-v3-host-restart')
 f=read(final.name);before=read('driver-heldout-v3-long100-final.json')
 summary['host_restart_persistence']={'identity_equal':f['initial_identity']==before['identity'],'index_space_equal':f['initial_index']==before['index'],'details_equal':f['initial_details']==before['final_details'],'tombstones_equal':f['initial_deleted']==before['deleted'],'scope_bindings_restored':len(f['existing_bindings']),'notes':'Read snapshots before any post-restart turn. Global index_status may include the other project; compare identity/index metadata literally and explain differences.'}
 summary['nomic_before']=json.loads((root/'nomic-process.json').read_text());owned('nomic-stop');start=time.monotonic()
 final=s.control('heldout-v3-nomic-down',plan['nomic_down'],allow_pending_index=True);summary['nomic_down_scores']=s.score(final,'heldout-v3-nomic-down');summary['nomic_outage_duration_before_restart_seconds']=time.monotonic()-start
 summary['nomic_outage_metrics']=read(final.name)['final_metrics'];owned('nomic-restart');summary['nomic_after']=json.loads((root/'nomic-process.json').read_text());start=time.monotonic()
 final=s.control('heldout-v3-nomic-recovery',plan['nomic_recovery']);summary['nomic_recovery_scores']=s.score(final,'heldout-v3-nomic-recovery');summary['post_ready_recovery_stage_seconds']=time.monotonic()-start
 summary['finished']=datetime.datetime.now(datetime.timezone.utc).isoformat();(reports/'v3-lifecycle-review.json').write_text(json.dumps(summary,indent=2),encoding='utf-8');print('Lifecycle completed; actual extraction faults remain separately controlled',flush=True)
if __name__=='__main__':main()
