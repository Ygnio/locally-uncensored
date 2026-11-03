"""Supervise one isolated llama-server; never stop a process not created here.

Windows only. A stop file cancels the owned test server. No operational launchers,
ports or settings are used. A retained process handle and creation-time/path checks
protect cleanup against PID reuse. Resource pressure stops this test only.
"""
import argparse
import ctypes as c
from ctypes import wintypes as w
import hashlib
import json
from pathlib import Path
import socket
import secrets
import subprocess
import time
import urllib.request


class Performance(c.Structure):
    _fields_ = [('cb', w.DWORD)] + [(n, c.c_size_t) for n in
        ['CommitTotal', 'CommitLimit', 'CommitPeak', 'PhysicalTotal',
         'PhysicalAvailable', 'SystemCache', 'KernelTotal', 'KernelPaged',
         'KernelNonpaged', 'PageSize']] + [(n, w.DWORD) for n in
        ['HandleCount', 'ProcessCount', 'ThreadCount']]


def resources():
    p = Performance()
    p.cb = c.sizeof(p)
    if not c.windll.psapi.GetPerformanceInfo(c.byref(p), p.cb):
        raise OSError('Cannot monitor memory; refuse inference')
    return {n: getattr(p, n) * p.PageSize for n in
            ['CommitTotal', 'CommitLimit', 'PhysicalTotal', 'PhysicalAvailable']}


def identity(process):
    handle = w.HANDLE(int(process._handle))
    name = c.create_unicode_buffer(32768)
    size = w.DWORD(len(name))
    if not c.windll.kernel32.QueryFullProcessImageNameW(handle, 0, name, c.byref(size)):
        raise OSError('Cannot verify owned process image')
    times = [w.FILETIME() for _ in range(4)]
    if not c.windll.kernel32.GetProcessTimes(handle, *(c.byref(t) for t in times)):
        raise OSError('Cannot verify owned process creation time')
    return {'pid': process.pid, 'executable': str(Path(name.value).resolve()),
            'creation_filetime': times[0].dwHighDateTime << 32 | times[0].dwLowDateTime}


def process_resources(process):
    class Counters(c.Structure):
        _fields_ = [('cb', w.DWORD), ('faults', w.DWORD)] + [(n, c.c_size_t) for n in
            ['peak_working_set', 'working_set', 'peak_paged_pool', 'paged_pool',
             'peak_nonpaged_pool', 'nonpaged_pool', 'pagefile', 'peak_pagefile', 'private_bytes']]
    info = Counters()
    info.cb = c.sizeof(info)
    if not c.windll.psapi.GetProcessMemoryInfo(w.HANDLE(int(process._handle)), c.byref(info), info.cb):
        return {'process_memory_unavailable': True}
    times = [w.FILETIME() for _ in range(4)]
    if not c.windll.kernel32.GetProcessTimes(w.HANDLE(int(process._handle)), *(c.byref(t) for t in times)):
        raise OSError('Cannot monitor owned CPU time')
    cpu = sum(t.dwHighDateTime << 32 | t.dwLowDateTime for t in times[2:]) / 10_000_000
    return {'process_working_set': info.working_set, 'process_private_bytes': info.private_bytes,
            'process_cpu_total_seconds': cpu}


