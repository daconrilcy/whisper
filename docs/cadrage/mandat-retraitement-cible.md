Avancée : besoin de retraitement ciblé qualifié ; décision utilisateur du 20/10/2026 sur l'identité audio et la mesure des lectures intégrée.
Blocage : design, plan et preuve de coût dans le parcours applicatif restent à produire ; aucune implémentation autorisée.
Décision : cadrer une paire TXT/SRT unique par traitement, la suppression ciblée et la réduction mesurée des lectures de source.

# Mandat de cadrage — retraitement ciblé des audios

**Statut :** expression de besoin pour le prochain cadrage, sans autorisation de code, de campagne de tests, de suppression de données réelles, d'installation ou de push. Base : `evo-startup-perf` au commit `1966101b` lors de la qualification, à revérifier. Le [plan R11](plan-retraitement-audio.md) et son [bilan](bilan-retraitement-audio.md) restent l'historique du comportement livré.

## Décisions utilisateur acquises

1. Deux fichiers dont les octets diffèrent, notamment un MP3 et un WAV du même son, sont **deux audios distincts**. Le SHA-256 des octets de la source est l'identité attendue ; un changement de nom seul ne change pas cette identité.
2. « Créer un nouveau » crée un dossier de transcription unique et traite normalement, sans dépendre de l'existence ou de l'intégrité des anciens dossiers et sans les modifier.
3. « Remplacer » liste les traitements du même audio et propose : le dernier **publié avec succès**, tous les précédents, ou un traitement désigné avec « Parcourir ». Le dossier parcouru peut être absent de la liste, mais doit être dans la destination courante et être reconnu comme traitement Whisper de ce même audio ; sinon il est refusé avant traitement.
4. Le nouveau traitement est publié dans un nouveau dossier. Les anciens traitements sélectionnés restent disponibles pendant le calcul. Après publication réussie seulement, l'application supprime leurs dossiers et leurs paires TXT/SRT historiques dans `exports/`, uniquement si leur propriété est prouvée. Un échec de suppression conserve le nouveau traitement et signale les anciens éléments restants. Les traitements non sélectionnés restent intacts.
5. Chaque **nouveau** traitement possède une seule paire `transcript.txt` / `transcript.srt` dans son propre dossier. Aucun nouveau TXT/SRT n'est publié dans `exports/`. Les exports historiques non sélectionnés ne font l'objet d'aucune migration ou suppression globale implicite.
6. Le cadrage doit **mesurer** le coût actuel des lectures du fichier source, proposer moins de lectures complètes et conserver la garantie qu'aucun ancien traitement n'est supprimé sur la base d'une identité audio devenue périmée.

La qualification indépendante de ces règles, avant l'ajout du point 6, a rendu **PASS** dans la conversation du 20/10/2026. Ses deux points de vigilance non bloquants portent sur la distinction entre traitements actifs et anciens conservés dans la liste, et sur la preuve de propriété des exports historiques.

## État du code à examiner

- `archive.rs::identify_source` calcule le SHA-256, analyse le nombre d'échantillons, puis recalcule le SHA-256 pour détecter un changement pendant cette analyse.
- `archive.rs::prepare_with_catalog_path` recalcule le SHA-256, réanalyse la source, puis recalcule le SHA-256. Un import peut ainsi effectuer **quatre lectures complètes pour SHA**, en plus des lectures nécessaires à l'analyse audio. Le nombre réellement exécuté selon le parcours doit être vérifié par mesure.
- Les empreintes des anciens traitements sont déjà enregistrées dans les données de publication ; leur détection ne nécessite pas de relire les anciens fichiers audio. Le scan des anciennes publications peut toutefois avoir son propre coût, à ventiler séparément.
- Une mesure ponctuelle en lecture seule avec PowerShell `Get-FileHash` a donné 22,4 ms sur un WAV de 5 min (9 600 044 octets) et 64,6 ms sur un WAV de 30 min (57 600 078 octets). **Ce n'est ni une mesure de la fonction Rust ni une mesure de bout en bout**, et le cache disque peut l'influencer.

## Travail demandé au cadrage

1. Établir la carte exacte des lectures de source pour « Créer un nouveau » et chaque mode « Remplacer » : admission, confirmation, préparation, décodage, publication et suppression. Séparer le nombre de lectures SHA, les analyses de durée et les scans des publications.
2. Mesurer, dans le **parcours applicatif** et hors fil UI, le nombre et la durée des lectures complètes de source, ainsi que le temps total, sur un WAV court et un WAV proche de la limite Parakeet de 30 min ; inclure un MP3 représentatif si disponible. Rapporter les tailles, le contexte de cache et les cas non exécutés. Utiliser d'abord les événements `archive_sha256` et `archive_audio_probe` existants ; toute instrumentation ou campagne supplémentaire exige un périmètre et un accord précis.
3. Comparer au moins deux stratégies de réduction des lectures, avec leurs limites : réutilisation d'une identité de source vérifiée entre étapes, lecture unique combinant empreinte et analyse si faisable, ou contrôle ciblé juste avant une opération destructive. Les métadonnées seules (chemin, taille, date) ne suffisent pas à autoriser la suppression.
4. Définir l'invariant vérifiable : la source confirmée, la transcription publiée et les anciens traitements choisis se rapportent aux identités attendues au moment où la suppression est engagée. Si l'identité a changé ou ne peut être établie, aucun ancien dossier ou export n'est supprimé ; le nouveau résultat et l'état réel sont signalés explicitement.
5. Concevoir la reprise après interruption entre publication et suppression, la sélection du dernier traitement publié, la validation du dossier parcouru, la preuve de propriété des anciens exports et la lecture des publications historiques sans les dupliquer.
6. Produire un design et, après son acceptation, un plan exécutable d'au plus dix tâches avec chemins d'écriture, mesures avant/après et critères de non-régression. Aucune cible de temps chiffrée n'est inventée avant la mesure.

## Critères de sortie du cadrage

- Tableau des lectures actuelles et des temps mesurés par étape, avec distinction WAV/MP3 et cache lorsque mesurables ; les absences de preuve sont `NOT RUN`.
- Stratégie retenue justifiant la réduction du nombre de lectures et un scénario où le fichier change après admission, sans suppression d'un ancien traitement sur identité périmée.
- Scénarios d'acceptation pour création indépendante, remplacement dernier/tous/choisi, refus d'un dossier non vérifiable, annulation/échec, suppression partielle, interruption/reprise et absence de nouvelle publication dans `exports/`.
- Revue indépendante du design puis plan accepté et accord écrit distinct avant toute implémentation ou suppression de données utilisateur.
