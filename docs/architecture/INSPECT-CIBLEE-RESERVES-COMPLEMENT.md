Avancée : R1/R4 fermés par revue indépendante ; N1/P4/P5/P6 vérifiés.
Blocage : aucun pour les réserves du plan ; dettes hors périmètre conservées.
Décision : réserves INSPECT-CIBLEE levées dans ce périmètre ; commits locaux, sans merge/push.

# Complément figé — 6 novembre 2026

Reprise autorisée du [plan](../cadrage/plan-levee-reserves-inspect-ciblee.md), base `d1829c61`, branche `inspect-ciblee`. Le [bilan initial](INSPECT-CIBLEE-BILAN.md) conserve son état de clôture.

**Correction.** Seul `crates/whisper-adapters/src/worker_ipc/tests/queue_inspection_characterization.rs` change : compteurs sans worker, admission distincte de l'intégrité, sept scénarios conservés. Catalogue et cas Turbo vérifient l'intégrité ; preuves rouges `34975d43` rapprochées des mêmes compteurs. Aucun produit, garde ou manifeste modifié.

**Revue indépendante : CLEAN pour code/validations**, un passage ; R1/R4 fermés, R2/R3 code non réouverts. P5 est vérifié ensuite séparément.

| Vérification | Résultat |
|---|---|
| SelfTests N1 variable / sans variable | 48/49 positifs et 75 négatifs, verts |
| Focus / sourceguard / catalogue | 7/7 ; 1/1 ; 4/4 |
| Format / build adapters / Clippy all-targets | Exit 0 ; 54 avertissements Clippy, sans nettoyage opportuniste |
| Garde générique standard | 220 assertions vertes ; 533,995 / 491,6981 / 533,9148 ms ; limites avec pause de 100 ms respectées |
| N1-1, 10:14:32–10:15:10 | 722 réussites / 0 échec / 128 ignorés |
| N1-2, 10:17:35–10:17:47 | 722 réussites / 0 échec / 128 ignorés |
| N1-3, 10:18:45–10:18:58 | 722 réussites / 0 échec / 128 ignorés |

Chaque N1 agrège 59 résumés ; +7 tests contre 715/0/128. Variable enfant `CARGO_TARGET_DIR` absente, cible externe vérifiée. P6 : six inventaires avant/après, zéro résidu ciblé dans les TEMP effectif/utilisateur.

**P5 humain.** Copie partielle arrêtée sur jonctions INetCache refusées. Récupération approuvée/exécutée par l'utilisateur : trois dossiers vides retirés, liens déplacés, copie résiduelle terminée, pile renommé en bloc. ACL inchangées, aucune élévation ; métadonnées des liens identiques.

| Destination sous `C:/wb/preserved/` | Fichiers | Octets | Liens |
|---|---:|---:|---:|
| `poll-preserved` | 117586 | 89160572401 | 181 |
| `pile-preserved` | 8485 | 11644240959 | 97 |

Sources : zéro fichier/zéro lien. Comptes vérifiés indépendamment par la principale ; C11 sur les quatre racines réelles PASS, sans lancer NativeBuild ni watcher. Aucun agent n'a déplacé/supprimé d'archive.

**Preuves/rétention.** [Archive](raw/INSPECT-CIBLEE-RESERVES-20261106.zip) : **111 fichiers, 231921 octets**, nombres/SHA comparés, sources inchangées. SHA ZIP : `F5A75BFCCFE162C0715F1F3EBCCD1C52D62BA3D97B3609C4682C9CEEFAFD1BE0`. Commandes, sorties, inventaires et procédures conservés.

Créations : preuves externes 64 fichiers/389237 octets ; campagne isolée 1144/24157919 ; contrôle synthétique 34/250360498. Commandes humaines dans `external/evidence-levee-reserves/retention-nettoyage-propose.txt` du ZIP. Aucun nettoyage agent. Cible Cargo réutilisable 4853119849 octets et cache persistant 651825324 conservés ; anciennes commandes de nettoyage périmées.

**Limites.** Incidents de lanceur (stdout vide, quoting JSON) arrêtés puis reprises autorisées ; ancien stderr C2 écrasé, observation antérieure conservée sans le présenter comme original brut. Session N1-1 récupérée sans relance. Les violations historiques du premier lot restent inchangées. C libre 457030987776→454844723200 octets, mémoire 19→20 %, aucune nouvelle WHEA/crash signalée. Aucun build natif, recette Windows ni gain release revendiqué ; mutex SHA et inspections micro/démarrage restent différés.
