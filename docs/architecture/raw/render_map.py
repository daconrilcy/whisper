"""Relevé statique documentaire ; aucune commande Cargo."""
from pathlib import Path
import json,re,collections,csv
ROOT=Path(__file__).resolve().parents[3]; DOC=ROOT/'docs/architecture'; RAW=DOC/'raw'
load=lambda n:json.loads((RAW/n).read_text(encoding='utf8'))
F=load('files.json'); FN=load('functions.json'); M=load('modules.json'); H=load('occurrences.json'); MAN=load('manifests.json')
BF={f['path']:f for f in F}; names={p['package']['name'] for p in MAN['packages']}; texts={p:(ROOT/p).read_text(encoding='utf-8-sig') for p in BF}
def ref(p,a=1,b=None): return '`'+p+':'+str(a)+(('-'+str(b)) if b and b!=a else '')+'`'
def fr(f): return ref(f['path'],1,f['total'])
def fn(p,n):
 p=p if p.startswith('crates/') else 'crates/'+p
 f=next(x for x in FN if x['path']==p and x['name']==n and not x['test'])
 return ref(p,f['start'],f['end'])
def src(p): return fr(BF['crates/'+p])
def table(h,rows):
 esc=lambda x:str(x).replace('|','\\|').replace('\n',' ')
 return '\n'.join(['| '+' | '.join(h)+' |','| '+' | '.join('---' for _ in h)+' |']+['| '+' | '.join(esc(v) for v in row)+' |' for row in rows])+'\n'
def write(p,s):
 q=DOC/p; q.parent.mkdir(parents=True,exist_ok=True); q.write_text(s,encoding='utf8')
HEADER='Avancée : relevé statique actualisé au 31 octobre 2026.\nBlocage : exécution native et outils absents [Non vérifié].\nDécision : description de l’existant, sans proposition de transformation.\n\n'
contexts=collections.defaultdict(set)
for f in F:
 p=Path(f['path'])
 if p.name in ('lib.rs','main.rs') and '/src/' in f['path']: contexts[f['path']].add('crate')
 if f['integration']: contexts[f['path']].add('integration::'+p.relative_to(Path('crates')/f['crate']/'tests').as_posix().removesuffix('.rs').replace('/','::'))
changed=True
while changed:
 changed=False
 for m in M+load('includes.json'):
  if not m.get('target'): continue
  for ctx in list(contexts[m.get('path',m.get('source'))]):
   label=(ctx+'::'+((m['scope']+'::') if m['scope'] else '')+m['name']) if 'name' in m else ctx
   if label not in contexts[m['target']]: contexts[m['target']].add(label); changed=True
labels={f['path']:' / '.join(sorted(contexts[f['path']])) or '[Non vérifié] cible non rattachée' for f in F}
def paths(s):
 ts=re.findall(r'::|[A-Za-z_][A-Za-z_0-9]*|[{},*;]',s); i=ts.index('crate')+1; out=[]
 def walk(prefix):
  nonlocal i
  here=prefix[:]
  while i<len(ts):
   t=ts[i]
   if t=='::': i+=1
   elif t=='{':
    i+=1
    while i<len(ts) and ts[i]!='}':
     walk(here)
     if i<len(ts) and ts[i]==',': i+=1
    if i<len(ts): i+=1
    break
   elif t in (',','}',';'): break
   elif t=='as': i+=2; break
   else: here.append(t); i+=1
  if here: out.append(here)
 walk([]); return out
edges=[]
for f in F:
 for u in f['imports']:
  for bits in paths(u['text']):
   path='crate::'+'::'.join(bits)
   targets=[z for z in F if z['crate']==f['crate'] and any(path==c or path.startswith(c+'::') for c in contexts[z['path']] if c!='crate')]
   target=max(targets,key=lambda z:max(len(c) for c in contexts[z['path']]))['path'] if targets else None
   edges.append(dict(source=f['path'],line=u['line'],test=u['test'],path=path,target=target))
(RAW/'use-crate-edges.json').write_text(json.dumps(edges,ensure_ascii=False,indent=2),encoding='utf8')
for f in FN:
 tests=set(n for a,b in BF[f['path']]['test_ranges'] for n in range(a,b+1)); f['test_lines']=f['lines'] if f['test'] else sum(n in tests for n in range(f['start'],f['end']+1)); f['production_lines']=f['lines']-f['test_lines']
