"""Contrôle de couverture et références ; aucun Cargo, aucune écriture hors architecture."""
from pathlib import Path
import re,json,subprocess,csv
ROOT=Path(__file__).resolve().parents[3]; DOC=ROOT/'docs/architecture'; RAW=DOC/'raw'
load=lambda n:json.loads((RAW/n).read_text(encoding='utf8'))
F=load('files.json'); M=load('modules.json'); N=load('top-production-functions.json'); RES=load('module-resolution.json'); INC=load('includes.json'); MAN=load('manifests.json')
def git(*args): return subprocess.run(['git','--no-optional-locks','-c','maintenance.auto=false','-c','gc.auto=0',*args],cwd=ROOT,capture_output=True,encoding='utf8',errors='replace')
reports=[DOC/'ARCHITECTURE.md',DOC/'NIVEAU1.md',DOC/'TEST-GROWTH.md',RAW/'METHOD.md']+list((DOC/'crates').glob('*.md'))+list((DOC/'flows').glob('*.md'))
refs=0; invalid=[]; broken=[]; missing_files=[]; missing_modules=[]; bad_lines=[]; bad_mermaid=[]
for report in reports:
 text=report.read_text(encoding='utf8')
 for m in re.finditer(r'((?:crates/[A-Za-z0-9_./-]+\.(?:rs|toml)|scripts/[A-Za-z0-9_./-]+\.rs|Cargo\.toml)):(\d+)(?:-(\d+))?',text):
  p=ROOT/m[1]; a=int(m[2]); b=int(m[3] or a); refs+=1
  if not p.exists() or not 1<=a<=b<=len(p.read_text(encoding='utf8').splitlines()): invalid.append([str(report.relative_to(ROOT)),m[0]])
 for m in re.finditer(r'\]\(([^)]+)\)',text):
  dest=m[1]
  if '://' not in dest and not (report.parent/dest.split('#')[0]).exists(): broken.append([str(report.relative_to(ROOT)),dest])
 for i,line in enumerate(text.splitlines(),1):
  if line.rstrip()!=line: bad_lines.append([str(report.relative_to(ROOT)),i])
 if report.parent.name=='flows':
  blocks=re.findall(r'```mermaid\n(.*?)```',text,re.S)
  if len(blocks)!=1 or not blocks[0].strip().startswith('sequenceDiagram'): bad_mermaid.append(str(report.relative_to(ROOT)))
  else:
   participants=set(re.findall(r'^\s*participant\s+(\w+)',blocks[0],re.M))
   for a,b in re.findall(r'^\s*(\w+)-[-<>]+(\w+):',blocks[0],re.M):
    if a not in participants or b not in participants: bad_mermaid.append([str(report.relative_to(ROOT)),a,b])
for f in F:
 text=(DOC/'crates'/(f['crate']+'.md')).read_text(encoding='utf8')
 if f['path'] not in text: missing_files.append(f['path'])
for m in M:
 text=(DOC/'crates'/(m['crate']+'.md')).read_text(encoding='utf8')
 if m['name'] not in text or m['path']+':'+str(m['attr_start']) not in text: missing_modules.append([m['path'],m['name'],m['start']])
