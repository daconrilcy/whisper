Avancée : règle transversale de capacités, qualification et adéquation au PC cadrée pour les moteurs présents et futurs.
Blocage : besoins RAM/VRAM qualifiés absents ; la détection automatique du couple Parakeet/NeMo courant reste à qualifier avant de proposer Auto seul.
Décision : DESIGN DRAFT ; liste de langues et modes conditionnés par les capacités, sans choix silencieux.

# Registre des capacités des moteurs — cadrage du 15 octobre 2026

**Base :** branche `evo-startup-perf`, document `docs/cadrage/evo-parakeet.md` à `74781f95`, arbre propre au préflight. **Portée :** règle du framework de l'application Rust, applicable à Turbo, Parakeet et à chaque moteur futur. **Autorité :** demande utilisateur du 15 octobre 2026 ; les structures et contrôles détaillés ci-dessous sont des propositions de design. Ce document ne constitue ni un plan accepté ni une autorisation de modifier le code, de mesurer la charge, d'installer ou de télécharger des modèles.

## 1. Décisions produit et frontières

| ID | Décision acquise et conséquence |
| --- | --- |
| DEC-CAP-01 | Le framework de l'application maintient un registre des capacités de tous les moteurs intégrés, y compris les futurs. Ce registre évolue avec le produit. |
| DEC-CAP-02 | Chaque entrée renseigne les langues prises en charge, la détection automatique de langue, la possibilité CPU/GPU et les besoins mémoire et disque. La qualification de ces données fait partie de l'intégration d'un moteur. |
| DEC-CAP-03 | L'interface désactive la langue automatique lorsqu'elle n'est pas prise en charge et grise toute langue non prise en charge afin qu'elle ne soit pas sélectionnable. Les mêmes refus s'appliquent à une demande qui contournerait le widget. |
| DEC-CAP-04 | L'application peut vérifier le PC et retirer la sélection exécutable d'une configuration dont l'insuffisance de ressources est établie, avec cause visible. |
| DEC-CAP-05 | Pour tout moteur sans détection automatique de langue prise en charge, Auto langue reste grisé/non sélectionnable ; une langue manuelle admissible doit être choisie avant transcription. |
| DEC-CAP-06 | La liste de langues remplace le champ de saisie libre ; les langues non prises en charge sont visibles grisées et non sélectionnables. |

Le registre décrit les **moteurs que le produit prend en charge** ; il ne crée ni plugin, ni découverte libre de fichiers, ni option de remplacement du modèle par l'utilisateur final. Il ne modifie pas l'identité du modèle choisi, le sens d'Auto calcul, ni les contrats d'arrêt et de conservation du son.

**Changement de contrat :** la réponse utilisateur à `Q-CAP-01` remplace l'obligation antérieure `DEC-PARA-16` d'offrir **les deux** modes pour Turbo et Parakeet par la règle de capacités commune. Un mode proposé doit toujours être réellement appliqué. Le couple GGUF Parakeet fourni + NeMo-Speech.cpp v0.2.0 reste en échec pour le **manuel** ; ce fait permet de griser ce mode, sans prouver que la détection automatique du couple intégré est qualifiée. Un moteur sans aucun mode linguistique admissible ne peut pas démarrer.

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
| Auto langue non pris en charge ou non qualifié pour le candidat | Contrôle Auto langue grisé avec la raison exacte (`Unsupported` ou `Unknown/Partial`) ; choix explicite obligatoire d'une langue manuelle admissible avant démarrage. Aucun choix automatique de la première langue. Auto calcul reste indépendant. L'inconnu n'est pas déclaré techniquement impossible. |
| Langue manuelle non prise en charge ou non imposable | Langue grisée/non sélectionnable pour cette configuration ; aucune demande manuelle n'est acceptée par l'application. Une langue reconnue automatiquement n'est pas présumée imposable. Auto langue reste utilisable uniquement si cette capacité est qualifiée et admissible. |
| Aucun mode linguistique admissible | Démarrage refusé avec cause ; présence du fichier modèle et aptitude linguistique restent distinguées. |
| Variante GPU incompatible, CPU admissible | Seule la variante GPU est refusée. GPU manuel ne bascule pas silencieusement ; Auto calcul peut choisir CPU du **même modèle** avant la première inférence selon son contrat. |
| Insuffisance RAM/VRAM/disque démontrée pour un profil | Configuration concernée désélectionnée ou lancement bloqué avec ressource et valeur applicables expliquées ; autre modèle ou calcul choisi seulement par l'utilisateur, sauf politique Auto calcul existante. |
| Vérification en cours, source absente ou observation ambiguë | État « vérification » ou « non qualifié/non vérifié » explicite ; ne pas afficher « mémoire insuffisante » ou « GPU incompatible » sans preuve. Politique de lancement de l'état indéterminé à fixer en `Q-CAP-02`. |

