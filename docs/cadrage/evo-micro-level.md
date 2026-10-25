Avancée : SPECTRE v2 implémenté, delta relu CLEAN après deux cycles correctifs et EXE personnel remis le 26 octobre 2026.
Blocage : recette Windows à confirmer ; réserve Parakeet CPU, deux tests matériels ignorés et formatage global antérieur conservés.
Décision : remettre le nouvel EXE pour observer les deux spectres en direct pendant le test ET la prise de son.

# Whisper — Réglage du niveau du microphone

## Référence et autorité

- Date : 25 octobre 2026. Base produit inspectée : `878cd1e9a3a37bd062af9802bec68f2eb45f5337`.
- Auteur des fichiers : session Codex principale. Contributions en lecture seule : `rust_product_framer`, `rust_requirements_analyst`, `rust_domain_architect`, `rust_architect` (v1.1), `rust_plan_writer` (v1.1), `rust_design_reviewer`.
- Besoin source : « Whisper — Réglage du niveau du microphone », CA-01 à CA-07, fourni dans cette conversation.
- Acceptation produit dans cette session : **« ok sur les recommandation Q1 à Q5 »**. La réponse suivante a confirmé les décisions ci-dessous et maintenu le corpus précis et les tolérances avant les vérifications concernées.
- Mission documentaire suivante : **« tu peux passer à la rédaction du plan »**. Elle autorise la rédaction dans `docs/cadrage/` et les commits locaux prévus par AGENTS.md ; aucun code produit, test, compilation, capture, installation, téléchargement ou push n'en découle.
- La revue conversationnelle antérieure était CLEAN pour une proposition DRAFT. La revue indépendante du fichier Git par `rust_design_reviewer` a ensuite conclu **DESIGN CLEAN sur `d10516808804f461ca1597b78d0d176f3f3baa27`**. Le constat R-MIC-D01 (identité avant ouverture) est fermé. Le conseil de fixture spectrale est précisé ci-dessous ; les versions et la revue PLANS sont suivies dans le plan associé.

## Objectif, périmètre et décisions acceptées

Permettre à l'utilisateur Windows de régler le niveau du microphone dans Whisper avant et pendant la capture, avec une jauge simple, une sonde réutilisable et un niveau mémorisé par identité. Les deux parcours microphone présents, Turbo et Parakeet, sont concernés. Aucun nouveau moteur ni modèle n'est demandé.

| ID | Décision acceptée | Effet et limite |
| --- | --- | --- |
| Q1 | Un seul WAV réglé, issu du même PCM que le traitement microphone | Aucun second WAV brut. Une saturation créée par amplification peut être irréversible. |
| Q2 | Bouton explicite « Tester le microphone », sans moteur, fichier ni retour haut-parleur | Aucun démarrage implicite de mesure à l'ouverture de l'application. Le prétest ne fait pas partie du WAV. |
| Q3 | Gain de -24 à +24 dB, pas de 1 dB ; neutre et premier usage à 0 dB | Changement abrupt au prochain bloc acquis, sans rampe ; confirmer la frontière appliquée. Aucun délai mural garanti avant mesure. |
| Q4 | Sonde mono au taux natif ; A avant gain, B après gain et limitation, avant rééchantillonnage | RMS 100 ms, jusqu'à 20 publications/s ; spectre Hann 2048 échantillons jusqu'à 10 publications/s ; péremption après 500 ms. Pas de diagnostic séparé des canaux. |
| Q5 | Comparaison de référence à 0 dB, puis voix faible/forte et changements de gain sur Turbo et Parakeet | Vérifier continuité et absence de régression ; corpus concret et règle détaillée de comparaison à arrêter avant la qualification concernée. Aucun seuil WER nouveau accepté. |

Périmètre initial : modification du volume Windows, gain automatique, réglage des imports et visualisations autres que la jauge simple exclus. La demande distincte du 25 octobre 2026 et sa précision suivante ouvrent l'affichage spectral pendant le test ET la capture microphone, selon l'extension ci-dessous, encore proposée. Diagnostic par canal, retour audio, reprise après panne et autres visualisations restent différés. Les garanties de publication existantes ne deviennent pas une transaction atomique TXT/SRT/WAV.

## Observations en lecture seule

Sources relatives à la racine du dépôt ; aucune exécution du produit ne les accompagne.

| Source | Observation |
| --- | --- |
| `crates/whisper-core/src/microphone.rs` | Préférence et choix effectif distincts ; préférence explicite absente conservée au repli de démarrage. |
| `crates/whisper-adapters/src/capture.rs` | `start_direct` résout l'identité WASAPI réellement sélectionnée ; `CaptureStream` ne la restitue pas. Deux files bornées de 100 blocs ; conversion commune par moyenne des canaux, limitation, rééchantillonnage 16 kHz et PCM16. |
| `crates/whisper-adapters/src/direct_live.rs`, `parakeet_live.rs` | Les deux parcours et leurs drains d'arrêt utilisent le convertisseur commun et écrivent un WAV. |
| `crates/whisper-core/src/path_settings.rs`, `crates/whisper-adapters/src/path_settings.rs` | Choix microphone déjà mémorisé ; deux producteurs de sauvegarde sous verrou préservent les propriétés non ciblées. Le lecteur des préférences modèle accepte les versions 1/2. |
| `crates/whisper-desktop/src/ui.rs` | Sélection microphone existante ; aucun réglage de gain ou sonde trouvé dans les composants ciblés. |
| `Cargo.toml`, `crates/whisper-adapters/Cargo.toml`, `Cargo.lock` | Rust édition 2024, version déclarée 1.98.1 ; CPAL 0.18.2, Rubato 0.16.2, rtrb 0.4.0. Rustfft 6.4.1 verrouillé transitivement, non déclaré directement. |

Le nom `CAPTURE_SATURATED` désigne un tampon plein, pas l'écrêtage du signal. La disponibilité réelle des périphériques, la charge, la continuité et la qualité sonore ne sont pas prouvées par cette lecture.

## Exigences, parcours et acceptation

REQ-M01 à M07, UC-M01 à M07 et AC-M01 à M07 sont distincts et correspondent respectivement à CA-01 à CA-07. Les tests ci-dessous sont futurs : **NOT RUN**.

