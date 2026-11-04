Avancée : worktree et plan créés ; préflight de sauvegarde effectué le 4 novembre 2026.
Blocage : phase 0 arrêtée, dossier source spike02 et son parent absents au chemin indiqué.
Décision : NON REPRODUIT ; ne pas fusionner une chaîne inexistante, retrouver la source historique avant reprise.

# BUILD-NATIF — bilan figé au point d'arrêt de phase 0

Complément ultérieur : [recherche élargie dans le profil Windows](../cadrage/build-natif-recherche-profil.md).

Complément ultérieur : [inventaire de phase 1 et provenance retrouvée](../cadrage/build-natif-inventaire-phase1.md).

Base main : `b9935b2422d47350cdd5b368265afa84e12d42ab`. Branche : `build-natif`, worktree `C:/wt/build-natif`. [Plan](../cadrage/plan-build-natif.md). Espace libre au préflight : 536 660 615 168 octets (environ 500 Gio).

**Archive phase 0 : NON CRÉÉE, empreinte indisponible.** Destination demandée : `C:/dev/whisper-native-archive/spike02-20261104.zip`. Aucun fichier source n'a été archivé, exclu ou modifié. Absence constatée par `Get-Item -LiteralPath target/parakeet-feasibility/spike02` puis `Get-ChildItem -LiteralPath target/parakeet-feasibility -Force`, exécutés depuis `C:/dev/whisper` : les deux commandes signalent « chemin d'accès […] n'existe pas ». Cela ne prouve ni suppression ni absence sur un autre emplacement. La dette déjà versionnée [BUILD-NATIF](../cadrage/dette-build-natif.md) signalait également ce parent absent.

| Composant | Version ou commit | Preuve |
| --- | --- | --- |
| Chaîne spike02 | Indisponible au chemin indiqué | Deux commandes de préflight ci-dessus |
| llama.cpp / NeMo / outils / CUDA / licences | NON INVENTORIÉS | Phase 1 non engagée, point d'arrêt phase 0 |

| DLL | SHA installé | SHA reconstruit | Exports | Imports | Verdict |
| --- | --- | --- | --- | --- | --- |
| Toutes DLL CPU/GPU du périmètre | NON RELEVÉ | NON PRODUIT | NON EXÉCUTÉ | NON EXÉCUTÉ | NON REPRODUIT |

| Essai | Sorties identiques |
| --- | --- |
| N2 Auto, CPU, GPU ; N1 comparé à main | NON EXÉCUTÉ |

**Risques :** sources et paramètres non sauvegardés ; origine NeMo, commit llama.cpp, GGML_NATIVE, instructions CPU et licences non établis. Aucun script natif créé, téléchargement, compilation, essai Cargo/N2, remplacement de DLL, nettoyage, push ou fusion. Les seuls nouveaux fichiers sont le plan et ce bilan ; les protections AGENTS.md restent applicables.

Revue indépendante en lecture seule : CLEAN pour le plan et le compte rendu d'arrêt de phase 0, aucun constat actionnable ; aucun script ni compilation validé. Contrôle Git des espaces : réussi avant commit.

**Décision demandée :** recommander la localisation de la copie originale de spike02 (autre disque, machine ou sauvegarde), puis sa sauvegarde vérifiée avant inventaire. Une recherche élargie est une alternative à préciser si son emplacement est inconnu. Aucune autorisation de suppression de `target/parakeet-feasibility/` demandée à ce stade : conservation maintenue, équivalence non prouvée. Tout complément sera consigné dans un nouveau document.
