"""Seal code/artifacts and explicit corpora. Never edit a completed manifest.

Development manifests label known regression material; final data must be authored
and committed only after production development ends. No paths/secrets from the
private contract are copied into the published pin.
"""
import argparse
import datetime
import hashlib
import json
import re
from pathlib import Path
from runpy import run_path
import sqlite3
import subprocess


def digest(path, lf=False):
    data=path.read_bytes()
    return hashlib.sha256(data.replace(b'\r\n',b'\n') if lf else data).hexdigest()


def validate_identity_fixture(fixture):
    assert isinstance(fixture, dict) and fixture.get('execution_complete') is True, 'Final identity fixture must be a completed GUI execution'
    identity = fixture.get('identity')
    assert isinstance(identity, dict) and type(identity.get('version')) is int and identity['version'] > 0, 'Final requires a persisted native identity'
    profile = identity.get('profile')
    assert isinstance(profile, dict) and all(isinstance(profile.get(key), str) and profile[key].strip() for key in ['name', 'description', 'personality', 'instructions']), 'Final requires the native identity profile'
    policy = fixture.get('policy')
    assert isinstance(policy, dict) and policy, 'Final requires a native learning policy'
    assert all(isinstance(policy.get(key), bool) for key in ['enabled', 'autonomous_preferences', 'automatic_indexing']), 'Final native policy flags are missing'
    assert policy.get('extraction_mode') in ['rules', 'llm', 'hybrid'], 'Final native extraction policy is missing'


