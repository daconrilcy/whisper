# Préflight et vérifications — P-WHISPER-28

Toutes les vérifications restent PROPOSED / NOT RUN. La matrice exécutable et le mapping AC autoritaire du parent D22 sont dans `16_L04_VERIFICATION_MATRIX.md`. Shell Developer PowerShell MSVC; cwd `C:\dev\whisper`; cible `x86_64-pc-windows-msvc`; Cargo séquentiel.

## Manifests

```powershell
python -B C:\Users\cyril\.codex\skills\rust-predev-design\scripts\predev_control.py verify-manifest --root C:\dev\whisper\docs\predev\L04-PREFLIGHT --manifest C:\dev\whisper\docs\predev\L04-PREFLIGHT\candidates\D-WHISPER-22\manifest.json
python -B C:\Users\cyril\.codex\skills\rust-predev-design\scripts\predev_control.py verify-manifest --root C:\dev\whisper\docs\predev\L04-PREFLIGHT --manifest C:\dev\whisper\docs\predev\L04-PREFLIGHT\candidates\P-WHISPER-28\manifest.json
```

Attendu : ok=true et IDs/digests exacts. Les contrôles au gate n’exécutent aucune commande Cargo de la matrice. Après CLEAN PLANS exact P28, construire pending selon `15_L04_CHECKPOINT_CONTRACT.md`, faire `check-state` sans --lot, recapturer dépôt/build env, lancer `check-state --lot L-WHISPER-04`, vérifier pending stable, promouvoir puis relancer le check lot. La promotion ne vérifie pas automatiquement le lot.

```powershell
python -B C:\Users\cyril\.codex\skills\rust-predev-design\scripts\predev_control.py check-state --root C:\dev\whisper\docs\predev --code-root C:\dev\whisper --state C:\dev\whisper\docs\predev\state.pending.P28-L04.json --lot L-WHISPER-04
python -B C:\Users\cyril\.codex\skills\rust-predev-design\scripts\predev_control.py promote-checkpoint --root C:\dev\whisper\docs\predev --code-root C:\dev\whisper --pending C:\dev\whisper\docs\predev\state.pending.P28-L04.json --current C:\dev\whisper\docs\predev\state.json
python -B C:\Users\cyril\.codex\skills\rust-predev-design\scripts\predev_control.py check-state --root C:\dev\whisper\docs\predev --code-root C:\dev\whisper --state C:\dev\whisper\docs\predev\state.json --lot L-WHISPER-04
```

Recapturer avant le gate : toplevel, HEAD/branch/status/diff global informatif et diff/hash sur quinze paths; vérifier dix égalités D22 et cinq absences, L03 completed, autorisation, railguard, détails, matrice effective et build environment. Ne pas considérer le worktree global propre. Les probes, tests, builds, campagnes audio/native/UI et livraison restent NOT RUN.
