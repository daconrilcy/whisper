Avancée : Clippy standard après P59 relève le test D2 à 138 lignes.
Blocage : le scénario vérifie 16 relectures corpus, sorties WAV/TXT/SRT et révisions de gain.
Décision : extraire des phases privées de test dans le même fichier, sans changer le protocole de qualification.

# Plan P60 — Décomposer le scénario D2 de lecture corpus CPU

## Constat et preuve

Clippy standard frais après P59 relève `clippy::too_many_lines (138/100)` pour `crates/whisper-adapters/src/direct_live/d2_corpus.rs::d2_corpus_real_cpu_turbo_and_parakeet` (ligne 147) dans `target/parakeet-tests/clippy-corrections-suivantes-20261113/scratch/20261114T230720192-d4b0bafbb4ae4489b842eaa0c94fb4cb/cargo-command.stdout.log`. Le fichier source est propre avant P60 et compte 641 lignes.

## Périmètre autorisé

- Un seul fichier source : `crates/whisper-adapters/src/direct_live/d2_corpus.rs`.
- Extraire uniquement des phases privées de test vers des helpers locaux dans le même fichier. Ne pas modifier `INPUTS`, `CONDITIONS`, `turbo_case`, `parakeet_case`, les corpus/références ou leurs empreintes.
- Le delta net du fichier est limité à +30 lignes. Le test et helpers respectent 100 lignes, 25 de complexité cognitive, 7 arguments et 250 de complexité de type.
- Préserver les conditions d’ignore et de configuration, la contrainte Parakeet CPU, l’absence de redirection CUDA, les vérifications des fichiers/workers/modèles, toutes les identités/destinations, le découpage du corpus et transitions de gain, les assertions d’artefacts, continuité des frames, ACK de gain et leurs révisions, le format complet des événements checkpoint et le marqueur `D2_CASE_DONE`.
- Aucun changement de production ou d’oracle, aucune édition hors de ce fichier. Ne pas activer `native-tests`, lancer le test ignoré, produire des artefacts D2 ou exécuter une qualification native.

## Critères d’acceptation P60-T1

Le code garde les 16 scénarios formés par `INPUTS` et `CONDITIONS`, le filtre d’exécution `parakeet-cpu`, et les mêmes champs/ordres de checkpoint et assertions. La compilation N1 conserve le test sous `ignored` ; le scénario réel D2 reste NON EXÉCUTÉ.

La mesure Clippy seuil 0 ne révèle aucun dépassement sur le test/helpers; Clippy standard passe avec ratchet à 70 sites ou moins. Revue indépendante CLEAN. fmt-check, SelfTest de garde, N1 fraîche, PE SelfTest, dix harnais exacts, inventaires et suites CPU, Parakeet CPU et common passent via `ci/guard-level1.ps1`.

## Vérifications et limites

Utiliser `ci/guard-level1.ps1`, LotRoot `target/parakeet-tests/clippy-corrections-suivantes-20261113`, commandes Cargo offline et locked, cibles fraîches dédiées. La N1 reçoit `-PersonalParakeetModelPath C:/Users/cyril/AppData/Local/Whisper/models/parakeet-tdt-0.6b-v3.q8_0.gguf`. Relever l’espace libre avant compilation. Ne pas modifier la baseline structurelle.

Tests modèle/natif ignorés : NON EXÉCUTÉS ; la recette D2 requiert des artefacts et ressources natives, n’est pas exécutée. Témoin PE réel : NON EXÉCUTÉ. Pas de GPU N2, build natif, essai dans la fenêtre Windows, téléchargement, nettoyage ou push.

Retour arrière limité aux helpers privés P60. L’utilisateur a autorisé P51–P60 le 14 novembre 2026 ; P60 est la dixième tâche.