def main(a, phase='22e'):
    repo=Path(__file__).resolve().parents[1]
    root=Path(a.root).resolve(strict=True)
    assert phase in ('22e','22f')
    assert root==repo/'.lucille-qa-runtime'/('lucille-synthetic-real'+phase)
    assert (root/'.lucille-isolated').read_bytes()==b'synthetic-data-only\n'
    output=Path(a.output).resolve()
    allowed=repo/'crates/lucille-memory/tests/fixtures'/('phase'+phase)
    assert output.is_relative_to(root/'reports') if a.kind=='development' else output.is_relative_to(allowed)
    assert not output.exists(),'Never overwrite a freeze'
    corpora={}
    for name in a.corpus:
        p=(repo/name).resolve(strict=True)
        assert p.is_relative_to(repo/'crates/lucille-memory/tests/fixtures') and p.suffix=='.json'
        corpora[p.relative_to(repo).as_posix()]=digest(p)
    assert a.extraction in corpora and a.retrieval in corpora
    tracked=subprocess.check_output(['git','ls-files','-z'],cwd=repo).decode().split('\0')
    extensions={'.ts','.tsx','.rs','.py','.ps1','.mjs','.cjs','.js','.json','.toml','.sql','.yaml','.yml','.html','.css','.lock'}
    sources={}
    for name in tracked:
        if not name or name.startswith(('docs/','crates/lucille-memory/tests/fixtures/')):continue
        p=repo/name
        if p.is_file() and (p.suffix in extensions or name in ['.gitignore','.nvmrc']):sources[name]=digest(p,True)
    assert 'src/lib/lucille-authority.ts' in sources
    assert 'qa/phase22e-driver.js' in sources
    assert re.fullmatch(r'[0-9a-f]{40}',a.production_commit)
    subprocess.run(['git','merge-base','--is-ancestor',a.production_commit,'HEAD'],cwd=repo,check=True)
    if a.kind=='final':
        dirty=subprocess.check_output(['git','diff','--name-only','HEAD'],cwd=repo,text=True).splitlines()
        assert not set(dirty).intersection(sources),'Commit final production source before freezing'
        changed_since_end=subprocess.check_output(['git','diff','--name-only',a.production_commit,'HEAD'],cwd=repo,text=True).splitlines()
        assert not set(changed_since_end).intersection(sources),'Production changed after declared completion'
        assert a.identity,'Final requires an actual completed native GUI identity/policy fixture'
    pin=json.loads((root/'reports/real-public-pin.json').read_text(encoding='utf-8'))
    assert pin['simulation'] is False and pin['memory_protocol']==3
    static=repo/('qa-dist-'+phase)
    gui, bridge=run_path(str(repo/'qa/phase22e-artifact-check.py'))['artifact_paths'](repo,phase)
    if a.without_gui:
        assert phase=='22f' and a.kind=='development', 'A final freeze requires the native GUI and frontend'
        gui_hash, frontend = None, {}
    else:
        assert (static/'.lucille-static-build').read_bytes()==b'synthetic-data-only\n'
        gui_hash=digest(gui)
        frontend={p.relative_to(static).as_posix():digest(p) for p in sorted(static.rglob('*')) if p.is_file()}
    fixture=json.loads(Path(a.identity).read_text(encoding='utf-8')) if a.identity else None
    if a.kind=='final':validate_identity_fixture(fixture)
    schema=5
    if phase=='22f':
        db_root=repo/'.lucille-qa-runtime/lucille-synthetic-full22f'
        assert (db_root/'.lucille-isolated').read_bytes()==b'synthetic-data-only\n'
        database=db_root/'lucille-phase1.sqlite3'
        if database.is_file():
            with sqlite3.connect(database.as_uri()+'?mode=ro',uri=True) as db:
                db.execute('pragma query_only=on')
                schema=db.execute('select max(version) from schema_migrations').fetchone()[0]
        else:
            assert a.kind=='development','Final freeze requires native schema observation'
            schema=None
    data=dict(id='phase'+phase+'-'+a.kind+'-'+a.label,kind=a.kind,
        created_utc=datetime.datetime.now(datetime.timezone.utc).isoformat(),
        base='b8b1d0e255c29e912edf7f6d18e9c48ac11dfcd1' if phase=='22f' else '92f75a0d661fa9804b468e55b3760090b47bdd66',
        source_head=subprocess.check_output(['git','rev-parse','HEAD'],cwd=repo,text=True).strip(),
        production_code_commit=a.production_commit,source_sha256_lf=sources,
        corpus_sha256=corpora,extraction_corpus=a.extraction,retrieval_corpus=a.retrieval,
        bridge_sha256=digest(bridge),native_gui_sha256=gui_hash,
        frontend_sha256=frontend,
        model_pin=pin,schema_version=schema,representation_version=5,index_version=5,atomic_protocol=3,
        source_authority_version=int(re.search(r'pub const SOURCE_AUTHORITY_VERSION: u32 = (\d+);',(repo/'crates/lucille-memory/src/source_authority.rs').read_text(encoding='utf-8')).group(1)),
        identity=fixture.get('identity') if fixture else None,policy=fixture.get('policy') if fixture else None,
        evaluation_contract='No source/prompt/policy/parameter/corpus/ranking changes during execution; actual raw generations, all fidelity reports, displayed text and usefulness are separate.')
    if phase=='22f':
        data['native_local_dependencies_sha256'] = {} if a.without_gui else {
            p.name:digest(p) for p in sorted(gui.parent.glob('*.dll')) if p.is_file()}
    output.parent.mkdir(parents=True,exist_ok=True)
    output.write_bytes((json.dumps(data,ensure_ascii=False,indent=2)+'\n').encode('utf-8'))
    print(json.dumps(dict(kind=a.kind,source_files=len(sources),corpora=len(corpora),sha256=digest(output))))


def cli(phase='22e'):
    p=argparse.ArgumentParser()
    for name in ['root','output','label','extraction','retrieval','production-commit']:p.add_argument('--'+name,required=True)
    p.add_argument('--corpus',action='append',required=True)
    p.add_argument('--kind',choices=['development','final'],required=True)
    p.add_argument('--identity')
    p.add_argument('--without-gui',action='store_true',help='Seal independent Phase 2.2F development stages before the GUI build is available.')
    main(p.parse_args(),phase)


if __name__=='__main__':cli()
