# ARCH-P01 v3 — réponse à DETAIL-P01

**Auteur :** `/root/arch_p01`, rôle `rust_architect`. **Date :** 2026-10-05, Europe/Paris. **Owner technique :** architecte ; suivi : AUTH-COORD. **Nature :** `reversible_detail`, version 3, `accepted` par l’autorité technique déléguée pour DETAIL-P01.

**Base D-WHISPER-19 :** `903926ba27c70fc3a0cc6954d9e50914a64993e40c339683e557bc4c604d7529`. **Plan P-WHISPER-02 :** `8113649a38976e0c14e1c82a10ed1a04dff7c1022aaff5587962a0d94b8b2bad`. Vérifications personnelles des deux manifests : PASS. Sources lues : D19/08/50, TECH-D18-01/04/08, contribution ARCH-D18-FINAL et P02/01/02/04.

## Disposition décidée

Workspace virtuel, six packages, `Cargo.lock` commun, resolver 3, édition 2024, Rust/Cargo **1.98.1**, cible **x86_64-pc-windows-msvc**. Packages `publish=false` ; default-members : core/adapters/desktop/bootstrap.

| Package / cibles | Placement et dépendances |
|---|---|
| `whisper-core`, lib | `src/{lib,domain,application,ports}.rs` : domaine, application, façade/vues et contrats IPC purs ; aucune dépendance aux autres packages. |
| `whisper-adapters`, lib | `src/lib.rs` et modules infrastructure selon P02 ; dépend de core ; aucun Whisper/CUDA. `decoder.rs` est un proxy IPC. |
| `whisper-worker-cpu`, bin homonyme | `src/main.rs` composition ; native_engine/decoder/encoder/IPC selon leurs lots ; core et moteur/codecs. |
| `whisper-worker-gpu`, bin homonyme | Organisation équivalente, package distinct ; core et moteur/codecs CUDA. |
| `whisper-desktop`, lib et bin homonyme | `src/lib.rs`, `ui/**` : core/framework UI ; `main.rs`, `root.rs` privés au binaire composent core/adapters/UI. Aucun moteur natif. |
| `whisper-bootstrap`, bin homonyme | `src/main.rs` composition ; acquisition/prerequisites selon L06 ; core/infrastructure installation, aucun moteur natif. |

Chaque package possède son manifeste. Aucun septième package ni chemin hors ownership L00. L00 fournit contrats/squelettes ; aucune transcription revendiquée.

## Contrôle des frontières et builds

Domaine sans application/OS/IO/UI/native ; application sans adaptateurs concrets/UI ; adapters sans UI/root. UI importe uniquement façade/vues/types purs core et framework. Les racines assemblent sans état métier parallèle. Cargo ne sépare pas les dépendances lib/bin : contrôler aussi imports et consumers `desktop/src/lib.rs`/`ui/**`.

Core/library UI : `forbid(unsafe_code)`. Unsafe Windows/native encapsulé et justifié ; contexte moteur détenu exclusivement par l’enfant, sans partage ad hoc Send/Sync.

Aucune feature produit cpu/gpu/cuda. CPU : `whisper-rs="=0.16.0"`, defaults désactivés, sans feature GPU. GPU : même déclaration avec `features=["cuda"]`. Déclarations locales ; aucun alias partagé activant CUDA. Lock : `whisper-rs-sys 0.15.0`, provenance native D19. Les quatre autres packages restent sans Whisper/CUDA.

Commandes futures, PowerShell, `C:\dev\whisper`, séparément :

```powershell
cargo build --locked --release -p whisper-worker-cpu --target x86_64-pc-windows-msvc --target-dir target/cpu
cargo build --locked --release -p whisper-worker-gpu --target x86_64-pc-windows-msvc --target-dir target/gpu
cargo build --locked --release -p whisper-desktop -p whisper-bootstrap --target x86_64-pc-windows-msvc --target-dir target/desktop
```

Jamais les deux workers dans une compilation. Resolver/target-dir ne neutralisent pas l’unification commune. Contrôler séparément leurs `cargo tree -e features`, puis imports PE. Conserver CUDA12.8/architecture61, DLL privées et VC Redist D19 ; nvcuda vient du pilote. [Source Cargo](https://doc.rust-lang.org/cargo/reference/features.html#feature-unification), consultée le 2026-10-05.

## UI et décodeur décidés

**UI : eframe/egui 0.35.0**, eframe exact, defaults désactivés, features accesskit/default_fonts/glow, renderer Glow explicite ; utiliser `eframe::egui`. Alternatives : WGPU ou assemblage direct winit/renderer. Glow évite un pipeline supplémentaire sans besoin présent. Windowing Windows/OpenGL via glow/glutin ; aucun calcul transcription dans le renderer. Persistance métier via application, inspection réseau désactivée. [API/features](https://docs.rs/eframe/0.35.0/eframe/), consultées le 2026-10-05 ; manifeste officiel local : MSRV1.92, MIT OR Apache-2.0.

**Décodeur : Symphonia 0.5.5**, exact, defaults désactivés, features wav/pcm/mp3, dépendance workers uniquement. Alternatives : Media Foundation/FFmpeg externe ; pile Rust commune retenue sans runtime codec supplémentaire. [API](https://docs.rs/symphonia/0.5.5/symphonia/), consultée le 2026-10-05 ; MPL-2.0, notices/accès aux sources à traiter en L06.

DecoderPort appartient à application : source revalidée/job/génération/plage → blocs PCM bornés, séquences et correspondance temporelle vers mono16kHz. Décodage/IO bloquants dans l’enfant ; source en lecture seule. Erreurs source/format/corruption/conversion/IO/protocole explicites. Annulation invalide génération ; résultats obsolètes rejetés avant journal. Saturation cesse l’entrée avec diagnostic ; contrôle indépendant, aucun kill automatique ou arrêt arbitrairement borné promis. Q-07 conserve profils et conversion détaillée.

## Impact et verdict

Aucune frontière, garantie, capacité GPU, confirmation durable ou politique d’arrêt D19 modifiée. Preuves documentaires ; build/UI/import produit restent NOT RUN selon P02. Aucun SPIKE exécuté.

**Verdict : ANSWERED_DETAIL ; aucun DESIGN_CHANGE_REQUIRED.** Persister cette contribution en TRANSPORT puis lier reçu/hash à DETAIL-P01 `answered` et au checkpoint. Aucun fichier écrit ni railguard activé par cet acteur.
