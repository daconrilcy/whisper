"""Lecture statique Rust, sans Cargo ni dependance tierce. Sorties dans ce dossier."""
from pathlib import Path
import re, json, bisect, subprocess, shutil, csv
ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / 'docs/architecture/raw'
def dump(name, value):
    (OUT/name).write_text(json.dumps(value, ensure_ascii=False, indent=2), encoding='utf-8')
def mask(s):
    # Preserve offsets/newlines; nested Rust comments and raw strings are recognized.
    out=list(s); i=0
    raw_re=re.compile(r'(?:br|cr|r)(#*)"')
    char_re=re.compile(r"'(?:\\(?:u\{[0-9a-fA-F_]+\}|x[0-9a-fA-F]{2}|.)|[^'\\\n])'")
    while i<len(s):
        end=None
        if s.startswith('//',i):
            end=s.find('\n',i); end=len(s) if end<0 else end
        elif s.startswith('/*',i):
            j=i+2; depth=1
            while j<len(s) and depth:
                if s.startswith('/*',j): depth+=1; j+=2
                elif s.startswith('*/',j): depth-=1; j+=2
                else: j+=1
            end=j
        else:
            raw=raw_re.match(s,i) if s[i] in 'bcr' else None
            if raw and (i==0 or not (s[i-1].isalnum() or s[i-1]=='_')):
                close='"'+raw[1]; j=s.find(close,raw.end()); end=len(s) if j<0 else j+len(close)
            elif s[i]=='"':
                j=i+1
                while j<len(s):
                    if s[j]=='\\': j+=2
                    elif s[j]=='"': j+=1; break
                    else: j+=1
                end=j
            elif s[i]=="'":
                char=char_re.match(s,i)
                if char: end=char.end()
        if end is not None:
            for k in range(i,end):
                if out[k]!='\n': out[k]=' '
            i=end
        else: i+=1
    return ''.join(out)
