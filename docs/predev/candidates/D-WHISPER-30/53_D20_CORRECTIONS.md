# D20 findings correction record for D-WHISPER-21

## D20-001 — effective foundational rules

D20 retained historical D19 snapshots. D21 preserves those files under `rules/` and adds 23 exact current snapshots under `rules-effective/`. Each snapshot hash equals its installed source hash at preparation. `sources/EFFECTIVE_RULES_MANIFEST.json` is the canonical digest-bound index; the D21 DESIGN manifest directly binds every snapshot and this index. `12_SOURCE_INDEX.json` and `16_SNAPSHOT_INDEX.json` point to that pack.

The pack includes the five agent-rules, seven specialist TOML profiles, `rust-predev-design/SKILL.md`, its six foundational references including `execution-evidence.md`, the active `predev_control.py`, `predev_host.py`, `verify_raw_handoff.py`, and `rust-predev-orchestration/SKILL.md`. It records current effective rules without rewriting D20/D19.

## D20-002 — candidate identity and current status

D21 replaces `00_START_HERE.md` with the exact candidate/base identity, D20 findings, current-state path and hash, pending checkpoint path/hash, and an explicit instruction to read the live controlled state after any promotion. Historical DRAFT/OPEN text in predecessor corpora is identified as historical.

## Evidence and remaining gates

The exact D20 reviewer output is `sources/reviews/R-WHISPER-DESIGN-20-v1-exact.md`; its transport manifest and host receipt are in `sources/review-transports/T-WHISPER-REVIEW-D20-02-EXACT/`. The predecessor D20 manifest/receipt/proposal are preserved in `sources/predecessors/D-WHISPER-20/`.

D21 is DRAFT pending independent review. P25 must be a PLANS successor whose DESIGN parent is D21 and whose manifest binds the D21 parent manifest. The controlled checkpoint, current code identity, 15-path L04 authorization, detail evidence and `check-state --lot L-WHISPER-04` remain separate gates. No code or product validation is claimed.