roles={
'whisper-core':('Bibliothèque de domaine et application','Définit commandes, vues et ports et transforme les commandes en effets.','Contient les politiques CPU/GPU et l’ordonnanceur Parakeet.',['src/application/contracts.rs','src/application/runtime/commands.rs','src/ports.rs','src/compute_policy.rs','src/parakeet_scheduler.rs']),
'whisper-adapters':('Bibliothèque d’adaptateurs','Exécute les effets avec capture, archives et transport vers les workers.','AsyncImportIo échange effets et événements avec des services sur threads séparés.',['src/worker_ipc/runtime/effects.rs','src/capture.rs','src/archive/publication.rs','src/worker_ipc/service/mod.rs']),
'whisper-worker-common':('Bibliothèque de support des workers','Fournit décodage audio, MP3, trames JSON et vérification SHA-256.','Expose l’alias MemoryObserver, un pointeur de fonction optionnel d’observation.',['src/decoder.rs','src/encoder.rs','src/ipc.rs','src/integrity.rs','src/observation.rs']),
'whisper-worker-cpu':('Worker binaire CPU','Lit les commandes IPC et exécute import, live ou décodage.','Charge ou réutilise un moteur whisper-rs CPU et émet ses résultats par IPC.',['src/startup.rs','src/session/mod.rs','src/engine_cache.rs','src/native_engine.rs','src/session/import.rs']),
'whisper-worker-gpu':('Worker binaire CUDA','Exécute import, live, décodage et qualification GPU depuis le protocole IPC.','Charge CUDA et rapproche le périphérique natif de l’identité demandée.',['src/session/mod.rs','src/gpu_commands.rs','src/native_engine.rs','src/gpu_attestation.rs']),
'whisper-worker-parakeet-cpu':('Worker binaire Parakeet, backend conditionnel','Charge Parakeet depuis ses bibliothèques natives puis traite son protocole IPC.','Le backend dépend de la feature et le mode décodage seul est distinct de la transcription.',['src/startup.rs','src/session.rs','src/native_ffi.rs','src/gpu_identity.rs','src/decode_only.rs']),
'whisper-desktop':('Binaire et bibliothèque de présentation','Compose application et adaptateurs puis ouvre une fenêtre eframe/Glow.','Les contrôleurs construisent les commandes import/micro et les vues présentent AppView.',['src/main.rs','src/root.rs','src/lib.rs','src/ui/model/controller/actions.rs','src/ui/model/view.rs']),
'whisper-bootstrap':('Binaire','Construit une vue par défaut et référence la version IPC et le type du transport.','Son point d’entrée réserve les opérations d’installation dans un commentaire.',['src/main.rs'])}
D=load('module-responsibilities.json')
def responsibility(f):
 p=f['path']; fs=[x for x in FN if x['path']==p]; source=fr(f)
 if f['test_file']:
  chosen=[x for x in fs if x['own_test']][:2] or fs[:2]
  return 'Tests ou supports : '+('définit '+', '.join('`'+x['name']+'` '+ref(p,x['start'],x['end']) for x in chosen) if chosen else 'déclare les sous-modules/réexportations de '+source)+'.'
 if p in D: return D[p]+' '+source
 if p.endswith('/lib.rs'): return 'Déclare les modules et interfaces de bibliothèque. '+source
 if p.endswith('/main.rs'): return ('Compose le desktop et ouvre la fenêtre.' if f['crate']=='whisper-desktop' else ('Construit une vue et référence le protocole/transport.' if f['crate']=='whisper-bootstrap' else 'Délègue le point d’entrée à startup::run.'))+' '+source
 chosen=[x for x in fs if not x['test']][:3]
 return ('Définit les opérations '+', '.join('`'+x['name']+'` '+ref(p,x['start'],x['end']) for x in chosen)+'.') if chosen else 'Contient les déclarations de '+source+'.'
external=[]
for f in F:
 for m in re.finditer(r'\buse\s+(whisper_[a-z_]+)\s*::[^;]+;',texts[f['path']]):
  a=texts[f['path']].count('\n',0,m.start())+1; b=a+m[0].count('\n'); external.append(dict(source=f['path'],crate=f['crate'],dependency=m[1].replace('_','-'),line=a,end=b,text=' '.join(m[0].split()),test=f['test_file'] or any(lo<=a<=hi for lo,hi in f['test_ranges'])))