files=[]; functions=[]; modules=[]; hits=[]; declarations=[]
patterns={
 'cfg_test':r'#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]',
 'cfg_feature':r'#\s*\[\s*cfg\s*\([^\]]*\bfeature\s*=',
 'unsafe':r'\bunsafe\b', 'unwrap':r'\.\s*unwrap\s*\(\s*\)',
 'expect':r'\.\s*expect\s*\(', 'TODO_FIXME_HACK':r'\b(?:TODO|FIXME|HACK)\b',
 'thread_spawn':r'\bthread\s*::\s*spawn\s*\(',
 'command_new':r'\bCommand\s*::\s*new\s*\(',
 'builder_spawn':r'\.spawn\s*\(',
}
for p in sorted((ROOT/'crates').rglob('*.rs')):
    rel=p.relative_to(ROOT).as_posix(); crate=p.relative_to(ROOT).parts[1]
    s=p.read_text(encoding='utf-8-sig'); code=mask(s); lines=s.splitlines(); starts=[0]+[m.end() for m in re.finditer('\n',s)]
    ln=lambda x:bisect.bisect_right(starts,x)
    stack=[]; pairs={}
    for m in re.finditer(r'[{}]',code):
        if m[0]=='{': stack.append(m.start())
        elif stack: pairs[stack.pop()]=m.start()
    items=[]
    for m in re.finditer(r'\b(mod|fn|struct|enum|trait|type|impl)\s+([A-Za-z_][A-Za-z_0-9]*|<)',code):
        kind,name=m.groups(); pos=m.start()
        # The first brace/semicolon outside (), [] is an item delimiter.
        j=m.end(); par=br=0
        while j<len(code):
            c=code[j]
            if c=='(': par+=1
            elif c==')': par-=1
            elif c=='[': br+=1
            elif c==']': br-=1
            elif c in '{;' and par==0 and br==0: break
            j+=1
        if j>=len(code): continue
        end=pairs.get(j,j) if code[j]=='{' else j
        # Include immediately preceding attributes (including multiline attributes).
        a=s.rfind('\n',0,pos)+1
        while a>0:
            prev=code[:a].rstrip()
            if not prev.endswith(']'): break
            level=0; k=len(prev)-1
            while k>=0:
                if prev[k]==']': level+=1
                elif prev[k]=='[':
                    level-=1
                    if level==0: break
                k-=1
            if k>0 and prev[k-1]=='#': a=code.rfind('\n',0,k-1)+1
            else: break
        attrs=s[a:pos]
        test=bool(re.search(r'#\s*\[\s*(?:cfg\s*\(\s*test\s*\)|test)\s*\]',attrs) or re.search(r'#\s*\[\s*cfg\s*\(\s*all\s*\(\s*test\s*,',attrs))
        items.append(dict(kind=kind,name=name,start=ln(pos),end=ln(end),offset=pos,stop=end,body=j,attr_start=ln(a),own_test=test,signature=' '.join(s[pos:j].split())[:250]))
    test_ranges=[(x['attr_start'],x['end']) for x in items if x['own_test']]
    # Also classify cfg(test) fields, parameters, statements and expressions.
    # These are deliberately separate from complete mod-tests ranges.
    for attr in re.finditer(r'#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]',code):
        j=attr.end()
        while j<len(code):
            while j<len(code) and code[j].isspace(): j+=1
            if code.startswith('#[',j):
                close=code.find(']',j+2)
                if close>=0: j=close+1; continue
            break
        par=br=0; end=j
        while end<len(code):
            c=code[end]
            if c=='(': par+=1
            elif c==')':
                if par==0: break
                par-=1
            elif c=='[': br+=1
            elif c==']': br-=1
            elif c=='{' and par==0 and br==0:
                end=pairs.get(end,end); break
            elif c in ',;' and par==0 and br==0: break
            end+=1
        test_ranges.append((ln(attr.start()),ln(min(end,len(s)-1))))
    integration='/tests/' in rel
    def istest(line): return integration or any(a<=line<=b for a,b in test_ranges)
    testlines={i for i in range(1,len(lines)+1) if istest(i)}
    row=dict(path=rel,crate=crate,total=len(lines),production=len(lines)-len(testlines),tests=len(testlines),test_ranges=test_ranges,integration=integration)
    imports=[]
    for m in re.finditer(r'\buse\s+crate\s*::[^;]+;',code):
        imports.append(dict(line=ln(m.start()),text=' '.join(s[m.start():m.end()].split()),test=istest(ln(m.start()))))
    row['imports']=imports
    row['crate_refs']=sorted(set(re.findall(r'\b(whisper_[a-z_]+)\s*::',code)))
    for x in items:
        x.update(path=rel,crate=crate,test=istest(x['start']))
        x['scope']='::'.join(y['name'] for y in items if y['kind'] in ('mod','impl','trait') and y['body']<x['offset']<y['stop'])
        if x['kind']=='fn' and code[x['body']]=='{':
            x['lines']=x['end']-x['start']+1; functions.append(x)
        if x['kind']=='mod': modules.append(x)
        if x['kind'] in ('struct','enum','trait','type'): declarations.append(x)
    row['types']=[dict(name=x['name'],kind=x['kind'],start=x['start'],end=x['end'],test=x['test'],scope=x['scope']) for x in items if x['kind'] in ('struct','enum','trait','type')]
    for key,pattern in patterns.items():
        for m in re.finditer(pattern,s):
            hits.append(dict(pattern=key,path=rel,line=ln(m.start()),test=istest(ln(m.start())),text=lines[ln(m.start())-1].strip()))
    # Thread builders and subprocesses can use aliases; textual counts deliberately do not resolve them.
    row['unmatched_braces']=len(stack)
    files.append(row)
