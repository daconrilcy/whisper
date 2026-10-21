import hashlib
import json
from pathlib import Path

LOT = Path(__file__).resolve().parent
RAW = LOT / "vad_raw.json"
OUT = LOT / "vad_analysis.json"


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def zero_runs(bits, frame_ms, duration_s):
    start = None
    result = []
    for index, value in enumerate(bits + "1"):
        if value == "0" and start is None:
            start = index
        if value == "1" and start is not None:
            begin = start * frame_ms / 1000
            end = min(index * frame_ms / 1000, duration_s)
            result.append((begin, end))
            start = None
    return result


def candidates(runs, frame_ms, duration_s, edge_frames):
    allowance = edge_frames * frame_ms / 1000
    result = []
    for raw_start, raw_end in runs:
        start = max(0, raw_start - allowance)
        end = min(duration_s, raw_end + allowance)
        if end - start >= 0.400 - 1e-9:
            result.append({
                "raw_start_s": round(raw_start, 3),
                "raw_end_s": round(raw_end, 3),
                "raw_nonspeech_ms": round((raw_end - raw_start) * 1000),
                "candidate_start_s": round(start, 3),
                "candidate_end_s": round(end, 3),
                "candidate_midpoint_s": round((start + end) / 2, 3),
            })
    return result


def cuts_after_spacing(intervals):
    kept = []
    suppressed = []
    for interval in intervals:
        midpoint = interval["candidate_midpoint_s"]
        if not kept or midpoint - kept[-1]["candidate_midpoint_s"] >= 1 - 1e-9:
            kept.append(interval)
        else:
            suppressed.append(interval)
    return kept, suppressed


def main():
    raw = json.loads(RAW.read_text(encoding="utf-8"))
    result = {
        "schema": "feas-pm-01-vad-prep-analysis-v1",
        "raw_sha256": sha256(RAW),
        "analysis_source_sha256": sha256(LOT / "analyze_probe.py"),
        "status": "AUTO_NON_VALIDE",
        "rules": {
            "strict": "one contiguous raw non-speech run >=400ms",
            "edge_allowance": "sensitivity only: expand each raw non-speech run by one VAD frame on each side before checking 400ms; no merging and not an approved cut rule",
            "cut_position": "midpoint of candidate non-speech interval",
            "spacing": "greedily retain earliest midpoint at least 1.000s after previously retained midpoint",
            "rounding": "interval boundaries and midpoint reported to 0.001s; eligibility checked before rounding",
        },
        "runs": [],
    }
    for run in raw["runs"]:
        duration = run["sample_count"] / raw["sample_rate_hz"]
        all_runs = zero_runs(run["decisions"], run["frame_ms"], duration)
        entry = {
            "id": run["id"],
            "mode": run["mode"],
            "mode_index": run["mode_index"],
            "frame_ms": run["frame_ms"],
            "duration_s": duration,
            "speech_frame_count": run["speech_frame_count"],
            "frame_count": run["frame_count"],
            "longest_raw_nonspeech_ms": round(max((b-a for a,b in all_runs), default=0)*1000),
        }
        for key, edge_frames in (("strict", 0), ("one_frame_edge_allowance", 1)):
            intervals = candidates(all_runs, run["frame_ms"], duration, edge_frames)
            kept, suppressed = cuts_after_spacing(intervals)
            entry[key] = {
                "candidate_intervals": intervals,
                "retained_cut_midpoints_s": [x["candidate_midpoint_s"] for x in kept],
                "suppressed_by_spacing": [x["candidate_midpoint_s"] for x in suppressed],
            }
        result["runs"].append(entry)
    OUT.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"{len(result['runs'])} runs analysed: {OUT}")
    print(f"analysis sha256: {sha256(OUT)}")


if __name__ == "__main__":
    main()
