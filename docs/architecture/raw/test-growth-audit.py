"""Analyse Git en memoire ; ecrit uniquement ses rapports dans docs/architecture/raw."""
from pathlib import Path,PurePosixPath
import subprocess,json,hashlib,re,collections,difflib,time,os,sys
os.environ['GIT_OPTIONAL_LOCKS']='0'
ROOT_REAL=Path.cwd(); OUT=ROOT_REAL/'docs/architecture/raw'
def git(*args): return subprocess.check_output(['git','--no-optional-locks','-c','maintenance.auto=false','-c','gc.auto=0',*args])
cache={}; DATA={}
class VP:
 def __init__(self,p):self.p=PurePosixPath(p)
 def __truediv__(self,s):return VP(self.p/s)
 def __lt__(self,o):return str(self.p)<str(o.p)
 def relative_to(self,o):return self.p.relative_to(o.p)
 @property
 def parent(self):return VP(self.p.parent)
 @property
 def stem(self):return self.p.stem
 def joinpath(self,*a):return VP(self.p.joinpath(*a))
 def as_posix(self):return self.p.as_posix()
 def is_file(self):return self.as_posix() in DATA
 def read_text(self,**kw):return DATA[self.as_posix()]
 def rglob(self,pattern):return [VP(p) for p in DATA if p.startswith(str(self.p)+'/') and p.endswith('.rs')]
def snapshot(ref):
 global DATA
 ls=git('ls-tree','-r','-z',ref,'--','crates').split(b'\0'); entries={}
 for item in ls:
  if not item:continue
  meta,path=item.split(b'\t'); mode,kind,sha=meta.split()
  if path.endswith(b'.rs') and kind==b'blob':entries[path.decode()]=sha.decode()
 missing=list(set(entries.values())-cache.keys())
 if missing:
  raw=subprocess.run(['git','--no-optional-locks','cat-file','--batch'],input=('\n'.join(missing)+'\n').encode(),stdout=subprocess.PIPE,check=True).stdout;pos=0
  for sha in missing:
   end=raw.index(b'\n',pos);n=int(raw[pos:end].split()[-1]);cache[sha]=raw[end+1:end+1+n].decode('utf-8-sig').replace('\r\n','\n');pos=end+2+n
 DATA={p:cache[sha] for p,sha in entries.items()}
 script=(OUT/'measure.py').read_text(encoding='utf8');scope=dict(ROOT=VP('.'),re=re,bisect=__import__('bisect'),dump=lambda *a:None)
 exec(script[script.index('def mask('):script.index('files=[];')],scope)
 block=script[script.index('files=[];'):script.index("dump('module-resolution.json',resolutions)")]
 exec(block,scope)
 files=scope['files']; funcs=scope['functions']; modules=scope['modules']
 return dict(ref=ref,sha=git('rev-parse',ref).decode().strip(),files=files,functions=funcs,modules=modules,data=DATA.copy(),summary=dict(files=len(files),production=sum(f['production'] for f in files),tests=sum(f['tests'] for f in files),total=sum(f['total'] for f in files),test_files=sum(f['test_file'] for f in files),test_functions=sum(f['test'] for f in funcs)))
def dump(name,value):(OUT/name).write_text(json.dumps(value,ensure_ascii=False,indent=2),encoding='utf8')
if __name__=='__main__' and '--compare' not in sys.argv:
 refs=['f82063ae^','f82063ae','7a4d20b2','2d9c3fe8','c753f7c6','2829ce45','7154e3fe','90ead44e','17cd4927','b09a8b66','998a732a','778ef0bc','HEAD']
 rows=[]
 for ref in refs:
  snap=snapshot(ref);row=dict(ref=ref,sha=snap['sha'],**snap['summary']);rows.append(row);print(json.dumps(row),flush=True)
 dump('test-growth-phases.json',rows)

def tokens(s):
 out=[];i=0
 raw_re=re.compile(r'(?:br|cr|r)(#*)"');char_re=re.compile(r"'(?:\\(?:u\{[0-9a-fA-F_]+\}|x[0-9a-fA-F]{2}|.)|[^'\\\n])'")
 while i<len(s):
  if s[i].isspace():i+=1;continue
  if s.startswith('//',i):
   end=s.find('\n',i);i=len(s) if end<0 else end;continue
  if s.startswith('/*',i):
   j=i+2;depth=1
   while j<len(s) and depth:
    if s.startswith('/*',j):depth+=1;j+=2
    elif s.startswith('*/',j):depth-=1;j+=2
    else:j+=1
   i=j;continue
  raw=raw_re.match(s,i)
  if raw:
   end=s.find('"'+raw[1],raw.end());end=len(s) if end<0 else end+1+len(raw[1]);out.append(s[i:end]);i=end;continue
  if s[i]=='"':
   j=i+1
   while j<len(s):
    if s[j]=='\\':j+=2
    elif s[j]=='"':j+=1;break
    else:j+=1
   out.append(s[i:j]);i=j;continue
  if s[i]=="'":
   m=char_re.match(s,i)
   if m:out.append(m[0]);i=m.end();continue
  m=re.match(r'[A-Za-z_0-9]+',s[i:])
  if m:out.append(m[0]);i+=len(m[0]);continue
  out.append(s[i]);i+=1
 return out

