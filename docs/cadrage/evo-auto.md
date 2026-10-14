Avancée : la base CPU/GPU et plusieurs composants Auto existent ; le parcours Windows ne propose pas encore Auto.
Blocage : aucun pour finaliser le cadrage ; l'implémentation attend une autorisation distincte.
Décision : un seul repli CPU informé est accepté avant le début du traitement audio.

# Évolution Auto — cadrage (14 octobre 2026)

**Base.** `evo-auto` part de `evo-arbo` à `4f0a0d2c`, descendant du jalon GPU `994b660a`. Dépôt initialement propre. Cadrage sans autorisation de code ; présélection existante GPU/CPU conservée, jamais Auto imposé.

## Besoin et critères

Avant chaque import WAV/MP3 ou séance micro, **Auto** vérifie qu'un GPU est utilisable par le moteur, le choisit en priorité ou démarre sur CPU. La préférence reste Auto et le GPU est réévalué à la session suivante ; le moteur effectif est visible. Modèle **large-v3-turbo**, qualité, sorties et modes manuels restent inchangés. Aucune bascule en cours de transcription.

| Critère | Vérification future |
|---|---|
| CA1 | Sélecteur Auto/CPU/GPU et modes manuels préservés. |
| CA2 | Avec GPU compatible, Auto obtient `Ready` CUDA attesté et affiché, sur micro/import. |
| CA3 | GPU absent ou initialisation échouée : une seule tentative CPU avant traitement, sans choix ni erreur GPU bloquante. |
| CA4 | Micro/import : même modèle, paramètres, qualité et sorties. |
| CA5 | Auto persiste après CPU ; une nouvelle session réévalue le GPU. |

## Existant et réemploi

| État | Référence, preuve actuelle et adaptation envisagée |
|---|---|
| Utilisé | `crates/whisper-desktop/src/ui.rs::start_import/start_live` transmet le choix ; `root.rs::compose` présélectionne GPU/CPU ; tests UI CPU/GPU. Conserver. |
| Implémenté, masqué | `crates/whisper-core/src/domain.rs::ComputeChoice::Auto`, `compute_policy.rs::ComputePolicy`, `crates/whisper-adapters/src/supervisor.rs::WorkerPaths::select` ; tests `crates/whisper-core/tests/compute_policy.rs`, `crates/whisper-adapters/tests/worker_control.rs`. Le fichier GPU seul ne prouve pas la compatibilité. |
| Partiel | `crates/whisper-adapters/src/worker_ipc.rs::ImportRuntime` a un repli Auto testé avec faux workers ; l'UI micro utilise `StartDirectLiveConfigured`, rejeté par `direct_live.rs::DirectLive::start`. Raccorder avant capture ; exclure le repli après début. |
| Utilisé, extensible | `worker_ipc.rs::probe_cuda_worker` exige modèle chargé et CUDA attesté ; test de worker absent dans ce fichier. `root.rs` l'appelle seulement au lancement. `core/src/application.rs` affiche le moteur au `Ready`. Réévaluer par session et afficher le motif. |
| Historique/documenté | `68da5fcf` introduit la politique L04, `39035623` masque Auto, `213f819b` raccorde GPU micro/import. `docs/predev/candidates/D-WHISPER-30/55_CHANGE_L04_SCOPE.md` et `docs/predev/candidates/P-WHISPER-31/16_L04_VERIFICATION_MATRIX.md` envisagent un repli en cours ; `docs/cadrage/evo-gpu.md` et son bilan fixent les modes manuels et la panne micro. |

**Vérification.** `cargo build --offline --locked --workspace` et `cargo test --offline --locked --workspace` réussissent à `4f0a0d2c`. Tests de politique et workers simulés : observés ; fonctionnement Auto Windows : déduit du code, non essayé. Le bilan GPU du 14 octobre qualifie seulement les modes manuels sur ce PC. Absence de GPU : non testable ici sans modifier le système ; aucune simulation système effectuée.

## Architecture et plan proposé — 8 tâches, implémentation non autorisée

Le domaine sépare préférence et moteur (`ComputeChoice`, `ComputePolicy`) ; l'application publie le moteur ; les adaptateurs sondent/démarrent ; la composition garde la présélection ; l'interface affiche Auto, cause et moteur. Réutiliser l'attestation CUDA avec erreurs typées à chaque session. Pour l'import, `Ready` précède une inférence lancée aussitôt : étendre le worker existant avec une étape explicite « commencer » après attestation, afin de garder le repli avant traitement. Le micro direct attend déjà `Ready` avant capture. Une sonde préalable séparée chargerait deux fois le modèle sans garantir la seconde initialisation. Mutualiser la décision Auto et l'unique tentative CPU ; annulation et échec CPU arrêtent sans nouvel essai. Les erreurs de modèle restent bloquantes, les modes manuels inchangés. Vérifier que le premier texte GPU reste sous 15 s.

1. Borner les états « préparation / traitement commencé » dans `ComputePolicy` et l'unique repli — CA2–CA5 ; tests de frontière et échec CPU.
2. Étendre l'attestation CUDA avec causes typées à chaque départ — CA2–CA3 ; tests injectés et sonde native.
3. Étendre l'IPC import avec autorisation explicite de commencer, puis raccorder `ImportRuntime` sans reprise après début — CA2–CA4 ; tests WAV/MP3 et échecs simulés.
4. Raccorder Auto au micro direct via `DirectLive`, avant capture, sans bascule en cours — CA2–CA4 ; tests de démarrage, arrêt et échecs simulés.
5. Afficher Auto, préférence conservée, raison du repli et moteur réel dans `ui.rs`/`application.rs` — CA1, CA5 ; tests UI et parcours.
6. Préserver les modes manuels et la présélection de `root.rs` — CA1 ; tests de non-régression CPU/GPU.
7. Vérifier modèle, paramètres, blocs confirmés et sorties sur micro/import — CA4 ; tests existants complétés et comparaison de qualité.
8. Qualifier dans la fenêtre Windows les deux parcours et les messages ; vérifier l'absence GPU sur une cible appropriée sans modifier ce PC — CA1–CA5 ; constat utilisateur.

**Q1 — décision acceptée le 14 octobre 2026.** En Auto, vérifier le GPU réel avant chaque transcription. S'il est absent, incompatible ou échoue à l'initialisation, tenter CPU **une seule fois avant tout traitement audio** ; afficher sans bloquer la cause et « CPU prend le relais ». Si CPU échoue, arrêter avec une erreur explicite. Garder la préférence Auto pour la prochaine session. Après début du traitement, aucun changement automatique : la panne micro garde le WAV/blocs confirmés, les TXT/SRT incomplets et la proposition de réimport CPU existants. GPU manuel ne replie jamais. Hors périmètre : supervision, reprise, benchmark, autre modèle, tray et MP3 de sortie. **Cette décision ne vaut pas autorisation d'implémenter.**
