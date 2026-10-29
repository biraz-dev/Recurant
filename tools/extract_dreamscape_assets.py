#!/usr/bin/env python3
"""Verify and extract the Dreamscape modular/equipment asset packages.

Run from the repository root: python3 tools/extract_dreamscape_assets.py
Optional group IDs select themes/equipment; theme selection includes common assets.
Use --list to list groups; --verify-only performs no writes. Python stdlib only.
Existing identical files are kept; different files and symlinks are never overwritten.
"""
from __future__ import annotations
import argparse
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import stat
import zipfile

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = 'MODULAR_ASSET_INVENTORY.json'
ALLOWED_ROOTS = {'art/kits/dreamscape-modular-v1/Recurant_55_Theme_Kits_Essentials', 'art/kits/dreamscape-modular-v1/Recurant_55_Theme_Kits_Source_Art', 'art/kits/seven-slot-equipment-v1'}

def digest(data):
    return hashlib.sha256(data).hexdigest()

def safe_path(name):
    rel = PurePosixPath(name)
    if not name or rel.is_absolute() or '..' in rel.parts or '\\' in name:
        raise ValueError(f'Unsafe package path: {name!r}')
    p = ROOT.joinpath(*rel.parts)
    if not p.resolve().is_relative_to(ROOT):
        raise ValueError(f'Path escapes repository: {name}')
    for part in (p, *p.parents):
        if part == ROOT:
            break
        if part.is_symlink():
            raise ValueError(f'Refusing symlink: {name}')
    return p

def checked_bytes(record):
    b = safe_path(record['path']).read_bytes()
    if len(b) != record['bytes'] or digest(b) != record['sha256']:
        raise ValueError(f'Checksum/size mismatch: {record["path"]}')
    return b

def archive_bytes(archive):
    # Binary parts are explicitly ordered complete ZIP byte segments, not new art.
    parts = archive['parts']
    if [p['number'] for p in parts] != list(range(1, len(parts) + 1)):
        raise ValueError('Archive part ordering is invalid')
    if sum(p['bytes'] for p in parts) != archive['bytes']:
        raise ValueError('Archive part sizes do not sum to the complete ZIP size')
    b = b''.join(checked_bytes(p) for p in parts)
    if digest(b) != archive['sha256']:
        raise ValueError(f'Reassembled ZIP checksum mismatch: {archive["id"]}')
    return b