def serve(args):
    root = Path(args.root).resolve(strict=True)
    if not root.name.startswith('lucille-synthetic-') or (root / '.lucille-isolated').read_bytes() != b'synthetic-data-only\n':
        raise ValueError('Marked synthetic root required')
    exe = Path(args.executable).resolve(strict=True)
    if 'phase21c-llama' not in exe.parts or exe.name != 'llama-server.exe':
        raise ValueError('Dedicated runtime copy required')
    model = Path(args.model).resolve(strict=True)
    port = 15288 if args.kind == 'gemma' else 15289
    with socket.socket() as probe:
        probe.bind(('127.0.0.1', port))
    before = resources()
    if before['PhysicalAvailable'] < 1024**3 or before['CommitLimit'] - before['CommitTotal'] < 3 * 1024**3:
        raise RuntimeError('Insufficient preflight headroom; no model started')
    report = root / 'reports'
    report.mkdir(exist_ok=True)
    stop = root / (args.kind + '.stop')
    if stop.exists():
        raise RuntimeError('Prior stop marker exists; review before restarting')
    key_file = root / (args.kind + '.api-key')
    if not key_file.exists():
        key_file.write_text(secrets.token_urlsafe(48), encoding='ascii')
    command = [str(exe), '--model', str(model), '--host', '127.0.0.1', '--port', str(port),
               '--alias', 'lucille-real-' + args.kind, '--parallel', '1', '--threads', '4',
               '--threads-batch', '4', '--batch-size', '128', '--ubatch-size', '64',
               '--fit', 'off', '--no-warmup', '--api-key-file', str(key_file),
               '--cors-origins', 'http://127.0.0.1:15283']
    if args.kind == 'gemma':
        command += ['--ctx-size', '65536', '--gpu-layers', '99', '--flash-attn', 'on',
                    '--cache-type-k', 'f16', '--cache-type-v', 'f16', '--reasoning', 'off']
    else:
        command += ['--ctx-size', '2048', '--gpu-layers', '0', '--embeddings', '--pooling', 'mean', '--embd-normalize', '-1']
    env = dict(__import__('os').environ)
    for key, folder in [('APPDATA', 'roaming'), ('LOCALAPPDATA', 'local'),
                        ('USERPROFILE', 'home'), ('TEMP', 'tmp'), ('TMP', 'tmp')]:
        dest = root / folder
        dest.mkdir(exist_ok=True)
        env[key] = str(dest)
    start = time.monotonic()
    manifest_path = root / (args.kind + '-process.json')
    with (report / (args.kind + '.stdout.log')).open('wb') as stdout, (report / (args.kind + '.stderr.log')).open('wb') as stderr:
        process = subprocess.Popen(command, cwd=root, env=env, stdout=stdout, stderr=stderr,
                                   creationflags=subprocess.CREATE_NO_WINDOW)
        owned = identity(process)
        manifest = {**owned, 'command': command, 'model': str(model), 'kind': args.kind,
                    'started_unix': time.time(), 'ready': False, 'inference_verified': False,
                    'runtime_sha256': hashlib.sha256(exe.read_bytes()).hexdigest()}
        manifest_path.write_text(json.dumps(manifest, indent=2), encoding='utf-8')
        print(json.dumps({'started': owned, 'port': port}), flush=True)
        pressure = 0
        try:
            with (report / (args.kind + '-resources.ndjson')).open('w', encoding='utf-8') as log:
                while process.poll() is None:
                    sample = {'elapsed_s': time.monotonic() - start, **resources(), **process_resources(process)}
                    try:
                        sample['gpu_csv'] = subprocess.check_output(['nvidia-smi', '--query-gpu=memory.used,memory.free,utilization.gpu', '--format=csv,noheader,nounits'], text=True, timeout=5, creationflags=subprocess.CREATE_NO_WINDOW).strip()
                    except (OSError, subprocess.SubprocessError):
                        sample['gpu_csv'] = None
                    log.write(json.dumps(sample) + '\n')
                    log.flush()
                    severe = sample['PhysicalAvailable'] < 512 * 1024**2 or sample['CommitLimit'] - sample['CommitTotal'] < 1024**3
                    pressure = pressure + 1 if severe else 0
                    if stop.exists() or pressure >= 3:
                        manifest['stop_reason'] = 'memory pressure' if pressure >= 3 else 'requested test shutdown'
                        break
                    if not manifest['ready']:
                        try:
                            with urllib.request.urlopen(f'http://127.0.0.1:{port}/health', timeout=1) as response:
                                if response.status == 200:
                                    manifest['ready'] = True
                                    manifest['initialization_seconds'] = time.monotonic() - start
                                    manifest_path.write_text(json.dumps(manifest, indent=2), encoding='utf-8')
                                    print('Endpoint healthy; real inference still must be verified.', flush=True)
                        except Exception:
                            pass
                    time.sleep(2)
        finally:
            if process.poll() is None:
                if identity(process) != owned:
                    raise RuntimeError('Ownership check failed; refusing termination')
                manifest['stop_identity_verified'] = True
                process.terminate()
                process.wait(timeout=30)
            manifest['exit_code'] = process.returncode
            manifest['elapsed_seconds'] = time.monotonic() - start
            manifest_path.write_text(json.dumps(manifest, indent=2), encoding='utf-8')
            print(json.dumps(manifest), flush=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', required=True)
    parser.add_argument('--executable', required=True)
    parser.add_argument('--model', required=True)
    parser.add_argument('--kind', choices=['gemma', 'nomic'], required=True)
    serve(parser.parse_args())
