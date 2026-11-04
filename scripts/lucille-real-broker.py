"""QA-only loopback broker: pinned real files + owned live processes, no mocks.

Run after explicit inventory and model-download authorization. Secrets remain in
the marked QA root. The published pin contains only hashes/revisions/contracts.
"""
import argparse
import ctypes as c
from ctypes import wintypes as w
import hashlib
from http.server import BaseHTTPRequestHandler, HTTPServer
import json
import math
import os
import re
from pathlib import Path
import sys
import time
import urllib.request
import importlib.util
sys.dont_write_bytecode = True

spec = importlib.util.spec_from_file_location('owned_server', Path(__file__).with_name('lucille-real-server.py'))
owned = importlib.util.module_from_spec(spec)
spec.loader.exec_module(owned)


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def atomic_schema_for(source, scope, project, version=2):
    spans = list(dict.fromkeys(v.strip() for v in re.split(r'[.!;\n]', source) if v.strip()))
    if not 0 < len(spans) <= 64 or len(source.encode()) > 12000:
        raise ValueError('Atomic source ceiling')
    schema = json.loads((Path(__file__).resolve().parents[1] / 'src/lib/lucille-atomic-schema.json').read_text(encoding='utf-8'))
    if version not in [2,3]: raise ValueError('Atomic version')
    schema['properties']['version'] = {'const': version}
    schema['properties']['scope'] = {'const': scope}
    schema['properties']['project'] = {'const': project}
    schema['properties']['claims']['items']['properties']['evidence'] = {'type': 'string', 'enum': spans}
    return schema