Le modèle ne doit pas être grisé globalement parce que **son GPU** échoue si sa voie CPU reste admissible ; les contrôles de calcul portent alors la cause. À l'inverse, aucun mode n'est sélectionnable si un blocage commun au modèle s'applique. Le catalogue et le refus applicatif avant séance utilisent **la même évaluation**. Les résultats de sondes tardifs sont liés à une génération et ne changent ni une séance figée ni une préférence plus récente.

Le champ texte actuel « Langue manuelle » est remplacé par une liste issue du registre, avec codes canoniques et raisons de désactivation. L'UI et l'admission applicative utilisent la même projection des capacités. Une ancienne valeur libre effectivement rencontrée n'est conservée comme choix exécutable que si sa correspondance avec une langue admissible est univoque ; sinon un nouveau choix est demandé. La liste ne crée pas à elle seule une préférence persistée. Une valeur injectée ou périmée est refusée côté application, même si le widget a montré un ancien état.

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

**PORT-LANGUAGE-OPTIONS proposé :** l'application projette, depuis la configuration exacte et la qualification courante, l'état et la cause d'Auto langue, la liste des codes canoniques connus avec leur caractère manuellement imposable, et l'éventuelle sélection encore exécutable. Avant chaque séance, elle valide `Automatique` ou `Manuelle(code)` contre cette même projection, puis fige le choix transmis au worker. Une préférence ancienne, un changement de modèle ou une sonde tardive ne sélectionne ni le français, ni la première langue, ni Auto en remplacement silencieux. Le statut `Unknown/Partial` demeure distinct de `Unsupported` ; sa politique d'admission reste `Q-CAP-02`.

**Gate d'intégration proposé :** une nouvelle combinaison moteur/modèle/révision/runtime/backend reçoit sa fiche versionnée et ses preuves avant d'être annoncée utilisable. Le responsable qualifie distinctement langues reconnues, manuel réel, Auto langue, CPU, GPU, besoins applicables et parcours import/micro ; il note les branches NOT RUN/PARTIAL/FAIL. Un changement d'artefact, de runtime, de worker/IPC, de build, de profil ou de matériel réévalue les preuves dépendantes ; les anciennes restent traçables, sans héritage automatique. Un résultat constructeur n'est pas la validation du raccordement dans l'application.

**État initial illustratif, non promesse de livraison :** Turbo exige une fiche propre à sa version actuelle, dont les valeurs ressources et la couverture linguistique restent à qualifier dans ce nouveau contrat. Pour Parakeet GGUF local + NeMo v0.2.0, la langue manuelle est `Unsupported` pour la voie de conditionnement inspectée (`num_prompts=0`) ; Auto langue du candidat intégré, CPU/GPU réels, fenêtres micro et ressources d'exécution restent NOT RUN/Unknown. La fiche NVIDIA déclare pourtant une détection automatique du modèle ; cette déclaration ne qualifie pas l'intégration présente. Conformément à `DEC-PARA-18`, Auto langue est **grisé/non sélectionnable** pour ce candidat, et, faute de manuel admissible, le modèle reste visible mais **non lançable** tant qu'aucun mode n'est qualifié. Le FAIL manuel demeure une preuve technique ; l'état Auto n'est pas étiqueté `Unsupported` sans preuve d'absence. Voir `docs/cadrage/evo-parakeet.md` pour les preuves et le constat indépendant.

## 6. Exigences et acceptation futures

Les liens de décision sont : `DEC-CAP-01 → CAP-31/34`, `DEC-CAP-02 → CAP-31/32/34`, `DEC-CAP-03/05/06 → CAP-33` et `DEC-CAP-04 → CAP-32`. Cette matrice sert à contrôler aussi les moteurs futurs ; elle n'attribue aucune capacité qualifiée aux deux moteurs actuels.

