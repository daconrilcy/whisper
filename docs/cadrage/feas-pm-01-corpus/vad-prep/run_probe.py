import hashlib
import json
import subprocess
from pathlib import Path

LOT = Path(__file__).resolve().parent
ROOT = next(parent for parent in LOT.parents if (parent / "Cargo.lock").is_file())
MANIFEST = ROOT / "docs/cadrage/feas-pm-01-corpus/manifest.json"
PROBE = ROOT / "target/parakeet-tests/cargo-shared/debug/feas_pm_01_vad_probe_frames.exe"
OUT = LOT / "vad_raw.json"
MODES = ["Quality", "LowBitrate", "Aggressive", "VeryAggressive"]
FRAME_DURATIONS_MS = [10, 20, 30]


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    result = {
        "schema": "feas-pm-01-vad-prep-raw-v1",
        "purpose": "Acoustic preparation only; no FEAS-PM-01 or A/B execution",
        "webrtc_vad_crate": "0.4.0; cached rlib from cargo-shared/debug/deps",
        "probe_source_sha256": sha256(LOT / "vad_probe.rs"),
        "probe_exe_sha256": sha256(PROBE),
        "sample_rate_hz": 16000,
        "frame_durations_ms": FRAME_DURATIONS_MS,
        "bit_order": "character i is VAD raw decision for consecutive frame i; 1=speech, 0=non-speech",
        "tail_rule": "last incomplete frame is right-padded with PCM zeros; derived intervals are clipped at actual WAV sample count",
        "state_rule": "new Vad instance per file and mode; all frames processed in WAV order; no reset within file",
        "runs": [],
    }
    for item in manifest["items"]:
        wav = ROOT / item["wav"]["path"]
        actual_hash = sha256(wav)
        if actual_hash != item["wav"]["sha256"]:
            raise RuntimeError(f"source hash mismatch: {item['id']}")
        for frame_ms in FRAME_DURATIONS_MS:
            for index, name in enumerate(MODES):
                output = subprocess.check_output([str(PROBE), str(index), str(frame_ms), str(wav)], text=True)
                mode_text, ms_text, sample_text, frame_text, bits = output.strip().split("\t")
                if mode_text != str(index) or ms_text != str(frame_ms) or len(bits) != int(frame_text) or set(bits) - set("01"):
                    raise RuntimeError(f"invalid raw output: {item['id']} {name} {frame_ms}")
                if int(sample_text) != item["wav_format"]["frames"]:
                    raise RuntimeError(f"PCM sample count mismatch: {item['id']}")
                result["runs"].append({
                    "id": item["id"],
                    "wav_path": item["wav"]["path"],
                    "wav_sha256": actual_hash,
                    "mode": name,
                    "mode_index": index,
                    "frame_ms": frame_ms,
                    "frame_count": len(bits),
                    "sample_count": int(sample_text),
                    "speech_frame_count": bits.count("1"),
                    "decisions": bits,
                })
    OUT.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"{len(result['runs'])} complete-file runs saved: {OUT}")
    print(f"raw sha256: {sha256(OUT)}")


if __name__ == "__main__":
    main()