| CA / REQ / UC / AC | Parcours et résultat observable | Responsabilité / preuve future |
| --- | --- | --- |
| 01 / M01 | Modifier le niveau sur un signal connu transforme le son Whisper et la valeur affichée ; le volume Windows reste identique. | Gain pur et capture ; comparaison numérique et vérification Windows utilisateur. |
| 02 / M02 | Régler avant capture et pendant un backlog : seuls les futurs échantillons changent ; aucun arrêt, trou, doublon ni changement de source provoqué par le réglage. | Révision par bloc d'acquisition ; test déterministe et capture réelle. |
| 03 / M03 | Silence puis niveaux variables ; après arrêt, déconnexion ou 500 ms sans mesure fraîche, l'ancienne valeur n'est plus actuelle. | Sonde/application/UI ; horloge simulée et fenêtre Windows. |
| 04 / M04 | Deux lecteurs exploitent RMS, pic, saturation et spectre A/B ; bloquer l'un ne bloque pas l'autre ou la capture. | Instantanés non exclusifs ; silence, sinus, multitone, écrêtage synthétique. |
| 05 / M05 | A et B réglés différemment retrouvent leurs gains après sélection et réouverture, y compris homonymes et changement du défaut Windows. | Identité effective et réglages ; fixtures de profil, puis deux entrées Windows. |
| 06 / M06 | A absent conserve identité/gain/préférence ; repli sur B avec gain de B et message. Reconnexion ne change pas une capture active ; disparition ne choisit aucun autre microphone. | Sélection/cycle de vie ; simulation et déconnexion manuelle. |
| 07 / M07 | Turbo et Parakeet préservent capture, vidange, durée et sorties ; imports et politiques CPU/GPU/Auto inchangés. | Intégration et régressions ; PCM neutre, WAV lisible, TXT/SRT utilisables et comparaison vocale Q5. |

### États visibles et transitions

- Repos : niveau mémorisé visible ; aucune mesure actuelle, aucun microphone ouvert implicitement.
- Ouverture du test : attente visible ; échec d'accès ou de source signalé sans conserver une ancienne jauge actuelle.
- Test actif : source réellement ouverte identifiable ; mesures actives sans moteur ni fichier. Arrêt/fermeture libèrent le flux et invalident les mesures.
- Démarrage depuis le test : arrêter le test puis ouvrir la capture. Vérifier l'identité attendue ; un défaut Windows différent provoque un message et exige une nouvelle action explicite. Aucune continuité du prétest n'est annoncée.
- Capture, y compris moteur en attente : source fixe, niveau modifiable ; distinguer niveau demandé et niveau appliqué. Changement de source refusé.
- Arrêt : plus de nouveaux échantillons admis ; niveau inactif pour cette session, mesures invalidées, drain/finalisation existants conservés.
- Erreur/déconnexion : cause visible et mesure indisponible ; aucun autre microphone ouvert automatiquement. Sorties partielles traitées selon le contrat existant.
- Fermeture normale : arrêt du mode actif et confirmation de sauvegarde ou erreur visible ; réouverture restaure choix/gains, jamais les mesures.

## Domaine et contrats techniques

### Identité et traitement

INV-01 : la préférence `WindowsDefault` ou identité explicite est distincte de l'identité du flux réellement ouvert. L'adaptateur restitue cette identité avec le format et la génération ; ne pas résoudre indépendamment le défaut pour deviner à qui attribuer un gain. Charger le gain avant de démarrer le flux. Une identité inconnue ne reçoit pas une préférence étrangère.