(RAW/'public-imports.json').write_text(json.dumps(external,ensure_ascii=False,indent=2),encoding='utf8')
# Selected APIs pair present definitions with concrete consumers, including qualified calls.
APIS={
'whisper-core':[
 ('Application','whisper-core/src/application/contracts.rs',None,'whisper-desktop/src/ui/model/controller.rs','dispatch'),
 ('AppCommand','whisper-core/src/application/contracts.rs',None,'whisper-desktop/src/ui/model/controller/actions.rs','start_import'),
 ('AppView','whisper-core/src/application/contracts.rs',None,'whisper-desktop/src/ui/model.rs','with_microphones'),
 ('ImportIoPort','whisper-core/src/ports.rs',None,'whisper-adapters/src/worker_ipc/service/mod.rs','submit'),
 ('ImportEffect','whisper-core/src/ports.rs',None,'whisper-adapters/src/worker_ipc/runtime/effects.rs','effect'),
 ('ImportEvent','whisper-core/src/ports.rs',None,'whisper-adapters/src/worker_ipc/runtime/effects.rs','effect'),
 ('DecodeRequest','whisper-core/src/ports.rs',None,'whisper-worker-common/src/decoder.rs','decode_requested_block'),
 ('ParakeetScheduler','whisper-core/src/parakeet_scheduler.rs',None,'whisper-adapters/src/worker_ipc/runtime/poll_unified.rs','poll_unified_sessions')],
'whisper-adapters':[
 ('AsyncImportIo','whisper-adapters/src/worker_ipc/service/mod.rs',None,'whisper-desktop/src/root.rs','compose'),
 ('WorkerPaths','whisper-adapters/src/supervisor.rs',None,'whisper-desktop/src/root.rs','compose'),
 ('WasapiMicrophonePort','whisper-adapters/src/capture.rs',None,'whisper-desktop/src/main.rs','main'),
 ('LocalModelCatalog','whisper-adapters/src/model_catalog.rs',None,'whisper-desktop/src/root.rs','compose'),
 ('probe_cuda_worker','whisper-adapters/src/worker_ipc/backends/mod.rs','probe_cuda_worker','whisper-desktop/src/root.rs','compose')],
'whisper-worker-common':[
 ('decode_wav_windows','whisper-worker-common/src/decoder.rs','decode_wav_windows','whisper-worker-cpu/src/decoder.rs','decode_wav_windows'),
 ('decode_requested_block','whisper-worker-common/src/decoder.rs','decode_requested_block','whisper-worker-gpu/src/decoder.rs','decode_requested_block'),
 ('LiveMp3Encoder','whisper-worker-common/src/encoder.rs',None,'whisper-worker-cpu/src/session/live.rs','run_live_service'),
 ('read_frame','whisper-worker-common/src/ipc.rs','read_frame','whisper-worker-gpu/src/ipc.rs','read_frame'),
 ('write_frame','whisper-worker-common/src/ipc.rs','write_frame','whisper-worker-cpu/src/ipc.rs','write_frame'),
 ('verify_sha256','whisper-worker-common/src/integrity.rs','verify_sha256','whisper-worker-cpu/src/session/import.rs','run_import'),
 ('MemoryObserver','whisper-worker-common/src/observation.rs',None,'whisper-worker-gpu/src/ipc.rs','observer')],
'whisper-desktop':[
 ('run','whisper-desktop/src/lib.rs','run','whisper-desktop/src/main.rs','main'),
 ('DesktopStartup','whisper-desktop/src/lib.rs',None,'whisper-desktop/src/main.rs','main')]
}
def api_rows(c):
 rows=[]
 for symbol,definition,method,consumer,call in APIS.get(c,[]):
  if method: definition_ref=fn(definition,method)
  else:
   t=next(t for t in BF['crates/'+definition]['types'] if t['name']==symbol and not t['test']); definition_ref=ref('crates/'+definition,t['start'],t['end'])
  rows.append(['`'+symbol+'`',definition_ref,fn(consumer,call)])
 return rows

