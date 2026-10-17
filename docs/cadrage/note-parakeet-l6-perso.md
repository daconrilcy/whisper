Avancée : L6-perso remplace L5 et L6 pour l'installation personnelle locale de l'unique utilisateur.
Blocage : le code actuel est lié au dépôt et à la clé Test R6 ; la qualification du candidat release est à faire.
Décision : proposer un profil Perso CPU/import distinct de Test et soumettre le plan à une revue indépendante avant tout code.

# Note de cadrage — L6-perso (proposition v1, 17/10/2026)

## Besoin, portée et décisions

Source : demande utilisateur du 17/10/2026 et complément du même jour. L'objectif est de pouvoir nettoyer `C:\dev\whisper\target\` sans casser l'application personnelle. L5 (acquisition réseau) et L6 (paquet distribuable) sont remplacés par L6-perso : copie locale de deux modèles déjà présents, application et worker release, cinq DLL CPU de référence, réglages conservés, installation et retour arrière documentés. Aucun réseau, installateur, distribution ni signature. Parakeet reste CPU, import FR/EN en passe unique jusqu'à 30 min ; Turbo, ses modes et le micro existants restent disponibles. Aucune capacité nouvelle de Parakeet n'est inférée de la qualification Test.

Le design parle de `Test` et `Final`, mais `Final` désigne un paquet distribué avec acquisition et d'autres variantes. Il n'est pas implémenté. `Perso` est donc une composition distincte et locale, sur le même catalogue fermé, sans prétendre satisfaire les critères de distribution Final. Test continue de lire les modèles du dépôt et le worker R6 sous `target/`.

L3 (GPU Parakeet), L4 (micro Parakeet), VAD adaptative et B8/import-vad passent en **backlog, non planifié** : utilisateur unique, CPU jugé très performant, aucun enregistrement de plus de 30 min prévu. Les notes [micro L4](note-parakeet-micro-l4.md) et [VAD adaptative](note-parakeet-vad-adaptative.md) restent des références de reprise, sans autorisation de code. La limite de 30 min et le refus explicite avant pending subsistent.

## Emplacement et provenance proposés

Racine résolue à l'exécution par `%LOCALAPPDATA%\Whisper\` (jamais `C:\Users\cyril` codé). Le fichier historique `%LOCALAPPDATA%\Whisper\settings.json` peut déjà exister ; il est source de migration et n'est pas effacé par le déploiement.

