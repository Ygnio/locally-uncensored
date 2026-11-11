"""Evaluate Phase 2.2F proposals on the pinned real models and native Store."""
from pathlib import Path
from runpy import run_path


if __name__ == '__main__':
    run_path(str(Path(__file__).with_name('test-lucille-phase22e-extraction.py')))['cli']('22f')