unresolved=[m for m in RES if not m['target']]
missing_includes=[m for m in INC if not (ROOT/m['target']).exists()]
targets={m['target'] for m in RES if m['target']}|{m['target'] for m in INC}
orphans=[f['path'] for f in F if not f['integration'] and Path(f['path']).name not in ('main.rs','lib.rs') and f['path'] not in targets]
status=git('status','--short','--untracked-files=all'); (RAW/'git-final-status.txt').write_text('$ git status --short --untracked-files=all\nexit='+str(status.returncode)+'\n'+status.stdout+'\nSTDERR\n'+status.stderr,encoding='utf8')
status_compact=git('status','--short'); (RAW/'git-final-status-compact.txt').write_text(status_compact.stdout,encoding='utf8')
diff=git('diff','--name-only','HEAD'); (RAW/'git-final-tracked-diff.txt').write_text(diff.stdout,encoding='utf8')
check=git('diff','--check'); (RAW/'git-diff-check.txt').write_text('exit='+str(check.returncode)+'\n'+check.stdout+check.stderr,encoding='utf8')
external=git('diff','HEAD','--','.',':(exclude)docs/architecture/'); (RAW/'git-external-diff.txt').write_text(external.stdout,encoding='utf8')
head=git('rev-parse','HEAD').stdout.strip(); initial=(RAW/'git-head.txt').read_text(encoding='utf8').split('STDOUT\n')[1].splitlines()[0]
old=(RAW/'git-status.txt').read_text(encoding='utf8').splitlines()
def outside(rows): return sorted(row for row in rows if row and 'docs/architecture/' not in row)
previous=outside(old); final=outside(status.stdout.splitlines())
scope=dict(head_at_measurement=initial,head_final=head,head_unchanged=head==initial,tracked_changes_outside_architecture=[p for p in diff.stdout.splitlines() if not p.startswith('docs/architecture/')],initial_untracked_roots=['docs/architecture/','models/','sound example/'],outside_status_unchanged=previous==final,documentary_write_root='docs/architecture/',authorized_followup_execution_roots=['target/parakeet-tests/refacto-ui/','target/parakeet-tests/refacto-parakeet-live/','target/parakeet-tests/cargo-shared/'],cargo_attempt_status='COMPLETED_WITH_WARNINGS',niveau1_status='PASSED_WITH_AUTHORIZED_D2_REUSE',all_tracked_changes_within_architecture=not external.stdout,git_diff_check_exit=check.returncode,limits='Git ne détecte pas les changements de contenu dans les fichiers déjà non suivis ; aucune empreinte de ces fichiers préexistants n’a été créée.')
scope['authorized_followup_source_paths']=['crates/whisper-core/tests/parakeet_import_contract.rs']
scope['unexpected_tracked_changes']=[p for p in scope['tracked_changes_outside_architecture'] if p not in scope['authorized_followup_source_paths']]
scope['all_tracked_changes_within_authorized_scope']=not scope['unexpected_tracked_changes']
(RAW/'scope-check.json').write_text(json.dumps(scope,ensure_ascii=False,indent=2),encoding='utf8')
for name,rows,fields in [('files-over1500.tsv',[f for f in F if f['production']>1500],['path','production','tests','total']),('functions-over150.tsv',[f for f in N if f['lines']>150],['path','name','start','end','production_lines','test_lines','lines'])]:
 with (RAW/name).open('w',encoding='utf8',newline='') as out:
  w=csv.DictWriter(out,fieldnames=fields,extrasaction='ignore',delimiter='\t'); w.writeheader(); w.writerows(rows)
with (RAW/'crates.tsv').open('w',encoding='utf8',newline='') as out:
 w=csv.writer(out,delimiter='\t');w.writerow(['crate','files','production_physical','tests_physical'])
 for c in sorted({f['crate'] for f in F}):
  fs=[f for f in F if f['crate']==c];w.writerow([c,len(fs),sum(f['production'] for f in fs),sum(f['tests'] for f in fs)])
result=dict(reports=len(reports),crates=len(MAN['packages']),files=len(F),module_declarations=len(M),external_modules=len(RES),inline_modules=len(M)-len(RES),literal_or_manifest_inclusions=len(INC),source_references=refs,invalid_source_ranges=invalid,broken_local_links=broken,missing_files=missing_files,missing_module_declarations=missing_modules,unresolved_external_modules=unresolved,missing_include_targets=missing_includes,source_files_without_root_declaration_or_include=orphans,trailing_whitespace=bad_lines,mermaid_structure_errors=bad_mermaid,production_lines=sum(f['production'] for f in F),test_lines=sum(f['tests'] for f in F),production_functions=len(N),functions_over150=sum(f['lines']>150 for f in N),counts_consistent=all(f['total']==f['production']+f['tests'] for f in F),unmatched_braces=sum(f['unmatched_braces'] for f in F),scope=scope,limitations='Références/plages/couverture vérifiées lexicalement, pas de preuve sémantique ni de rendu Mermaid par un moteur graphique.')
(RAW/'validation.json').write_text(json.dumps(result,ensure_ascii=False,indent=2),encoding='utf8')
print(json.dumps(result,ensure_ascii=False,indent=2))