def clippy_section():
 result=load('cargo-results.json'); prod=load('clippy-complexity-production.json'); tests=load('clippy-complexity-tests.json')
 text='Commande : `cargo clippy --offline --locked --workspace --all-targets --message-format=json -- -W clippy::too_many_lines -W clippy::cognitive_complexity`. Code 0, build-finished=true ; '+str(result['warnings_emitted'])+' diagnostics warning émis, dont 257 too_many_lines et 50 cognitive_complexity (doublons de cibles inclus). Sources : [JSON brut](raw/cargo-clippy.stdout.jsonl), [stderr](raw/cargo-clippy.stderr.txt), [résumé](raw/cargo-results.json). Aucun test exécuté et aucun correctif appliqué.\n\n'
 text+='[Déduction] Le contexte de compilation est attribué en reliant chaque diagnostic à l’artefact suivant du même package/cible dans le flux JSON. Les spans et scores proviennent directement de Clippy. Un même source hors tests peut être compilé avec cfg(test) ; ces métriques sont conservées séparément. [Méthode](raw/METHOD.md).\n\nFonctions dans les sources hors tests, compilation non-test : **'+str(len(prod))+' signalées**, donc seulement cinq entrées disponibles pour ce top 20. Seuil cognitif observé : 25.\n\n'
 def rows(ds):
  return [[i,'`'+d['function']+'`',d['metric'],ref(d['path'],d['function_start'],d['function_end']),ref(d['path'],d['line'],d['end'])] for i,d in enumerate(ds[:20],1)]
 text+=table(['Rang','Fonction','Score','Plage fonction','Span Clippy'],rows(prod))
 text+='\nÀ titre distinct, poll_unified_sessions atteint 71 dans la compilation test, contre 64 hors tests. Les variantes sont dans [production compilée sous tests](raw/clippy-complexity-production-in-tests.json) et ne sont pas fusionnées au top production. `crates/whisper-adapters/src/worker_ipc/runtime/poll_unified.rs:382-1373`.\n\nTop 20 des fonctions/auxiliaires dans les sources classées tests (**39 fonctions distinctes signalées**) :\n\n'+table(['Rang','Fonction de test/support','Score','Plage fonction','Span Clippy'],rows(tests))
 text+='\n'+str(result['unique_diagnostics'])+' diagnostics distincts par (lint, fichier, lignes, message), sans fusion de scores différents. Sources des diagnostics émis : '+str(result['by_source_context']['production'])+' dans des plages hors tests et '+str(result['by_source_context']['tests'])+' dans des plages tests, doublons de compilation inclus. La compilation non-test/test représente '+str(result['by_compilation_context']['non_test'])+'/'+str(result['by_compilation_context']['test'])+' messages ; cette seconde ventilation porte sur les cibles, pas sur la nature des sources. [Détail TSV](raw/clippy-diagnostics.tsv).'
 return text

