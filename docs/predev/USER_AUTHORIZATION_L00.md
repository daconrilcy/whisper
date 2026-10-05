# Autorisation utilisateur — préflight et implémentation L-WHISPER-00

Source : message utilisateur reçu le 2026-10-05 dans la session Codex active. L'utilisateur demande expressément (1) de répondre à DETAIL-P01 comme rust_architect, (2) de persister la réponse en TRANSPORT, (3) de faire activer le railguard par l'autorité compétente, (4) de persister l'autorisation d'implémenter L-WHISPER-00 sur les seuls chemins ci-dessous, (5) d'établir le checkpoint IMPLEMENTATION, puis (6) de faire exécuter `check-state --lot L-WHISPER-00` par rust_implementer. Il interdit toute modification de code pour cette mission.

Acte de gouvernance : HUMAN-USER est l'autorité compétente pour activer la proposition de railguard et autorise son activation sur le périmètre du futur produit Whisper Windows V1.

Autorisation de coder : HUMAN-USER autorise l'implémentation du lot L-WHISPER-00, après préflight positif seulement, limitée strictement à :

- `Cargo.toml`
- `Cargo.lock`
- `rust-toolchain.toml`
- `crates/whisper-core/**`
- `crates/whisper-adapters/**`
- `crates/whisper-worker-cpu/**`
- `crates/whisper-worker-gpu/**`
- `crates/whisper-desktop/**`
- `crates/whisper-bootstrap/**`

Cette autorisation ne permet pas de commencer le lot si le contrôle `check-state --lot L-WHISPER-00` échoue. La présente mission de préflight ne modifie aucun code.
