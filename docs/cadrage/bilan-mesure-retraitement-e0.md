Avancée : la référence R11 E0 a exécuté 18 imports Parakeet CPU sur trois sources, avec sorties TXT/SRT publiées et garde PASS.
Blocage : les lectures natives internes du worker et certaines phases sans identifiant d'opération ne sont pas ventilables par import ; aucun gain du futur retraitement n'est encore mesuré.
Décision : utiliser cette référence pour finaliser le plan RTC, puis comparer le candidat sur la même matrice.

# Bilan E0 — coût du retraitement R11 existant

Date : 20/10/2026. Branche `evo-startup-perf` ; candidat d'instrumentation `7b2b5666`, revu CLEAN au deuxième cycle. Banc et journaux sous `target/parakeet-tests/retraitement-cible/mesure-avant/`, Cargo partagé sous `target/parakeet-tests/cargo-shared/`. Aucun traitement personnel n'a été ouvert ou supprimé. Les sources, le modèle, le worker CPU R6 et les cinq DLL ont été lus sans modification.

## Exécution et confinement

- Préflight : plus de 30 Gio libres sur C:, sources présentes, `CARGO_HOME`, `TEMP` et `TMP` sous la racine E0, `CARGO_TARGET_DIR` sur le Cargo partagé. Les auto-tests ont refusé les chemins extérieurs, la destination personnelle, le `CARGO_HOME` absent, la jonction sortante et les écritures factices hors racine ; le témoin interne a réussi.
- `cargo fmt --all -- --check` : PASS ; compilation hors ligne verrouillée du test : PASS ; `r11_application_baseline` : PASS, **18/18 imports**, 0 FAIL, 1437,95 s de test. Le délai de 10 min par import et la borne de 3 h n'ont pas été atteints.
- Garde pendant la recette : 0 événement attribué hors racine et 0 erreur de watcher. Elle a classé séparément 1 207 événements du TEMP réel comme bruit tiers non attribué. Cette classification par nom ne prouve pas exhaustivement l'origine de chaque événement Windows ; aucune écriture de cette catégorie n'est revendiquée comme autorisée pour le projet.
- Les 10 ressources contrôlées avant/après — trois audios, modèle Parakeet, worker CPU R6 et cinq DLL — ont conservé exactement leur taille et SHA-256. Le modèle était `e3880d0a…`, le worker `1d8f57b9…`. Les empreintes complètes et les chemins sont dans `logs/resources-before.json` et `logs/resources-after.json` du banc ; **0 divergence**.
- Espace libre observé : environ 140,15 Gio avant la recette et 140,14 Gio après. Les fixtures et journaux E0 restent sous la racine du lot ; aucun nettoyage non autorisé n'a été fait.

## Matrice mesurée

Chaque ligne correspond à une demande applicative dans le même processus (`PID 63240`), jusqu'à l'écriture de TXT et SRT. Les identifiants abrégés `01` à `12` sont les suffixes hexadécimaux des identifiants complets présents dans `logs/Harness.stderr.log` et `logs/timing.jsonl`. « SHA » compte uniquement les lectures complètes de **l'audio original par l'adaptateur**, sans les SHA des exports, modèles, DLL et worker. Les probes et scans sont les événements produit attribués à la demande. Les temps d'admission et confirmation sont des sous-phases du temps total et ne s'additionnent pas à celui-ci.

