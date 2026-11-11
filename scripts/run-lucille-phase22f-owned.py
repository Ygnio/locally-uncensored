"""Phase 2.2F lifecycle for the marked isolated profile and pinned models."""
from pathlib import Path
from runpy import run_path


if __name__ == '__main__':
    run_path(str(Path(__file__).with_name('run-lucille-phase22e-owned.py')))['cli']('22f')
