Avancée : la recette MSIX CPU a réussi sur Windows 11 x64 pour le lancement, l’import WAV CPU et le micro.
Blocage : le rapport d’assemblage ne consigne pas le commit Git exact de l’arbre source compilé.
Décision : conserver le résultat comme prototype CPU qualifié fonctionnellement, avec réserve de traçabilité ; aucune publication n’est autorisée.

# Bilan — prototype MSIX CPU — 23 novembre 2026

## Périmètre

Recette locale du paquet CPU prototype prévu par le [mandat MSIX CPU](mandat-prototype-msix-cpu-20261122.md), sur Windows 11 Professionnel x64, build 26300, Intel Core i9-14900K. Aucun essai de vitesse ou d’évaluation supplémentaire de qualité n’a été fait. Le Snapdragon n’a pas participé à cette recette ; sa VM peut être fermée.

## Identité du candidat

- Archive CPU P7 source : SHA-256 `b3231dbc6e05fa10495a0ef23e96e2e3cf18c4be96e8c8add3d4f0ff7a1b72db` ; manifeste `81692e3080775e0b10636aded3a6951d67723f03362d70139625f44306d1a305`.
- MSIX `WhisperTurbo-CPU-prototype.msix` : 2 188 205 438 octets ; SHA-256 `c7cdfa92b1c4e52b6e42d4e9cfd871f2ab5939b02f2d23d4173420d1fae3a33e`.
- Signature cryptographique : PASS. Certificat de test `CN=WhisperTurbo MSIX CPU Prototype`, empreinte `C27F44B3B7F8CDE72A6C39838008D308BA3F1B82`.
- Paquet installé : `WhisperTurbo.CpuPrototype_0.1.0.0_x64__tchm1sypv7cw8`, statut AppX `0`, dans `C:\Program Files\WindowsApps\WhisperTurbo.CpuPrototype_0.1.0.0_x64__tchm1sypv7cw8`.
- Les 24 fichiers de l’inventaire assemblé sont présents dans l’installation et leurs tailles et SHA-256 correspondent à l’inventaire du rapport d’assemblage. Le desktop et le worker CPU observés s’exécutaient depuis cette racine package.

Le modèle large-v3-turbo chargé est celui du package : SHA-256 `1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69`. L’état et les journaux de l’application ont été créés dans `LocalState\WhisperTurbo`; aucune copie du modèle ou du worker n’a été relevée dans cette racine de données.

## Résultats de recette

| Vérification | Résultat observé |
|---|---|
| Démarrage du point d’entrée MSIX et affichage du desktop | PASS ; fenêtre Whisper ouverte. |
| Import WAV en CPU | PASS ; moteur CPU affiché, job terminé et publication confirmée. WAV de référence : 128 078 octets, SHA-256 `1841725ca1b9c019c8c53ac5983a13792d081f145f222f98355591c5964f50ad`, 64 000 échantillons. TXT : 91 octets, SHA-256 `d3a5be03217bad627d3c551768c569284622a96ed609e59d0a11ea22ecae0d65`; SRT : 156 octets, SHA-256 `1c8ff95559600278c28b4a8ae2d119ea28d413548a3428cba4bd2cbad4a0da35`. |
| Microphone CPU | PASS ; consentement Windows accordé pour cette recette, capture terminée et écran indiquant le job micro `Complete`. WAV : 385 324 octets, SHA-256 `ae10f2ff1670a0b857906e0b1b672bc1ad2f334fb4fde44a51f4bd6883783b24`; TXT : 66 octets, SHA-256 `4e0654b5a77eb66df656cfdb54b63c450d6085c66f26ea1ae4b51ef4f80c1400`; SRT : 166 octets, SHA-256 `2d4cd344c360f00b02587d02e0b24f4974293d9e0966a94dceaaaebda195ba15`. |
| Routage des actifs | PASS pour ce paquet CPU : identité du worker depuis `WindowsApps`, modèle depuis `WindowsApps\…\models`, données/settings/logs sous le `LocalState` du package, exports dans le dossier de transcription choisi. |

Ces observations prouvent le parcours fonctionnel de ce candidat MSIX CPU sur cette machine. Elles ne qualifient ni l’édition MSIX NVIDIA, ni la compatibilité complète Snapdragon/MSIX, ni la distribution Store.

## Réserve de traçabilité et état système

Le rapport d’assemblage conserve l’archive P7 source, son manifeste, les empreintes des exécutables et l’inventaire signé. Il ne contient pas le commit Git exact de l’arbre utilisé pour produire ces binaires ; ce lien ne peut donc pas être reconstitué à partir de ce rapport seul. Cette réserve empêche de déclarer la provenance reproductible de l’assemblage entièrement close.

Le paquet prototype reste installé. Le certificat public de test reste dans `Cert:\LocalMachine\TrustedPeople`. Aucune clé privée PFX n’a été importée. Ni désinstallation du paquet ni retrait du certificat n’a été effectué ; ces opérations requièrent un mandat distinct. Ce paquet reste un prototype local signé pour test et n’est pas distribuable.

## Suite

Pour poursuivre la qualification MSIX, préparer un mandat distinct pour l’édition NVIDIA, puis un autre pour le parcours Snapdragon avec contrôle Glow. Résoudre séparément la traçabilité de la source avant de qualifier l’assemblage reproductible. Les retours de ces lots seront consignés dans de nouveaux documents, sans réécrire ce bilan.