| Source | Passe | Choix | ID | Total s | SHA original (n / s) | Probe n | Scan n | Admission s | Confirmation s |
| --- | ---: | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| WAV 5 min | 1 | Initial | 01 | 61,20 | 4 / 0,385 | 3 | 5 | 24,095 | — |
| WAV 5 min | 1 | Créer | 02 | 37,47 | 6 / 0,582 | 4 | 7 | 0,220 | 0,235 |
| WAV 5 min | 1 | Remplacer | 03 | 37,17 | 6 / 0,581 | 4 | 7 | 0,216 | 0,228 |
| WAV 5 min | 2 | Initial | 04 | 36,32 | 4 / 0,389 | 3 | 5 | 0,222 | — |
| WAV 5 min | 2 | Créer | 05 | 36,97 | 6 / 0,585 | 4 | 7 | 0,225 | 0,239 |
| WAV 5 min | 2 | Remplacer | 06 | 37,38 | 6 / 0,579 | 4 | 7 | 0,219 | 0,225 |
| WAV 30 min | 1 | Initial | 07 | 188,62 | 4 / 2,332 | 3 | 5 | 1,202 | — |
| WAV 30 min | 1 | Créer | 08 | 188,30 | 6 / 3,547 | 4 | 7 | 1,190 | 1,226 |
| WAV 30 min | 1 | Remplacer | 09 | 189,20 | 6 / 3,528 | 4 | 7 | 1,199 | 1,238 |
| WAV 30 min | 2 | Initial | 0a | 187,28 | 4 / 2,371 | 3 | 5 | 1,215 | — |
| WAV 30 min | 2 | Créer | 0b | 186,43 | 6 / 3,498 | 4 | 7 | 1,192 | 1,211 |
| WAV 30 min | 2 | Remplacer | 0c | 188,86 | 6 / 3,514 | 4 | 7 | 1,201 | 1,231 |
| MP3 15 s | 1 | Initial | 0d | 10,16 | 4 / 0,020 | 3 | 5 | 0,034 | — |
| MP3 15 s | 1 | Créer | 0e | 10,07 | 6 / 0,035 | 4 | 7 | 0,038 | 0,047 |
| MP3 15 s | 1 | Remplacer | 0f | 10,02 | 6 / 0,029 | 4 | 7 | 0,033 | 0,040 |
| MP3 15 s | 2 | Initial | 10 | 10,37 | 4 / 0,020 | 3 | 5 | 0,038 | — |
| MP3 15 s | 2 | Créer | 11 | 10,22 | 6 / 0,030 | 4 | 7 | 0,031 | 0,045 |
| MP3 15 s | 2 | Remplacer | 12 | 10,12 | 6 / 0,029 | 4 | 7 | 0,034 | 0,042 |

Les 32 lectures de l'original par source représentent 307 201 408 octets / 3,101 s pour `spkr0.wav`, 1 843 202 496 octets / 18,789 s pour `spkr0_x6.wav`, et 15 514 624 octets / 0,162 s pour le MP3. Les durées de probe étaient inférieures au milliseconde par opération ; les scans de publications prenaient 0,006 à 0,057 s au total par opération. Le premier WAV court inclut une admission à 24,095 s liée notamment aux premières vérifications SHA du modèle et des artefacts ; il ne constitue pas un « temps froid » garanti.

## Ce que la mesure permet et ne permet pas

- L'adaptateur a effectué une seule lecture complète de chaque modèle et de chaque artefact de worker/DLL sur la session ; les autres vérifications ont utilisé le cache d'identité. Le worker a été relancé 18 fois. Les traces adaptateur agrégées donnent 149,573 s de démarrage-à-prêt et 1 037,165 s de demande-au-résultat worker, mais elles sont `unscoped` dans ce journal : **leur ventilation certaine par cellule reste NOT RUN**. Le temps d'inférence natif propre au worker gelé reste NOT RUN.
- Les phases de préparation/persistance et les SHA des exports sont également enregistrés, mais plusieurs n'ont pas d'identifiant d'opération. Les agréger dans une cellule précise ajouterait une attribution inférée ; le tableau n'en fait pas. L'attente humaine est nulle dans ce harness automatisé, et sa préparation/compilation n'est pas comprise dans les 18 temps applicatifs.
- Le coût de relecture de l'original augmente avec la taille (2,33 à 3,55 s par demande pour le WAV de 30 min). La réduction visée dans le design, de 6 lectures à 1 pour Créer et 2 pour Remplacer, est une **hypothèse de conception**, pas un gain de temps total démontré : copie et lecture du snapshot restent à mesurer sur le futur candidat avec cette même matrice.
- La garde utilise FileSystemWatcher et le filtrage du bruit TEMP : elle peut constater les événements attribuables surveillés, sans garantir l'absence physique de toute écriture non observée. Aucun dossier utilisateur n'a servi de destination de transcription.

État E0 : **PASS pour la matrice R11 et le confinement observé ; PARTIAL pour la ventilation native et les phases sans identifiant**, qui restent NOT RUN au niveau par import. E0 ne qualifie pas le futur comportement RTC.
