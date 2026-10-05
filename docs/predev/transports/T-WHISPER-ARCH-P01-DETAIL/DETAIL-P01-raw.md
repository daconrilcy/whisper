# ARCH-P01 v2 — contribution finale à DETAIL-P01

Auteur : `/root/arch_p01`, rôle `rust_architect`. Date : 2026-10-05, Europe/Paris. Cette contribution originale remplace intégralement ARCH-P01 v1.

Base : **D-WHISPER-19**, digest `903926ba27c70fc3a0cc6954d9e50914a64993e40c339683e557bc4c604d7529`. Plan : **P-WHISPER-02**, digest `8113649a38976e0c14e1c82a10ed1a04dff7c1022aaff5587962a0d94b8b2bad`. Les deux manifests ont été vérifiés par `predev_control.py verify-manifest` : PASS.

Entrée : **DETAIL-P01**, réponse version 2, nature `reversible_detail`, approval `accepted` par l’autorité technique explicitement déléguée. Owner technique : architecte ; suivi : AUTH-COORD. Lots concernés : L-WHISPER-00/01/02. Sources intellectuelles : D19/08, D19/50, TECH-D18-01/04/08, P02/01/02/04 et contribution ARCH-D18-FINAL. Son fichier brut a été lu et son SHA256 vérifié : `5690e50c6f287c2415ff840ab80ef8410083710dae974c3ff5d55bc80639e9b5`.

## Disposition physique retenue

Workspace virtuel racine, six packages, un `Cargo.lock`, `resolver = "3"`, édition 2024, Rust/Cargo **1.98.1**, cible **x86_64-pc-windows-msvc**. `default-members` contient core, adapters, desktop et bootstrap ; les workers sont sélectionnés explicitement. Chaque package possède son manifeste et `publish = false`.

| Package / cibles | Modules physiques | Dépendances et responsabilité |
|---|---|---|
| `whisper-core`, lib `whisper_core` | `src/lib.rs`, `domain.rs`, `application.rs`, `ports.rs` | Domaine, application, façade/vues et contrats IPC purs. Aucune dépendance vers les autres packages. |
| `whisper-adapters`, lib `whisper_adapters` | `src/lib.rs` ; capture, journal, archive, queue_store, recovery, settings, os, worker_ipc, supervisor, decoder selon les lots | Core et infrastructure parent. Aucun Whisper/CUDA. `decoder.rs` est un proxy vers l’enfant. |
| `whisper-worker-cpu`, bin `whisper-worker-cpu` | `src/main.rs` ; native_engine, decoder, encoder, IPC selon les lots | Core et moteur/codec ; contexte possédé par ce processus. Aucun lien Cargo vers worker GPU ou UI. |
| `whisper-worker-gpu`, bin `whisper-worker-gpu` | Même organisation, dans son package distinct | Core et moteur/codec CUDA ; aucun lien Cargo vers worker CPU ou UI. |
| `whisper-desktop`, lib `whisper_desktop` et bin `whisper-desktop` | `src/lib.rs`, `src/ui/**` ; `src/main.rs`, `src/root.rs` | Library UI : core/framework UI. Binaire : composition core/adapters/UI. Aucun moteur natif. |
| `whisper-bootstrap`, bin `whisper-bootstrap` | `src/main.rs` ; acquisition/prerequisites et adaptateurs installation selon L06 | Core et infrastructure installation. Aucun Whisper/CUDA ni inférence. |

L00 crée les contrats et squelettes ; les effets concrets restent aux lots P02. Aucun septième package, workspace supplémentaire ou chemin hors ownership L00 n’est nécessaire.

## Frontières vérifiables

`domain` dépend uniquement de std/types purs ; application/ports dépendent du domaine. Aucun OS/IO/UI/native dans core. Adapters implémente les ports et n’importe ni UI ni composition. Les adaptateurs workers dépendent des contrats core et bibliothèques requises. Les racines assemblent sans répliquer scheduler, générations ou état métier.

`whisper-desktop/src/lib.rs` et `ui/**` importent uniquement façade/vues/types publics core et framework UI. Seuls `main.rs`/`root.rs` importent adapters. `root.rs` est privé au binaire et absent de la library.

Cargo ne distingue pas les dépendances lib/bin d’un package : les frontières internes desktop exigent donc contrôle des imports et revue des consumers, en complément de metadata/tree. Inspecter dépendances normales, build, dev, features et cibles.

Core et library UI portent `forbid(unsafe_code)`. Interop Windows/native encapsulée dans les adaptateurs concernés, avec invariant/commentaire et ownership. Aucun contexte Whisper, handle codec ou `unsafe impl Send/Sync` ajouté pour partager le contexte entre processus/threads.

## Features et builds CPU/GPU

Aucune feature produit `cpu`, `gpu` ou `cuda` sur les six packages. Le backend dépend du worker sélectionné et de son attestation effective.

- CPU : `whisper-rs = "=0.16.0"`, `default-features = false`, aucune feature GPU.
- GPU : même version/options, `features = ["cuda"]`.
- Lock : conserver `whisper-rs-sys 0.15.0` et provenance native D19.
- Déclarations locales aux manifests workers ; aucun alias partagé activant CUDA.
- Core/adapters/desktop/bootstrap ne dépendent jamais de Whisper/CUDA, même optionnellement.

