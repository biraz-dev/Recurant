#!/usr/bin/env python3
"""Verify and extract packaged asset kits; no external dependencies or game changes.

Run from the repository root: python3 tools/extract_asset_kits.py
Optional kit arguments select a subset; --verify-only performs no writes.
Existing identical files are kept. Different existing files are never overwritten.
"""
from __future__ import annotations
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import stat
import zipfile

ROOT = Path(__file__).resolve().parents[1]

def sha256(path):
    h = hashlib.sha256()
    with path.open('rb') as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b''):
            h.update(chunk)
    return h.hexdigest()

def safe_path(name):
    rel = PurePosixPath(name)
    if rel.is_absolute() or '..' in rel.parts or '\\' in name or not name:
        raise ValueError(f'Unsafe archive path: {name!r}')
    p = ROOT.joinpath(*rel.parts)
    if not p.resolve().is_relative_to(ROOT):
        raise ValueError(f'Path escapes repository: {name}')
    for ancestor in (p, *p.parents):
        if ancestor == ROOT:
            break
        if ancestor.is_symlink():
            raise ValueError(f'Refusing symlink: {ancestor.relative_to(ROOT)}')
    return p

def validate(record):
    archive = safe_path(record['archive'])
    if archive.stat().st_size != record['bytes'] or sha256(archive) != record['sha256']:
        raise ValueError(f'Archive checksum mismatch: {record["archive"]}')
    expected = {e['path']: e for e in record['files']}
    if len(expected) != record['file_count']:
        raise ValueError('Invalid file-count inventory')
    with zipfile.ZipFile(archive) as z:
        infos = z.infolist()
        if len(infos) != len(expected) or set(z.namelist()) != set(expected):
            raise ValueError(f'Archive membership mismatch: {archive.name}')
        for info in infos:
            e = expected[info.filename]
            if not info.filename.startswith(record['content_root'] + '/'):
                raise ValueError('Archive member outside declared kit root')
            p = safe_path(info.filename)
            if stat.S_ISLNK(info.external_attr >> 16) or info.is_dir():
                raise ValueError(f'Unexpected member type: {info.filename}')
            if info.file_size != e['bytes']:
                raise ValueError(f'Member size mismatch: {info.filename}')
            if hashlib.sha256(z.read(info)).hexdigest() != e['sha256']:
                raise ValueError(f'Member checksum mismatch: {info.filename}')
            if p.exists() and (not p.is_file() or sha256(p) != e['sha256']):
                raise ValueError(f'Existing file differs; move it aside first: {info.filename}')
    return archive

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('kits', nargs='*', help='Optional kit IDs from ASSET_INVENTORY.json')
    parser.add_argument('--verify-only', action='store_true', help='Verify without extracting')
    args = parser.parse_args()
    inventory = json.loads((ROOT / 'ASSET_INVENTORY.json').read_text())
    lookup = {r['kit'] for r in inventory['archives']}
    unknown = set(args.kits) - set(lookup)
    if unknown:
        parser.error('Unknown kits: ' + ', '.join(sorted(unknown)))
    selected = set(args.kits or lookup)
    records = [r for r in inventory['archives'] if r['kit'] in selected]
    # Validate every selected archive before creating any asset files.
    archives = [(r, validate(r)) for r in records]
    for record, archive in archives:
        if args.verify_only:
            print(f'Verified {record["kit"]} volume {record["volume"]}/{record["volume_count"]}: {record["file_count"]} files')
            continue
        written = kept = 0
        with zipfile.ZipFile(archive) as z:
            for e in record['files']:
                p = safe_path(e['path'])
                if p.exists():
                    if not p.is_file() or sha256(p) != e['sha256']:
                        raise ValueError(f'File changed since validation: {e["path"]}')
                    kept += 1
                    continue
                p.parent.mkdir(parents=True, exist_ok=True)
                p = safe_path(e['path'])
                flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL
                flags |= getattr(os, 'O_NOFOLLOW', 0)
                fd = os.open(p, flags, 0o644)
                try:
                    with os.fdopen(fd, 'wb') as f:
                        f.write(z.read(e['path']))
                except BaseException:
                    p.unlink(missing_ok=True)
                    raise
                if sha256(p) != e['sha256']:
                    raise ValueError(f'Extracted checksum mismatch: {e["path"]}')
                written += 1
        print(f'{record["kit"]} volume {record["volume"]}/{record["volume_count"]}: extracted {written}, kept {kept} identical files')
    print('Asset files only; engine integration and runtime QA remain separate.')

if __name__ == '__main__':
    main()