| Sous-dossier | Contenu prévu | Provenance |
| --- | --- | --- |
| `app\` | `whisper-desktop.exe`, `whisper-worker-cpu.exe`, `whisper-worker-gpu.exe` release et dépendances privées réellement nécessaires | `cargo build --offline --locked --release` et inventaire des imports ; Turbo garde ses parcours existants |
| `worker\` | `whisper-worker-parakeet-cpu.exe` release | crate `whisper-worker-parakeet-cpu`, nouveau SHA |
| `native\` | `nemo_speech_asr_c.dll`, `nemo_speech_asr.dll`, `ggml.dll`, `ggml-base.dll`, `ggml-cpu.dll` | copies vérifiées de `C:\dev\whisper\target\parakeet-tests\l2\l2b\native\` |
| `models\` | `ggml-large-v3-turbo.bin`, `parakeet-tdt-0.6b-v3.q8_0.gguf` | copies vérifiées de `C:\dev\whisper\models\` ; jamais téléchargées |
| `settings\` | `settings.json`, `settings.json.bak` avant première écriture ou migration ; futures préférences dans cette racine | copie du réglage actuel `%LOCALAPPDATA%\Whisper\settings.json` si présent |
| `.staging\<id>\`, `backups\<id>\` | version en préparation et version précédente récupérable | script de mise à jour personnel, hors exécution normale |

Le SHA des modèles est celui du registre (`1fc70f77…` Turbo, `e3880d0a…` Parakeet) ; les cinq DLL doivent correspondre aux SHA de [la fiche R6](fiche-parakeet-r6.md). Le worker release aura une **nouvelle** identité : son SHA et la clé agrégée ne peuvent être prédits par le cadrage. L'installation est refusée si une source est absente ou divergente ; aucun repli vers Downloads, Test ou une ancienne DLL. Les chemins du worker, des DLL et du modèle sont vérifiés sous la même racine Perso avant lancement. Les fichiers nécessaires au lancement de Turbo et les imports DLL transitifs du release sont inventoriés avant de figer le lot ; absence bloquante, sans téléchargement implicite.

## Contrats de fonctionnement

La composition choisit explicitement Test ou Perso ; elle ne déduit pas le profil d'un modèle trouvé. Le mode Perso résout `app/`, `worker/`, `native/`, `models/` et `settings/` depuis la racine stable ; il ne lit aucun artefact de `target/` à l'exécution. Les deux modèles restent visibles ; seul un modèle validé est sélectionnable. Une clé worker/DLL/modèle différente rend Parakeet indisponible avec cause. Les pending liés à une ancienne clé sont conservés mais non repris par la nouvelle sans qualification explicite. Un import de plus de 30 min est refusé avant pending avec « Fichier trop long pour Parakeet, utilisez Turbo » ; aucune bascule silencieuse.

Le réglage actuel est copié octet pour octet ; avant toute première écriture Perso, `settings/settings.json.bak` contient les octets d'origine et n'est jamais écrasé. Une migration v1→v2 suit la règle existante de sauvegarde v1 ; les préférences modèle, calcul, sortie, micro et GPU sont conservées. Si la copie ou la migration échoue, l'ancien réglage reste utilisable, l'installation n'est pas publiée et la cause est rapportée. Test et Perso ne doivent pas écrire dans le même fichier de réglages.

La mise à jour se fait application arrêtée : préparer sous `.staging/<id>`, vérifier SHA et complétude, sauvegarder l'installation précédente sous `backups/<id>`, publier la nouvelle version comme un ensemble cohérent, puis vérifier un lancement hors dépôt. En cas d'échec de copie/publication ou de recette, revenir à l'ensemble précédent et à ses réglages, sans mélanger des fichiers de générations différentes. La procédure et les commandes de retour arrière seront dans `scripts/install-personal.ps1` et `docs/cadrage/recette-parakeet-l6-perso.md` après autorisation du code. Aucune écriture sous `%LOCALAPPDATA%` n'est effectuée pendant ce cadrage.

## Qualification attendue et limites

La fiche Perso contiendra SHA du worker **release**, du modèle et de chaque DLL réellement chargée, clé de profil et périmètre CPU/import. Sur ce candidat exact : A4 FR87/FR27/EN15 (TXT/SRT, horodatages, `faire? Je`, pas de U+FFFD), EN5 réel, EN30 synthétique, cinq scénarios R8 de refus, puis suite workspace `--no-fail-fast`. Les tests R8 ne deviennent pas des permissions de panne injectée dans l'exécutable installé : les builds de recette et celui de l'application sont isolés, et l'option `PARAKEET_L2_ACCEPTANCE_TEST_BUILD` est absente de la build installée. Vérifier par un parcours applicatif réel sous la racine Perso, non seulement par un harness. La recette manuelle de la fenêtre Windows reste à l'utilisateur.

Tâche annexe : les tests dépendant de variables de campagne `PARAKEET_*`/`WHISPER_*` se déclarent `NOT RUN` lorsque l'entrée manque, tout en conservant leurs assertions une fois la campagne activée. La suite workspace **sans ces variables** doit rendre zéro FAIL et lister ses NOT RUN. Les tests ordinaires indépendants continuent de s'exécuter. Un test ignoré n'est pas compté PASS.

Risques ouverts : dépendances release/Turbo non encore inventoriées, temps et résultat de la nouvelle qualification, vrai enregistrement FR long absent et qualité absolue sans vérité terrain. Aucun GPU Parakeet, live/micro Parakeet, VAD longue, installateur, signature ni acquisition réseau n'est inclus.
