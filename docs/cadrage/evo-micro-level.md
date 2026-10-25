Avancée : R1 à R7 fermés après le cycle supplémentaire autorisé ; candidat personnel recompilé, 16 sorties Turbo CPU produites.
Blocage : premier cas Parakeet CPU sans segments, cause indéterminée ; D2 partiel et recette Windows non exécutée.
Décision : conserver candidat et preuves ; arbitrer le diagnostic ciblé Parakeet avant toute correction ou reprise, selon le bilan du plan.

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

Exclus : modification du volume Windows, gain automatique, réglage des imports, visualisations autres que la jauge simple. Différés jusqu'à demande distincte : diagnostic par canal, retour audio, reprise après panne et toute nouvelle visualisation. Les garanties de publication existantes ne deviennent pas une transaction atomique TXT/SRT/WAV.

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

Bilan actualisé au 25 octobre 2026 : le plan associé conserve les autorisations, l'extension précise de trois fixtures et les preuves. Après les deux premiers cycles, l'utilisateur autorise explicitement **un cycle supplémentaire limité à R7 et le rejeu des 32 cas CPU avec 52 minutes supplémentaires**. Le helper du test est corrigé pour injecter l'audio brut ; preuve A/B/WAV réussie et R7 fermé par le même reviewer, comme R1 à R6. Le rejeu frais produit les 16 sorties Turbo CPU, puis échoue au premier Parakeet CPU après 1 882 secondes, sans segments ; 15 autres cas Parakeet non exécutés. Le diagnostic en lecture seule ne démontre ni invocation effective du worker ni cause du résultat vide ; le harnais panique avant de conserver le diagnostic d'arrêt. Aucun défaut produit Parakeet n'est établi, aucune nouvelle correction ni relance faite. Le candidat personnel courant compile, preuves archivées, mais la validation **D2 Parakeet reste BLOCKED/PARTIAL**. Textes Turbo descriptifs non identiques selon gain, sans nouveau seuil WER ni garantie de qualité équivalente. Comparaison GPU et recette Windows restent à l'utilisateur ; pas d'acceptation produit finale. Le plan détaille le diagnostic ciblé proposé pour arbitrage.

Accord ultérieur de réalisation, 25 octobre 2026 : l'utilisateur demande explicitement l'implémentation du plan associé, autorise l'adaptation et la vérification de la garde existante ainsi que les tests prévus sous `target/parakeet-tests/micro-level/` et la compilation partagée sous `target/parakeet-tests/cargo-shared/`. Il confirme pour D2 les quatre WAV FR/EN du corpus FEAS-PM-01 et leurs textes corrigés `_copie.txt`, avec référence à 0 dB, variantes faible/forte et changement de gain, comparaison de continuité/durée/texte sans nouveau seuil WER ; il choisit le profil personnel existant. Le détail de cet accord et les résultats de réalisation sont suivis dans le plan. Les mentions précédentes de mission documentaire conservent leur portée historique ; aucune ancienne dérogation de campagne n'est réutilisée.

Le plan associé est `docs/cadrage/plan-evo-micro-level.md`, neuf tâches maximum conformément au découpage déjà présenté. La revue DESIGN ci-dessus permet sa rédaction ; les précisions de statut et de fixture seront incluses dans la relecture du couple cadrage/plan. Les détails D1/D2/D3 ont leurs échéances propres et ne valent ni refus de rédiger ni autorisation d'exécuter.
