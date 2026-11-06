Avancée : correction committée, N1 774 réussis/0 échec/128 ignorés, revue CLEAN.
Blocage : aucun pour ce correctif ; limites natives distinctes maintenues.
Décision : clôturer la correction N1 et remettre le binaire, sans fusion ni push.

# Correction N1 — clôture du 6 novembre 2026

Mandat accepté « Je t'autorise », versionné dans `155cd6aa` ; baseline produit `671e979e`, correction `a2ef198d`. Les quatre tâches du mandat sont réalisées : cadrage, implémentation déléguée, vérifications, revue/remise. Le bilan précédent reste figé et reçoit seulement le lien vers ce complément.

**Cause :** trois tests simulaient les workers mais ouvraient réellement le microphone Windows. Ces sources étaient identiques à `origin/main` (`48827173`) ; cela n'autorisait pas à considérer le N1 rouge comme fusionnable.

**Changement :** seuls `crates/whisper-adapters/src/direct_live.rs` et `direct_live/tests.rs` sont modifiés, 68 insertions/15 suppressions. Injection privée FnOnce dans le démarrage commun ; la production conserve `start_direct_with_gain`. Les cinq démarrages des trois scénarios utilisent une capture synthétique avec gain, sans supprimer ni affaiblir les assertions. Le scénario Stop vérifie en plus 16 000 échantillons non silencieux et leur PCM strictement identique après repli CPU. Aucun ignore, skip, retour anticipé, dépendance, feature ou unsafe ajouté ; aucune configuration Windows changée.

| Vérification | Résultat |
|---|---|
| Trois tests exacts Auto/repli/Ready/Stop | Chacun 1 réussi, 0 échec, 0 ignoré |
| N1 complet sans filtre, 59 résumés | **774 réussis, 0 échec, 128 ignorés** |
| Format, diff, Clippy adapters all-targets | Codes 0, avertissements existants |
| Build release desktop personal-install | Code 0 ; 28,43 s |
| Revue indépendante | **CLEAN**, une revue, aucune correction requise |

N1 exécuté par la garde avec `-WithoutCargoTargetDir`, `test --offline --locked --no-fail-fast`, dans un shell dédié : variable absente dans l'enfant, cible partagée effective vérifiée. Les trois noms concernés apparaissent bien `ok`. La référence 722/0 rapportée pour main n'a pas été réexécutée ici. Reviewer indépendant `/root/review_loop/reviewer`, via coordinateur, zéro constat/requête/lacune restant ; aucune réussite déduite du seul rapport d'implémentation.

**Remise :** `deliverables/inspect-micro-demarrage/correction-n1/`, trois EXE ; anciens dossiers avant/après inchangés. Desktop 15 305 216 octets, SHA-256 `c3da165351d27f76ba0bd898dc1b090c4a31cc24caeae44e9dc43ddece464d8a`. Mêmes workers Turbo, copies SHA et PE des trois EXE réussis ; 20 ressources installées revérifiées conformes. Ressources Parakeet/modèles toujours chargés depuis l'installation personnelle stable.

**Limites :** sélections worker-cpu/worker-parakeet-cpu et validation native/Windows complète NON EXÉCUTÉES ; distinctes du N1 par défaut désormais vert. Aucun moteur suspendu ou worker livré reconstruit, aucun téléchargement ; libfvad seule compilation C autorisée. Pas de nouvelle mesure de performance revendiquée pour ce correctif.

**Preuves/rétention :** [archive](raw/correction-n1-micro-synthetique.zip), 34 journaux, arborescence et chaque SHA vérifiés ; ZIP SHA-256 `f79bfad9bc3c97357f3f2e9b2a413c5dbf3fc1228b30635375ee4297d97b4efc`. Cible/cache partagés préservés, aucun cache/cible isolé à supprimer ni nettoyage manuel ; environ 431 Gio libres. Commits locaux, binaires non suivis, aucune fusion ni push.

[Complément : validation des sélections workers CPU et Parakeet CPU](validation-workers-inspect-micro-demarrage.md).
