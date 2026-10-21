Avancée : onze dossiers de bancs historiques ont été supprimés ; `target/` est passé de 128,019 à 22,262 Gio de taille logique.
Blocage : aucun ; les preuves sont archivées localement sous `target/` et ne sont pas suivies par Git.
Décision : conserver les ressources qualifiées et le Cargo partagé ; ne pas étendre le nettoyage sans nouveau périmètre.

# Nettoyage de `target/` — 21 octobre 2026

Le nettoyage manuel a été exécuté depuis PowerShell 7 avec le script
`target/parakeet-tests/cleanup-evidence/2026-10-21/nettoyer-target.ps1 -Execute`.
Le préflight a validé les onze chemins exacts, l'absence de jonction ou de lien
symbolique dans ces chemins, ainsi que les 1 258 petits fichiers archivés.
L'utilisateur a ensuite confirmé la suppression dans la console.

## Périmètre supprimé

Les onze dossiers supprimés figurent dans
`target/parakeet-tests/cleanup-evidence/2026-10-21/manifest.tsv` :

| Type | Chemins relatifs à `target/parakeet-tests/` | Taille logique |
| --- | --- | ---: |
| Bancs `l1b-runtime` clos | `r11/l1b-runtime`, `l3/l3b/l1b-runtime`, `l3/l3c/l1b-runtime`, `l6-perso/l1b-runtime`, `r10/l1b-runtime` | 101,376 Gio |
| Copies de cache Cargo de lots clos | `retraitement-cible/{mesure-avant,rtc-01,rtc-02,rtc-03,rtc-04,rtc-05}/cargo-home` | 4,387 Gio |

Total du manifeste : **113 561 988 670 octets (105,763 Gio logiques)**.
Les cinq bancs contenaient 67 liens physiques vers le même modèle Turbo que
`models/ggml-large-v3-turbo.bin`. Leur retrait réduit fortement la taille
additionnée des chemins, sans libérer autant d'espace physique.

Le journal `deletion-log-20261021-095927.tsv` enregistre **11 résultats
`DELETED` sur 11**. L'espace libre mesuré par le script est passé de
**185,83 à 191,90 Gio**, soit **6,07 Gio récupérés**. Une vérification
indépendante a confirmé l'absence des onze dossiers et une taille restante
de `target/` de **23 903 727 025 octets (22,262 Gio)**.

## Archives et ressources conservées

L'archive locale est
`target/parakeet-tests/cleanup-evidence/2026-10-21/`. Elle conserve
l'arborescence de **1 258 petits fichiers (6 243 301 octets)** issus des cinq
bancs, ainsi que `manifest.tsv`, `proofs.tsv`, le journal de suppression et
le script utilisé. `proofs.tsv` contient le SHA-256 de chaque fichier archivé.
Les 1 258 fichiers ont été revérifiés après le nettoyage : nombre, taille et
SHA-256 conformes. Cette archive est **ignorée par Git** ; le commit de ce
bilan ne publie ni ces fichiers ni le script. Une suppression globale de
`target/` ferait donc aussi disparaître l'archive.

Ont été vérifiés présents après l'opération : les deux modèles sous
`models/`, `sound example/spkr0.wav`, `target/parakeet-tests/cargo-shared/`,
le worker CPU R6 et ses cinq DLL, le worker GPU L3a et ses neuf DLL, et le
`cargo-home` du lot `catalogue-verifie`. Aucun fichier suivi par Git n'a été
modifié par la suppression. Aucun test de transcription ou suite Cargo n'a
été relancé après ce nettoyage ; les contrôles portent sur les fichiers,
le manifeste, l'archive et le journal.
