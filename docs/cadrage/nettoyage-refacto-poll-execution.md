Avancée : autorisation de suppression reçue ; contrôles préalables réussis le 7 novembre 2026 à 21 h 47 ; 457 dossiers toujours présents.
Blocage : la commande de suppression a été rejetée par le contrôle automatique de l'outil avant lancement, avec le motif « blocked by policy ».
Décision : aucune suppression par l'agent ; remise de la commande à l'utilisateur conformément à l'AGENTS.md, sans push.

# Refacto-poll — remise du nettoyage autorisé

L'utilisateur autorise dans la session courante : « Tu peux maintenant supprimer les 457 dossiers recensés, après les contrôles prévus, en conservant les preuves, les livrables et les caches partagés. Sans push. » Le manifeste a été approuvé dans `07147012`. Cette autorisation lève la restriction de préparation ; le présent document complète la [proposition](nettoyage-refacto-poll.md), sans réécrire les bilans figés.

L'outil a refusé la commande complète avant de créer le processus PowerShell. Aucun préflight ni aucune suppression de cette tentative n'a démarré. La partie préalable a ensuite été exécutée séparément, **en lecture seule**, de 21:46:25 à 21:47:07 : **PASS**. Aucune commande de suppression n'a été réessayée avec un autre mécanisme. La clause de l'[AGENTS.md](../../AGENTS.md), section « Règle permanente des campagnes locales », prévoit : « Si l'outil refuse la suppression, l'agent produit la liste exacte des chemins, leur taille, la vérification d'archivage des preuves et la commande à exécuter ; l'utilisateur supprime, puis l'agent vérifie l'absence des chemins. »

- [Liste exacte](nettoyage-refacto-poll-dossiers.csv) : **457 dossiers**, 122 432 fichiers, **395 913 712 885 octets**, soit **368,723 Gio logiques** ; aucune suppression.
- [Manifeste approuvé](nettoyage-refacto-poll-copies.csv) : les 18 303 copies et audios sont inclus dans l'autorisation.
- Les chemins canoniques, l'absence de jonction dans la sélection, les tailles, nombres et dates de fichiers concordent ; les 3 770 noms de liens physiques restent internes à la sélection. Les 16 jonctions exclues et leurs cibles restent hors suppression.
- Les trois archives et leurs 77 415 membres au total ont été relus ; les SHA-256 et les nombres de membres correspondent à la proposition. Les preuves actuelles sélectionnées correspondent aux membres archivés. Les deux copies des archives historiques dans le worktree ont aussi les SHA-256 attendus.
- Aucun processus de compilation, Whisper ou garde pertinent détecté ; agents précédents terminés et autre session refacto-poll inactive. Les ressources protégées déclarées sont présentes, notamment les livrables et les cinq caches partagés/persistants. Aucun fichier de résultats de suppression n'a été créé.
- Espace libre observé : **95 724 077 056 octets, soit 89,150 Gio**. Il ne s'agit pas d'un résultat après nettoyage.

## Commande à exécuter par l'utilisateur

Dans **PowerShell 7.2 ou ultérieur**, sans campagne ni application Whisper active, copier le bloc suivant. Il charge les commandes déjà préparées, active les 457 dossiers autorisés, puis **refait tous les contrôles avant suppression réelle**. Il ne supprime pas les racines N1/N2 ni les parents des chemins recensés. Arrêt sur toute erreur ; en cas de suppression partielle, conserver la sortie et ne pas relancer aveuglément.

```powershell
$ErrorActionPreference = 'Stop'
$rapport = Get-Content -LiteralPath 'C:\dev\whisper\docs\cadrage\nettoyage-refacto-poll.md' -Raw
$commandes = [regex]::Match($rapport, '(?s)```powershell\r?\n(.*?)\r?\n```').Groups[1].Value
if (-not $commandes.Contains('$Executer = $false') -or -not $commandes.Contains('$InclureCopies = $false')) {
    throw 'Commandes différentes de la préparation : arrêt.'
}
$commandes = $commandes.Replace('$Executer = $false', '$Executer = $true').Replace('$InclureCopies = $false', '$InclureCopies = $true')
& ([scriptblock]::Create($commandes))
```

Ce bloc de remise a été vérifié syntaxiquement, **pas exécuté par l'agent**. Après le retour de l'utilisateur, vérifier l'absence des 457 chemins, les archives, les livrables, les caches et les exclusions, puis consigner l'espace libre réellement observé et le résultat dans le compte rendu de nettoyage. Aucune réussite de suppression n'est anticipée ici.
