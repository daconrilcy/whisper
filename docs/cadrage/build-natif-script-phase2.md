Avancée : script préparatoire et pins écrits, contrôles statiques réussis, NeMo récupéré au commit exact.
Blocage : dossier gitlink llama vide refusé ; voie de compilation CMake absente de la garde existante.
Décision : LIVRÉ AVEC RÉSERVES après deux cycles de revue ; NON REPRODUIT, phases 3/4 non engagées.

# BUILD-NATIF — bilan figé de préparation, 4 novembre 2026

L'utilisateur a autorisé la préparation sur révisions retrouvées avec hypothèses explicites. Fichiers : `ci/build-native.ps1`, `native/build-config.json`, `native/.gitignore`, plan actualisé et [procédure](../architecture/NATIVE-BUILD.md). Aucune modification Rust ou installation.

| Vérification | Résultat réel |
| --- | --- |
| Parseur PowerShell ; contrôle Git des espaces | PASS |
| Auto-tests | PASS, 6 contrôles négatifs (chemins, version, empreinte, environnement vcpkg) |
| CheckOnly CPU / GPU | PASS, outils présents conformes aux versions fixées |
| Construction sans garde | Refusée avant création du dossier de sortie ; absence du dossier vérifiée |
| Préparation sources1 | FAILED en 2,6 s ; NeMo au commit fixé, llama/vcpkg non récupérés |
| Compilation, équivalence statique, N1, N2 | NON EXÉCUTÉS |

Revue indépendante : trois constats initiaux (absence de surveillance CMake, destinations vcpkg héritées, vcpkg.exe non vérifié) corrigés ; deuxième revue **CLEAN uniquement pour le candidat préparatoire**. Un essai réel ultérieur a révélé le cas gitlink vide ; aucune troisième revue/correction ni relance automatique.

Les conclusions de ce bilan ont ensuite été relues indépendamment : **CLEAN documentaire**, archive et absence de bootstrap/CMake confirmées, sans nouvelle revue du script ni levée des réserves.

**Cause prouvée :** `git -C native/cache/nemo ls-tree HEAD llama.cpp` donne `160000 commit bd4f514db14d87fded667787a7a963bfbaa98e89`. Le dossier `llama.cpp` existe, vide et sans reparse point. `Get-Source` assimile tout dossier existant à un checkout déjà initialisé ; il échoue avant Git init. Aucun fichier supprimé ou déplacé.

**Traces :** `target/native-build/cpu/sources1/`, archivées dans [phase2-preparation.zip](../architecture/raw/native-build/phase2-preparation.zip), SHA-256 `71fcf33a6c922ea74668bc527c97096fc4777660b5dba819995a74c4f75a023b`. Vérification : 5 fichiers, tailles et SHA concordants, plus manifeste dans l'archive (6 entrées). Cette archive de traces n'est pas l'archive spike02 de phase 0, toujours impossible. Cache NeMo persistant conservé ; aucun nettoyage proposé ou exécuté pour débloquer la préparation.

**Arbitrage recommandé pour la reprise :** autoriser le correctif du cas gitlink vide, puis une extension bornée de `ci/guard-native.ps1` et `ci/native-profiles.json`, avec adaptation de `ci/build-native.ps1` : profils CPU/GPU limités à ce script et ses pins, racines `native/cache/` et `target/native-build/`, réutilisation de la surveillance existante, préflight/auto-tests, captures sorties/codes, arrêt sur violation ou échec. Tester les destinations héritées, versions/pins incorrects et refus hors racines ; aucun lanceur arbitraire, nouvelle surveillance, service ou modification de permissions. Le plan et NATIVE-BUILD portent les détails ; consignes normatives éventuelles dans AGENTS.md seulement.

Alternative : conserver le candidat bloqué. Aucune fusion recommandée en l'état ; aucune autorisation de supprimer le spike demandée. La règle AGENTS sur la seule chaîne de production reste à ajouter après livraison d'une chaîne fonctionnelle. Les faits ultérieurs seront dans un nouveau document.
