Avancée : une nouvelle règle R2 a été préparée dans l'outil autonome Q2, avec trois simulations ajoutées mais non exécutées.
Blocage : un appel Cargo a précédé le préflight obligatoire des racines ; son éventuel effet dans le Temp du profil reste non attribuable, et ETW ne prouve pas l'absence de délégation vers un broker.
Décision : NO-GO pour ce candidat et arrêt de l'approche de classification par ETW global ; aucune tentative Q2 ni FEAS-PM-01.

# FEAS-PM-01 — clôture du lot R2 de classification

## Résultat et périmètre

Le lot autorisé le 22 octobre 2026 visait uniquement la classification R2 dans l'outil autonome sous `target/parakeet-tests/micro-feasibility/confinement-q2/`. Le fichier `src/main.rs` a été modifié pour distinguer `JOB_Q2_PROUVÉ`, `ÉTRANGER_PROUVÉ` et `INCONNU`, décider l'origine avant d'exiger le chemin, et conserver les événements ambigus jusqu'à un échec conservateur. Trois simulations ont été ajoutées, dont l'ID 16 sans chemin issu d'une génération étrangère terminée. `launcher.rs`, `Cargo.toml` et `Cargo.lock` n'ont pas changé. Le code produit, le jeton, les ACL et le Job Object n'ont pas été modifiés.

Ce correctif est **non vérifié** : aucune simulation, compilation ni revue indépendante du candidat n'a été réalisée. Les préconditions explicites de la revue demandée — correction validée par simulations — ne sont donc pas remplies. Aucun contrôleur ou session ETW, lanceur ou sonde Q2, essai CUDA, stratégie A/B ou FEAS-PM-01 n'a été exécuté. Les seules entrées non suivies visibles par Git restent `models/` et `sound example/`, préexistantes et non touchées.

## Incident de préflight Cargo

L'implémenteur a lancé `cargo metadata --offline --locked --no-deps` avant de fixer et vérifier `CARGO_HOME`, `TEMP`, `TMP` et `CARGO_TARGET_DIR` dans les racines exigées par `AGENTS.md`. Cet ordre est contraire à la garde préalable, même si la commande ne demandait ni compilation ni téléchargement. À ce moment, `CARGO_HOME` et `CARGO_TARGET_DIR` étaient absents ; `TEMP` et `TMP` pointaient vers `C:\Users\cyril\AppData\Local\Temp`. La commande s'est terminée au plus tard au contrôle horodaté du 22 octobre 2026 à 07:04:50 CEST ; son heure exacte et son code de sortie n'ont pas été capturés.

L'examen en lecture seule n'a trouvé aucun changement visible du cache Cargo du profil depuis le 20 octobre. Le dossier Temp du profil porte une modification vers 07:04:54 CEST, sans fichier enfant actuellement visible dans la fenêtre examinée. Il n'existe **ni preuve d'une écriture de Cargo hors racine, ni preuve de son absence** ; l'auteur de cette activité Temp reste indéterminé. Aucun autre Cargo ni test n'a été lancé après ce constat. Le lot n'a pas été repris automatiquement.

## Identité du delta non qualifié

Le snapshot du candidat antérieur est conservé sous `target/parakeet-tests/micro-feasibility/confinement-q2/r2-20261022-baseline/`. Empreintes SHA-256 relevées en lecture seule :

| Fichier | Candidat antérieur | État actuel |
| --- | --- | --- |
| `src/main.rs` | `3783688B3750578F481678815AD84F8508AFD6302E66E18EBA19AE99A1A143B5` | `8FAB8EB612FD130A4B935DD187BFCC66BC97AD7B30B6F3B8B0AFACCEE0046A63` |
| `launcher.rs` | `25FA7FCB5152FA9324321154CC4CE7EBF2E83627AC09101EDD3EDF20DFE37B8E` | inchangé |
| `Cargo.toml` | `3ABEE8F1499EFD52B67DDBBE7842641B6056CE680767D3B09C35142F2DB92648` | inchangé |
| `Cargo.lock` | `88F064311B773DD3D7F777A7A4474529FBF5646A7A61BDECE597885DDDCFFBB0` | inchangé |

Les exécutables existants sont ceux du candidat précédent ; aucune nouvelle empreinte d'exécutable ne représenterait le nouveau code. `target/` est ignoré par Git : ce document versionne le bilan, pas le delta source ni son snapshot. Aucun nettoyage de ces fichiers n'est autorisé par ce bilan.

## Limite de la classification et nouvelle approche à instruire

L'identité du processus, sa période de vie et `IsProcessInJob` peuvent établir qu'un événement vient d'un PID hors du Job. Ces données ne démontrent pas qu'un service ou broker extérieur n'a pas reçu une opération déléguée par Q2. Pour ne pas masquer cette possibilité, le delta laisse les événements hors Job après `READY` dans l'état `INCONNU`. Sur une trace ETW globale, des écritures extérieures ordinaires peuvent alors faire échouer la campagne ou saturer l'attente. Le cas R2 ayant motivé ce lot reste donc sans solution de surveillance à la fois stricte et exploitable avec les seules preuves disponibles. Le mécanisme **préventif** du jeton restreint et des ACL n'a pas été mis en échec par ce constat ; il demeure non qualifié en exécution.

**Approche plus simple à examiner séparément :** isoler la qualification dans un environnement Windows dédié aux processus d'essai et observer les modifications de son système de fichiers comme un tout, au lieu de classer les écritures d'une trace ETW globale de l'hôte. Les processus du banc et leurs délégations seraient traités comme des modifications de l'environnement à justifier ; une écriture hors racines resterait un échec. Cette piste exige un nouveau cadrage de l'isolation, de la preuve d'attribution, des privilèges et de la compatibilité NVIDIA avant tout essai. Elle ne constitue pas une solution déjà qualifiée, ni une autorisation d'installer, de configurer ou d'exécuter quoi que ce soit.

Décision requise : choisir une autre architecture de surveillance et statuer sur le sort du delta R2 non vérifié. Aucun nouveau cycle ETW, essai Q2, CUDA ou FEAS-PM-01 ne découle de ce rapport.