def main(args):
    root = Path(args.root).resolve(strict=True)
    if (root / '.lucille-isolated').read_bytes() != b'synthetic-data-only\n':
        raise ValueError('Synthetic root required')
    contract_path = Path(args.contract).resolve(strict=True)
    if root not in contract_path.parents:
        raise ValueError('Contract must remain in QA root')
    contract = json.loads(contract_path.read_text(encoding='utf-8'))
    expected = contract['models']
    stamps = {}
    for kind, pin in expected.items():
        path = Path(contract['paths'][kind]).resolve(strict=True)
        if sha(path) != pin['fingerprint']:
            raise ValueError('Actual weights do not match pinned fingerprint')
        stamps[kind] = (path, path.stat().st_size, path.stat().st_mtime_ns)
        print('Actual file hash verified: ' + kind, flush=True)
    exe = Path(contract['runtime_path']).resolve(strict=True)
    if 'phase21c-llama' not in exe.parts or sha(exe) != contract['runtime_sha256']:
        raise ValueError('Dedicated runtime hash mismatch')
    open_process = c.windll.kernel32.OpenProcess
    open_process.argtypes = [w.DWORD, w.BOOL, w.DWORD]
    open_process.restype = w.HANDLE
    close = c.windll.kernel32.CloseHandle
    close.argtypes = [w.HANDLE]

    def verify(kind):
        manifest = json.loads((root / (kind + '-process.json')).read_text(encoding='utf-8'))
        if not manifest.get('ready') or 'exit_code' in manifest:
            raise ValueError('Owned model is unavailable')
        handle = open_process(0x1000, False, manifest['pid'])
        if not handle:
            raise ValueError('Owned model is not running')
        try:
            proxy = type('OwnedHandle', (), {'_handle': int(handle), 'pid': manifest['pid']})()
            actual = owned.identity(proxy)
            if actual != {k: manifest[k] for k in actual} or Path(actual['executable']) != exe:
                raise ValueError('Owned PID/path/creation identity changed')
        finally:
            close(handle)
        path, size, modified = stamps[kind]
        if (path.stat().st_size, path.stat().st_mtime_ns) != (size, modified) or Path(manifest['model']) != path:
            raise ValueError('Model file changed after fingerprint verification')
        if kind == 'gemma' and manifest['command'][manifest['command'].index('--ctx-size') + 1] != '65536':
            raise ValueError('Gemma context changed')
        return manifest

    def upstream(kind, endpoint, payload):
        manifest = verify(kind)
        before = owned.resources()
        if before['PhysicalAvailable'] < 512 * 1024**2 or before['CommitLimit'] - before['CommitTotal'] < 1024**3:
            raise ValueError('Resource pressure; no new test inference')
        port = 15288 if kind == 'gemma' else 15289
        key = (root / (kind + '.api-key')).read_text(encoding='ascii')
        request = urllib.request.Request(f'http://127.0.0.1:{port}' + endpoint,
                                        data=json.dumps(payload).encode(), headers={
                                            'Content-Type': 'application/json', 'Authorization': 'Bearer ' + key})
        with urllib.request.urlopen(request, timeout=120) as response:
            result = json.load(response)
        if verify(kind)['creation_filetime'] != manifest['creation_filetime']:
            raise ValueError('Model changed during request')
        return result

    pin = {'simulation': False, 'models': expected, 'runtime_sha256': contract['runtime_sha256']}
    if 'memory_protocol' in contract:
        if contract['memory_protocol'] != 3: raise ValueError('Unexpected protocol')
        pin['memory_protocol'] = 3
    (root / 'reports' / 'real-public-pin.json').write_text(json.dumps(pin, indent=2), encoding='utf-8')

    class Handler(BaseHTTPRequestHandler):
        def log_message(self, *_):
            pass

        def send(self, code, data):
            body = json.dumps(data).encode()
            self.send_response(code)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Access-Control-Allow-Origin', 'http://127.0.0.1:15283')
            self.send_header('Access-Control-Allow-Headers', 'Content-Type,Authorization')
            self.send_header('Access-Control-Allow-Methods', 'GET,POST,OPTIONS')
            self.send_header('Content-Length', str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def do_OPTIONS(self):
            self.send(200, {})

        def do_GET(self):
            try:
                if self.path in ['/lucille/extraction-manifest', '/lucille/embedding-manifest']:
                    kind = 'gemma' if 'extraction' in self.path else 'nomic'
                    verify(kind)
                    self.send(200, {**pin, 'space': expected.get('nomic'), 'context_tokens': 65536})
                elif self.path == '/v1/models':
                    verify('gemma')
                    self.send(200, {'object': 'list', 'data': [{'id': 'lucille-real-gemma', 'object': 'model', 'owned_by': 'isolated-real-validation', 'context_length': 65536}]})
                else:
                    self.send(404, {'error': 'No such QA route'})
            except Exception as error:
                self.send(503, {'error': type(error).__name__, 'state': 'unavailable; no simulated fallback'})

        def do_POST(self):
            started = time.monotonic()
            try:
                if self.headers.get('Origin') not in [None, 'http://127.0.0.1:15283']:
                    raise ValueError('Unknown origin')
                size = int(self.headers.get('Content-Length', '0'))
                if not 0 < size <= 256 * 1024:
                    raise ValueError('Payload ceiling')
                data = json.loads(self.rfile.read(size))
                if self.path == '/v1/chat/completions':
                    stream = bool(data.get('stream'))
                    data['stream'] = False
                    data['model'] = 'lucille-real-gemma'
                    data['max_tokens'] = min(768, int(data.get('max_tokens', 512)))
                    messages = data.get('messages', [])
                    if any(m.get('role') == 'system' and any(marker in m.get('content', '') for marker in ['LUCILLE_ATOMIC_EXTRACTION_V2','LUCILLE_ATOMIC_EXTRACTION_V3']) for m in messages):
                        if data.get('tools') or data.get('tool_choice'):
                            raise ValueError('Extraction forbids tools')
                        source = json.loads(messages[-1]['content'])
                        version = 3 if 'LUCILLE_ATOMIC_EXTRACTION_V3' in messages[0]['content'] else 2
                        if version == 3 and contract.get('memory_protocol') != 3: raise ValueError('V3 pin required')
                        schema = atomic_schema_for(source['user_text'], source['scope'], source['project'], version)
                        data['temperature'] = 0
                        data['response_format'] = {'type': 'json_object', 'schema': schema}
                    if contract.get('memory_protocol') == 3:
                        source_meta = locals().get('source', {}) if any(m.get('content', '').startswith('LUCILLE_ATOMIC_EXTRACTION_V3') for m in messages if isinstance(m.get('content'), str)) else {}
                        (root / 'reports' / 'inflight-gemma.json').write_text(json.dumps({'began_unix': time.time(), 'scope': source_meta.get('scope'), 'project': source_meta.get('project'), 'extraction': bool(source_meta), 'request_sha256': hashlib.sha256(json.dumps(data, sort_keys=True).encode()).hexdigest()}), encoding='utf-8')
                    result = upstream('gemma', self.path, data)
                    kind = 'gemma'
                elif self.path == '/v1/embeddings':
                    if data.get('model') != expected['nomic']['model'] or not isinstance(data.get('input'), str):
                        raise ValueError('Unexpected embedding contract')
                    if len(data['input'].encode()) > expected['nomic']['context_tokens']:
                        raise ValueError('Conservative byte/token input bound exceeded')
                    if not data['input'].startswith(('search_query: ', 'search_document: ')):
                        raise ValueError('Task prefix required')
                    result = upstream('nomic', self.path, {**data, 'model': 'lucille-real-nomic'})
                    vector = result['data'][0]['embedding']
                    if len(vector) != expected['nomic']['dimension'] or not all(math.isfinite(v) for v in vector):
                        raise ValueError('Invalid actual vector')
                    # Nomic v1.5 model card: full-dimensional layer norm then L2.
                    # The positive scalar LN denominator cancels in final L2.
                    mean = sum(vector) / len(vector)
                    centered = [v - mean for v in vector]
                    norm = math.sqrt(sum(v * v for v in centered))
                    if norm <= 1e-12:
                        raise ValueError('Zero embedding norm')
                    result['data'][0]['embedding'] = [v / norm for v in centered]
                    result['model'] = expected['nomic']['model']
                    kind = 'nomic'
                else:
                    self.send(404, {'error': 'No such QA route'})
                    return
                with (root / 'reports' / 'real-broker-calls.ndjson').open('a', encoding='utf-8') as log:
                    log.write(json.dumps({'completed_unix': time.time(), 'kind': kind, 'latency_seconds': time.monotonic()-started, 'request': data, 'response': result}, ensure_ascii=False) + '\n')
                if self.path == '/v1/chat/completions' and stream:
                    # Adapt the actual bounded upstream response to OpenAI SSE.
                    # No response text, token usage or finish reason is invented.
                    choice = result['choices'][0]
                    chunks = [dict(result, choices=[{'index':0,'delta':choice['message'],'finish_reason':None}]),
                              dict(result, choices=[{'index':0,'delta':{},'finish_reason':choice['finish_reason']}])]
                    body = ''.join('data: ' + json.dumps(chunk) + '\n\n' for chunk in chunks) + 'data: [DONE]\n\n'
                    raw = body.encode()
                    self.send_response(200)
                    self.send_header('Content-Type','text/event-stream')
                    self.send_header('Access-Control-Allow-Origin','http://127.0.0.1:15283')
                    self.send_header('Content-Length',str(len(raw)))
                    self.end_headers()
                    self.wfile.write(raw)
                else:
                    self.send(200, result)
            except Exception as error:
                reasons = {'Resource pressure; no new test inference': 'resource_pressure',
                           'Owned model is unavailable': 'owned_model_unavailable',
                           'Owned model is not running': 'owned_model_not_running',
                           'Owned PID/path/creation identity changed': 'owned_identity_changed'}
                with (root / 'reports' / 'real-broker-errors.ndjson').open('a', encoding='utf-8') as log:
                    log.write(json.dumps({'completed_unix': time.time(), 'error': type(error).__name__,
                                          'reason': reasons.get(str(error), 'other_bounded_request_failure')}) + '\n')
                try:
                    self.send(503, {'error': type(error).__name__, 'state': 'failed; no mock response'})
                except (ConnectionError, OSError):
                    pass  # Client cancellation cannot manufacture a successful reply.


    current = type('OwnedHandle', (), {'_handle': -1, 'pid': os.getpid()})()
    process_identity = owned.identity(current)
    (root / 'broker-process.json').write_text(json.dumps(process_identity, indent=2), encoding='utf-8')
    print('Verified real QA broker starting on 127.0.0.1:15284', flush=True)
    server = HTTPServer(('127.0.0.1', 15284), Handler)
    server.timeout = 1
    try:
        while not (root / 'broker.stop').exists():
            server.handle_request()
    finally:
        server.server_close()
        print('Owned broker closed gracefully.', flush=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', required=True)
    parser.add_argument('--contract', required=True)
    main(parser.parse_args())