allrows=[]
for package in MAN['packages']:
 c=package['package']['name']; fs=[f for f in F if f['crate']==c]; typ,role,role2,spec=roles[c]; refs=' ; '.join(src(c+'/'+p) for p in spec); prod=sum(f['production'] for f in fs); tests=sum(f['tests'] for f in fs)
 txt=HEADER+'# '+c+'\n\n## Rôle et type\n\n'+role+' '+refs+'\n\n'+role2+' '+refs+'\n\nType : **'+typ+'**. '+refs+'\n\nMesures : **'+str(prod)+' production / '+str(tests)+' tests** ; lignes physiques, vides/commentaires inclus. Sources par fichier ci-dessous ; [méthode](../raw/METHOD.md).\n\n## Dépendances déclarées\n\n'
 cargo=package['manifest_path']; ls=(ROOT/cargo).read_text(encoding='utf8').splitlines(); rows=[]; groups=[(k,package.get(k,{})) for k in ['dependencies','dev-dependencies','build-dependencies']]+[(t+' / '+k,v.get(k,{})) for t,v in package.get('target',{}).items() for k in ['dependencies','dev-dependencies','build-dependencies']]
 for group,ds in groups:
  for name,value in ds.items():
   ln=next((i for i,s in enumerate(ls,1) if re.match(re.escape(name)+r'\s*[.=]',s)),1); rows.append([name,'interne' if name in names else 'externe',group,json.dumps(value,ensure_ascii=False),ref(cargo,ln)])
 txt+=table(['Dépendance','Nature','Groupe/condition','Déclaration','Source'],rows)+'\nMetadata et Clippy workspace/all-targets ont terminé avec code 0 ; [résultats](../raw/cargo-results.json). [Non vérifié] Transitives (metadata sans dépendances), autres combinaisons de features et exécution des tests. Features déclarées : '+ref(cargo,1,len(ls))+'.\n\n## Interfaces publiques et usages intercrates\n\n'
 consumers=[e for e in external if e['dependency']==c and e['crate']!=c]
 txt+=table(['Consommateur','Contexte','Import observé','Source'],[[e['crate'],'tests' if e['test'] else 'hors tests / cfg','`'+e['text']+'`',ref(e['source'],e['line'],e['end'])] for e in consumers]) if consumers else '[Non vérifié] Aucun import consommateur d’une autre crate relevé ; les autres usages ne sont pas exclus.\n'
 if c in APIS: txt+='\nInterfaces principales avec définitions et sites d’usage (pour desktop, entre cibles du même package) :\n\n'+table(['Interface','Définition','Usage observé'],api_rows(c))
 txt+='\nDéfinitions publiques détaillées par module ci-dessous ; réexportations de racine : '+src(c+'/src/'+('lib.rs' if 'crates/'+c+'/src/lib.rs' in BF else 'main.rs'))+'. Imports textuels, pas un graphe de symboles résolu.\n'
 if c.startswith('whisper-worker-') and c!='whisper-worker-common': txt+='\nInterface entre processus : '+src('whisper-core/src/'+('parakeet_ipc.rs' if c.endswith('parakeet-cpu') else 'ipc.rs'))+'.\n'
 if package.get('package',{}).get('build'):
  txt+='\nCible de build partagée : `scripts/recette-build.rs:1-7` (7 lignes hors tests, comptées une seule fois hors des totaux par crate). Elle refuse recette-hooks en release. Déclaration : '+ref(cargo,7)+'.\n'
 txt+='\n## Frontières constatées\n\n'; ops=[]
 categories={'Disque':r'\b(?:fs|File|OpenOptions)\s*::|\.sync_all\s*\(', 'Processus':r'\bCommand\s*::\s*new\s*\(|\.kill\s*\(|\.try_wait\s*\(', 'Threads/canaux':r'\bthread\s*::|mpsc\s*::|\.spawn\s*\(', 'Entrées/sorties standard':r'\b(?:stdin|stdout|stderr)\s*\(', 'GPU/natif':r'\b(?:WhisperContextParameters|Library|NativeParakeet)\b|\.use_gpu\s*\(|cuda_devices\s*\(', 'Réseau explicite':r'\b(?:TcpStream|UdpSocket|reqwest|TcpListener)\b'}
 for label,pattern in categories.items():
  hits=[]; seen=set()
  for f in fs:
   if f['test_file']: continue
   for m in re.finditer(pattern,texts[f['path']]):
    line=texts[f['path']].count('\n',0,m.start())+1
    if not any(a<=line<=b for a,b in f['test_ranges']) and f['path'] not in seen: seen.add(f['path']); hits.append(ref(f['path'],line))
  ops.append([label,' ; '.join(hits) if hits else '[Non vérifié] Aucun motif hors tests relevé ; effets indirects non exclus.'])
 txt+=table(['Frontière (repérage textuel)','Sites hors tests, sous leurs cfg'],ops)+'\nLes sites désignent du code présent, pas une exécution ; [lignes brutes](../raw/rg-threads-processes-io.txt).\n\n## Carte des fichiers et modules\n\n'+table(['Module/cible','Responsabilité et source','Prod','Tests'],[[labels[f['path']],responsibility(f),f['production'],f['tests']] for f in fs])
 ms=[m for m in M if m['crate']==c]
 txt+='\n## Déclarations de modules\n\n'+table(['Déclaration','Forme','Contexte','Cible/plage','Source'],[[((m['scope']+'::') if m['scope'] else '')+m['name'],'inline' if m['body']<m['stop'] else 'externe','tests' if m['test'] else 'hors tests / cfg',m.get('target') or 'plage locale',ref(m['path'],m['attr_start'],m['end'])] for m in ms])+'\n## Détail des modules\n\n[Non vérifié] Relations `use crate::` textuelles : alias, réexportations, glob imports, chemins `self`/`super`, références hors `use`, macros et cibles distinctes ne forment pas ici un graphe complet.\n'
 for f in fs:
  p=f['path']; funcs=[x for x in FN if x['path']==p]; types=f['types']; outgoing=[e for e in edges if e['source']==p]; incoming=[e for e in edges if e['target']==p and e['source']!=p]
  txt+='\n### '+labels[p]+' — `'+p+'`\n\n'+responsibility(f)+'\n\nMesure : '+str(f['production'])+' production / '+str(f['tests'])+' tests / '+str(f['total'])+' physiques. '+fr(f)+'\n\n'
  txt+='Types : '+('; '.join('`'+x['name']+'` ('+x['kind']+', '+('tests' if x['test'] else 'hors tests')+') '+ref(p,x['start'],x['end']) for x in types) or 'aucun struct/enum/trait/alias relevé dans '+fr(f))+'.\n\nFonctions avec corps : '+('; '.join('`'+x['name']+'`'+(' [tests]' if x['test'] else '')+' '+ref(p,x['start'],x['end']) for x in funcs) or 'aucune relevée dans '+fr(f))+'.\n\n'
  pubs=['`'+line.strip().replace('`','')+'` '+ref(p,i) for i,line in enumerate(texts[p].splitlines(),1) if re.search(r'\bpub\s+(?:(?:async|unsafe|const|extern)\s+)*(?:fn|struct|enum|trait|type|mod|use|const|static)\b',line)]
  txt+='Déclarations `pub` (visibilité sous conditions) : '+('; '.join(pubs) or 'aucune relevée dans '+fr(f))+'.\n\nDépend de, via `use crate::` : '+('<br>'.join(('tests : ' if e['test'] else 'hors tests : ')+'`'+e['path']+'` → '+('`'+labels[e['target']]+'`' if e['target'] else '[Non vérifié] symbole/réexport non résolu')+' '+ref(p,e['line']) for e in outgoing) or '[Non vérifié] Aucun import de cette forme relevé. '+fr(f))+'.\n\nUtilisé par, via ces imports : '+('<br>'.join(('tests : ' if e['test'] else 'hors tests : ')+'`'+labels[e['source']]+'` '+ref(e['source'],e['line']) for e in incoming) or '[Non vérifié] Aucun dépendant par cet index ; absence d’usage non démontrée.')+'.\n'
  for m in [m for m in ms if m['path']==p and m['body']<m['stop']]:
   a=m['attr_start']; b=m['end']; tl=set(n for lo,hi in f['test_ranges'] for n in range(lo,hi+1)); tc=sum(n in tl for n in range(a,b+1)); local=[x for x in funcs if m['start']<=x['start']<=b]
   txt+='\n#### Inline `'+((m['scope']+'::') if m['scope'] else '')+m['name']+'`\n\n'+('Tests/auxiliaires' if m['test'] else 'Opérations')+' : '+(', '.join('`'+x['name']+'` '+ref(p,x['start'],x['end']) for x in local[:4]) or 'déclarations de '+ref(p,a,b))+'. '+ref(p,a,b)+'\n\nMesure locale : '+str(b-a+1-tc)+' production / '+str(tc)+' tests ; sous-ensemble non additionnable. '+ref(p,a,b)+'\n\nTypes locaux : '+(', '.join('`'+x['name']+'` '+ref(p,x['start'],x['end']) for x in types if a<=x['start']<=b) or 'aucun relevé dans '+ref(p,a,b))+'.\n\nImports locaux : '+('; '.join('`'+e['path']+'` '+ref(p,e['line']) for e in outgoing if a<=e['line']<=b) or 'aucun `use crate::` relevé dans '+ref(p,a,b))+'. [Non vérifié] Dépendants propres à ce module inline non résolus ; voir le conteneur.\n'
 txt+='\n## Comptages textuels\n\n'+table(['Motif','Production','Tests'],[[key,sum(h['pattern']==key and h['path'].startswith('crates/'+c+'/') and not h['test'] for h in H),sum(h['pattern']==key and h['path'].startswith('crates/'+c+'/') and h['test'] for h in H)] for key in sorted({h['pattern'] for h in H})])+'\nChaque occurrence sourcée : [occurrences.tsv](../raw/occurrences.tsv). Chaînes/commentaires inclus ; appels effectifs non résolus.\n'
 diagnostics=[d for d in load('clippy-diagnostics.json') if d['path'].startswith('crates/'+c+'/')]
 txt+='\n## Diagnostics Clippy de cette crate\n\nCommande workspace/all-targets, code 0, aucun test exécuté. '+str(len(diagnostics))+' messages warning concernant les sources de cette crate, répétitions entre cibles incluses : '+str(sum(d['source_test'] is False for d in diagnostics))+' sur plages hors tests et '+str(sum(d['source_test'] is True for d in diagnostics))+' sur plages tests. [Sortie brute](../raw/cargo-clippy.stdout.jsonl), [détail sourcé](../raw/clippy-diagnostics.tsv).\n'
 txt+=table(['Diagnostic','Plages hors tests','Plages tests'],[[code,sum(d['code']==code and d['source_test'] is False for d in diagnostics),sum(d['code']==code and d['source_test'] is True for d in diagnostics)] for code in sorted({d['code'] for d in diagnostics})])
 write(Path('crates')/(c+'.md'),txt); allrows.append([c,typ,role+' '+refs,prod,tests])