def check_archive(archive, root_prefix):
    expected = {f['path']: f for f in archive['files']}
    if len(expected) != len(archive['files']):
        raise ValueError('Duplicate file in archive inventory')
    with zipfile.ZipFile(io.BytesIO(archive_bytes(archive))) as z:
        infos = z.infolist()
        if len(infos) != len(expected) or set(z.namelist()) != set(expected):
            raise ValueError(f'Archive membership mismatch: {archive["id"]}')
        for info in infos:
            e = expected[info.filename]
            if not info.filename.startswith(root_prefix + '/'):
                raise ValueError('Member outside declared package root')
            p = safe_path(info.filename)
            if info.is_dir() or stat.S_ISLNK(info.external_attr >> 16):
                raise ValueError(f'Unexpected member type: {info.filename}')
            if info.file_size != e['bytes'] or digest(z.read(info)) != e['sha256']:
                raise ValueError(f'Member checksum/size mismatch: {info.filename}')
            if p.exists() and (not p.is_file() or digest(p.read_bytes()) != e['sha256']):
                raise ValueError(f'Existing file differs; move it aside first: {info.filename}')

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('groups', nargs='*', help='Group IDs shown by --list')
    parser.add_argument('--with-sources', action='store_true', help='Also extract optional original masters and restore their exact source paths')
    parser.add_argument('--list', action='store_true', help='List available groups')
    parser.add_argument('--verify-only', action='store_true', help='Verify without writing')
    args = parser.parse_args()
    manifest = json.loads(safe_path(MANIFEST).read_text())
    lookup = {g['id']: g for g in manifest['groups']}
    if len(lookup) != len(manifest['groups']):
        raise ValueError('Duplicate group IDs')
    if args.list:
        for g in manifest['groups']:
            print(f'{g["id"]}: {g["title"]}' + (' [optional source]' if g.get('optional') else ''))
        return
    unknown = set(args.groups) - set(lookup)
    if unknown:
        parser.error('Unknown groups: ' + ', '.join(sorted(unknown)))
    selected = set(args.groups or [g['id'] for g in manifest['groups'] if not g.get('optional')])
    if args.with_sources:
        selected.add(manifest['source_restoration']['group'])
    pending = list(selected)
    while pending:
        for dependency in lookup[pending.pop()]['requires']:
            if dependency not in lookup:
                raise ValueError('Missing dependency group')
            if dependency not in selected:
                selected.add(dependency)
                pending.append(dependency)
    packages = []
    paths_seen = set()
    for g in manifest['groups']:
        if g['id'] not in selected:
            continue
        index = json.loads(checked_bytes(g['inventory']))
        if index['content_root'] not in ALLOWED_ROOTS:
            raise ValueError('Unexpected extraction root')
        if sum(len(a['files']) for a in index['archives']) != index['file_count']:
            raise ValueError('Group file count mismatch')
        if index['id'] != g['id']:
            raise ValueError('Group inventory identity mismatch')
        for archive in index['archives']:
            for f in archive['files']:
                if f['path'] in paths_seen:
                    raise ValueError(f'Duplicate output across packages: {f["path"]}')
                paths_seen.add(f['path'])
            check_archive(archive, index['content_root'])
        packages.append(index)
    source_copies = []
    source_info = manifest.get('source_restoration')
    if source_info and source_info['group'] in selected:
        if source_info['source_root'] not in ALLOWED_ROOTS or source_info['target_root'] not in ALLOWED_ROOTS:
            raise ValueError('Unexpected source restoration roots')
        stored = {f['path']: f for package in packages for archive in package['archives'] for f in archive['files']}
        alias_entry = stored[source_info['alias_map']]
        alias_map = json.loads(checked_bytes(alias_entry))
        targets = set()
        for item in alias_map['files']:
            target = source_info['target_root'] + '/' + item['target']
            source = source_info['source_root'] + '/' + item['blob']
            p = safe_path(target)
            safe_path(source)
            e = stored.get(source)
            if not e or e['bytes'] != item['bytes'] or e['sha256'] != item['sha256']:
                raise ValueError('Source alias does not match verified blob inventory')
            if target in targets:
                raise ValueError('Duplicate source alias target')
            targets.add(target)
            if p.exists() and (not p.is_file() or digest(p.read_bytes()) != item['sha256']):
                raise ValueError(f'Existing source differs; move it aside first: {target}')
            source_copies.append((source, target, item))
    # All selected parts, ZIPs, file hashes, destinations and conflicts pass first.
    for index in packages:
        if args.verify_only:
            print(f'Verified {index["id"]}: {index["file_count"]} files')
            continue
        written = kept = 0
        for archive in index['archives']:
            with zipfile.ZipFile(io.BytesIO(archive_bytes(archive))) as z:
                for e in archive['files']:
                    p = safe_path(e['path'])
                    if p.exists():
                        if not p.is_file() or digest(p.read_bytes()) != e['sha256']:
                            raise ValueError(f'File changed after validation: {e["path"]}')
                        kept += 1
                        continue
                    p.parent.mkdir(parents=True, exist_ok=True)
                    p = safe_path(e['path'])
                    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, 'O_NOFOLLOW', 0)
                    fd = os.open(p, flags, 0o644)
                    try:
                        with os.fdopen(fd, 'wb') as f:
                            f.write(z.read(e['path']))
                    except BaseException:
                        p.unlink(missing_ok=True)
                        raise
                    if digest(p.read_bytes()) != e['sha256']:
                        raise ValueError(f'Extracted checksum mismatch: {e["path"]}')
                    written += 1
        print(f'{index["id"]}: extracted {written}; kept {kept} identical files')
    if source_copies:
        if args.verify_only:
            print(f'Verified {len(source_copies)} original source aliases without writing')
        else:
            restored = kept_sources = 0
            for source, target, item in source_copies:
                p = safe_path(target)
                if p.exists():
                    if not p.is_file() or digest(p.read_bytes()) != item['sha256']:
                        raise ValueError(f'Source changed after validation: {target}')
                    kept_sources += 1
                    continue
                b = checked_bytes({'path': source, 'bytes': item['bytes'], 'sha256': item['sha256']})
                p.parent.mkdir(parents=True, exist_ok=True)
                p = safe_path(target)
                flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, 'O_NOFOLLOW', 0)
                fd = os.open(p, flags, 0o644)
                try:
                    with os.fdopen(fd, 'wb') as f:
                        f.write(b)
                except BaseException:
                    p.unlink(missing_ok=True)
                    raise
                if digest(p.read_bytes()) != item['sha256']:
                    raise ValueError(f'Restored source checksum mismatch: {target}')
                restored += 1
            print(f'Original source aliases: restored {restored}; kept {kept_sources} identical files')
    print('Asset files only. Engine integration, seven-slot data mapping and runtime QA remain separate.')

if __name__ == '__main__':
    main()
