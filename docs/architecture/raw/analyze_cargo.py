"""Analyse des sorties Cargo JSON ; écrit uniquement des relevés documentaires."""
from pathlib import Path
import json,re,collections,csv
ROOT=Path(__file__).resolve().parents[3]; R=Path(__file__).parent
read=lambda n:json.loads((R/n).read_text(encoding='utf-8-sig'))
F=read('files.json'); N=read('functions.json'); BF={x['path']:x for x in F}; meta=read('cargo-metadata.stdout.jsonl'); runs=read('cargo-guard-runs.json')
rows=[]
for line in (R/'cargo-clippy.stdout.jsonl').read_text(encoding='utf-8-sig').splitlines():
 try: rows.append(json.loads(line))
 except ValueError: pass
def norm(p):
 p=p.replace('\\','/'); prefix=ROOT.as_posix()+'/'
 return p[len(prefix):] if p.lower().startswith(prefix.lower()) else p
pending=collections.defaultdict(list); diags=[]
def key(x): return (x.get('package_id'),x.get('target',{}).get('name'),tuple(x.get('target',{}).get('kind',[])))
for index,x in enumerate(rows):
 if x.get('reason')=='compiler-message':
  m=x['message']; primary=next((s for s in m['spans'] if s.get('is_primary')),m['spans'][0] if m['spans'] else {})
  path=norm(primary.get('file_name','')); start=primary.get('line_start'); end=primary.get('line_end'); file=BF.get(path)
  candidates=[n for n in N if n['path']==path and start and n['start']<=start<=n['end']]
  function=min(candidates,key=lambda n:n['end']-n['start']) if candidates else None
  source_test=(file['test_file'] or any(a<=start<=b for a,b in file['test_ranges'])) if file and start else None
  match=re.search(r'\((\d+)/(\d+)\)',m['message'])
  record=dict(stream_index=index,package_id=x['package_id'],target=x['target']['name'],target_kind=x['target']['kind'],level=m['level'],code=(m.get('code') or {}).get('code'),message=m['message'],path=path,line=start,end=end,source_test=source_test,function=function['name'] if function else None,function_start=function['start'] if function else None,function_end=function['end'] if function else None,metric=int(match[1]) if match else None,threshold=int(match[2]) if match else None,compilation_test=None,context_attribution='[Non vérifié] pas encore relié à un artefact',rendered=m.get('rendered'))
  pending[key(x)].append(record); diags.append(record)
 elif x.get('reason')=='compiler-artifact':
  for d in pending.pop(key(x),[]):
   d['compilation_test']=x['profile']['test']; d['features']=x['features'];d['context_attribution']='[Déduction] même package/cible, artefact suivant dans le flux Cargo';d['artifact_stream_index']=index

def unique(ds):
 out={}
 for d in ds:
  k=(d['code'],d['path'],d['line'],d['end'],d['message'])
  if k not in out:out[k]=d
 return list(out.values())
cognitive=[d for d in diags if d['code']=='clippy::cognitive_complexity']
production=sorted(unique(d for d in cognitive if d['source_test'] is False and d['compilation_test'] is False),key=lambda d:-d['metric'])
tests=sorted(unique(d for d in cognitive if d['source_test'] is True),key=lambda d:-d['metric'])
variants=sorted(unique(d for d in cognitive if d['source_test'] is False and d['compilation_test'] is True),key=lambda d:-d['metric'])
for name,ds in [('clippy-diagnostics.json',diags),('clippy-complexity-production.json',production),('clippy-complexity-tests.json',tests),('clippy-complexity-production-in-tests.json',variants)]: (R/name).write_text(json.dumps(ds,ensure_ascii=False,indent=2),encoding='utf8')
with (R/'clippy-diagnostics.tsv').open('w',encoding='utf8',newline='') as out:
 fields=['level','code','path','line','end','function','source_test','compilation_test','metric','threshold','message']; w=csv.DictWriter(out,fields,delimiter='\t',extrasaction='ignore');w.writeheader();w.writerows(diags)
summary=dict(status='COMPLETED_WITH_WARNINGS',head=read('cargo-preflight.json')['head'],metadata_exit_code=next(x['exit_code'] for x in runs if x['cargo_args'][0]=='metadata'),clippy_exit_code=next(x['exit_code'] for x in runs if x['cargo_args'][0]=='clippy' and x['exit_code']==0),clippy_seconds=next(x['seconds'] for x in runs if x['cargo_args'][0]=='clippy' and x['exit_code']==0),build_finished=next(x['success'] for x in rows if x.get('reason')=='build-finished'),workspace_packages=len(meta['workspace_members']),warnings_emitted=sum(d['level']=='warning' for d in diags),errors_emitted=sum(d['level']=='error' for d in diags),unique_diagnostics=len(unique(diags)),by_code=dict(collections.Counter(d['code'] for d in diags)),by_source_context={'production':sum(d['source_test'] is False for d in diags),'tests':sum(d['source_test'] is True for d in diags),'unknown':sum(d['source_test'] is None for d in diags)},by_compilation_context={'non_test':sum(d['compilation_test'] is False for d in diags),'test':sum(d['compilation_test'] is True for d in diags),'unknown':sum(d['compilation_test'] is None for d in diags)},cognitive_production_functions=len(production),cognitive_test_source_functions=len(tests),source_scope='workspace --all-targets ; pas --all-features ; aucun test execute',attribution='[Déduction] Le profil test des diagnostics est relié à l’artefact suivant du même package/cible dans le flux JSON. Les spans et métriques viennent directement de Clippy.',guard_history=['Premier SelfTest interrompu : Changed sur .git, origine inconnue.','SelfTest de reprise réussi, garde inchangée.','Metadata réussie.','Premier Clippy interrompu : Changed sur .git/objects/pack/tmp_pack_XL5c72, origine inconnue.','Relance Clippy réussie avec la garde inchangée, autorisation de retenter malgré les événements extérieurs.'],excluded_tools=['cargo-modules absent du PATH','cargo-depgraph absent du PATH','tokei et scc absents du PATH'])
static=read('manifests.json'); static_names={p['package']['name'] for p in static['packages']}; member_ids=set(meta['workspace_members']); measured_names={p['name'] for p in meta['packages'] if p['id'] in member_ids}
static_edges=sorted((p['package']['name'],d) for p in static['packages'] for d in p.get('dependencies',{}) if d in static_names)
measured_edges=sorted((p['name'],d['name']) for p in meta['packages'] if p['id'] in member_ids for d in p['dependencies'] if d['kind'] is None and d['name'] in measured_names)
summary.update(metadata_members_match_static_inventory=static_names==measured_names,metadata_normal_internal_edges_match_static_graph=static_edges==measured_edges,normal_internal_edges=measured_edges)

(R/'cargo-results.json').write_text(json.dumps(summary,ensure_ascii=False,indent=2),encoding='utf8')
print(json.dumps(summary,ensure_ascii=False,indent=2))
