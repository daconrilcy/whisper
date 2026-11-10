# REFACTO-POLL-PARAKEET — bilan figé

Avancée : T1–T6 et S2 terminés ; le refactoring nettoyage est fusionnable seul.
Blocage : la recette dans la fenêtre Windows reste à faire par l’utilisateur.
Décision : arrêt au checkpoint nettoyage ; aucun polling refactor, merge ou push.

**Périmètre.** Base `4c04818c`, branche `refacto-poll-parakeet`. `run_cleanup_plan_locked` : complexité Clippy JSON 32→3 ; lignes 500→102. Dix helpers extraits, scores 1–6 ; closures 1–2, toutes fonctions/closures ≤15. `poll_parakeet_worker` demeure à 30, inchangé et reporté. Aucune dépendance, primitive système, visibilité publique ou comportement produit ajouté.

| Extraction (commit) | Complexité |
|---|---:|
| validation successor records (`7e9ff142`) | 1 |
| ouverture index vérifié (`c2696b06`) | 1 |
| validation fichiers successor (`6bcca39f`) | 5 |
| adoption conteneurs absents (`d12c0b08`) | 6 |
| conservation ancêtres (`bc24b924`) | 6 |
| vérification objets cibles (`25e4ac6c`) | 5 |
| capture entrées non sélectionnées (`08202f7a`) | 5 |
| retrait objet cible (`54632cb3`) | 1 |
| retrait objets ordonnés (`9b04134e`) | 4 |
| retrait conteneurs (`443e6a8a`) | 5 |

**Preuves.** Caractérisation/références gelées avant code : 41 traces nettoyage, 37 polling ; aucune régénération ensuite. Traces identiques après chaque extraction ; 66 identités N1 ajoutées. N1 final ×3 : chacun 944/0/129, mêmes 1073 identités ; fmt vert. Clippy qualité 338 avertissements contre 340 avant, aucun nouveau dans les fichiers touchés. Workers CPU 12/0/9, Parakeet CPU 26/0/11, common 11/0/1 ; PE 4 positifs/15 négatifs, témoin réel non exécuté. S2 filtre exact ×50 : 50/50, zéro échec ; cause historique non démontrée. Revue indépendante finale **CLEAN**, base `4c04818c`, candidat `443e6a8a`, documentation `0d9925da`.

N2 PersonalUnified : premier appel arrêté par la garde pendant compilation (événement Created/Changed sous une ancienne racine N1 ; attribution inconnue). Après accord, reprise unique sous garde inchangée : 4/4 PASS. Les huit TXT/SRT bruts sont identiques octet par octet par cas à la référence gelée. WHEA avant/après : 1, RecordId 527992. Build release Personal vert ; remise contient desktop et deux workers Turbo seulement. Fenêtre et recette humaine : **NON EXÉCUTÉES**.

**Verrou et disque.** La revue confirme verrou externe conservé pendant toute la durée originale de `run_cleanup_plan_locked`; aucune frontière déplacée. Handles d’index, ancêtres et cibles restent détenus aux mêmes étapes. Ordre `map.remove` avant `mark_delete`, erreur et écriture finale inchangés. Les racines d’exécution sont sous `target/refacto-poll-parakeet` et `target/n2-native`; archive ZIP vérifiée : 6683 membres, SHA-256 `D5CF2F87205A6B59846357FC5828C898B914652880A2A367542EEE676C7B0F12`. Espace libre début : 470551474176 octets ; avant reprise N2 : 429322694656 octets. Taille finale des racines et libre de clôture consignés au checkpoint T6.

**Défauts observés, non corrigés.** Cas historique Q07 (message terminal d’une tentative ancienne) et C07 (adoption d’un chemin absent) restent tels quels. Les erreurs de compilation des fixtures pendant le travail sont archivées, corrigées uniquement dans les tests avant gel ; aucun défaut produit corrigé.

**Dette / recette opérateur.** Pas de refactor polling. L’utilisateur doit ouvrir `deliverables/refacto-poll-parakeet/whisper-desktop.exe) : (1) import Parakeet CPU, confirmer moteur, TXT/SRT lisibles et source intacte ; (2) micro GPU 30 s puis Stop, confirmer blocs et TXT/SRT/WAV ; (3) courte nouvelle session, confirmer aucune sortie héritée ; (4) import long, annuler après début réel, confirmer temporaires de cet import disparus et sorties publiées précédentes intactes. Rapporter les chemins temporaires et diagnostics exacts. Le dossier remis utilise les workers Parakeet, DLL et modèles vérifiés dans l’installation stable ; il n’est pas autonome.

**Rétention.** Aucun fichier supprimé par l’agent. Les commandes proposées après inventaire vérifié sont limitées aux racines temporaires et à la cible Cargo propre au lot ; préserver `target/cargo-home-shared`, la ZIP, worktree, installation stable et remise. Voir le relevé de clôture T6.

Suivi post-T6 : [inventaire et phase T7](REFACTO-POLL-PARAKEET-SUIVI-POST-T6.md).

Suivi des validations T10 : [REFACTO-POLL-PARAKEET-SUIVI-T10.md](REFACTO-POLL-PARAKEET-SUIVI-T10.md).
