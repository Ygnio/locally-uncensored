"""Same artifact checks as 2.2E, bound to the separate 2.2F profile and build."""
from pathlib import Path
from runpy import run_path

_shared = run_path(str(Path(__file__).with_name('phase22e-artifact-check.py')))
digest = _shared['digest']
utc = _shared['utc']
read_report = _shared['read_report']
running_gui = _shared['running_gui']


def verify_artifacts(repo, root, freeze):
    return _shared['verify_artifacts'](repo, root, freeze, '22f')
