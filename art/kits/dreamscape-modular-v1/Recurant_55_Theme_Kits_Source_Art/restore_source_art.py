#!/usr/bin/env python3
"""Restore exact source bytes safely; no downloads, path traversal or implicit overwrites."""
import argparse,pathlib,json,hashlib,shutil,sys
p=argparse.ArgumentParser();p.add_argument('destination');p.add_argument('--verify-only',action='store_true');p.add_argument('--only-theme');a=p.parse_args()
root=pathlib.Path(__file__).resolve().parent;rawdest=pathlib.Path(a.destination)
if rawdest.is_symlink() or any(p.is_symlink() for p in rawdest.parents):raise SystemExit('Refusing symlink destination')
dest=rawdest.resolve();manifest=json.loads((root/'SOURCE_ALIAS_MAP.json').read_text());verified=set();selected=[]
for item in manifest['files']:
 rel=pathlib.PurePosixPath(item['target']);blobrel=pathlib.PurePosixPath(item['blob'])
 if rel.is_absolute() or '..' in rel.parts or blobrel.is_absolute() or '..' in blobrel.parts:raise SystemExit('Unsafe path in source map')
 if a.only_theme and not str(rel).startswith('themes/'+a.only_theme+'/'):continue
 blob=(root/pathlib.Path(*blobrel.parts)).resolve()
 if root not in blob.parents or not blob.is_file():raise SystemExit('Missing/unsafe source blob: '+item['blob'])
 if item['blob'] not in verified:
  if blob.stat().st_size!=item['bytes'] or hashlib.sha256(blob.read_bytes()).hexdigest()!=item['sha256']:raise SystemExit('Source hash mismatch: '+item['blob'])
  verified.add(item['blob'])
 rawtarget=dest/pathlib.Path(*rel.parts)
 if any(p.is_symlink() for p in [rawtarget,*rawtarget.parents]):raise SystemExit('Refusing symlink destination component')
 target=rawtarget.resolve()
 if dest not in target.parents:raise SystemExit('Destination path escaped target root')
 if target.exists() and (not target.is_file() or hashlib.sha256(target.read_bytes()).hexdigest()!=item['sha256']):raise SystemExit('Refusing to overwrite differing file: '+str(rel))
 selected.append((blob,target,item))
if not a.verify_only:
 for blob,target,item in selected:
  target.parent.mkdir(parents=True,exist_ok=True)
  if not target.exists():shutil.copyfile(blob,target)
  if hashlib.sha256(target.read_bytes()).hexdigest()!=item['sha256']:raise SystemExit('Post-copy verification failed')
print(json.dumps({'verified_unique_blobs':len(verified),'source_paths':len(selected),'mode':'verify_only' if a.verify_only else 'restored_exact_bytes'}))
