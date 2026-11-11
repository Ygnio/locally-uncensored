"""Drive the isolated Phase 2.2F Tauri host through its real DOM observer."""
from pathlib import Path
from runpy import run_path


if __name__ == '__main__':
    run_path(str(Path(__file__).with_name('test-lucille-phase22e-control.py')))['cli']('22f')