prod_fs=sorted(F,key=lambda f:f['production'],reverse=True); prod_fn=sorted([f for f in FN if not f['test']],key=lambda f:f['production_lines'],reverse=True); test_fn=sorted([f for f in FN if f['test']],key=lambda f:f['lines'],reverse=True)
for name,data in [('top-production-functions',prod_fn),('top-test-functions',test_fn)]: (RAW/(name+'.json')).write_text(json.dumps(data,ensure_ascii=False,indent=2),encoding='utf8')
graph=['flowchart TD']
for p in MAN['packages']:
 c=p['package']['name']; graph.append('    '+c.replace('-','_')+'["'+c+'"]')
 for d in p.get('dependencies',{}):
  if d in names: graph.append('    '+c.replace('-','_')+' --> '+d.replace('-','_'))
arch=HEADER+'# Cartographie du dépôt Whisper\n\n## 1. Vue d’ensemble\n\n'+'\n'.join([
'- Huit crates sont déclarées dans '+ref('Cargo.toml',1,11)+'.',
'- Le desktop compose l’application et ouvre eframe/Glow. '+fn('whisper-desktop/src/main.rs','main')+' ; '+fn('whisper-desktop/src/lib.rs','run')+'.',
'- Le cœur route les commandes ; adapters exécute les effets. '+fn('whisper-core/src/application/runtime/commands.rs','dispatch')+' ; '+fn('whisper-adapters/src/worker_ipc/runtime/effects.rs','effect')+'.',
'- Les workers Turbo partagent le décodage et les services de worker-common. '+src('whisper-worker-cpu/src/decoder.rs')+' ; '+src('whisper-worker-gpu/src/decoder.rs')+' ; '+src('whisper-worker-common/src/lib.rs')+'.',
'- Parakeet utilise un autre protocole et un backend conditionnel. '+src('whisper-core/src/parakeet_ipc.rs')+' ; '+fn('whisper-worker-parakeet-cpu/src/gpu_identity.rs','configured_backend')+'.',
'- L’import UI ajoute à la file ; Traiter et Reprendre sont explicites. '+fn('whisper-desktop/src/ui/model/controller/actions.rs','start_import')+' ; '+fn('whisper-desktop/src/ui/model/view/queue.rs','queue_ui')+'.',
'- Des chemins live direct, historique et unifié sont présents. '+fn('whisper-adapters/src/worker_ipc/runtime/live.rs','start_direct_live')+' ; '+fn('whisper-adapters/src/worker_ipc/runtime/live.rs','start_unified_live')+'.',
'- Bootstrap possède un point d’entrée de dix lignes. '+src('whisper-bootstrap/src/main.rs')+'.',
'- '+str(len(F))+' fichiers Rust : '+str(sum(f['production'] for f in F))+' lignes physiques production et '+str(sum(f['tests'] for f in F))+' tests ; [sources par fichier](raw/files.tsv).'])+'\n\n## 2. Graphe des crates\n\nFlèche : consommateur → dépendance interne normale déclarée ; packages confirmés par [metadata brut](raw/cargo-metadata.stdout.jsonl). Sources : '+' ; '.join(ref(p['manifest_path'],1,len((ROOT/p['manifest_path']).read_text(encoding='utf8').splitlines())) for p in MAN['packages'])+'. Conditions et autres groupes dans les fiches.\n\n```mermaid\n'+'\n'.join(graph)+'\n```\n\n## 3. Tableau des crates\n\n'+table(['Nom','Type','Rôle et source','Lignes prod','Lignes tests'],allrows)+'\nLignes physiques : vides/commentaires inclus. Tests externes héritant du `cfg(test)` du déclarant ; features de recette hors tests. [Méthode](raw/METHOD.md).\n\n## 4. Carte des modules par crate\n\n'
for c in roles:
 arch+='### ['+c+'](crates/'+c+'.md)\n\n'+table(['Module/cible','Responsabilité et source','Prod','Tests'],[[labels[f['path']],responsibility(f),f['production'],f['tests']] for f in F if f['crate']==c])+'\n'
 inline=[m for m in M if m['crate']==c and m['body']<m['stop']]
 if inline:
  rows=[]
  for m in inline:
   f=BF[m['path']]; a=m['attr_start']; b=m['end']; tl=set(n for lo,hi in f['test_ranges'] for n in range(lo,hi+1)); tc=sum(n in tl for n in range(a,b+1)); rows.append([((m['scope']+'::') if m['scope'] else '')+m['name'],'tests/auxiliaires' if m['test'] else 'déclarations de la plage',b-a+1-tc,tc,ref(m['path'],a,b)])
  arch+='Modules inline (sous-ensembles non additionnables) :\n\n'+table(['Module','Responsabilité déclarative','Prod','Tests','Source'],rows)+'\n'
