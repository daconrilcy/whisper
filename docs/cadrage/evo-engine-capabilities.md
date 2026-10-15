Avancée : règle transversale de capacités, qualification et adéquation au PC cadrée pour les moteurs présents et futurs.
Blocage : besoins RAM/VRAM qualifiés absents ; la langue manuelle du couple Parakeet/NeMo courant échoue à l'exigence existante.
Décision : DESIGN DRAFT ; conserver les faits inconnus comme tels et refuser les choix dont l'incompatibilité est établie.

# Registre des capacités des moteurs — cadrage du 15 octobre 2026

**Base :** branche `evo-startup-perf`, document `docs/cadrage/evo-parakeet.md` à `74781f95`, arbre propre au préflight. **Portée :** règle du framework de l'application Rust, applicable à Turbo, Parakeet et à chaque moteur futur. **Autorité :** demande utilisateur du 15 octobre 2026 ; les structures et contrôles détaillés ci-dessous sont des propositions de design. Ce document ne constitue ni un plan accepté ni une autorisation de modifier le code, de mesurer la charge, d'installer ou de télécharger des modèles.

## 1. Décisions produit et frontières

| ID | Décision acquise et conséquence |
| --- | --- |
| DEC-CAP-01 | Le framework de l'application maintient un registre des capacités de tous les moteurs intégrés, y compris les futurs. Ce registre évolue avec le produit. |
| DEC-CAP-02 | Chaque entrée renseigne les langues prises en charge, la détection automatique de langue, la possibilité CPU/GPU et les besoins mémoire et disque. La qualification de ces données fait partie de l'intégration d'un moteur. |
| DEC-CAP-03 | L'interface désactive la langue automatique lorsqu'elle n'est pas prise en charge et grise toute langue non prise en charge afin qu'elle ne soit pas sélectionnable. Les mêmes refus s'appliquent à une demande qui contournerait le widget. |
| DEC-CAP-04 | L'application peut vérifier le PC et retirer la sélection exécutable d'une configuration dont l'insuffisance de ressources est établie, avec cause visible. |

Le registre décrit les **moteurs que le produit prend en charge** ; il ne crée ni plugin, ni découverte libre de fichiers, ni option de remplacement du modèle par l'utilisateur final. Il ne modifie pas l'identité du modèle choisi, le sens d'Auto calcul, ni les contrats d'arrêt et de conservation du son.

**Contrat existant conservé :** `DEC-PARA-16` exige actuellement détection automatique **et** langue manuelle réellement appliquée pour Turbo et Parakeet. La règle générale de grisement ne retire pas cette exigence particulière. Le couple GGUF Parakeet fourni + NeMo-Speech.cpp v0.2.0 reste en échec pour le manuel ; la fiche de capacités doit exposer ce fait, sans le faire passer pour une livraison conforme. L'arbitrage `Q-CAP-01` reste ouvert tant que l'utilisateur n'a pas explicitement révisé ou confirmé ce contrat.

## 2. Ce que le registre distingue

Une **configuration moteur** est identifiée par modèle, révision/format/quantification/empreinte de l'artefact, runtime et build, worker et version IPC, variante CPU ou GPU, architecture prise en charge et options pertinentes (fenêtre, précision, concurrence, langue). Une preuve acquise pour un autre fichier ou une autre variante n'est pas transférée sans justification.

| Dimension | Valeur à qualifier, sans déduction abusive |
| --- | --- |
| Langues reconnues | Codes canoniques et portée linguistique établie. Reconnaître une langue ne prouve pas qu'elle peut être imposée manuellement. |
| Langues imposables | Ensemble dont le choix manuel influence réellement la reconnaissance ; rejet explicite d'une valeur invalide ou non prise en charge. |
| Détection automatique | Capacité distincte des deux ensembles précédents et d'Auto **calcul**. Un champ `auto` accepté par une API ne suffit pas. |
| Calcul | CPU et GPU par runtime/build/backend et architecture/périphérique qualifiés. Inventaire GPU et capacité déclarée ne prouvent pas une inférence GPU effective. |
| Ressources | RAM hôte, VRAM du GPU visé, disque du modèle installé, temporaire d'acquisition et espace de travail nécessaire au profil. Valeur, unité, conditions, source et limite obligatoires lorsqu'un seuil est affirmé. |
| Preuve | Source primaire ou essai attribué, date, candidat exact, environnement, méthode, résultat et limites. Déclaration fournisseur, inspection du code, compilation, inférence et qualification du paquet restent des preuves distinctes. |