Commandes futures, PowerShell depuis `C:\dev\whisper`, invocations séparées :

```powershell
cargo build --locked --release -p whisper-worker-cpu --target x86_64-pc-windows-msvc --target-dir target/cpu
cargo build --locked --release -p whisper-worker-gpu --target x86_64-pc-windows-msvc --target-dir target/gpu
cargo build --locked --release -p whisper-desktop -p whisper-bootstrap --target x86_64-pc-windows-msvc --target-dir target/desktop
```

Ne jamais sélectionner les deux workers dans la même compilation. Ni resolver 3 ni target-dir distinct ne supprime l’unification dans une invocation commune. Contrôler séparément `cargo tree --locked -p whisper-worker-cpu -e features --target x86_64-pc-windows-msvc` et l’équivalent GPU. Le lock commun n’atteste pas l’isolation des binaires.

Conserver CUDA **12.8**, architecture **61** éprouvée pour GTX 1080 Ti, DLL privées et VC Redist D19 ; `nvcuda.dll` appartient au pilote. Vérifier les imports PE des EXE exacts. Sources : [unification Cargo](https://doc.rust-lang.org/cargo/reference/features.html#feature-unification), consultée le 2026-10-05 ; manifest officiel local whisper-rs 0.16.0, SHA256 `fead7612fd1e7cc8a46fdd8790bcf1a06e55897ee12d997a371f42b3c2c9cabe`.

## UI concrète

Retenir **eframe 0.35.0 / egui 0.35.0**, version eframe exacte, `default-features = false`, features **accesskit/default_fonts/glow**, renderer **Glow** explicite ; utiliser `eframe::egui`.

Options examinées : eframe/WGPU, eframe/Glow, assemblage direct winit/renderer. Glow évite un pipeline WGPU propre sans besoin présent. Sources : [features 0.35.0](https://docs.rs/crate/eframe/0.35.0/features), [API native](https://docs.rs/eframe/0.35.0/eframe/), consultées le 2026-10-05. Manifest officiel local : MSRV 1.92, édition 2024, MIT OR Apache-2.0 ; SHA256 `bdb78404faf9d4df763ab0091e8da12114abc8c147ad6accd9326b384b60691d`.

Dépendances natives : windowing Windows/OpenGL via glow/glutin. Aucun lien avec le calcul CUDA. Scheduler et durabilité restent application ; UI possède seulement présentation. Tray/hotkey/autostart passent par OSAdapter. Features persistence et inspection réseau désactivées.

Preuve documentaire d’API ; build et vrai consumer produit restent NOT RUN. Toute incompatibilité constatée avec la cible acceptée revient au coordinateur avant modification de capacité.

## Décodeur concret et contrat

Retenir **Symphonia 0.5.5**, version exacte, `default-features = false`, features **wav/pcm/mp3**, dépendance workers uniquement. Options : Symphonia, Media Foundation, FFmpeg externe. La pile Rust commune aux formats requis évite runtime codec ou processus supplémentaire.

[API 0.5.5](https://docs.rs/symphonia/0.5.5/symphonia/) et [package/licence](https://docs.rs/crate/symphonia/0.5.5), consultés le 2026-10-05 : WAV/PCM/MP3, MPL-2.0. Notices et accès aux sources effectivement distribuées relèvent de L06 ; [FAQ primaire MPL](https://www.mozilla.org/en-US/MPL/2.0/FAQ/), consultée le même jour.

`adapters/decoder.rs` implémente le proxy DecoderPort ; décodage/IO bloquants et contexte décodeur résident dans l’enfant, notamment worker CPU pour L01/L02. Cela respecte TECH-D18-04.

Contrat possédé par application : identité source revalidée + job/génération/plage → blocs PCM bornés avec séquence, taux/canaux, plage source et correspondance vers mono 16 kHz. Lecture source seule ; aucune suppression, archive audio d’import, publication ou confirmation durable. Erreurs typées source/format/corruption/conversion/IO/protocole/saturation/annulation. Contrôle indépendant ; annulation invalide génération et arrête coopérativement ; résultats obsolètes refusés avant journal. Saturation cesse l’entrée et produit diagnostic. Aucun arrêt FFI/IO arbitrairement borné ni kill automatique.

Q-07 conserve profils/bitrate et modalités de conversion avant leurs lots. Le banc formats D19 ne prouve pas transcription MP3, conversion ni récupération produit ; V-IMPORT-MP3 reste NOT RUN.

## Impact et verdict

Aucun changement des frontières, capacités GPU, garanties de confirmation, arrêt, archives ou reprise D19. Les risques unification/features, initialisation UI et profils/conversion MP3 sont respectivement vérifiés en V-BOUNDARY/V-PE, V-UI et Q-07/V-IMPORT-MP3. Ces validations produit restent NOT RUN ; aucun SPIKE exécuté.

**Verdict : ANSWERED_DETAIL. Aucun DESIGN_CHANGE_REQUIRED.**

Mutation proposée : DETAIL-P01 `answered`, réponse v2/auteur/preuve vers son TRANSPORT ; ne retoucher ni remplacer D19/P02. Terminé : choix, matrice, contrats et sources. Restant à l’hôte : persistance fidèle, reçu/hash, provenance d’export et référence au checkpoint. Aucun fichier écrit, railguard activé ou produit exécuté par cet acteur.