arch+='Déclarations externes et inline : fiches et [modules.json](raw/modules.json). Imports/dépendants `use crate::` détaillés par fichier.\n\n## 5. Points chauds\n\n### 5.1 Fichiers : top 20 par production\n\n'+table(['Rang','Fichier et plage','Production','Tests','> 1 500 prod'],[[i,fr(f),f['production'],f['tests'],'oui' if f['production']>1500 else ''] for i,f in enumerate(prod_fs[:20],1)])+'\n'+str(sum(f['production']>1500 for f in F))+' fichier(s) > 1 500 lignes production. [Liste complète](raw/files-over1500.tsv).\n\n### 5.2 Fonctions : top 20 de production\n\nPlages physiques, signature/commentaires inclus, distinctes du comptage Clippy. Tri par lignes hors tests ; closures et macros non développées.\n\n'+table(['Rang','Fonction','Fichier : début–fin','Prod','Tests internes','Physique','> 150 physique'],[[i,'`'+f['name']+'`',ref(f['path'],f['start'],f['end']),f['production_lines'],f['test_lines'],f['lines'],'oui' if f['lines']>150 else ''] for i,f in enumerate(prod_fn[:20],1)])+'\n'+str(sum(f['lines']>150 for f in prod_fn))+' fonctions hors tests > 150 lignes physiques. [Liste complète](raw/functions-over150.tsv) ; [tests séparés](raw/top-test-functions.json).\n\n### 5.3 Complexité Clippy : top 20\n\n'+clippy_section()+'\n\n### 5.4 Comptages textuels\n\n'+table(['Motif','Production','Tests'],[[key,sum(h['pattern']==key and not h['test'] for h in H),sum(h['pattern']==key and h['test'] for h in H)] for key in sorted({h['pattern'] for h in H})])+'\nChaque occurrence sourcée : [occurrences.tsv](raw/occurrences.tsv) ; [recherche brute](raw/rg-markers.txt). Chaînes/commentaires inclus ; `.spawn` peut aussi concerner un processus.\n\n## 6. Flux principaux\n\n'
arch+='\n'.join('- ['+title+'](flows/'+name+'.md)' for name,title in [('import','Import de fichier'),('direct','Transcription en direct'),('queue','File d’attente'),('recovery','Reprise après interruption'),('publication','Publication sur disque'),('compute','Choix CPU/GPU'),('startup','Démarrage et composition')])+'\n\n## 7. Écarts constatés\n\n'+(RAW/'observations.fragment.md').read_text(encoding='utf8')+'\n\n## 8. Zones non vérifiées et limites de cette cartographie\n\n'+(RAW/'limits.fragment.md').read_text(encoding='utf8')+'\n'
write(Path('ARCHITECTURE.md'),arch)
print(json.dumps(dict(files=len(F),module_declarations=len(M),inline_modules=sum(m['body']<m['stop'] for m in M),production=sum(f['production'] for f in F),tests=sum(f['tests'] for f in F),production_functions=len(prod_fn),functions_over150=sum(f['lines']>150 for f in prod_fn)),ensure_ascii=False))