# Resolve external module declarations and propagate cfg(test) through files.
by_path={f['path']:f for f in files}
resolutions=[]
for m in modules:
    if m['body']<m['stop']: continue
    p=ROOT/m['path']; text=p.read_text(encoding='utf-8-sig'); ls=text.splitlines()
    attr='\n'.join(ls[max(0,m['attr_start']-1):m['start']])
    explicit=re.search(r'#\[path\s*=\s*"([^"]+)"\]',attr)
    base=p.parent if p.stem in ('lib','main','mod') else p.parent/p.stem
    scopes=m['scope'].split('::') if m['scope'] else []
    base=base.joinpath(*scopes)
    candidates=([p.parent/explicit[1],base/explicit[1]] if explicit else [base/(m['name']+'.rs'),base/m['name']/'mod.rs'])
    target=next((q.relative_to(ROOT).as_posix() for q in candidates if q.is_file()),None)
    m['target']=target
    resolutions.append(dict(source=m['path'],line=m['start'],name=m['name'],target=target,test=m['test']))
includes=[]
for f in files:
    p=ROOT/f['path']; text=p.read_text(encoding='utf-8-sig')
    for m in re.finditer(r'include!\(\s*"([^"]+)"\s*\)',text):
        target=(p.parent/m[1]).relative_to(ROOT).as_posix(); line=text.count('\n',0,m.start())+1
        test=any(a<=line<=b for a,b in f['test_ranges'])
        includes.append(dict(source=f['path'],line=line,target=target,test=test))
    for m in re.finditer(r'include!\(concat!\(\s*env!\("CARGO_MANIFEST_DIR"\),\s*"([^"]+)"',text):
        target='crates/'+f['crate']+'/'+m[1].lstrip('/'); line=text.count('\n',0,m.start())+1
        includes.append(dict(source=f['path'],line=line,target=target,test=True))
dump('includes.json',includes)
test_files={f['path'] for f in files if f['path'].split('/',2)[-1].startswith('tests/') or '/tests/' in f['path']}
changed=True
while changed:
    changed=False
    for m in modules+includes:
        if m.get('target') and (m['test'] or m.get('path',m.get('source')) in test_files) and m['target'] not in test_files:
            test_files.add(m['target']); changed=True
for f in files:
    f['integration']=f['path'].split('/',2)[-1].startswith('tests/')
    f['test_file']=f['path'] in test_files
    if f['test_file']:
        f['test_ranges']=[(1,f['total'])]; f['tests']=f['total']; f['production']=0
        for x in f['types']+f['imports']: x['test']=True
for x in functions+modules+declarations:
    if x['path'] in test_files: x['test']=True
for x in hits:
    if x['path'] in test_files: x['test']=True
for x in resolutions:
    if x['source'] in test_files: x['test']=True
dump('module-resolution.json',resolutions)

dump('files.json',files); dump('functions.json',functions); dump('modules.json',modules); dump('occurrences.json',hits)
with (OUT/'occurrences.tsv').open('w',encoding='utf-8',newline='') as f:
    w=csv.DictWriter(f,fieldnames=['pattern','path','line','test','text'],delimiter='\t'); w.writeheader(); w.writerows(hits)
with (OUT/'files.tsv').open('w',encoding='utf-8',newline='') as f:
    w=csv.writer(f,delimiter='\t'); w.writerow(['path','production_physical','test_physical','total_physical'])
    w.writerows((r['path'],r['production'],r['tests'],r['total']) for r in files)
status=subprocess.run(['git','--no-optional-locks','-c','maintenance.auto=false','-c','gc.auto=0','status','--short','--untracked-files=all'],cwd=ROOT,capture_output=True,text=True).stdout
(OUT/'git-status.txt').write_text(status,encoding='utf-8')
availability={x:shutil.which(x) for x in ['cargo','tokei','scc','cargo-modules','cargo-depgraph','rust-analyzer']}
dump('tools.json',availability)
print(json.dumps(dict(files=len(files),production=sum(r['production'] for r in files),tests=sum(r['tests'] for r in files),functions=len(functions),modules=len(modules),unmatched=sum(r['unmatched_braces'] for r in files)),ensure_ascii=False))
