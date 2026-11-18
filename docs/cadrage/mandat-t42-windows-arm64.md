Avancée : deux configurations Intel et un Snapdragon X2 Elite ont servi en recette manuelle. Sur le Snapdragon, les quatre parcours desktop ont réussi après installation du pack graphique Microsoft.
Blocage : T41 et T42 restent non exécutés. Les témoins AMD, v2 exact et sous v2 ainsi que les preuves de clôture ci-dessous manquent.
Décision proposée : préparer T42 sur un runner CI Windows ARM64 interactif identifié, avec le pack provisionné avant l’essai. Valider son accès et la méthode de capture avant de réserver le créneau d’exécution.

## Témoins et portée au 18 novembre 2026

| Témoin | Acquis | Reste à établir |
|---|---|---|
| Intel i9-14900K et GTX 1080 Ti | CPU, GPU et Auto exercés ; GPU choisi en Auto ; import et micro rapportés conformes. | N2 CUDA réutilisation des imports, gate technique distinct avec son propre mandat. |
| Intel i7-1185G7, Iris Xe sans NVIDIA | Import et micro CPU/Auto fonctionnels ; repli Auto CPU et démarrage sans alerte nvcuda.dll observés. | Complément T33 sur le candidat exact, identité des fichiers et environnement indépendant du dépôt et des outils de développement. |
| Snapdragon X2 Elite, Adreno | Worker x64 CPU documenté hors interface. Sur la réservation interactive, fenêtre et import/micro en CPU et Auto validés par l’utilisateur après installation du pack. | Version du pack et mesures OS/PE corrélées aux deux processus pendant un import GUI à recueillir en T42. Le ZIP de cette recette manuelle n’a pas été récupéré. |
| AMD x64 compatible | Aucun essai matériel AMD documenté. | Recette T43 sur paquet exact, installation, import, moteur effectif et exports. |
| x86-64-v2 exact | Aucun témoin qualifié au sens de T01. | Runner T41 et admission T43 : confirmer tout v2 et l’absence des extensions v3/v4 énumérées dans T01, puis installation et transcription. Un AMD vérifié répondant à T01 pourrait cumuler les deux rôles. |
| x86-64 sous v2 | Refus logiciel couvert séparément en T32. | T43/AC-03a : machine capable de lancer le bootstrap x64, au moins un prérequis v2 absent ; relever le refus explicite et l’état produit inchangé avant/après. |

## Mandat soumis pour T42 / AC-08

1. **Admission du runner.** L’opérateur CI indique l’identifiant du runner ou de la réservation, le mécanisme CI autorisé permettant d’y lancer le job, le compte de recette, le bureau interactif déverrouillé et la destination approuvée de récupération des résultats avant déconnexion. Une réservation accessible uniquement par SSH ou un lancement manuel du PS1 ne remplit pas cette condition. Pour une réservation Enterprise, consigner l’édition réellement testée ; la revendication Home/Pro reste limitée à ses propres témoins.
2. **Environnement graphique.** Dans ce même compte, relever build Windows ARM64, session, adaptateur et version du pilote, puis nom/version/architecture de Microsoft.D3DMappingLayers. Sur le Snapdragon, le pack de compatibilité OpenGL du Microsoft Store (produit 9NQPSL29BFFF) est le prérequis observé. L’image peut être prééquipée. Sinon, le mandat proposé permet à l’opérateur une installation depuis la fiche officielle avant le job, si permise par le fournisseur, sans élévation ni compte personnel. Store inaccessible ou version invérifiable : arrêt avant le job. La recette applicative utilise ensuite le modèle hors ligne.
3. **Entrées et installation.** Un seul essai CPU, sur un profil de test avec destination disponible pour l’installation neuve. Comparer avant exécution les empreintes du bootstrap, du desktop, des workers, du WAV et du modèle aux valeurs du candidat dans la [fiche du dernier correctif](recette-second-pc-gpu-fix.md) et aux valeurs ci-dessous. Vérifier aussi les exécutables installés. Ne pas reconstruire. La recette guidée précédente et le remplacement d’installation qu’elle propose ne servent pas de pilote CI.
4. **Pilotage et attribution.** Lancer le desktop avec stderr redirigé dès la création. Depuis le bureau interactif, le pilote UI Automation sélectionne CPU, le WAV via le dialogue natif, une sortie neuve et déclenche la transcription. Vérifier la fenêtre interactive et la disponibilité des contrôles selon T02 ; la saisie ciblée n’est admissible que dans le dialogue identifié. Pour le PID desktop et le PID du worker CPU de cet import, relever pendant leur vie l’identité de l’image, SHA, PE AMD64 pur, et les valeurs brutes des appels OS prévus par T02 (IsWow64Process2 et ProcessMachineTypeInfo selon disponibilité). Corréler chemins, PID et lignes WHISPER_CPU_CAPABILITIES_V1 du desktop et du worker relayé. Expliquer toute valeur ambiguë avec les autres mesures ; rejeter une attribution par nom seul. La disponibilité d’un collecteur en lecture seule et du pilote UIA est à confirmer au préflight. Si un nouvel outil ou workflow est nécessaire, en soumettre les chemins et les droits avant mise en œuvre.
5. **Résultat et récupération.** Attendre la fin de l’import ; vérifier le moteur CPU, un TXT français non vide correspondant à l’extrait et un SRT lisible, ordonné, dans ses quatre secondes. Archiver ensemble les rapports machine/pack, les preuves OS/PE/capacités, le journal du job et les exports. Récupérer et relire l’archive **avant libération du runner**. AC-08 PASS seulement si toute la chaîne concorde. Prérequis, mesures obligatoires ou récupération indisponibles : preuve incomplète, aucun PASS (runner/UIA non reproductible : NOT RUN). Un candidat effectivement exécuté qui échoue au démarrage ou à l’import est un échec produit à consigner.

**Identités attendues du candidat Turbo message-fix (SHA-256)**

| Élément | Valeur |
|---|---|
| Bootstrap | 59c94bac7bf36260048e885bd24ee0f8ee31b3598e91d0ef0acb3fcb257efc4b |
| Desktop | 01e2ea8ba221ed3245b9227c2fdb2bd5a36da073bff8305b255ad4dfe1be1f48 |
| Worker CPU | a16a5452610ab1dae8af349b2d97d1eea36f44a263125a15fd884288a3ec960b |
| Worker GPU livré | d874136c6d9561ca06a80e7c2b150513e10a05e46e37edd4cb28ba6eaabd17a9 |
| fr_passage_28-32.wav | 1841725ca1b9c019c8c53ac5983a13792d081f145f222f98355591c5964f50ad |
| ggml-large-v3-turbo.bin | 1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69 |

L’accord demandé portera sur le runner nommé et un job T42, avec récupération de ses sorties. La création d’un service, d’un workflow distant, une dépense, la publication et la modification du produit sont hors mandat. T42 seul ne clôt pas T45. Les scénarios de panne Auto retirés du périmètre restent exclus. [Plan T01/T02 et T41–T45](plan-implementation-distribution-cpu-x64.md) · [Recette manuelle Snapdragon](recette-windows-arm64.md)
