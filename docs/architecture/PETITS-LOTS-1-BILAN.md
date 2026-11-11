Avancée : PETITS-LOTS-1 exécuté selon ses conditions; SelfTest niveau 1 réussi (64 positifs, 99 négatifs).
Blocage : le N1 complet n’a pas été exécuté; sources du cache Cargo et cible canonique absentes. Suppression temporaire refusée par l’outil.
Décision : T1 déjà conforme; arrêts T2/T3 appliqués; aucun changement produit, merge ou push.

# Bilan — PETITS-LOTS-1

Suivi après clôture : [vérification du nettoyage](PETITS-LOTS-1-SUIVI-NETTOYAGE.md).

Base : worktree `C:/wt/petits-lots`, branche `petits-lots-1`, HEAD initial `981b0bd5`; état propre. Aucun changement concurrent pertinent.

| Point | Résultat et preuve |
|---|---|
| T0 | Préflight conforme; plus de 30 Gio libres. |
| T1 | Déjà conforme : `/deliverables/` ignoré, aucun fichier suivi, aucune référence opérationnelle dans `ci/`, `native/` ou `crates/`. Les 14 documents trouvés sous `docs/` sont historiques/manuels. Les 9 EXE locaux sont conservés. Aucun changement ni commit. |
| T2/T4 | Arrêt avant édition : `GGML_NATIVE=ON`; les quatre manifestes CPU r2/GPU r5 indiquent `ON` et `configSha256=034756A1…7804A9D`, égal au SHA-256 de `native/build-config.json`. Auto-tests conditionnels non exécutés. |
| T3/T5 | Arrêt : aucun fichier rustfmt; la garde accepte seulement `cargo fmt --check`. Formatage, revue, build, N1 précommit et commits de format/blame non exécutés. |
| T6 | `ci/guard-level1.ps1 -Mode SelfTest -LotRoot target/parakeet-tests/petits-lots-1` : code 0, 64 positifs/99 négatifs; sondes `CARGO_TARGET_DIR` présentes et absentes réussies. Modèle persistant vérifié, SHA-256 attendu. N1 complet non exécuté : `target/cargo-home-shared/registry/src` et `target/parakeet-tests/cargo-shared` absents. |
| T7 | Sorties archivées dans [`PETITS-LOTS-1-SELFTEST.zip`](raw/PETITS-LOTS-1-SELFTEST.zip) : 11/11 entrées, SHA-256 du contenu vérifiés, 129 802 octets; SHA-256 de l’archive `619505DA0519C86B3CDBCDE755F73A69E59C5EDFCD99F1FD2ED8F563202BBC5E`. La racine temporaire `target/parakeet-tests/petits-lots-1` reste présente après refus de suppression par l’outil. Aucun nettoyage exécuté. |

Déplacement proposé seulement (source présente, destination absente) :

```powershell
Move-Item -LiteralPath 'C:\dev\whisper\deliverables' -Destination 'C:\wb\deliverables' -ErrorAction Stop
```

Nettoyage proposé après vérification de l’archive et du chemin exact; le retrait du worktree est différé car ses cinq commits ne sont pas conservés ailleurs :

```powershell
$p = (Resolve-Path -LiteralPath 'C:\wt\petits-lots\target\parakeet-tests\petits-lots-1').Path
if ($p -cne 'C:\wt\petits-lots\target\parakeet-tests\petits-lots-1') { throw 'Chemin inattendu.' }
if ((Get-FileHash 'C:\wt\petits-lots\docs\architecture\raw\PETITS-LOTS-1-SELFTEST.zip' -Algorithm SHA256).Hash -cne '619505DA0519C86B3CDBCDE755F73A69E59C5EDFCD99F1FD2ED8F563202BBC5E') { throw 'Archive non vérifiée.' }
Remove-Item -LiteralPath $p -Recurse -ErrorAction Stop
```

Aucun merge ni push. Validation dans la fenêtre Windows non applicable.
