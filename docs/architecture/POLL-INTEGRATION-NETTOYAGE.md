Avancée : worktree C:/wt/poll et branche refacto/poll retirés ; ressources ignorées par Git.
Blocage : aucun ; preuves et données conservées hors du worktree.
Décision : nettoyage autorisé terminé, compte rendu figé le 4 novembre 2026 ; aucun push.

# POLL-INTEGRATION — préservation et retrait du worktree

Autorisation utilisateur : retirer le worktree et sa branche fusionnée,
préserver d'abord les preuves, ignorer models/ et sound example/.

**Préservation :** target/, models/ et sound example/ ont été déplacés depuis
C:/wt/poll vers **C:/dev/whisper/target/poll-preserved/**, avec leur arborescence.
Les caches persistant et isolé du worktree restent donc conservés. Les
ressources du dépôt principal et ses deux documents non suivis sont intacts.

Avant retrait : branche ancêtre de main, aucun changement suivi dans le
worktree, aucun agent ou processus de campagne actif. Racines absolues
vérifiées sans jonction ; 181 jonctions de fixtures relevées, toutes internes
au worktree, sans cible externe. Les fichiers sont conservés ; les anciennes
jonctions de fixtures pointant sur C:/wt/poll ne sont plus des racines actives.

Les **7 archives ZIP**, totalisant **10 843 941 octets**, sont identiques
avant/après déplacement par taille et SHA-256. Les **301 fichiers** sous
models/ et sound example/ correspondent aussi aux copies du dépôt principal,
taille et SHA-256 vérifiés. L'inventaire initial du worktree comptait
124 401 fichiers et 89 369 901 188 octets apparents ; ce déplacement sur le
même volume ne constitue pas une mesure de gain physique.

**Retrait :** git worktree remove C:/wt/poll, puis git branch -d refacto/poll
réussis, sans force. Absence du dossier et de l'enregistrement Git vérifiée ;
branche retirée à 65a903bc, déjà fusionnée. Main et le tag avant-fusion-poll
restent préservés. Espace libre final relevé : 560 609 587 200 octets.

**Gitignore :** ajout des règles racine /models/ et /sound example/, validées
par git check-ignore ; git diff --check réussi. Aucun Cargo nécessaire pour
ce changement de configuration et de documentation. Aucun code ou test modifié.

Anciennes preuves target du worktree : consulter désormais
[l'arborescence préservée](../../target/poll-preserved/target/).
Le cache N2 conservé se trouve désormais sous target/poll-preserved/target/n2-native/cargo-home ;
les anciennes commandes citant C:/wt/poll sont historiques, ne pas les réutiliser.
