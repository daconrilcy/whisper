# Hôte documentaire Whisper

`invoke.ps1` exécute le helper `predev_host.py` dans un conteneur Docker Linux. Il accepte uniquement les propositions et policies situées sous `C:\dev\whisper\docs\predev\L04-PREFLIGHT`. Le dépôt est monté en lecture seule ; seul `docs/predev/L04-PREFLIGHT` est monté en écriture. Le pack documentaire gelé est monté en lecture seule. Le conteneur utilise une image par digest, sans réseau, avec système racine en lecture seule, capacités Linux retirées et `no-new-privileges`.

Le wrapper vérifie avant chaque action le SHA du manifeste qualifié, le digest du pack, les hashes des 24 fichiers et l'absence de fichiers supplémentaires. Le helper conserve ses contrôles de policy sourcée, chemins exacts, hashes de base, candidats immuables et reçus. Le wrapper n'attribue aucun verdict de revue ou statut READY.

## Utilisation

```powershell
& 'C:\Users\cyril\.codex\tools\whisper-doc-host\invoke.ps1' -Action Probe
& 'C:\Users\cyril\.codex\tools\whisper-doc-host\invoke.ps1' -Action Audit -Proposal 'C:\dev\whisper\docs\predev\L04-PREFLIGHT\<proposal>.json' -Policy 'C:\dev\whisper\docs\predev\L04-PREFLIGHT\<policy>.json'
& 'C:\Users\cyril\.codex\tools\whisper-doc-host\invoke.ps1' -Action Apply -Proposal 'C:\dev\whisper\docs\predev\L04-PREFLIGHT\<proposal>.json' -Policy 'C:\dev\whisper\docs\predev\L04-PREFLIGHT\<policy>.json'
& 'C:\Users\cyril\.codex\tools\whisper-doc-host\invoke.ps1' -Action CheckCheckpoint -Pending 'C:\dev\whisper\docs\predev\state.pending.<version>.json'
& 'C:\Users\cyril\.codex\tools\whisper-doc-host\invoke.ps1' -Action CheckCheckpoint -Pending 'C:\dev\whisper\docs\predev\state.pending.<version>.json' -Lot L-WHISPER-04
& 'C:\Users\cyril\.codex\tools\whisper-doc-host\invoke.ps1' -Action PromoteCheckpoint -Pending 'C:\dev\whisper\docs\predev\state.pending.<version>.json' -Lot L-WHISPER-04
```

La policy doit porter `roots:["/workspace/docs/predev/L04-PREFLIGHT"]`, une source d'autorisation humaine existante et hashée sous cette racine, l'acteur proposant distinct de l'hôte, et les chemins documentaires exacts que cet acte autorise. La proposition doit porter le digest du pack `0b8f4184cbe5097e220c545de472f0f45e6dc36f607e365136fef660205389fb`. Les chemins d'entrée du wrapper sont traduits vers `/workspace/docs/predev/L04-PREFLIGHT/...` ; les chemins internes du protocole restent ceux du conteneur.

Exécuter `Audit` puis `Apply` sur les **mêmes octets** de proposition et policy. Conserver la sortie JSON et vérifier le reçu et le manifeste. `CheckCheckpoint` utilise un montage en lecture seule de `docs/predev`. `PromoteCheckpoint` donne accès en écriture à `docs/predev` pour un pending direct de ce dossier. Si ce pending est en phase `IMPLEMENTATION`, **seul** `-Lot L-WHISPER-04` est accepté et toutes ses tâches pendantes doivent appartenir à L04. Le contrôle du lot et la promotion utilisent une copie privée des mêmes octets dans le conteneur. Un checkpoint DESIGN/PLANS peut être promu sans lot pour conserver un brouillon contrôlé ; cela n'autorise pas la reprise de code. Relancer `CheckCheckpoint -Lot L-WHISPER-04` avant toute reprise de code. La qualification ci-dessous porte sur le mécanisme ; les documents D23/P29 exigent encore leurs propositions, autorisations, revues et gates propres. Pour `PromoteCheckpoint`, toutes les promotions canoniques doivent passer par ce wrapper ; les invocations directes du contrôleur et les autres écrivains de renommage doivent rester arrêtés pendant la promotion. Le verrou `O_EXCL` sérialise les appels au wrapper ; le handle Windows bloque les écritures en place ordinaires pendant l'échange Docker. Les écrivains de renommage hors wrapper ne sont pas techniquement exclus et sont hors du contrat qualifié.

Voir [QUALIFICATION.md](QUALIFICATION.md) pour les preuves et limites.
