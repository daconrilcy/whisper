# Sources et décisions — D-WHISPER-09

## Autorité et limites

Les références `thread/turn` ci-dessous désignent des messages **utilisateur** accessibles par `read_thread` dans l'application Codex. Les propositions de l'annexe initiale et des spécialistes ne sont pas des acceptations en elles-mêmes. Le message utilisateur de ce chat du 2026-10-05 confirme explicitement : « Oui, tout est inclus en V1 » pour démarrage Windows, réglages, raccourci global et choix du dossier. L'autorisation documentaire est `docs/predev/authorization.md` (SHA-256 `bbe954936ff6e4108c11024301f3edb68fa1c3499a44096552794a9ef353b771`) ; elle n'est pas une acceptation produit.

Source initiale jointe par l'utilisateur : `C:\Users\cyril\.codex\attachments\5d8b8ae7-430b-46f8-b28f-a3e57f2c031f\Texte collé.txt` (SHA-256 `c6e750d6b46af206d22db3f84f0cb695540c53ed2243fce96be56b72e57508aa`), citée par le message `01a108b7-d4c6-7930-b0c4-e1be76774a9a` du chat `01a108b7-9f64-7671-bd72-21db5f9b42d6`. C'est une **proposition de départ** ; son phasage V0.1–V0.4 n'a pas été retenu comme autorité sur la V1 actuelle. Elle décrit l'accès tray, le raccourci, le démarrage Windows, les réglages et le dossier de sortie ; l'utilisateur a confirmé leur inclusion en V1 dans le présent chat.

## Messages utilisateur fondant le périmètre

| Source chat / turn | Décision attestée et statut |
|---|---|
| `01a108b7-9f64-7671-bd72-21db5f9b42d6 / 01a109ed-f4a8-73d0-a958-e254e13519b0` | « inclure également la transcription de fichier wave », usage « personnel », « texte progressif » : accepté. |
| `01a10a6a-418e-7cd2-ab69-162ffe16db34 / 01a10a87-5cfb-7212-8f82-dc340b519038` | Capture en arrière-plan ; live refusé pendant import ; anglais/français et visée temps réel ; WAV source conservé à son emplacement ; réponses libres acceptées. |
| même chat / `01a10aaa-5794-79f3-b4d5-e6e16e7db853` | Stop traite l'audio capté ; Stop + Reprise ; TXT/SRT et audio dans un dossier de transcription configurable ; récupérer après coupure ; langue détectée ou forcée ; MP3 d'archive ; cible GTX 1080 Ti : accepté. L'interprétation initiale des 60 s est **remplacée** par le message plus récent ci-dessous. |
| même chat / `01a10ab8-7ded-72b0-9635-8e8b9c045fd8` | Deux heures CPU/GPU **indicatives** ; réglage ultérieur, réponse ultérieure précisant option d'arrêt ou sans limite ; langue manuelle fixée avant capture ; MP3 d'archive uniquement ; nouveau passage déclenché par utilisateur. |
| même chat / `01a10acc-bda9-7863-98d6-029a7a27a990` | Option de durée entraînant arrêt automatique ou enregistrement sans limite ; les deux heures ne sont pas un plafond obligatoire. |
| même chat / `01a10add-6bb9-7de3-9e9f-bc6d84c1b5dc` | Après environ 60 s sans progrès visible : avertir, diagnostiquer, arrêter et alerter seulement si panne avérée ; silence VAD exclu du compteur parlé, temps total aussi affichable ; conservation manuelle et historique fondé sur fichiers ; langue manuelle prioritaire pour import ; modèle téléchargé à l'installation ; Auto GPU→CPU avec signalement. |
| même chat / `01a10ae5-80fa-7653-a243-6082a8f0e1f3` | GPU forcé absent/incompatible : arrêter, signaler et proposer CPU sans bascule automatique. |
| `01a10aea-a9da-7811-a779-cd0cf5dbe42f / 01a10aed-808c-7053-b4fa-4719daa2be3b` | Modèle pendant installation ; aucun plafond de latence live accepté, mais délai mesuré ; import pendant live en attente ; silences détectés exclus du compteur ; MP3 garde les silences. |
| même chat / `01a10afc-61be-7cf0-8d0b-559475feea4c` | Fragments confirmés conservés après coupure, perte du fragment/file ouverte à mesurer ; fermer fenêtre masque et continue ; Quitter en live finalise ; Quitter en import annule et préserve source ; échec de finalisation conserve récupérable et signale. |
| même chat / `01a10b08-01a2-7371-99d5-ac1cb5f846f1` | Essais CPU/GPU Python rapportés, GPU observé manuellement ; **ne prouve pas** la pile Rust proposée. |
| même chat / `01a10b10-7845-7ca0-9626-704e3c574bf8` | Fermer pendant import masque et continue ; live pendant import refusé ; imports non commencés conservés après crash et Quitter, proposés Traiter/Retirer au retour. |
| présent chat, réponse du 2026-10-05 | Démarrage Windows, réglages, raccourci global et choix du dossier sont tous **inclus en V1**. L'utilisateur signale que les réponses originales sont dans un chat précédent ; les références ci-dessus les localisent. |
| présent chat, réponses suivantes du 2026-10-05 | Plusieurs imports : ordre de demande et confirmation avant reprise d'un import interrompu par crash ; Supprimer dans l'historique efface les archives MP3/TXT/SRT après confirmation, sans toucher les sources importées ; Quitter bloqué reste ouvert jusqu'à résolution ou action manuelle ; logs techniques sans audio ni texte transcrit. **Accepté.** |
| présent chat, réponse ultérieure du 2026-10-05 | Stop + Reprise : **un dossier, MP3 par passage et TXT/SRT cumulés** pour la même transcription. **Accepté.** |
| présent chat, réponse ultérieure du 2026-10-05 | Seuils de VAD, perte de fragment non confirmé et délai du texte live : **mesurer sur corpus, puis décider après essais**. Aucun seuil n’est accepté à ce stade. |

