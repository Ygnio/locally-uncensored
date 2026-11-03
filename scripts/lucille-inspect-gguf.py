"""Read-only GGUF metadata/hash inventory; does not load tensors or run inference."""
import argparse
import hashlib
import json
from pathlib import Path
import struct


def inspect(path):
    path = Path(path).resolve(strict=True)
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        for block in iter(lambda: stream.read(4 * 1024 * 1024), b''):
            digest.update(block)
    with path.open('rb') as stream:
        def scalar(fmt):
            size = struct.calcsize('<' + fmt)
            data = stream.read(size)
            if len(data) != size:
                raise ValueError('Truncated GGUF')
            return struct.unpack('<' + fmt, data)[0]

        def string():
            length = scalar('Q')
            if length > 64 * 1024 * 1024:
                raise ValueError('Oversized GGUF string')
            return stream.read(length).decode('utf-8')

        def value(kind):
            formats = {0: 'B', 1: 'b', 2: 'H', 3: 'h', 4: 'I', 5: 'i',
                       6: 'f', 7: '?', 10: 'Q', 11: 'q', 12: 'd'}
            if kind in formats:
                return scalar(formats[kind])
            if kind == 8:
                return string()
            if kind == 9:
                subtype, count = scalar('I'), scalar('Q')
                if count > 2_000_000 or subtype == 9:
                    raise ValueError('Unsupported array')
                sample = []
                for index in range(count):
                    item = value(subtype)
                    if count <= 128 or index < 4:
                        sample.append(item)
                return sample if count <= 128 else {'count': count, 'first': sample}
            raise ValueError('Unsupported GGUF metadata type')

        if stream.read(4) != b'GGUF':
            raise ValueError('Not GGUF')
        version = scalar('I')
        if version != 3:
            raise ValueError('Only GGUF v3 supported')
        tensors, count = scalar('Q'), scalar('Q')
        if count > 100_000:
            raise ValueError('Oversized metadata')
        metadata = {}
        for _ in range(count):
            name, kind = string(), scalar('I')
            metadata[name] = value(kind)
    return {'path': str(path), 'bytes': path.stat().st_size,
            'sha256': digest.hexdigest(), 'gguf_version': version,
            'tensor_count': tensors, 'metadata': metadata}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('model')
    parser.add_argument('--output', required=True, help='Isolated report destination')
    args = parser.parse_args()
    result = inspect(args.model)
    Path(args.output).write_text(json.dumps(result, indent=2, ensure_ascii=False), encoding='utf-8')
    print(json.dumps({k: v for k, v in result.items() if k != 'metadata'}))
