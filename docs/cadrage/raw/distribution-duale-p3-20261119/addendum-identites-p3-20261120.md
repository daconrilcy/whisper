Avancée : les 42 épingles du candidat concordent ; trois valeurs du relevé d’identité P3 sont corrigées ci-dessous.
Blocage : le ZIP P3 ne contient pas le relevé d’identité, conservé comme fichier distinct ; le ZIP candidat de remise reste bloqué par les notices P4.
Décision : lire cet addendum avec la preuve P3 figée et le relevé d’identité externe.

# Addendum de traçabilité P3 — 20 novembre 2026

Le compte rendu P3 est conservé tel qu’archivé. Cet addendum corrige les références de fichier et trois SHA-256 mal recopiées dans `identites-p3-20261120.txt`. Le fichier concerné a le SHA-256 `17ba742469f38b6c864123dfb387800b005d290944fd5da954278ad46ebb696c`. Le ZIP `preuves-p3.zip` a le SHA-256 `41b9cc841d1ad8ff7844f04a21e826ef0fe0087ff6cbbbd1caab7405e96358c` et contient 10 entrées de diagnostics et de publications ; le relevé d’identités est un fichier distinct, non une entrée du ZIP.

Les trois valeurs fautives du relevé et les valeurs attendues sont :

| Fichier | Valeur écrite dans le relevé | Valeur vérifiée dans le candidat et épinglée par `DUAL_FILES` |
|---|---|---|
| `native/gpu/nemo_speech_asr_c.dll` | `c082987836d5207bf79f52ba645d3d2ed1c150616d6405c09482f4c61195ebb` | `c082987836d5207bf79f52ba53d46505641c150616d6405c09482f4c61195ebb` |
| `native/cpu/vcomp140.dll` | `55aba23cdcd6484fbb06f4155b8cab75adfce7a881f10afd0c49457165e677164` | `55aba23cdcd6484fbb06f4155b8ca75adfce7a881f10afd0c49457165e677164` |
| `native/gpu/vcomp140.dll` | `55aba23cdcd6484fbb06f4155b8cab75adfce7a881f10afd0c49457165e677164` | `55aba23cdcd6484fbb06f4155b8ca75adfce7a881f10afd0c49457165e677164` |

Une relecture SHA-256 du candidat P3 et comparaison à la table `DUAL_FILES` a donné 42/42 correspondances, sans écart ni fichier manquant. Les valeurs attendues ci-dessus proviennent de cette comparaison. Le chemin correct du fichier d’identités est `identites-p3-20261120.txt` ; le compte rendu P3 utilisait le nom abrégé `identites-p3.txt`. Aucun fichier du candidat ni de l’archive de preuve n’a été modifié.

Le relevé textuel original conserve les trois erreurs comme trace historique ; utiliser cet addendum pour ces valeurs. L’addendum ne modifie pas les conclusions de routage P3 et ne ferme aucune exigence P4.

Complément P4 : [préflight identité, sources et notices](../distribution-duale-p4-20261120/preflight-identite-notices-p4-20261120.md).
