Avancée : inventaire et archives vérifiés le 9 novembre 2026 ; aucune suppression effectuée.
Blocage : accord de suppression requis ; deux jonctions doivent être retirées comme liens uniquement.
Décision : proposer le nettoyage des fichiers N1 et d'une arborescence de test vide, conserver le worktree et ses caches.

# Proposition de nettoyage après livraison du correctif catalogue

Le correctif et son bilan sont présents sur `main` et `codex/fix-catalog-test` à `e2953ce0`, fusionnés et poussés dans la session. Le worktree `C:/wt/fix-catalog-test` est propre ; son seul contenu ignoré est `target/`. Les agents du lot sont terminés. Aucun Cargo, rustc, worker ou autre processus utilisant ce chemin n'a été observé, hors des commandes de lecture de cet inventaire. Ce constat ponctuel ne constitue pas un verrou : le vérifier de nouveau avant exécution.

## Suppressions proposées, après accord sur le manifeste

Base N1 : `C:/wt/fix-catalog-test/target/parakeet-tests/fix-catalog-test/`.

| Chemin sous cette base | Fichiers | Taille logique | Justification |
|---|---:|---:|---|
| `temp/` | 1 232 | 31 443 894 octets | Fixtures et sorties des tests terminés ; résultats textuels archivés séparément. |
| `scratch/` | 61 | 759 057 octets | 60 journaux archivés et un faux modèle d'auto-test de 24 octets. |
| `evidence/` | 13 | 17 759 octets | Copies exactes des preuves déjà archivées. |
| **Total N1** | **1 306** | **32 220 710 octets (30,73 Mio)** | Après retrait des deux jonctions, supprimer la base N1 elle-même. |

Également proposé : `C:/wt/fix-catalog-test/target/parakeet-tests/refacto-parakeet-live/`, contenant seulement 12 répertoires, racine incluse, aucun fichier ni lien. Le descendant `p3-workers/runtime-paths-88780` provient du test `crates/whisper-worker-parakeet-cpu/src/runtime_paths/tests.rs:7`, qui crée ces répertoires ; il ne s'agit pas d'un runtime installé.

Le [manifeste exact](nettoyage-fix-catalog-test-chemins.csv) contient 2 534 chemins absolus : 1 306 fichiers (dont les 225 WAV temporaires et les copies de fixtures), 1 226 répertoires et 2 jonctions. Il indique tailles, cibles des liens et archive de conservation lorsqu'applicable ; aucune suppression par motif ni extension à un parent n'est proposée. Les tailles sont logiques, pas une mesure de blocs NTFS réellement récupérables.

**Jonctions.** Dans `scratch/20261109T110605330-e9351d6ace634f818398e7d056327cd2/`, `junction-fixture` et `fresh-fixture/targets` ciblent toutes deux `junction-target` dans ce même dossier. Retirer les deux liens seuls, sans suivre leur cible, puis vérifier qu'aucun point de réanalyse ne subsiste avant toute suppression récursive. L'inventaire n'a pas traversé les liens et n'en a trouvé aucun autre dans `target/` ; les ancêtres du worktree sont ordinaires.

## Préservation déjà préparée

- [Archive initiale](raw/correction-test-catalogue.zip), suivie par Git : 73 entrées, 115 154 octets, présentes aussi dans le worktree. Chaque entrée a été comparée par SHA-256 à son original N1 : zéro différence. SHA ZIP : `7fc9b2b4e0edba33ad07ce49dad833c10480402ad2294aa6c06db46443d23dfd`.
- [Complément de traces](raw/nettoyage-fix-catalog-test-traces.zip), créé pour cette proposition : 538 fichiers textuels résiduels (`log`, `jsonl`, `json`, `txt`, `srt`), 289 180 octets avant compression, 271 010 octets archivés. Arborescence préservée ; nombre, taille et SHA-256 de chaque entrée vérifiés. SHA ZIP : `2f12c97a613f2f09b27a2b3944cf19c8e485b94418c0bde7e23158bc34a03c1d`. Ces fichiers incluent des données synthétiques ; leur conservation ne constitue pas une qualification supplémentaire.

Les 695 autres fichiers sont des fixtures temporaires régénérables, faux binaires, symboles et audios synthétiques. Aucun modèle réel de 713 975 456 octets ne subsiste dans la sélection. Les WAV échantillonnés sont produits par les tests, notamment `capture_non_silent_pcm_baseline` et les scénarios `unified_poll` ; le manifeste rend leur suppression explicite.

## Chemins exclus et décision

Conserver les caches présents du worktree : `C:/wt/fix-catalog-test/target/cargo-home-shared/` (651 825 324 octets) et `C:/wt/fix-catalog-test/target/parakeet-tests/cargo-shared/` (3 903 617 682 octets), soit environ **4,24 Gio**. Les caches homologues de `C:/dev/whisper`, les fixtures permanentes `ci/fixtures/pe/`, les sources, les documents/archives Git et le modèle installé sous `C:/Users/cyril/AppData/Local/Whisper/models/` ont été vérifiés présents et sont exclus.

Ne pas supprimer le worktree entier, `target/`, `target/parakeet-tests/`, sa branche ni les autres worktrees. Enlever le worktree supprimerait aussi ses caches protégés. Recommandation : approuver uniquement les deux racines proposées et les objets du manifeste, avec traitement séparé des jonctions et recontrôle des archives, chemins et usages avant exécution. Aucun Cargo, test, déplacement ou nettoyage n'a été lancé par cet inventaire. Le bilan de livraison reste figé ; cette proposition est un document distinct.