Chaque affirmation porte un statut `Supported`, `Unsupported`, `Unknown` ou `Partial` pour sa portée exacte, ainsi que la nature de la preuve. `Unknown` signifie absence de preuve suffisante ; il ne devient ni « zéro », ni « supporté », ni « non supporté ». `Partial` conserve les sous-conditions acquises et ouvertes. Une contradiction établit `Unsupported` ou un `FAIL` pour la combinaison concernée sans effacer l'historique des preuves.

La **capacité du moteur**, la **qualification de cette capacité**, la **présence et validité du fichier**, la **disponibilité du runtime**, les **ressources observées sur le PC** et le **calcul/langue effectivement appliqués à une séance** sont six faits distincts. L'interface affiche une conclusion contextuelle et ses causes ; elle ne reconstruit pas elle-même ces règles.

## 3. Règles d'admissibilité et interface

L'application évalue la combinaison demandée : modèle, langue, calcul, source import/micro et profil d'inférence. La préférence enregistrée demeure distincte de la sélection **exécutable maintenant**. Une indisponibilité du fichier, une capacité absente ou une pénurie temporaire ne réécrit pas la préférence et ne déclenche pas un autre modèle.

| Situation prouvée | Effet proposé et observable |
| --- | --- |
| Modèle absent, invalide, acquisition annulée | Nom conservé dans le catalogue, grisé et non sélectionnable ; raison et action de réparation adaptées. |
| Auto langue non pris en charge | Contrôle Auto langue désactivé avec raison ; aucun passage silencieux à une langue manuelle. Auto calcul reste indépendant. |
| Langue manuelle non prise en charge ou non imposable | Langue grisée/non sélectionnable pour cette configuration ; aucune demande manuelle n'est acceptée par l'application. Une langue reconnue automatiquement n'est pas présumée imposable. |
| Variante GPU incompatible, CPU admissible | Seule la variante GPU est refusée. GPU manuel ne bascule pas silencieusement ; Auto calcul peut choisir CPU du **même modèle** avant la première inférence selon son contrat. |
| Insuffisance RAM/VRAM/disque démontrée pour un profil | Configuration concernée désélectionnée ou lancement bloqué avec ressource et valeur applicables expliquées ; autre modèle ou calcul choisi seulement par l'utilisateur, sauf politique Auto calcul existante. |
| Vérification en cours, source absente ou observation ambiguë | État « vérification » ou « non qualifié/non vérifié » explicite ; ne pas afficher « mémoire insuffisante » ou « GPU incompatible » sans preuve. Politique de lancement de l'état indéterminé à fixer en `Q-CAP-02`. |

Le modèle ne doit pas être grisé globalement parce que **son GPU** échoue si sa voie CPU reste admissible ; les contrôles de calcul portent alors la cause. À l'inverse, aucun mode n'est sélectionnable si un blocage commun au modèle s'applique. Le catalogue et le refus applicatif avant séance utilisent **la même évaluation**. Les résultats de sondes tardifs sont liés à une génération et ne changent ni une séance figée ni une préférence plus récente.

L'interface actuelle possède un champ texte « Langue manuelle », sans liste de langues. Le besoin nouveau de langues grisées implique une liste visible des langues du registre ; le sort de la saisie libre existante est demandé en `Q-CAP-04`. Dans tous les cas, une valeur saisie ou injectée reste validée côté application contre la capacité effective, même si le widget la montre.

Auto calcul n'est pas un substitut de modèle. Il examine, pour le modèle et la langue retenus, GPU puis CPU selon la politique acceptée ; il n'utilise pas une combinaison non qualifiée comme si elle était attestée. La désactivation d'Auto langue ne change pas cette politique.

## 4. Ressources et contrôle du PC

**Aucun minimum RAM/VRAM n'est établi pour Parakeet par la taille de son GGUF ni par le build partiel.** Un fichier disque de 713 975 456 octets (constat local du 15 octobre) ne décrit pas la mémoire requise. Le fichier Turbo actuellement présent mesure 1 624 555 275 octets ; ces tailles sont des observations de stockage, pas des besoins d'exécution ni des engagements de distribution.

