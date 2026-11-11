"""Seal Phase 2.2F sources, inputs and isolated artifacts."""
from pathlib import Path
from runpy import run_path


if __name__ == '__main__':
    run_path(str(Path(__file__).with_name('freeze-lucille-phase22e.py')))['cli']('22f')
