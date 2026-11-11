"""Read-only attestation for actual E GUI fault/transport evaluations.

Source/corpus verification is separate. A saved pin or old ready file alone
cannot attest the currently running GUI and model servers.
"""
import ctypes
from ctypes import wintypes
import datetime
import hashlib
import json
import os
from pathlib import Path
import time
import urllib.request


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def utc(value):
    result = datetime.datetime.fromisoformat(value.replace('Z', '+00:00'))
    assert result.tzinfo is not None, 'Timestamp must carry its timezone'
    return result.astimezone(datetime.timezone.utc)


def read_report(path, timeout=120):
    """Wait for a whole JSON report, without accepting a partial write as data.

    The producer creates final files exclusively, but creation can precede the
    end of its write. Permanently malformed reports still fail within the bound.
    """
    path = Path(path)
    until = time.monotonic() + timeout
    error = None
    while time.monotonic() < until:
        try:
            return json.loads(path.read_text(encoding='utf-8'))
        except (FileNotFoundError, json.JSONDecodeError, UnicodeDecodeError) as current:
            error = current
            time.sleep(.05)
    raise RuntimeError('Whole QA report unavailable within timeout: ' + path.name) from error


def running_gui(record, expected):
    assert os.name == 'nt', 'This native GUI attestation requires Windows'
    kernel = ctypes.WinDLL('kernel32', use_last_error=True)
    kernel.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
    kernel.OpenProcess.restype = wintypes.HANDLE
    handle = kernel.OpenProcess(0x1000 | 0x100000, False, record['pid'])
    if not handle:
        raise OSError('Manifest GUI process is unavailable; old ready data is insufficient')
    try:
        kernel.WaitForSingleObject.argtypes = [wintypes.HANDLE, wintypes.DWORD]
        assert kernel.WaitForSingleObject(handle, 0) == 0x102, 'Manifest GUI has exited'
        image = ctypes.create_unicode_buffer(32768)
        size = wintypes.DWORD(len(image))
        kernel.QueryFullProcessImageNameW.argtypes = [wintypes.HANDLE, wintypes.DWORD, wintypes.LPWSTR, ctypes.POINTER(wintypes.DWORD)]
        assert kernel.QueryFullProcessImageNameW(handle, 0, image, ctypes.byref(size))
        assert Path(image.value).resolve(strict=True) == expected, 'GUI process image changed'
        times = [wintypes.FILETIME() for _ in range(4)]
        kernel.GetProcessTimes.argtypes = [wintypes.HANDLE, *([ctypes.POINTER(wintypes.FILETIME)] * 4)]
        assert kernel.GetProcessTimes(handle, *[ctypes.byref(t) for t in times])
        ticks = times[0].dwHighDateTime << 32 | times[0].dwLowDateTime
        created = datetime.datetime(1601, 1, 1, tzinfo=datetime.timezone.utc) + datetime.timedelta(microseconds=ticks // 10)
        assert created == utc(record['created']), 'GUI process creation identity changed'
    finally:
        kernel.CloseHandle.argtypes = [wintypes.HANDLE]
        kernel.CloseHandle(handle)


def artifact_paths(repo, phase):
    build = repo.parent / 'build'
    if phase == '22f':
        return build / 'phase22f/lucille-full-validation.exe', build / 'phase22f/real_validation_bridge.exe'
    assert phase == '22e'
    return (build / 'tauri/x86_64-pc-windows-msvc/debug/lucille-full-validation.exe',
            build / 'memory/debug/examples/real_validation_bridge.exe')


def verify_artifacts(repo, root, freeze, phase='22e'):
    repo = Path(repo).resolve(strict=True)
    root = Path(root).resolve(strict=True)
    assert root == repo / '.lucille-qa-runtime' / ('lucille-synthetic-real' + phase)
    assert (root / '.lucille-isolated').read_bytes() == b'synthetic-data-only\n'
    reports = root / 'reports'
    gui = json.loads((root / 'gui-processes.json').read_text(encoding='utf-8'))
    assert Path(gui['real_root']).resolve(strict=True) == root
    assert Path(gui['root']).resolve(strict=True) == repo / '.lucille-qa-runtime' / ('lucille-synthetic-full' + phase)
    native, bridge = artifact_paths(repo, phase)
    native = native.resolve(strict=True)
    assert Path(gui['gui']['executable']).resolve(strict=True) == native
    assert digest(native) == freeze['native_gui_sha256'], 'Frozen native GUI changed'
    assert digest(bridge) == freeze['bridge_sha256'], 'Frozen native bridge changed'
    if phase == '22f':
        dependencies = {p.name:digest(p) for p in sorted(native.parent.glob('*.dll')) if p.is_file()}
        assert dependencies == freeze['native_local_dependencies_sha256'], 'Frozen native DLL inventory changed'
    static = repo / ('qa-dist-' + phase)
    assert (static / '.lucille-static-build').read_bytes() == b'synthetic-data-only\n'
    actual = {p.relative_to(static).as_posix(): digest(p) for p in static.rglob('*') if p.is_file()}
    assert actual and actual == freeze['frontend_sha256'], 'Frozen frontend inventory/hash changed'
    expected_pin = freeze['model_pin']
    assert expected_pin['simulation'] is False
    assert json.loads((reports / 'real-public-pin.json').read_text(encoding='utf-8')) == expected_pin, 'Saved model pin changed'
    # Both live endpoints validate the owned server process identity. These are
    # manifest reads, not embedding or generation requests.
    for kind in ['extraction', 'embedding']:
        with urllib.request.urlopen('http://127.0.0.1:15284/lucille/' + kind + '-manifest', timeout=15) as response:
            live = json.load(response)
        assert {key: live.get(key) for key in expected_pin} == expected_pin, 'Live model/runtime pin differs from freeze'
        assert live['context_tokens'] == 65536, 'Gemma context changed'
    ready = json.loads((reports / 'driver-ready-progress.json').read_text(encoding='utf-8'))
    assert ready['native'] is True and ready['href'] == 'http://127.0.0.1:15283/'
    assert utc(ready['ready']) >= utc(gui['gui']['created']), 'Stale native ready observation'
    running_gui(gui['gui'], native)
    return {'native_gui_verified': True, 'bridge_verified': True, 'frontend_verified': True,
            'live_model_pin_verified': True, 'ready': ready['ready'], 'gui_pid': gui['gui']['pid']}