La fiche distingue `ArtifactBytes`, exigence documentée applicable, borne d'allocation dérivée et démontrée, observation d'une exécution réussie pour un profil identifié, ou `Unknown`. Une courte exécution réussie ne prouve pas un minimum universel ni un maximum pour tous les fichiers audio. Le backlog micro peut croître avec la durée de capture ; il n'est pas ramené fictivement à la taille du modèle.

Le contrôle hôte proposé lit, hors thread UI et sans charger un modèle : RAM totale et disponible, espace accessible à l'utilisateur sur les volumes réellement concernés, identité du GPU et informations mémoire pertinentes pour le **worker** cible. L'observation porte heure, processus/périphérique, portée et erreur éventuelle. Sur Windows, [`GlobalMemoryStatusEx`](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-globalmemorystatusex) expose une disponibilité volatile et [`GetDiskFreeSpaceExW`](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getdiskfreespaceexw) l'espace accessible à l'utilisateur. Le [budget vidéo DXGI](https://learn.microsoft.com/en-us/windows/win32/api/dxgi1_4/nf-dxgi1_4-idxgiadapter3-queryvideomemoryinfo) est rapporté au processus interrogé : le bureau ne peut pas en déduire directement une garantie VRAM pour le worker.

Une comparaison n'autorise `Insufficient` que si besoin et observation portent même unité, volume, périphérique, variante et conditions. Une valeur absente donne `Uncertain`, pas `Insufficient`. Le contrôle préalable est revalidé avant préparation ; il ne réserve ni RAM, ni VRAM, ni disque. Si l'allocation échoue malgré un contrôle favorable, l'erreur réelle est annoncée et les garanties existantes sur l'audio déjà admis et les sorties incomplètes s'appliquent. Aucun contrôle de charge continu ni campagne de vitesse n'est ajouté.

## 5. Frontières Rust proposées et qualification d'un nouveau moteur

| Frontière | Responsabilité |
| --- | --- |
| Domaine pur `whisper-core` | Types d'identité, capacité, unité, statut, préférence et motifs d'admissibilité ; aucune lecture Windows ou de fichier. |
| Application `whisper-core` | Jointure du registre, de l'état des artefacts, du runtime et du PC ; projection des options et validation avant séance. Ports possédés : `PORT-CAPABILITIES`, `PORT-HOST`, `PORT-ELIGIBILITY`. |
| Adaptateurs `whisper-adapters` | Vérification des artefacts, lecture Windows RAM/disque/GPU et erreurs de mesure typées ; aucune décision métier parallèle. |
| Workers | Identité du runtime/build effectivement lié, résultat de préparation et attestation modèle/calcul réellement utilisés. |
| UI/composition | Injection du registre correspondant à la release et affichage de la projection applicative ; aucune liste de langues, seuil ou compatibilité GPU codés séparément dans l'UI. |

**Gate d'intégration proposé :** une nouvelle combinaison moteur/modèle/révision/runtime/backend reçoit sa fiche versionnée et ses preuves avant d'être annoncée utilisable. Le responsable qualifie distinctement langues reconnues, manuel réel, Auto langue, CPU, GPU, besoins applicables et parcours import/micro ; il note les branches NOT RUN/PARTIAL/FAIL. Un changement d'artefact, de runtime, de worker/IPC, de build, de profil ou de matériel réévalue les preuves dépendantes ; les anciennes restent traçables, sans héritage automatique. Un résultat constructeur n'est pas la validation du raccordement dans l'application.

**État initial illustratif, non promesse de livraison :** Turbo exige une fiche propre à sa version actuelle, dont les valeurs ressources et la couverture linguistique restent à qualifier dans ce nouveau contrat. Pour Parakeet GGUF local + NeMo v0.2.0, la langue manuelle est `Unsupported` pour la voie de conditionnement inspectée (`num_prompts=0`) ; CPU/GPU réels, fenêtres micro et ressources d'exécution restent NOT RUN/Unknown. La fiche peut être honnête sans rendre le produit conforme à l'exigence particulière `DEC-PARA-16`. Voir `docs/cadrage/evo-parakeet.md` pour les preuves et le constat indépendant.

## 6. Exigences et acceptation futures

Les liens de décision sont : `DEC-CAP-01 → CAP-31/34`, `DEC-CAP-02 → CAP-31/32/34`, `DEC-CAP-03 → CAP-33` et `DEC-CAP-04 → CAP-32`. Cette matrice sert à contrôler aussi les moteurs futurs ; elle n'attribue aucune capacité qualifiée aux deux moteurs actuels.

| IDs proposés | Critère observable, état actuel |
| --- | --- |
| REQ/UC/AC-CAP-31 | Chaque moteur intégré a une fiche liée à son candidat exact, avec support et qualification distincts ; une autre révision n'hérite pas d'une preuve non justifiée. Tester CPU qualifié/GPU inconnu, manuel absent/Auto présent et valeur RAM inconnue. NOT RUN. |
| REQ/UC/AC-CAP-32 | Une insuffisance RAM/VRAM/disque établie retire uniquement la combinaison concernée, garde la préférence, explique la cause ; une mesure absente est indéterminée, pas insuffisante. Résultat tardif périmé ignoré. NOT RUN. |
| REQ/UC/AC-CAP-33 | Changer de modèle met à jour les langues et calculs disponibles ; Auto langue et langue manuelle non pris en charge sont grisés ; une demande qui contourne le widget est refusée. Auto calcul reste indépendant. NOT RUN. |
| REQ/UC/AC-CAP-34 | Intégrer une nouvelle révision avec déclaration contredite ou GPU non testé maintient la contradiction et les lacunes visibles ; aucun PASS global. Les besoins chiffrés sans source/conditions ne servent pas à désélectionner. NOT RUN. |

**Contrôles transversaux :** modèle absent puis réparé ; GPU insuffisant mais CPU du même modèle admissible ; valeur mémoire inconnue ; langue non prise en charge ; ancienne préférence ; preuve devenue périmée ; échec d'allocation après contrôle favorable. Les essais Windows réels, seuils et corpus de langues restent à définir avant validation. Aucune preuve n'est produite par cette rédaction.

## 7. Questions et état

| ID / classe | Décision ou fermeture attendue |
| --- | --- |
| Q-CAP-01 / BLOCKING produit Parakeet | La règle générique ne révise pas `DEC-PARA-16`. Conserver manuel+Auto obligatoires pour Turbo/Parakeet maintient le blocage jusqu'à une autre voie Parakeet ; accepter Parakeet Auto seul réviserait explicitement le périmètre. Réponse utilisateur demandée, non présumée. |
| Q-CAP-02 / STRUCTURING admission | Une capacité obligatoire `Unknown/Partial` ne peut pas être affichée comme supportée. Avant le lot, préciser si une tentative contrôlée est autorisée pour lever l'incertitude ou si seule une combinaison qualifiée peut être lancée dans l'application normale. Recommandation : qualifier d'abord, ne pas promettre une configuration non établie. |
| Q-CAP-03 / STRUCTURING ressources | Architecte : établir, par profil et par preuve recevable, besoins et observations comparables, fraîcheur et marges justifiées, puis détailler l'effet d'une pénurie temporaire. Aucune valeur ou campagne de charge n'est autorisée par ce document. |
| Q-CAP-04 / STRUCTURING UI | L'UI actuelle saisit librement la langue ; le nouvel affichage de langues grisées demande une liste. Conserver saisie et ajouter liste, ou remplacer la saisie par la liste : réponse utilisateur demandée. Les deux doivent refuser les valeurs non supportées dans l'application. |

**Revue et plan :** DESIGN DRAFT. Le constat `PARA-REV-01` et le FAIL linguistique restent ouverts dans le cadrage Parakeet. Après revue indépendante de ce document et décisions nécessaires, le plan Rust peut découper le registre, ses contrôles et la qualification des moteurs en lots bornés. L'implémentation du code, une campagne de mesure ou un téléchargement demandent toujours le plan accepté, les chemins/lot et l'accord précis applicables.

**Attribution et revue :** `rust_product_framer` (PRODUCT-PARA v0.5), `rust_requirements_analyst` (REQ-PARA v0.5), `rust_domain_architect` (DOM-PARA v0.6) et `rust_architect` (ARCH-PARA v0.8) ont contribué en lecture seule ; la session principale a rédigé et versionné. Le reviewer indépendant a examiné le commit `50b8dfdf` et n'a relevé aucun nouveau finding documentaire REQUIRED ; verdict **BLOCKED pour DESIGN READY** en raison de `Q-CAP-01–04`, des ressources non qualifiées et de `PARA-REV-01`. Les AC restent **NOT RUN**. Ce constat ne clôt aucune question en attente ni ne qualifie le produit dans la fenêtre Windows.
