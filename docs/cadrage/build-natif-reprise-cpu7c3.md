Avancée : correctif NativeBuild qualifié et CLEAN ; espace précédent préservé, jonction retirée avec cible inchangée.
Blocage : cpu7c3 refusé avant compilation ; appel interne de la garde par chemin relatif incompatible avec le contrôle du parent.
Décision : arrêt respecté ; reprise proposée par le point d'entrée public absolu, sous arbitrage utilisateur.

# Reprise CPU — refus du parent de cpu7c3

Après les gates et la revue [CLEAN du correctif](build-natif-control-qualification-bilan.md), la session principale a lancé `pwsh -File ci/guard-native.ps1 -Mode Run -Profile NativeDllCpu -LotRoot target/native-build -NativeBuildId cpu7c3`. C'est une erreur d'invocation de la session principale : `ci/build-native.ps1:256–257` recherche le chemin **absolu** du script de garde dans la commande de son parent. Le chemin relatif ne satisfait pas ce contrôle. La règle est conservée, aucun correctif de code n'est nécessaire pour cet arrêt.

**Preuves.** `guard-result.json` : FAIL, enfant 153196, sortie 1, du 5 novembre **12:31:14,998 au 12:31:20,161 Paris** ; stderr : « Native build parent is not the pinned guard script. » Le refus précède Get-Tools, la création du manifeste de build et tout appel MSVC/CMake/bootstrap. Pas de DLL produite, pas de nouvelle compilation native consommée : **deux CPU commencées historiquement sur quatre, deux restantes** ; aucune tentative cpu8.

La préparation de l'espace fixe a réussi : `junction-preparation.json` porte `REMOVED_TARGET_PRESERVED`, terminé à 12:31:12,506 Paris ; le contenu précédent est dans `target/native-build/guard/NativeDllCpu-cpu7c3/previous-workspace`. Aucun contenu supprimé ni fusionné ; la cible de la jonction est inchangée. L'enfant refusé est terminé (PID absent vérifié). La durée de préparation inclut les parcours stricts avant/après ; aucun de ces contrôles n'a été allégé.

**Commande de reprise proposée**, après arbitrage :

```powershell
& (Join-Path $PSHOME 'pwsh.exe') -NoLogo -NoProfile -File 'C:\wt\build-natif\ci\build-native.ps1' -Variant cpu -BuildId cpu7c4
```

Ce point d'entrée construit lui-même l'appel absolu à la garde (`ci/build-native.ps1:405–408`). Puis cpu8 seulement si PASS, arrêt au premier échec et borne du complément toujours **13:00 Paris / 12:00 UTC**. L'arbitrage demandé dans la session est en attente à la rédaction ; aucune reprise implicite.

**Archive vérifiée** : [parent-stop.zip](../architecture/raw/build-natif-cpu7c3/parent-stop.zip), 8 fichiers + manifeste, 4 746 octets, SHA-256 `901f71190f63d00dd294e75feb40ed15415140ddf65fcdb983b89e991fc9ea6b`. Nombres, tailles et SHA contrôlés après réouverture du ZIP ; copie persistante de même empreinte. Les anciens bilans/archives et les espaces protégés restent intacts. Aucun remplacement de DLL, nettoyage, push ou fusion.