def case_functions(snap):
 rows=[]
 for f in snap['functions']:
  text=snap['data'][f['path']]; lines=text.splitlines(True);a=sum(map(len,lines[:f['attr_start']-1]));full=text[a:f['stop']+1];body=text[f['body']:f['stop']+1]
  attrs=text[a:f['offset']]
  if not re.search(r'#\s*\[\s*test\s*\]',attrs):continue
  rows.append(dict(path=f['path'],crate=f['crate'],name=f['name'],start=f['attr_start'],end=f['end'],lines=f['end']-f['attr_start']+1,scope=f['scope'],tokens=tokens(full),body_tokens=tokens(body),text=full,attrs=attrs))
 return rows

def compare(before,after,prefix='test-growth'):
 old=case_functions(before);new=case_functions(after);remaining_old=set(range(len(old)));remaining_new=set(range(len(new)));matches=[]
 def do_match(i,j,kind):
  remaining_old.remove(i);remaining_new.remove(j);matches.append(dict(kind=kind,old=old[i],new=new[j]))
 for field,kind in [('tokens','tokens_identiques'),('body_tokens','corps_identique_attributs_ou_signature_changes')]:
  for j in sorted(remaining_new):
   choices=[i for i in remaining_old if old[i]['name']==new[j]['name'] and old[i][field]==new[j][field]]
   if choices:
    i=max(choices,key=lambda i:(old[i]['path']==new[j]['path'],old[i]['crate']==new[j]['crate'],old[i]['scope']==new[j]['scope']));do_match(i,j,kind)
 # Pair remaining same names within crates, using closest body for repeated support test names.
 for j in sorted(remaining_new):
  choices=[i for i in remaining_old if old[i]['name']==new[j]['name'] and old[i]['crate']==new[j]['crate']]
  if choices:
   i=max(choices,key=lambda i:difflib.SequenceMatcher(None,old[i]['body_tokens'],new[j]['body_tokens'],autojunk=False).ratio());do_match(i,j,'corps_modifie_a_relire')
 def compact(r):return {k:v for k,v in r.items() if k not in ('tokens','body_tokens','text','attrs')}
 dupl=[];groups=collections.defaultdict(list)
 for r in new:groups[tuple(r['body_tokens'])].append(r)
 for group in groups.values():
  if len(group)<2:continue
  prior=[r for r in old if r['body_tokens']==group[0]['body_tokens']]
  if len(group)>len(prior):dupl.append(dict(before=len(prior),after=len(group),old=[compact(r) for r in prior],new=[compact(r) for r in group]))
 result=dict(before=before['summary'],after=after['summary'],before_cases=len(old),after_cases=len(new),match_counts=dict(collections.Counter(m['kind'] for m in matches)),matches=[dict(kind=m['kind'],old=compact(m['old']),new=compact(m['new'])) for m in matches],added=[compact(new[j]) for j in sorted(remaining_new)],removed=[compact(old[i]) for i in sorted(remaining_old)],growing_duplicate_bodies=dupl)
 patches=[]
 for m in matches:
  if m['kind']=='tokens_identiques':continue
  patches.extend(difflib.unified_diff(m['old']['text'].splitlines(),m['new']['text'].splitlines(),fromfile=m['old']['path']+':'+str(m['old']['start']),tofile=m['new']['path']+':'+str(m['new']['start']),lineterm=''))
 (OUT/(prefix+'-changed-cases.diff')).write_text('\n'.join(patches)+'\n',encoding='utf8')
 dump(prefix+'-cases.json',result)
 print(json.dumps({k:v for k,v in result.items() if k not in ['matches','added','removed','growing_duplicate_bodies']},ensure_ascii=False));print('added',len(result['added']),'removed',len(result['removed']),'growing_duplicate_groups',len(dupl))
 return old,new,matches,result
if __name__=='__main__' and '--compare' in sys.argv:
 before=snapshot('f82063ae^');after=snapshot('HEAD');compare(before,after)


def partition(snap):
 cases=case_functions(snap);result=collections.Counter();perfile=[]
 for f in snap['files']:
  p=f['path'];text=snap['data'][p];lines=text.splitlines();testlines=set(range(1,len(lines)+1)) if f['test_file'] else {i for a,b in f['test_ranges'] for i in range(a,b+1)}
  testlines={i for i in testlines if i<=len(lines)}
  case_lines={i for c in cases if c['path']==p for i in range(c['start'],c['end']+1)}
  helper_lines={i for fn in snap['functions'] if fn['path']==p and fn['test'] for i in range(fn['attr_start'],fn['end']+1)}
  # Imports outside functions; comments/strings masked by the same lexer as the map.
  scope=dict(re=re);src=(OUT/'measure.py').read_text(encoding='utf8');exec(src[src.index('def mask('):src.index('files=[];')],scope);code=scope['mask'](text)
  imports=set()
  for m in re.finditer(r'\buse\s+[^;]+;',code):imports.update(range(text.count('\n',0,m.start())+1,text.count('\n',0,m.end()-1)+2))
  count=collections.Counter()
  for i in testlines:
   kind='fonctions_test' if i in case_lines else 'fonctions_support' if i in helper_lines else 'imports_hors_fonctions' if i in imports else 'autres_types_constantes_modules_blancs_commentaires'
   count[kind]+=1
  result.update(count);perfile.append(dict(path=p,**count))
 return dict(totals=dict(result),files=perfile)
