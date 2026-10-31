"""Collecte read-only; toutes les sorties sont voisines de ce script."""
from pathlib import Path
import subprocess, json, tomllib, shutil
ROOT=Path(__file__).resolve().parents[3]; OUT=Path(__file__).parent
def command(name,args):
    if args[0]=='git': args=args[:1]+['--no-optional-locks','-c','maintenance.auto=false','-c','gc.auto=0']+args[1:]
    r=subprocess.run(args,cwd=ROOT,capture_output=True,encoding='utf-8',errors='replace')
    (OUT/name).write_text('$ '+' '.join(args)+'\nexit='+str(r.returncode)+'\nSTDOUT\n'+r.stdout+'\nSTDERR\n'+r.stderr,encoding='utf-8')
    return r.returncode
command('rg-unused-ports.txt',['rg','-n',r'\b(?:Clock|JobRepository|WorkerPort)\b','crates','-g','*.rs'])
command('rg-inclusions.txt',['rg','-n',r'include!|#\[path','crates','-g','*.rs'])
command('git-head.txt',['git','rev-parse','HEAD'])
command('git-diff-initial.txt',['git','diff','--stat'])
command('git-tracked-rust.txt',['git','ls-files','*.rs'])
command('rg-markers.txt',['rg','-n',r'#\[cfg\(test\)\]|#\[cfg\([^\]]*feature\s*=|\bunsafe\b|\.unwrap\(\)|\.expect\(|TODO|FIXME|HACK|thread::spawn|Command::new','crates','-g','*.rs'])
command('rg-threads-processes-io.txt',['rg','-n',r'thread::|Command::|\.spawn\(|std::net|reqwest|TcpStream|UdpSocket|fs::|File::|OpenOptions|Library::','crates','-g','*.rs'])
command('rg-public-imports.txt',['rg','-n','-U',r'\b(?:pub\s+)?use\s+whisper_[a-z_]+\s*::[^;]+;','crates','-g','*.rs'])
command('rg-campaign-dead-code.txt',['rg','-n',r'\br7_|\bl2a2_|\bl3c_|l04_campaign|allow\(dead_code\)|\bClock\b|\bJobRepository\b|\bWorkerPort\b','crates','-g','*.rs'])
root=tomllib.loads((ROOT/'Cargo.toml').read_text(encoding='utf-8'))
manifest={'workspace':root,'packages':[],'warning':'Lecture des Cargo.toml, pas une sortie cargo metadata; aucune resolution Cargo.'}
for path in root['workspace']['members']:
    p=ROOT/path/'Cargo.toml'
    d=tomllib.loads(p.read_text(encoding='utf-8')); d['manifest_path']=p.relative_to(ROOT).as_posix()
    d['rust_source_files']=[x.relative_to(ROOT).as_posix() for x in sorted((ROOT/path).rglob('*.rs'))]
    manifest['packages'].append(d)
    (OUT/(path.split('/')[-1]+'-Cargo.toml.txt')).write_text(p.read_text(encoding='utf-8'),encoding='utf-8')
(OUT/'workspace-Cargo.toml.txt').write_text((ROOT/'Cargo.toml').read_text(encoding='utf-8'),encoding='utf-8')
(OUT/'manifests.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2),encoding='utf-8')
tools={x:shutil.which(x) for x in ['cargo','cargo-clippy','tokei','scc','cargo-modules','cargo-depgraph']}
commands=[
 ('cargo metadata --format-version 1 --no-deps','[Non vérifié] NON EXECUTE : la garde locale exige CARGO_HOME/TEMP/TMP dans une racine de campagne et target canonique; ecritures interdites hors docs/architecture. Les manifestes sont lus directement.'),
 ('tokei ou scc','[Non vérifié] INDISPONIBLES dans PATH; aucune installation. Mesures physiques lexicales dans files.tsv, pas de SLOC tokei.'),
 ('cargo modules structure --package <chacune des 8 crates>','[Non vérifié] INDISPONIBLE : cargo-modules absent de PATH. Aucun Cargo lance; inventaire lexical dans modules.json.'),
 ('cargo depgraph --workspace-only','[Non vérifié] INDISPONIBLE : cargo-depgraph absent de PATH. Graphe declare reconstruit depuis Cargo.toml, sans pretendue sortie de cet outil.'),
 ('cargo clippy --workspace --all-targets -- -W clippy::too_many_lines -W clippy::cognitive_complexity','[Non vérifié] NON EXECUTE : compilation, build scripts et caches incompatibles avec ecritures uniquement dans docs/architecture et garde target canonique. Aucun resultat de complexite disponible.'),
]
(OUT/'requested-commands.txt').write_text('Collecte 2026-10-31; aucun outil installe, aucun Cargo execute.\n\n'+json.dumps(tools,indent=2)+'\n\n'+'\n\n'.join(a+'\n'+b for a,b in commands),encoding='utf-8')
(OUT/'recette-build.rs.txt').write_text((ROOT/'scripts/recette-build.rs').read_text(encoding='utf8'),encoding='utf8')
print('Sorties rg/Git, manifestes et statuts des commandes enregistres.')

# Preserve current measured status when successful Cargo evidence already exists.
if (OUT/'cargo-results.json').exists():
    result=json.loads((OUT/'cargo-results.json').read_text(encoding='utf8'))
    status='Dernieres mesures conservees : HEAD '+result['head']+' ; voir cargo-guard-runs.json pour dates/commandes.\n\n'+json.dumps(tools,indent=2)+'\n\n'
    status+='cargo metadata --offline --locked --format-version 1 --no-deps\nEXECUTE : code 0 ; cargo-metadata.stdout.jsonl et cargo-metadata.stderr.txt.\n\n'
    status+='cargo clippy --offline --locked --workspace --all-targets --message-format=json -- -W clippy::too_many_lines -W clippy::cognitive_complexity\nEXECUTE : code 0, build-finished=true ; '+str(result['warnings_emitted'])+' warnings emis (doublons de cibles inclus), 0 erreur. cargo-clippy.stdout.jsonl et cargo-clippy.stderr.txt.\n\n'
    status+='cargo modules structure --package <chacune des 8 crates>\n[Non vérifié] Indisponible dans le PATH ; aucune installation.\n\n'
    status+='cargo depgraph --workspace-only\n[Non vérifié] Indisponible dans le PATH ; aucune installation.\n\n'
    status+='tokei ou scc\n[Non vérifié] Indisponibles dans le PATH ; mesures lexicales distinctes dans files.tsv.\n\n'
    status+='Historique : auto-test initial et premiere tentative Clippy interrompus par evenements Git d’origine inconnue. Reprises autorisees avec garde inchangee ; auto-tests puis metadata et relance Clippy reussis. Les interruptions restent des echecs, pas des PASS. Voir cargo-attempt.json, cargo-guard-event.json, cargo-clippy-interruption-event.json, cargo-guard-runs.json et cargo-results.json.\n'
    (OUT/'requested-commands.txt').write_text(status,encoding='utf8')
