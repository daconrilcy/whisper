Avancée : suppressions exécutées par l'utilisateur et vérifiées complètes le 9 novembre 2026.
Blocage : aucun ; preuves, caches partagés et modèle source préservés.
Décision : nettoyage clôturé dans le périmètre approuvé ; worktree et branche conservés.

# Vérification après nettoyage de fix-catalog-test

Références : [proposition approuvée](proposition-nettoyage-fix-catalog-test.md) et [manifeste](nettoyage-fix-catalog-test-chemins.csv), commit `159e9d41`. L'utilisateur a exécuté le bloc PowerShell remis dans la session et signalé sa fin sans erreur. La présente vérification est en lecture seule ; aucune suppression supplémentaire ni campagne Cargo n'a été exécutée.

**Suppression complète.** Les 2 534 chemins du manifeste ont été contrôlés : **0 restant**, soit 1 306 fichiers, 1 226 répertoires et 2 jonctions absents. Les deux racines `C:/wt/fix-catalog-test/target/parakeet-tests/fix-catalog-test/` et `C:/wt/fix-catalog-test/target/parakeet-tests/refacto-parakeet-live/` sont absentes. Le manifeste est inchangé : SHA-256 `559bdf5ca7faa95424ce8cd47ab4a456097249add6a32226e6da50c9170d5a61`. Le volume logique inventorié supprimé est de 32 220 710 octets (30,73 Mio).

**Preuves conservées.** Les deux archives dans `C:/dev/whisper/docs/cadrage/raw/` sont suivies par Git, toutes leurs entrées ont été relues et leurs SHA-256 correspondent exactement aux valeurs antérieures :

| Archive | Entrées | Octets | SHA-256 |
|---|---:|---:|---|
| `correction-test-catalogue.zip` | 73 | 115 154 | `7fc9b2b4e0edba33ad07ce49dad833c10480402ad2294aa6c06db46443d23dfd` |
| `nettoyage-fix-catalog-test-traces.zip` | 538 | 271 010 | `2f12c97a613f2f09b27a2b3944cf19c8e485b94418c0bde7e23158bc34a03c1d` |

La copie de l'archive initiale présente dans le worktree possède aussi le même SHA-256. Le bilan de livraison initial reste inchangé.

**Caches préservés.** Inventaire sans traverser les liens ; aucune jonction ni aucun autre point de réanalyse trouvé dans ces quatre caches :

| Racine | Fichiers | Octets |
|---|---:|---:|
| `C:/wt/fix-catalog-test/target/cargo-home-shared/` | 22 417 | 651 825 324 |
| `C:/wt/fix-catalog-test/target/parakeet-tests/cargo-shared/` | 5 040 | 3 903 617 682 |
| `C:/dev/whisper/target/cargo-home-shared/` | 22 417 | 651 825 324 |
| `C:/dev/whisper/target/parakeet-tests/cargo-shared/` | 12 902 | 10 383 206 576 |

Pour les deux caches du worktree, les nombres de fichiers et tailles sont exactement ceux de l'inventaire préalable. Il s'agit d'une vérification de présence, nombre et taille, pas d'un contrôle d'empreinte de chaque fichier de cache.

**Autres éléments conservés.** Le worktree est propre à `e2953ce0` ; les fixtures permanentes `ci/fixtures/pe/` sont présentes dans les deux checkouts (10 fichiers chacun). Le modèle installé `C:/Users/cyril/AppData/Local/Whisper/models/parakeet-tdt-0.6b-v3.q8_0.gguf` reste à 713 975 456 octets avec le SHA-256 attendu `e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e`.

Espace libre observé après nettoyage : 476 838 502 400 octets, soit 444,09 Gio. La variation globale du volume n'est pas utilisée pour mesurer le gain propre à ce nettoyage. Aucun push supplémentaire ; bilan figé à cette clôture.