| présent chat, message `01a10bae-9b2c-78d2-944c-5e72d703271c` | Test sur PC propre hors scope pour l’instant ; réaliser les mesures VAD, perte et délai live. Changement traité dans `22_CHANGE-001.md`. |

## État des apports spécialisés

Le corpus D-WHISPER-01 et son reçu ne contiennent pas les résultats bruts des spécialistes ni de checkpoint. Ce manque historique n'est pas maquillé : les rôles d'auteur passés ne sont pas authentifiés par le reçu. D-WHISPER-09 est une correction documentaire faite dans le présent chat sous la responsabilité de l'auteur de cette correction ; l'indépendance du reviewer s'applique uniquement à sa revue. Les propositions précédentes restent consultables dans les chats ci-dessus, sous réserve des limites de `read_thread`. Les choix techniques ci-dessous sont proposés par l'auteur de la correction et exigent une revue indépendante.

## Règles effectives et état du dépôt

Référentiel consulté : `C:\Users\cyril\.codex\skills\rust-predev-design\SKILL.md`, `references/handoff-contract.md`, `deliverable-quality.md`, `engineering-contract.md`, `workflow-schema.md`, `runtime-qualification.md`, les cinq fichiers sous `C:\Users\cyril\.codex\agent-rules`, et `rust-predev-orchestration/SKILL.md`. Le freeze exact et les copies d'octets sont `15_PACK_MANIFEST.json`, `16_SNAPSHOT_INDEX.json` et `rules/` dans ce candidat ; `13_INITIAL_PROPOSAL.txt`, `14_USER_MESSAGES.json`, `17_AUTHORIZATION.md` et `18_REVIEW_RAW.json`, `20_REVIEW_RAW_D3.json`, `23_REVIEW_RAW_D4.json` et `24_REVIEW_RAW_D5.json` et `27_REVIEW_RAW_D6.json`, `28_REVIEW_RAW_D7.json` et `31_REVIEW_RAW_D8.json` conservent les autres sources et revues disponibles. Le résultat D3 est aussi dans le TRANSPORT `T-WHISPER-REVIEWS-03`, et l’état courant est contrôlé séparément sous `docs/predev/state.json`. Aucun `AGENTS.md`, `RAILGUARD.md`, `Cargo.toml` ni code Rust produit dans ce dépôt au début de la correction. Les tests du futur produit restent NOT RUN.
