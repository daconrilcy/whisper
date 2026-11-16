# Bilan L2 — édition Turbo

Avancée : gate L2→L3 passée pour le candidat décrit ci-dessous ; édition, ressources et licences inventoriées.
Blocage : aucun blocage d'implémentation L2 ; l'assemblage et la redistribution restent soumis aux vérifications L3/L4 et aux conditions des licences.
Décision : préparer L3 avec ces entrées, sans présenter les essais matériels ou la distribution comme validés.

Le code du candidat est versionné localement au commit `cc42b0f7` sur `codex/cpu-distribution-l2-reprise`.

Le diagnostic de capacités du worker GPU est émis sur stderr et relayé après contrôles de rôle, PID et empreinte. Sur Windows, le fichier est maintenu ouvert en partage lecture pendant le lancement et le contrôle ; en cas d'identité indisponible, le relai est abandonné sans bloquer le worker. La revue indépendante est **CLEAN** après un cycle de correction ; elle ne prétend pas mesurer directement l'image mappée en mémoire.

| Entrée | SHA-256 | Constat |
|---|---|---|
| Worker CPU L1 | `03566517fa8580414770f346cc0a6be9846da45d8c928ead6a32cc10635f782a` | PE AMD64 sans imports CUDA ; empreinte intégrée au desktop. |
| Worker GPU | `d874136c6d9561ca06a80e7c2b150513e10a05e46e37edd4cb28ba6eaabd17a9` | Build portable et PE AMD64/imports CUDA PASS ; architecture CUDA compilée `sm_61`. |
| Desktop Turbo | `af89e1b4d5ce3c40c8c7e1647fd9a7f4dfe5a7aaa9cd0cce2e47f2b190640d15` | Build release et PE AMD64 sans imports CUDA PASS. |
| Modèle fourni séparément | `1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69` | Fichier local vérifié ; remise hors ligne, selon l'[inventaire](inventaire-contrat-installation-turbo.md). |

SelfTest de la garde : PASS, 191 cas positifs et 192 négatifs. N1 sans `CARGO_TARGET_DIR` : 976 tests réussis, 129 ignorés ; le test Windows du fichier protégé a réussi. Les sélections worker CPU, Parakeet CPU et common, leurs contrôles PE, Clippy structurel et `fmt --check` ont réussi. La première commande groupée, lancée par erreur avec Windows PowerShell 5, a été refusée avant Cargo ; les sélections ont été reprises avec PowerShell 7. Un premier `fmt --check` a échoué puis le seul formatage a été corrigé et relu ; les tests et Clippy précèdent ce changement sans effet sémantique. Les builds ont été exécutés hors ligne par la garde ; le desktop émet trois avertissements `dead_code`.

L'[archive de validation](preuves-validation-l2.zip) contient 38 fichiers de journaux et de compilation, avec leurs chemins relatifs ; nombre et SHA-256 de chaque entrée comparés aux originaux. Les [traces d'inclusions natives](preuves-inclusions-natives-l2.zip) sont conservées séparément. Les cibles contenant les trois candidats restent des entrées nécessaires à L3 ; elles ne sont pas supprimées à cette clôture. Le cache partagé est conservé. Les autres cibles de campagne restent conservées tant que leur inutilisation et la rétention de toutes leurs preuves ne sont pas établies ; espace libre après campagne : environ 217 Gio.

Les [notices](index-notices-turbo.md) et [conditions de remise](preparation-assemblage-l3-turbo.md#conditions-concrètes-de-remise-des-runtimes-en-l3) sont inventoriées. L'utilisateur a confirmé ses droits sur le code propre et déclaré une licence Visual Studio Community en usage individuel personnel. [Microsoft](https://learn.microsoft.com/en-us/visualstudio/releases/2022/redistribution) autorise sous les termes de cette licence la remise avec le programme, sans modification, des fichiers Visual C++ redistribuables listés ; les conditions NVIDIA s'appliquent aux DLL CUDA. L3 devra vérifier les fichiers effectivement copiés, leurs notices et conditions et la résolution sur installation propre ; la provenance initiale des DLL CUDA non signées n'est pas authentifiée par leur signature.

Les essais d'inférence CUDA, NVIDIA/Auto, x86-64-v2 exact, Windows sur Arm et la validation de la fenêtre restent différés selon le plan. Aucun paquet installable ni droit de distribution effective n'est déduit de ce bilan.