| IDs proposés | Critère observable, état actuel |
| --- | --- |
| REQ/UC/AC-CAP-31 | Chaque moteur intégré a une fiche liée à son candidat exact, avec support et qualification distincts ; une autre révision n'hérite pas d'une preuve non justifiée. Tester CPU qualifié/GPU inconnu, manuel absent/Auto présent et valeur RAM inconnue. NOT RUN. |
| REQ/UC/AC-CAP-32 | Une insuffisance RAM/VRAM/disque établie retire uniquement la combinaison concernée, garde la préférence, explique la cause ; une mesure absente est indéterminée, pas insuffisante. Résultat tardif périmé ignoré. NOT RUN. |
| REQ/UC/AC-CAP-33 | La liste remplace la saisie libre. Changer de modèle met à jour Auto langue et les langues manuellement imposables ; les options non admissibles sont grisées avec cause. Si Auto langue est absent ou non qualifié, une langue manuelle admissible doit être choisie explicitement ; si le manuel est absent, Auto n'est offert que s'il est qualifié ; sans mode admissible, la séance est refusée. Une demande qui contourne le widget ou reprend un choix périmé est refusée. Auto calcul reste indépendant. NOT RUN. |
| REQ/UC/AC-CAP-34 | Intégrer une nouvelle révision avec déclaration contredite ou GPU non testé maintient la contradiction et les lacunes visibles ; aucun PASS global. Les besoins chiffrés sans source/conditions ne servent pas à désélectionner. NOT RUN. |

**Contrôles transversaux :** modèle absent puis réparé ; GPU insuffisant mais CPU du même modèle admissible ; valeur mémoire inconnue ; langue non prise en charge ; ancienne préférence ; preuve devenue périmée ; échec d'allocation après contrôle favorable. Pour CAP-33, vérifier séparément Auto absent + manuel admissible, Auto qualifié + manuel absent, aucun mode admissible, langue grisée injectée, changement de modèle et ancienne valeur libre ambiguë. Les essais Windows réels, seuils et corpus de langues restent à définir avant validation. Aucune preuve n'est produite par cette rédaction.

## 7. Questions et état

| ID / classe | Décision ou fermeture attendue |
| --- | --- |
| Q-CAP-01 / CLOSED produit | Réponse utilisateur du 15 octobre : règle commune des capacités ; sans détection automatique de langue, Auto est grisé et une langue manuelle admissible doit être renseignée. L'obligation antérieure d'offrir les deux modes à chaque moteur est remplacée ; chaque mode offert reste réellement appliqué. Cela ne qualifie pas Auto Parakeet. |
| Q-CAP-02 / STRUCTURING admission | Une capacité obligatoire `Unknown/Partial` ne peut pas être affichée comme supportée. Avant le lot, préciser si une tentative contrôlée est autorisée pour lever l'incertitude ou si seule une combinaison qualifiée peut être lancée dans l'application normale. Recommandation : qualifier d'abord, ne pas promettre une configuration non établie. |
| Q-CAP-03 / STRUCTURING ressources | Architecte : établir, par profil et par preuve recevable, besoins et observations comparables, fraîcheur et marges justifiées, puis détailler l'effet d'une pénurie temporaire. Aucune valeur ou campagne de charge n'est autorisée par ce document. |
| Q-CAP-04 / CLOSED produit | Réponse utilisateur du 15 octobre : remplacer la saisie libre par la liste ; chaque langue non admissible est grisée et l'application refuse les valeurs invalides même hors widget. |

**Revue et plan :** DESIGN DRAFT. Le constat `PARA-REV-01` reste ouvert dans le cadrage Parakeet. Le FAIL manuel est conservé comme preuve ; Auto Parakeet reste non qualifié. Après revue indépendante de ce document et décisions nécessaires, le plan Rust peut découper le registre, ses contrôles et la qualification des moteurs en lots bornés. L'implémentation du code, une campagne de mesure ou un téléchargement demandent toujours le plan accepté, les chemins/lot et l'accord précis applicables.

**Attribution et revue :** `rust_product_framer` (PRODUCT-PARA v0.6), `rust_requirements_analyst` (REQ-PARA v0.6), `rust_domain_architect` (DOM-PARA v0.7) et `rust_architect` (ARCH-PARA v0.9) ont contribué en lecture seule ; la session principale a rédigé et versionné. La revue indépendante de `50b8dfdf` a rendu BLOCKED pour DESIGN READY dans l'état antérieur des questions. Sur le delta `e9bc2415`, le même reviewer conclut **CLEAN documentaire**, sans nouveau constat ; `Q-CAP-01/04` sont fermées côté produit. `Q-CAP-02/03`, les ressources et capacités non qualifiées ainsi que `PARA-REV-01` maintiennent **DESIGN BLOCKED**. Les AC restent **NOT RUN** ; la revue ne qualifie pas le produit dans la fenêtre Windows.
