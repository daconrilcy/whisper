"""Prépare les ressources de test sous la racine autorisée, sans toucher aux sources."""
from pathlib import Path
import json,re,hashlib,os,subprocess,datetime,shutil
repo=Path(__file__).resolve().parents[3];raw=repo/'docs/architecture/raw';base=repo/'target/parakeet-tests/refacto-parakeet-live'
assert shutil.disk_usage(repo).free>=30*1024**3
head=subprocess.check_output(['git','--no-optional-locks','rev-parse','HEAD'],cwd=repo,text=True).strip();assert head=='88850e8afd45e3b4c827914fad4c55a796b479db'
assert not subprocess.check_output(['git','--no-optional-locks','diff','HEAD','--name-only','--','crates','scripts','Cargo.toml','Cargo.lock','.cargo','rust-toolchain.toml'],cwd=repo,text=True).strip()
reg=(repo/'crates/whisper-core/src/model_registry.rs').read_text(encoding='utf8');d2=(repo/'crates/whisper-adapters/src/direct_live/d2_corpus.rs').read_text(encoding='utf8')
def sha(p):
 h=hashlib.sha256()
 with p.open('rb') as f:
  for b in iter(lambda:f.read(8*1024*1024),b''):h.update(b)
 return h.hexdigest()
def constant(name):return re.search(name+r':\s*&str\s*=\s*"([^"]+)"',reg)[1]
checks=[]
def verify(p,expected=None,size=None):
 digest=sha(p);actual_size=p.stat().st_size;ok=(expected is None or digest==expected) and (size is None or size==actual_size)
 checks.append(dict(path=str(p),size=actual_size,sha256=digest,expected_sha256=expected,expected_size=size,matched=ok));assert ok,str(p);return digest
for name,wav,txt,wh,th in re.findall(r'\(\s*"([^"]+)",\s*"([^"]+\.wav)",\s*"([^"]+\.txt)",\s*"([a-f0-9]{64})",\s*"([a-f0-9]{64})"',d2): verify(repo/wav,wh);verify(repo/txt,th)
source=base/'p3-workers/localappdata/Whisper';worker=source/'worker/whisper-worker-parakeet-cpu.exe';verify(worker,constant('PARAKEET_PERSONAL_CPU_WORKER_SHA256'))
dlls=['nemo_speech_asr_c.dll','nemo_speech_asr.dll','ggml.dll','ggml-base.dll','ggml-cpu.dll'];h=hashlib.sha256(b'nemo-speech-asr 0.2.0')
for name in dlls:h.update(name.encode());h.update(verify(source/'native/cpu'/name).encode())
for name in ['ggml-cuda.dll','cudart64_12.dll','cublas64_12.dll','cublasLt64_12.dll']:
 p=source/'native/cpu'/name
 if p.exists():dlls.append(name);h.update(name.encode());h.update(verify(p).encode())
assert h.hexdigest()==constant('PARAKEET_PERSONAL_CPU_RUNTIME_SHA256')
models=[]
for name,size,digest in re.findall(r'file_name:\s*"([^"]+)",\s*size_bytes:\s*([\d_]+),\s*sha256:\s*"([a-f0-9]{64})"',reg):
 p=repo/'models'/name;verify(p,digest,int(size.replace('_','')));models.append(p)
mp3=base/'retry-eight/fixtures/pu-l1-input.mp3';verify(mp3,'eebce5a2af18d4f929af5c243c50a6bc31b0910a583577cb940e0e4895e7cd80',22822)
turbo=repo/'target/parakeet-tests/cargo-shared/release/whisper-worker-cpu.exe';verify(turbo)
execution=base/'niveau1-88850e8'/('retry-'+datetime.datetime.now().strftime('%Y%m%dT%H%M%S'))
assert not execution.exists();cursor=base
while cursor!=repo.parent:
 assert cursor.resolve()==cursor.absolute(),str(cursor)
 cursor=cursor.parent
execution.mkdir(parents=True);assert execution.resolve().is_relative_to(base.resolve())
local=execution/'localappdata';installed=local/'Whisper';copies=[]
for p,dest in [(worker,installed/'worker'/worker.name),(turbo,installed/'app'/turbo.name)]+[(source/'native/cpu'/name,installed/'native/cpu'/name) for name in dlls]+[(p,installed/'models'/p.name) for p in models]:
 dest.parent.mkdir(parents=True,exist_ok=True);assert dest.parent.resolve().is_relative_to(execution.resolve());shutil.copyfile(p,dest);assert sha(dest)==sha(p);copies.append(dict(source=str(p),destination=str(dest),sha256=sha(dest)))
settings={
 'WHISPER_L1B_RUNTIME_ROOT':execution/'l1b',
 'WHISPER_MIC_LEVEL_D2_ROOT':execution/'d2-native',
 'WHISPER_MIC_LEVEL_D2_ENGINE':'parakeet-cpu',
 'PARAKEET_L2_ARCHIVE_ROOT':execution/'l2-archive',
 'PARAKEET_RTC02_EXEC_ROOT':execution/'rtc02',
 'WHISPER_PU_L1_MP3_FIXTURE':mp3}
for name,p in settings.items():
 if isinstance(p,Path) and name!='WHISPER_PU_L1_MP3_FIXTURE':p.mkdir(parents=True,exist_ok=True);assert p.resolve().is_relative_to(execution.resolve())
settings={k:str(v) for k,v in settings.items()}
record=dict(at=datetime.datetime.now().astimezone().isoformat(),head=head,guard='target/parakeet-tests/refacto-parakeet-live/guard.ps1',guard_sha256=sha(base/'guard.ps1'),execution_root=str(execution),test_environment=settings,test_localappdata=str(local),guard_host_localappdata=os.environ['LOCALAPPDATA'],cargo_config_overrides=["env.LOCALAPPDATA.value='"+local.as_posix()+"'",'env.LOCALAPPDATA.force=true'],original_personal_installation_present=(Path(os.environ['LOCALAPPDATA'])/'Whisper').exists(),all_resource_checks_matched=True,resource_checks=checks,copied_resources=copies,free_gib=round(shutil.disk_usage(repo).free/1024**3,2),scope='Tests du commit courant ; le worker natif D2 est la ressource gelee a965c950 attendue par le registre, pas un worker reconstruit ou remplace.',resources_modified_outside_execution_root=False)
(raw/'niveau1-retry-preflight.json').write_text(json.dumps(record,ensure_ascii=False,indent=2),encoding='utf8')
print(json.dumps({k:v for k,v in record.items() if k not in ('resource_checks','copied_resources')},ensure_ascii=False,indent=2))