Complément de l'architecte sur le réglage au repos : le port microphone consulte en lecture seule l'identité WASAPI du défaut, sans construire/démarrer un flux (`default_input_device().id()` existe déjà dans le chemin d'ouverture). Avant ouverture, l'application présente la **source prévue**, son gain ou 0 dB et aucune mesure. La commande cible explicitement cette identité et la révision du contexte de sélection. Consultation impossible : pas de cible, réglage désactivé et cause visible. À l'ouverture, l'identité réellement ouverte fait autorité : charger son propre gain avant `play()`, actualiser source/niveau si le défaut a changé et ne transférer aucun gain de la source prévue. Le contrôle plus strict déjà accepté du passage test -> capture reste applicable. En repli d'une préférence absente, la commande règle la source effective identifiée, sans changer la préférence absente ni son gain. Commande sur un ancien contexte rejetée, jamais réattribuée. Aucun éditeur supplémentaire de source absente.

INV-02 : le gain est fini, borné et exprimé en dB ; facteur `10^(dB/20)`. Chaque commande est ciblée par session/génération/révision. La frontière est le prochain bloc acquis après publication de la commande ; les blocs portent une décision cohérente valeur/révision/index. Révisions anciennes ignorées, doublons idempotents. Un backlog garde son réglage historique, y compris au drain d'arrêt. L'application confirme le premier index concerné ; la présentation ne simule pas cette confirmation.

INV-03 : normalisation -> moyenne des canaux A -> gain historique -> comptage des dépassements -> limitation [-1,1] B -> conversion existante 16 kHz/PCM16. Le même PCM réglé alimente WAV et consommateurs de transcription. Conserver nombre, ordre et axe temporel ; ne pas retraiter les TXT/SRT déjà confirmés. À 0 dB, conserver le chemin numérique antérieur.

### Sonde et concurrence

INV-04 : les mesures A/B partagent la même plage audio et le même taux natif. B est l'entrée réglée du convertisseur, pas une promesse d'identité avec le PCM final. Distinguer indice de proximité des rails en A, dépassement créé par gain avant limitation, éventuelle limitation finale du convertisseur et tampon plein. Aucune mesure ne prouve l'écrêtage analogique amont.

INV-05 : instantané immutable avec identité, session/génération, séquence, plage d'échantillons, temps monotone d'acquisition, taux, nombre de canaux analysés et révision(s). Rejeter les anciennes sessions/identités et évaluer l'âge à l'acquisition, pas à la livraison. Silence valide distinct d'absence ; invalider immédiatement à l'arrêt/erreur. Fenêtre traversant un changement : déclarer les révisions concernées.

INV-06 : un propriétaire de flux, test ou capture ; aucun double accès ouvert par la jauge. Pas de FFT, sauvegarde ou appel UI dans le rappel audio. Traitement/stockage et consommateurs graphiques hors rappel. La sonde peut perdre des mises à jour et rendre cette lacune identifiable ; elle ne bloque jamais la voie audio ou un autre lecteur. Aucune perte audio ne devient silencieusement une capture complète.

RMS/pics : fenêtre glissante de `ceil(Fs * 0,100)` échantillons au taux natif ; RMS = racine de la moyenne des carrés, pic = maximum absolu ; dBFS par `20 log10(amplitude)` pour les valeurs positives, silence explicite à zéro. Conserver des puissances linéaires finies, sans NaN ou infini sérialisé.

Précisions techniques ARCH-v1.2 du `rust_architect`, 25 octobre 2026, dans Q4 : N=2048, Hann périodique `w[n]=0,5*(1-cos(2*pi*n/N))`, FFT non normalisée de `w*x`, calcul f64. PSD unilatérale `c[k]*|FFT(w*x)[k]|²/(Fs*sum(w²))`, c=1 pour DC/Nyquist et 2 ailleurs. Produire 1025 puissances avec fréquence `k*Fs/N`, unité FS²/Hz ; conversion éventuelle `10*log10(PSD)` en dBFS/Hz, référence 1 FS²/Hz. La somme PSD*Fs/N retrouve `sum((w*x)²)/sum(w²)`. Source primaire locale consultée par l'architecte : `rustfft-6.4.1/src/lib.rs`, API `FftPlanner::plan_fft_forward`, `process_with_scratch`, `get_inplace_scratch_len`. Plan et buffers réutilisés ; aucune FFT dans le rappel audio.

FFT sur 2048 échantillons contigus, sans remplissage artificiel de la première fenêtre. Les cadences 20/10 Hz sont des maxima de publication, distincts des durées de fenêtre. Fenêtre multirévision : `mixed_gain=true`, première/dernière révision et plage ; ne pas prétendre à un gain unique ou à un simple rapport spectral.

Bornes techniques : conserver les 100 blocs audio existants ; file de sonde séparée de quatre blocs mono A/B, chacun d'au plus 20 ms au taux natif (découper un bloc plus grand). Insertion sans attente ; débordement compte les mises à jour perdues, casse la continuité de la fenêtre de sonde et exige une fenêtre complète reconstruite. Calculateur : deux fenêtres FFT de 2048, deux fenêtres RMS bornées par Fs, buffers/scratch FFT dimensionnés à l'ouverture ; dernier instantané partagé sans historique ni file par lecteur. Une nouvelle commande peut remplacer une commande non encore acquise ; seule la révision réellement prise par la capture est confirmée. Accusé d'application et accusé de sauvegarde restent distincts.

Tolérances algorithmiques proposées par l'architecte pour la réalisation : A/B identiques au neutre hors limitation ; RMS/pic erreur absolue <=1e-6 FS ; conversion dB hors silence <=0,001 dB ; identité de Parseval erreur <=`1e-12 + 1e-6*référence` ; compteurs de dépassement, ordre, nombre et frontières d'index exacts. Pour la fixture sinus non saturée au bin 64 de N=2048, loin de DC/Nyquist : puissance intégrée amplitude²/2 et rapport pré/post gain². Pour les autres bins, l'oracle général est la puissance pondérée de Parseval ci-dessus ; ne pas généraliser amplitude²/2. Précision du conseil ADVISORY du reviewer avant T5. Ce sont des critères techniques, pas des seuils de qualité vocale acceptés implicitement.

### Persistance et erreurs

INV-07 : carte optionnelle identité -> gain dans le schéma v2 ; absence = 0 dB. Aucun rattachement par nom, aucune valeur globale du défaut Windows. Une entrée invalide est neutralisée seule avec message ; les autres restent intactes. Le choix préféré absent reste mémorisé et le repli emploie le gain de sa propre identité.

INV-08 : sauvegarde hors rappel avec révision/accusé ; une sauvegarde ancienne ne remplace pas la plus récente. Les deux producteurs existants préservent la carte et les autres propriétés. Erreur visible, gain actif conservé en mémoire sans annoncer une durabilité non confirmée. Réutiliser le contrat de remplacement existant ; aucune garantie nouvelle de reprise après panne.

### Responsabilités et dépendances

| Couche | Responsabilité | Dépendances et vérification future |
| --- | --- | --- |
| Domaine dans core | Gain, identité, validité, invariants purs | std et calcul pur ; aucun CPAL/IO/UI. Revue imports et tests ciblés. |
| Application dans core | Commandes, états et ports test/capture/mesures/réglages | Domaine et ports internes ; aucun adaptateur concret ou UI. |
| Adapters | Acquisition, conversion, calcul spectral, diffusion, stockage et routage | Core et infrastructure existante ; aucune dépendance UI/root. |
| UI desktop | Commandes et présentation de gain/jauge/états | API application et types publics purs ; aucun nouvel accès direct à la capture. |
| Composition root | Injecter les adaptateurs | Toutes couches, sans nouvelle règle métier parallèle. |

Choix FFT : rendre directe la version Rustfft 6.4.1 déjà verrouillée est l'option technique du plan, sans nouvelle DLL ou frontière native. Les manifests exacts et la disponibilité hors ligne devront être inclus/vérifiés avant T5 ; aucune édition de dépendance ou téléchargement n'est autorisé par ce document. Une impossibilité revient à l'architecte, sans remplacement silencieux de pile.

## Risques, preuves et détails restants

| ID / critères | Risque et fermeture prévue | État |
| --- | --- | --- |
| R1 / CA-02 | Gain rétroactif au backlog ; frontière et drain prouvés sur séquence connue. | PRODUCT_VALIDATION — NOT RUN |
| R2 / CA-05/06 | Mauvaise identité/repli/défaut changé ; tester homonymes, absence et changement entre test/capture. | PRODUCT_VALIDATION — NOT RUN |
| R3 / CA-03/04/07 | Sonde ou lecteur lent bloquant ; lecture concurrente, charge et comptage audio exact. | PRODUCT_VALIDATION — NOT RUN |
| R4 / CA-01/04/07 | Saturation et artefact de changement abrupt ; distinguer les points de mesure et vérifier son/PCM. | PRODUCT_VALIDATION — NOT RUN |
| R5 / CA-05 | Écrasement de gains ou sauvegarde faussement confirmée ; deux producteurs, erreurs et révisions désordonnées. | PRODUCT_VALIDATION — NOT RUN |
| R6 / CA-07 | Régression imports, moteurs ou finalisation ; tests ciblés puis recette réelle. | PRODUCT_VALIDATION — NOT RUN |
| R7 / CA-04 | Calcul FFT et disponibilité dépendance : version verrouillée observée ; aucun coût runtime mesuré. | Faisabilité documentaire partielle ; validation de charge future, pas un essai réussi. |

- D1, owner architecte, précisé dans ARCH-v1.2 ci-dessus : normalisation PSD, tolérances synthétiques et bornes d'instantanés. Avant T5, vérifier leur traduction dans les tests et la disponibilité hors ligne de la dépendance.
- D2, owner utilisateur assisté par l'architecte, avant T8/T9 : corpus de référence, durée/charge, transcription attendue et règle détaillée de comparaison. Q5 accepte la méthode, pas un corpus ou seuil WER inventé. Cela ne bloque pas la rédaction du plan.
- D3, owner session principale avant toute campagne : garde applicable et racines précisément autorisées ; aucune garde générique n'a été qualifiée ici. Aucune création de garde ou ancienne dérogation n'est autorisée implicitement.
- Packaging, profils et ressources : reprendre AGENTS.md pour le profil effectivement construit ; la recette Windows reste à l'utilisateur. Aucun installateur nouveau.
- Railguard nouveau : non applicable, aucun demandé. AGENTS.md existant reste actif. Reprise après panne et confidentialité de nouvelles archives : pas de fonctionnalité ajoutée ; le test préalable ne crée aucun fichier.

## Passage à la planification

Retour utilisateur suivant : arrêt du test micro confirmé dans la fenêtre Windows. Le test était ensuite bloqué après une capture parce que son bouton/handler refusaient tout job encore identifié, même Complete ; le correctif UI utilise l'activité réelle et préserve le résultat précédent. Une alerte rouge « Écrêtage après gain détecté » lit le compteur numérique de limitation existant, uniquement pour une mesure actuelle et une capture/test active ; elle disparaît dès Stop, y compris avec un relevé historique encore stocké pendant Draining. Les trois tests headless ciblés passent et la même revue indépendante ferme R9 puis déclare ce delta CLEAN. Le plan contient preuves et nouveau hash EXE. La sonde calcule déjà la PSD A/B avec FFT 2 048/Hann, 1 025 composantes au taux natif jusqu'à 10 publications/s ; le graphique de fréquences n'est pas ajouté. Recette de reprise/saturation à confirmer par l'utilisateur ; qualité Parakeet CPU inchangée.

Retour de recette Windows ultérieur : l'utilisateur confirme le dernier EXE et signale un test micro impossible à arrêter, avec le dernier message de gain à 0 dB/trame 0. Le correctif dans `ui.rs` retire le groupe microphone du conteneur global désactivé pendant `busy`, qui neutralisait aussi Arrêter ; source/actualisation restent verrouillées pendant l'activité. Un test egui sur le rendu complet clique Tester -> reçoit Started/Applied/Meter -> clique le vrai Stop -> vérifie invalidation -> redémarre ; il passe, et la revue indépendante est CLEAN sur ce delta. Le nouvel EXE personnel est compilé et hashé, sans fenêtre pilotée ni nouveau test moteur. Le message de gain reste une dernière application, distincte de l'activité de la jauge. La confirmation matérielle dans la fenêtre reste à l'utilisateur. Le plan contient le build, les empreintes et la recette ; la réserve Parakeet CPU ci-dessous reste inchangée.

Bilan final au 25 octobre 2026 : l'utilisateur a ensuite autorisé le correctif produit ciblé R8, les tests CPU/GPU et les 16 cas Parakeet CPU plafonnés à 20 minutes supplémentaires. L'adaptateur accepte maintenant les résultats CPU corrélés sans le marqueur réservé au GPU, après validation du backend Ready ; l'attestation GPU et les autres contrôles sont conservés, Auto transmis sous son choix effectif. **R1 à R8 sont fermés par la revue indépendante**. Le premier essai Parakeet accéléré a révélé que la fixture surchargeait la voie ASR ; son régime d'injection/polling a été corrigé localement à cadence réelle sans modifier le moteur. Les 16 cas cadencés ont fini en 1 016 s, soit 1 054 s cumulées avec le run accéléré, sous le budget de 1 200 s. Les WAV conservent exactement les frames sources et les gains/révisions attendus, mais **16/16 transcriptions Parakeet restent `complete=false`**, diagnostic de portion audio non confirmée et textes fragmentaires. Le même export que l'application est utilisé ; la cause précise de ces portions résiduelles à 1× n'est pas établie, aucune nouvelle correction ASR n'est appliquée. Sélection demandée exécutée : 16 Turbo CPU et 16 Parakeet CPU cadencés, avec sources/textes/preuves conservés. Candidat personnel compilé et dossier de recette vérifié, preuves archivées. Verdict **LIVRÉ AVEC RÉSERVES**, sans CLEAN global : CA-07/V7 Parakeet reste PARTIAL/FAILED ; comparaison GPU et fenêtre Windows à l'utilisateur. Le plan détaille les résultats, limites et empreintes finales.

Accord ultérieur de réalisation, 25 octobre 2026 : l'utilisateur demande explicitement l'implémentation du plan associé, autorise l'adaptation et la vérification de la garde existante ainsi que les tests prévus sous `target/parakeet-tests/micro-level/` et la compilation partagée sous `target/parakeet-tests/cargo-shared/`. Il confirme pour D2 les quatre WAV FR/EN du corpus FEAS-PM-01 et leurs textes corrigés `_copie.txt`, avec référence à 0 dB, variantes faible/forte et changement de gain, comparaison de continuité/durée/texte sans nouveau seuil WER ; il choisit le profil personnel existant. Le détail de cet accord et les résultats de réalisation sont suivis dans le plan. Les mentions précédentes de mission documentaire conservent leur portée historique ; aucune ancienne dérogation de campagne n'est réutilisée.

Le plan associé est `docs/cadrage/plan-evo-micro-level.md`, neuf tâches maximum conformément au découpage déjà présenté. La revue DESIGN ci-dessus permet sa rédaction ; les précisions de statut et de fixture seront incluses dans la relecture du couple cadrage/plan. Les détails D1/D2/D3 ont leurs échéances propres et ne valent ni refus de rédiger ni autorisation d'exécuter.

## Extension SPECTRE v2 — affichage en direct pendant le test et la capture microphone

### Source, statut et carte du contenu

Demande distincte du 25 octobre 2026 : « Peux tu afficher le spectre des fréquences dans l’interface pendant le test du micro, avant et après gain ? ». Après qualification et revue contradictoire PASS, la session principale a demandé : « M’autorises-tu à compléter les deux documents micro-level existants avec cette extension et son plan, à te soumettre avant de coder ? ». Réponse exacte : **« Je t autorise »**. Cet accord autorise cette rédaction et ses commits locaux, pas l'implémentation, une compilation ou une nouvelle campagne.

Précision utilisateur suivante : **« Je souhaite que le spectre soit visible pendant la prise de son. Et non pas seulement avant »**. CHANGE-S02 remplace la restriction test-only de CHANGE-S01 : afficher en direct pendant le test ET la capture microphone, Turbo et Parakeet, y compris pendant l'attente du moteur si l'acquisition a commencé. « Avant gain » et « Après gain et limitation » désignent deux points de mesure du même signal acquis au même instant ; ils ne désignent pas des moments avant/après l'enregistrement. Cette correction est intégrée à la mission documentaire en cours ; elle ne vaut pas autorisation de coder ou d'exécuter la campagne.

CHANGE-S01 puis CHANGE-S02 sont des DESIGN_CHANGE et PLAN_CHANGE. Base documentaire avant cette précision : `69dcc9ff`, branche `evo-startup-perf` ; les sources micro-level sont déjà modifiées/non suivies et sont préservées. Leur présence locale est nécessaire au futur lot ; HEAD seul ne contient pas cette implémentation. Aucun code n'est commité par cette mission.

Le présent ajout regroupe besoin, exigences, domaine, architecture, états, risques et acceptation. Le plan associé porte tâches, chemins et vérifications de l'extension. Les sections historiques ci-dessus conservent leurs résultats et réserves ; leurs anciens verdicts CLEAN ne couvrent pas SPECTRE. Auteur des deux fichiers : session principale ; contributions de qualification et d'architecture en lecture seule. Statut initial : proposition à relire puis à soumettre à l'utilisateur.

### Besoin, exigences et critères observables

Utilisateur : personne réglant ou utilisant son microphone dans l'application Windows existante. Résultat : observer en direct la répartition des fréquences avant gain et après gain/limitation, et comparer l'effet du réglage pendant le test explicite et pendant la capture destinée à transcription. Le test reste sans moteur ni enregistrement ; la vraie capture conserve ses moteurs, sorties et règles existants.

Chaque ligne relie le besoin qualifié CA-S01 à S07 à une exigence REQ-S, un parcours UC-S et un critère AC-S de même numéro. Les preuves produit sont **NOT RUN** ; les méthodes sont dans le plan.

| IDs | Scénario et résultat attendu | Responsabilité |
| --- | --- | --- |
| S01 | Démarrer Tester OU la capture micro Turbo/Parakeet : deux zones nommées « Avant gain » et « Après gain et limitation » portent fréquence en Hz et densité en dBFS/Hz. Elles suivent le signal tant que l'acquisition est active, même si le moteur attend. Aucun graphe pour un import de fichier ou une transcription sans acquisition micro. | Présentation desktop et parcours de capture. |
| S02 | Sur sinus puis changement de gain, comparer la même fenêtre audio A/B avec mêmes axes/échelles. À 0 dB sans limitation les courbes coïncident ; à +6 dB sans limitation B est décalée de +6 dB aux composantes significatives. Une fenêtre traversant le changement affiche « Gain modifié dans cette fenêtre ». | Sonde, contrat et présentation. |
| S03 | Au taux natif Fs, montrer les 1 025 composantes FFT2048 de 0 à Fs/2 ; aucune fréquence inventée au-delà. Avant une fenêtre complète : attente, sans remplissage fictif. | Adaptateur et projection graphique. |
| S04 | Silence valide distinct d'attente, indisponibilité ou péremption. À plus de 500 ms depuis acquisition de la mesure spectrale, retirer les courbes actuelles ; aucune publication RMS intermédiaire ne rafraîchit cet âge. | Application et présentation, horloge contrôlée. |
| S05 | Arrêt demandé, erreur, perte de source, fermeture ou changement de session/source invalident les courbes. Test -> capture efface le spectre du test puis affiche l'attente et les nouvelles mesures de capture ; capture -> drain/finalisation retire immédiatement les courbes. Nouveau test après capture finalisée : attente puis ses propres données. | Cycles application/test/capture. |
| S06 | Pendant les mises à jour du test/de la capture et au redimensionnement, les commandes de test/capture/arrêt, le gain, la saturation et le texte transcrit restent accessibles. L'utilisateur confirme la lisibilité et le fonctionnement dans la fenêtre Windows. | Desktop et recette utilisateur. |
| S07 | Réutiliser le flux et la FFT existants, conserver seulement le dernier spectre, sans historique ni calcul graphique dans le rappel audio. Mises à jour perdues remplaçables, sans attente sur la voie audio ni modification du PCM WAV/STT. | Adaptateur/application/UI. |

Exclusions SPECTRE : affichage spectral pour import ou transcription sans acquisition microphone active ; historique/export ; diagnostic par canal ; fréquence fondamentale ; retour haut-parleur ; égalisation/AGC ; nouvelle acquisition, nouvelle FFT, moteur, modèle, dépendance ou frontière native. Le graphe ne prouve pas l'absence de saturation analogique avant acquisition. La réserve Parakeet CPU antérieure n'est pas réouverte par cette extension.

### Proposition de présentation et états visibles

DEC-S01 proposée : deux graphes alignés verticalement, titres indépendants et axes communs, sous les commandes et l'alerte de saturation. L'alternative de deux courbes superposées économise la hauteur mais masque leur coïncidence à 0 dB ; elle n'est pas retenue. Les commandes restent hors de toute zone graphique pouvant les désactiver ; une zone défilante ou des hauteurs adaptées maintiennent leur accès sur petite fenêtre. La disposition est à confirmer lors de l'acceptation du plan, pas par une décision préalable distincte.

DEC-S02 proposée : axe fréquence linéaire de 0 à Fs/2 avec graduations Hz/kHz, incluant DC et Nyquist ; axe vertical fixe commun de -120 à 0 dBFS/Hz, sans ajustement automatique indépendant. Conversion `10*log10(PSD)` (référence 1 FS²/Hz) ; zéro reste un silence, jamais une valeur mesurée à -120. Le plancher est un seuil de dessin, pas un plancher du microphone. Annoter les valeurs sous/hors échelle ; les données ne sont pas modifiées pour entrer dans le cadre. Alternative logarithmique non retenue pour cette première version, car elle exclurait DC et compliquerait l'affichage intégral ; pas de sélecteur supplémentaire.

Repos : aucune courbe. Ouverture/test/capture sans spectre complet : « En attente du spectre… ». Capture Initializing avec flux acquis : mêmes courbes qu'en Active, sans attendre le premier texte ni le moteur. Initializing sans flux acquis : attente seulement. Paire complète valide : courbes et unité ; silence : « Silence détecté » sans fausse énergie au plancher. Une seule courbe silencieuse conserve son propre état, l'autre reste lisible. Mesure rejetée/perdue : « Spectre indisponible » ; mesure trop ancienne : « Spectre périmé », sans anciennes courbes actuelles. Stop/erreur/fermeture : retrait immédiat dès la commande, sans attendre la fin du drain. Passage test -> capture : invalider le cache du test, puis accepter exclusivement la nouvelle session de capture. Tout nouveau test/capture repart sans cache. Drain, finalisation et transcription seule après arrêt : aucun spectre actuel. Les jauges RMS/pic et l'alerte d'écrêtage gardent leur propre fenêtre et validité.

Q-S01 (détail réversible, owner desktop, avant recette) : couleurs, nombre de graduations et hauteur, suivant le thème et la largeur disponibles ; aucun changement des unités, des bornes ou des états sans mise à jour de DEC-S01/02. Pas de question produit structurante restant à résoudre avant la revue documentaire.

### Domaine, frontières et contrat spectral proposé

Contribution technique : `rust_architect`, lecture seule des sources locales le 25 octobre 2026 ; intégration par l'auteur principal. DEC-S03 v2 remplace sa variante test-only : événement distinct `MicrophoneSpectrum(SpectrumUpdate)` pour le test et les parcours de capture Turbo/Parakeet, plutôt qu'un ajout de 2 050 valeurs à chaque `MicrophoneMeter`. Réutiliser la sonde déjà alimentée par chacun de ces parcours. Le format est un contrat applicatif local, pas une modification du protocole des workers natifs. Les politiques CPU/GPU/Auto et leurs règles de repli restent celles du produit existant.

Le domaine pur dans `whisper-core/src/microphone_level.rs` porte la paire et sa validité : job, génération, identité réelle, séquence spectrale, époque de continuité, taux natif, FFT2048, plage `[first_frame,last_frame)` de 2 048 échantillons, temps monotone d'acquisition et premières/dernières valeurs/révisions de gain avec drapeau multirévision calculés sur **cette fenêtre FFT**. Les métadonnées RMS100ms ne peuvent pas être réutilisées comme description de la fenêtre FFT. Les voies A/B partagent exactement cette identité, plage, époque et acquisition.

DEC-S04 : préserver `Eq` et serde du contrat existant avec, pour chaque composante, `Silence` si PSD=0 ou un entier signé en milli-dB de `10*log10(PSD)` arrondi au milli-dB. Vérifier finitude, positivité avant logarithme et représentabilité, sans tronquer aux bornes de dessin ; entrée négative/non finie, nombre différent de 1 025 composantes, paire incomplète, taux/plage incohérents rendent la paire indisponible. Erreur de conversion <=0,0005 dB sur valeurs positives représentables ; aucune nouvelle tolérance vocale. La PSD linéaire f64 et son calcul existants restent la référence dans l'adaptateur. L'alternative d'introduire des flottants dans les événements imposerait des modifications de l'égalité et de plusieurs contrats, sans bénéfice produit ici.

Le calculateur de sonde produit et retient la dernière paire avec ses métadonnées propres, hors rappel audio. Un tick RMS sans nouvelle PSD ne la supprime pas et ne change ni son numéro ni son âge. Reset, rupture de continuité, mise à jour perdue ou invalidation du flux retirent aussi ce dernier spectre ; un état d'invalidation identifiable par époque est consultable sans attendre une nouvelle FFT. Le pump du propriétaire actif (test ou capture) transmet les nouveautés et une invalidation une seule fois par époque, pas une file historique. Les événements d'acquittement du gain et événements de contrôle/texte gardent leur progression, mais l'ordonnancement ne doit affamer ni jauge, ni spectre, ni traitement ASR : coalescence au dernier état et alternance bornée des publications de mesure. Pas d'acquittement de gain déduit d'une courbe.

L'application est propriétaire du dernier état spectral courant. Elle admet les données sous un prédicat core commun « propriétaire microphone actif » : test actif ; capture directe Initializing/Active ; ou contexte de capture LiveCaptureState décrit ci-dessous. Dans tous les cas, exiger acquisition effective, identité/génération/source attendues, temps d'acquisition non futur, séquence croissante et époque non ancienne ; aucun spectre ne peut activer à lui seul une session inexistante. Un changement d'époque invalide la paire précédente avant reconstruction ; une livraison retardée ne la réactive pas. Elle n'actualise pas le cache sur événement RMS ; stop demandé, erreur, source perdue ou changement de session l'effacent. Démarrer la capture efface les données du test mais autorise ensuite celles de la nouvelle capture. L'UI utilise ce même prédicat core et filtre à nouveau validité/âge (périmée si >500 ms), même sans nouvel événement, grâce à un repaint planifié pendant test et capture. Aucune courbe n'est persistée ni rechargée au lancement.

Admission LiveCaptureState et contexte commun (fermeture proposée de SPECTRE-V2-R01, apport ARCH-SPECTRE-v2.1 du même architecte, lecture seule) : tout démarrage micro soumis avec succès, test ou capture, installe un contexte applicatif volatil de propriétaire avec job/génération, acquisition autorisée et identité effective initialement inconnue. Cela inclut `StartLiveConfigured`, sans dépendre de `direct_capture_phase` (qui reste None sur ce chemin). Le contexte autorise une attente, pas une courbe ; le prédicat core commun exige ce contexte pour tous les parcours. Une première paire fraîche, issue de la sonde du flux effectivement acquis et dont l'identité correspond à ce propriétaire, établit l'acquisition et la source réelle ; pour WindowsDefault, l'identité effective est fournie par l'adaptateur propriétaire, jamais devinée depuis le nom du défaut. Source explicite : refuser une autre identité. Après cette liaison, tout relevé doit conserver l'identité liée jusqu'à changement de contexte. Ce contexte ne peut exister pour un import ou être déduit du seul JobState ni du seul relevé. Stop demandé (avant tout effet asynchrone), perte/erreur, fin d'acquisition, nouveau propriétaire ou passage import retirent l'admission et le cache ; drain/finalisation n'admettent plus de mesures, y compris tardives. Les reprises autorisées réinitialisent explicitement contexte/époque, sans nouvelle politique Auto ni API de gain pour le parcours historique. Une simple perte de continuité invalide la fenêtre mais ne recrée pas le propriétaire.

| Consommateur | Dépendances et responsabilités autorisées | Interdits / contrôle futur |
| --- | --- | --- |
| Domaine/core | Types purs et bornes du spectre ; std/serde déjà présents. | Aucun egui, FFT ou accès périphérique. Revue imports et tests de contrat. |
| Application/core | Événements/ports qu'elle possède, filtre identité/époque/séquence, état courant et arrêt. | Aucun appel direct adaptateur/UI. Tests de livraison désordonnée et transitions. |
| Adaptateur | Sonde existante, conversion vers contrat core et publication test/capture Turbo/Parakeet. | Pas de second flux, nouveau moteur ou calcul dans callback ; revue rappel et tests de sonde/pump. |
| Desktop | Vue applicative, projections et Painter egui existant. | Pas de lecture de sonde concrète ni logique de traitement audio. Tests du vrai widget et revue imports. |
| Composition | Assemblage existant inchangé. | Aucun nouvel état métier parallèle. |

Bornes : 1 025 composantes par voie, un dernier spectre retenu par sonde et par vue, aucun historique ni file non bornée de courbes ; au plus un événement spectral remplaçable en attente. Fréquence maximale du calcul inchangée (10 Hz). Reprojection graphique bornée aux 2 050 points par frame, hors thread audio. Le rappel d'acquisition, le PCM partagé WAV/STT, les fenêtres RMS, le gain et sa mémorisation restent gouvernés par le contrat initial.

DEC-S05 : dessiner avec le Painter egui de l'eframe `=0.35.0` déjà déclaré, sans dépendance graphique, changement de Cargo.lock, technologie ou code natif supplémentaire. Les API Painter de découpage du rectangle, tracé de ligne et texte existent dans les sources Rust locales ; la preuve du rendu réel reste à produire. La réduction éventuelle de points pour la largeur du widget ne doit pas masquer un pic ; pour cette version, le tracé borné des 1 025 points est suffisant.

### Sources, risques et readiness SPECTRE

Preuves documentaires, lecture seule du 25 octobre 2026 : `microphone_probe.rs:26–62,197–210,250–276,353–500` établit cadences, validité, conversion actuelle sans PSD, fenêtres RMS/FFT distinctes et calcul Hann2048/1 025 valeurs. `worker_ipc.rs:1455–1584` établit le propriétaire test, sa file d'événements et son pump. `microphone_level.rs:218–240` et `ports.rs:426–462` établissent les contrats Eq/serde. `application.rs:743–760` établit le stockage de jauge et son retrait à l'arrêt ; les branches d'erreur et arrêt seront incluses au plan. `ui.rs:1323–1508,1590–1604` établit les commandes et le repaint existants ; `whisper-desktop/Cargo.toml` établit eframe. Ce sont des preuves de structure, pas un essai du futur graphe.

| Risque | Mesure de conception / preuve attendue |
| --- | --- |
| RISK-S01 : gain RMS attribué à tort au spectre | Métadonnées dérivées des 2 048 échantillons FFT ; test où seule cette fenêtre traverse un changement. |
| RISK-S02 : courbe périmée conservée pendant publications RMS, ou après perte | Horodatage spectral indépendant, cache par époque, invalidation explicite et horloge simulée ; ancien événement livré après Stop rejeté. |
| RISK-S03 : commandes masquées par les graphes, arrêt redevenu inaccessible | Commandes au-dessus du bloc graphique, mise en page adaptée ; test headless du vrai rendu complet puis recette utilisateur. |
| RISK-S04 : coût ou file disproportionnés | Dernière valeur remplaçable, 10 Hz maximum, aucune FFT supplémentaire, tests RMS intercalés et producteur plus rapide que lecteur. |
| RISK-S05 : silence/échelle trompeurs | Zéro explicite, unités densité, mêmes bornes sans autoscale, indicateurs hors échelle et tests mathématiques. |

Applicabilité : scope/use_cases/errors/domain/dependencies/concurrency/platform/acceptance/risks couverts ci-dessus et dans le plan. Plateforme : application Windows Rust existante, mêmes périphériques et installation personnelle ; pas de nouvelle dépendance native. Persistence/migration : N/A, état spectral seulement volatil, préférences et exports inchangés. Installation : seul futur remplacement du desktop de recette, ressources du profil vérifiées selon AGENTS.md. Reprise après panne et nouveau railguard : N/A hors besoin. Aucun SPIKE ou mécanisme de qualification supplémentaire proposé : la réutilisation est démontrée par les sources ; les tests comportementaux restent NOT RUN avant code.

Complément de l'apport ARCH-SPECTRE-v1 reçu après rédaction : conserver aussi le compteur de pertes dans le contrat spectral ; valider identité non vide, gains/révisions cohérents et compteur non régressif pour une même époque. Détecter toute révision différente dans la fenêtre, y compris un retour final à la valeur initiale. Un événement invalide rend le seul spectre indisponible, sans arrêter le test ni modifier l'audio. Admission applicative : rejeter également les relevés déjà âgés de plus de 500 ms. L'ordonnancement doit tenir compte de la limite actuelle de quatre événements traités par refresh (`application.rs`) ; pas d'augmentation non bornée de cette limite. Pour le nouveau chemin spectral, l'UI ne dépend que du core ; la dépendance concrète desktop/adapters préexistante reste hors refactor.

Preuve locale Painter précisée par l'architecte : `target/parakeet-tests/micro-level/cargo-home/registry/src/index.crates.io-1949cf8c6b5b557f/egui-0.35.0/src/painter.rs:71,213,318,327,406,469` (découpage, formes, lignes, cadre et texte). L'apport brut lu par la session principale correspond aux décisions techniques ci-dessus ; ses numéros DEC internes sont remappés vers DEC-S03/04/05 du présent document, sans changement de contenu technique.

Historique SPECTRE v1 : **DESIGN CLEAN sur `1587696ada1d871e1aa33aaa10a00b329a882b52`**, verdict indépendant `rust_design_reviewer` (`spectre_design_review`), aucun constat REQUIRED ouvert sur ce périmètre test-only. Revue directe du fichier Git, comparée à `1b31d858` ; fenêtres, contrats, API Painter et couverture S01–S07 contrôlés. Le reviewer confirme son indépendance de rédaction/conception. Ce verdict antérieur ne couvre pas CHANGE-S02 ni les nouveaux chemins de capture. L'acceptation produit et l'autorisation d'implémenter seront demandées sur le couple design/plan v2 relu ; elles ne sont déduites ni de la demande initiale ni de l'accord documentaire.

Plan v1 associé ensuite déclaré **PLANS CLEAN sur `c168006db7784da53b64b320c98d0eece808fe6e`** par le même reviewer indépendant, après lecture du fichier Git et vérification de la couverture S01–S07/ST1–ST7/V-S1–V-S5. Ce verdict reste historique et ne couvre pas v2. Les décisions produit, la campagne proposée DEC-CAMP-S01 et l'implémentation attendent l'accord précis de l'utilisateur ; code, tests, build et recette SPECTRE restent NOT RUN.

Complément ARCH-SPECTRE-v2, contribution lecture seule du même architecte sur base `69dcc9ff` et sources locales : consommer le dernier état spectral depuis `MicrophoneTestCapture::pump`, `direct_live::poll`, `parakeet_live::poll` et le chemin microphone `LiveCaptureState` existant dans `worker_ipc::poll`. Les deux modules de capture ajoutés au plan ne changent ni l'inférence ni les politiques moteur. Sources primaires locales : `direct_live.rs:374–456,833–861`, `parakeet_live.rs:743–755,1092–1115`, `worker_ipc.rs:6671–6746`, `application.rs:676–759,1131–1156,2565–2600`.

DEC-S03 v2, précisions d'ordonnancement : avant un retour de télémétrie, faire avancer le traitement audio/moteur non bloquant et préserver ses événements dans les structures bornées existantes. Au plus un retour de télémétrie consécutif lorsqu'un événement texte/contrôle/terminal attend ; au poll suivant, cet événement est prioritaire. Alterner RMS et spectre lorsqu'ils attendent tous deux. Stop/erreur restent prioritaires. La capture ne doit pas attendre l'inférence pour produire une mesure. Pas de nouveau backlog spectral ni augmentation non bornée des files de texte.

Une invalidation par époque doit rester observable jusqu'à consommation même si une nouvelle paire arrive avant le prochain poll : retenir un marqueur d'invalidation et la dernière paire, pas un historique de transitions. Invalider l'ancien avant d'admettre le nouveau. Le repli Auto existant de `direct_live::retry_cpu` invalide déjà la sonde sans changer nécessairement job/génération : l'époque spectrale distingue ce reset, retire les anciennes mesures et impose une fenêtre complète reconstruite ; ne pas lier la courbe au numéro d'instance worker ni changer l'identité du micro.

RISK-S06 : télémétrie publiée avant les événements moteurs et retard du texte/arrêt. Fermeture attendue : tests courts à producteurs continus de jauge/spectre, événements texte/terminaux simulés et consommateur applicatif limité à quatre événements par refresh, sur les trois propriétaires actifs et le chemin LiveCaptureState. Vérifier progression bornée des événements, PCM strictement identique vers WAV/STT avec/sans lecteur spectral, et spectre indépendant du moteur retardé. RISK-S07 : ancienne courbe après reset Auto à génération identique ; fermeture par invalidation d'époque observée puis reconstruction, sans changement des politiques Auto. Aucune campagne vocale requise pour ces preuves.

Revue v2 initiale sur `b06136a16570b386b9e00cc65cab0b38b35bd59c` : DESIGN/PLANS FINDINGS, un constat REQUIRED IMPORTANT SPECTRE-V2-R01 sur l'admission LiveCaptureState dépourvue de DirectCapturePhase. La précondition équivalente ci-dessus et les tests du plan proposent sa fermeture ; seul le même reviewer peut la confirmer. Les autres contrôles v2 étaient concluants. Les résultats produit restent NOT RUN ; seuls les verdicts portant explicitement sur v2 pourront qualifier cette révision.

Relecture corrective v2 : le même reviewer indépendant déclare **SPECTRE-V2-R01 FERMÉ, DESIGN CLEAN / PLANS CLEAN sur `d6abc4381716cef97cea11d30fa7deadd6cdc1d2`**, après lecture directe des deux fichiers Git comparée à `b06136a1`. Contexte propriétaire, liaison à la source et oracles liés ST3/ST5/V-S3/V-S4 vérifiés ; aucun constat REQUIRED ouvert. Ces verdicts couvrent la prise de son en direct ajoutée par CHANGE-S02. Acceptation du plan/campagne et autorisation d'implémentation restent à recevoir ; code, tests, build et recette NOT RUN. Le présent ajout enregistre seulement le verdict, sans changer le contrat.

Autorisation de réalisation reçue ensuite : à la question « Valides-tu cette version et autorises-tu son implémentation, les tests selon DEC-CAMP-S01 et la remise du nouvel EXE personnel ? », l'utilisateur répond **« Ok tu peux implémenter »**. Cette réponse accepte SPECTRE v2, ses sept tâches et chemins, DEC-CAMP-S01 et la remise personnelle décrits dans le plan à `dd602f1f`. Aucun téléchargement, installation, modification de ressources personnelles ou push n'en découle. La recette physique reste à l'utilisateur. Préflight courant : HEAD `dd602f1f`, branche `evo-startup-perf`, delta micro-level préexistant conservé ; espace libre C: 129 973 547 008 octets. Les résultats produit seront consignés après exécution et revue.

### Bilan de réalisation SPECTRE v2 — 26 octobre 2026

Lot réalisé par `rust_implementer` après l'accord consigné à `c6e94c06`. Le travail micro-level antérieur a été conservé ; onze copies ordinaires pré-SPECTRE restent sous `target/parakeet-tests/micro-level/spectre-v2-baseline/` pour distinguer les deltas dans un arbre déjà modifié. Aucun moteur, modèle, manifeste Cargo, paramètre personnel ou frontière native supplémentaire ajouté par SPECTRE.

Deux graphes alignés « Avant gain » et « Après gain et limitation » sont reliés à la sonde pendant le test ET l'acquisition micro Turbo/Parakeet, y compris avant que le moteur soit prêt. Les mesures portent la fenêtre FFT, les révisions de gain, l'identité, l'époque, la séquence et l'âge d'acquisition ; mise à jour RMS et mise à jour spectrale restent indépendantes. L'application gère un propriétaire microphone commun, y compris le parcours historique sans DirectCapturePhase, puis retire les courbes dès Stop, erreur ou fin d'acquisition. L'effet du gain est une comparaison simultanée A/B, pas un affichage réservé à l'avant-enregistrement.

La revue indépendante du delta par `rust_reviewer`, orchestrée par `rust_review_coordinator`, déclare **CLEAN sur candidate_epoch2**, après deux cycles correctifs maximum. Les quatre constats SPECTRE-R01 à R04 sont fermés par le même reviewer : progression du parcours historique sous charge de compteurs ; nettoyage à Stop pendant Committing ; barrière d'époque/temps au repli Auto ; preuves par les vraies boucles Turbo/Parakeet de progression du texte et de fin, avec WAV identiques aux références. Les fichiers source/tests n'ont pas changé après cette revue ; ST7 n'a produit que le build et les artefacts de remise.

Les tests ciblés cœur, sonde, capture, IPC, Turbo, Parakeet et rendu egui passent, dont le clic Stop/retest, une petite fenêtre 640×480, les livraisons tardives et les comparaisons WAV octet pour octet. Deux tests WASAPI matériels sont ignorés. `git diff --check` passe. Le formatage global échoue seulement sur `crates/whisper-core/src/model_selection_app.rs:144`, antérieur et hors périmètre ; ce fichier est resté inchangé. Ce résultat a arrêté sa commande, a été conservé, puis la session principale a arbitré la reprise ciblée sans élargir les éditions. Il ne s'agit pas d'un succès global de formatage.

Build personnel gardé **PersonalRelease-20261026T000151351-312012d0**, SelfTest préalable réussi, exit 0 en 21 s. Nouvel EXE : `target/parakeet-tests/micro-level/recette-personnelle/whisper-desktop.exe`, 14 365 696 octets, SHA-256 **469AC98129F93E280DCD7E61CD07CA19A2DF5EF26D158A9CA99FA61A0DB3E913**. Les deux workers Turbo l'accompagnent ; dossier de trois EXE uniquement. Les 18 ressources installées du profil personnel ont été vérifiées en lecture seule puis comparées par la session principale aux relevés avant implémentation : tailles et SHA-256 tous inchangés. Aucun processus utilisateur fermé ni fenêtre pilotée.

Le plan contient les empreintes des trois EXE, les références des tests et l'archive des preuves. Validation restante : l'utilisateur observe les graphes pendant test puis vraie capture Turbo/Parakeet, gain/saturation, texte accessible, arrêt et reprise. La réserve Parakeet CPU issue de D2 reste inchangée ; ces fixtures simulées ne requalifient ni l'ASR ni le GPU. Espace libre final : **129 556 926 464 octets**, toujours supérieur à 30 Gio. Aucun nettoyage, commit de code ou push.
