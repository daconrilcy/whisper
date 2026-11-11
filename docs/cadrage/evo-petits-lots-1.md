Avancée : cadrage de PETITS-LOTS-1 établi sur la base 267e820f.
Blocage : les preuves actuelles ne rendent admissibles ni le point 2 ni le point 3.
Décision : conserver les conditions d’arrêt demandées ; ne pas modifier les gardes.

# Cadrage — PETITS-LOTS-1

**État :** cadrage proposé, limité à un lot de maintenance du dépôt. Il ne change aucun comportement de transcription.

## BESOIN ET PÉRIMÈTRE

Sur le poste Windows mono-utilisateur, garder les binaires de remise hors du suivi Git, rendre explicite la configuration native seulement si cela documente une valeur déjà effective, et isoler les changements de formatage des changements fonctionnels. Le besoin concerne le dépôt et les pratiques de livraison, pas un parcours utilisateur Whisper.

Base observée : branche `petits-lots-1`, worktree `C:/wt/petits-lots`, HEAD `267e820f`, propre au contrôle initial. Le dépôt principal n’a pas été modifié. Le worktree a été créé selon la commande demandée.

| ID | Résultat vérifiable | État observé / condition d’arrêt |
|---|---|---|
| CA-01 | Les 9 EXE ne sont pas suivis; aucun script/test ne consomme `deliverables/`; l’ignore est effectif, les fichiers locaux sont conservés; commande de déplacement proposée seulement. | À la base, `.gitignore:3` contient déjà `/deliverables/`, `git ls-files -- deliverables/` est vide, aucune référence applicative/script ciblée. Les 15 documents sous `docs/` qui mentionnent ce chemin sont des références historiques/manuelles. Point déjà satisfait; pas de commit vide. Si un script ou un test en dépend au préflight, arrêter le point. Réexaminer les EXE locaux sans inférer leur présence depuis le worktree. |
| CA-02 | N’ajouter `GGML_NATIVE=OFF` qu’après preuve d’une valeur effective OFF en CPU et GPU et absence de modification de garde/empreinte/liste/requalification. | Refusé à la base : `native/build-config.json` fixe ON; les quatre manifestes CPU r2 et GPU r5 indiquent `effectiveOptions.GGML_NATIVE=ON` et le même hash de configuration. Le changement vers OFF n’est pas une explicitation. Abandonner ce point; ses auto-tests conditionnels ne sont pas exécutés. Si une autre preuve admissible modifie ce constat, faire relire par un relecteur générique; les auto-tests conditionnels restent après le N1 du point 3. |
| CA-03 | Le diff rustfmt est uniquement de forme, revu comme tel, puis build/N1 avant commit; le commit blame est distinct. | Voie interdite par la garde actuelle : `ci/guard-level1.ps1` n’accepte que `cargo fmt --check`, et refuse `cargo fmt --all` comme `cargo fmt --all --check`. Modifier la garde est explicitement interdit. Abandonner le point selon la règle du lot; aucun commit blame sans commit de formatage. Une voie future admissible exigerait `rust_review_coordinator`, limité à la mise en forme, et revalidation des critères. |
| CA-04 | `ci/guard-level1.ps1 -SelfTest`, puis N1 `cargo test --no-fail-fast` sans `CARGO_TARGET_DIR`; noter résultat réel, attendu 944/0/129. | Préflight requis avant toute commande. Le SelfTest est distinct : il exécute ses sondes metadata et sort avant les préconditions du N1 complet; son résultat doit être rapporté séparément et ne prouve pas le N1. Pour le N1, la garde peut associer l’absence de variable à la cible canonique ou à `-FreshTarget`; cette dernière est distincte du chemin canonique requis par `guard-native`. À la base, la cible canonique et le cache persistant, y compris `registry/src`, sont absents. Le N1 complet est bloqué par le cache offline absent, quelle que soit la cible; `fetch --locked` seul ne suffit pas et aucun fetch n’est demandé. Réexaminer sans créer de mécanisme. |
| CA-05 | Rédiger `docs/architecture/PETITS-LOTS-1-BILAN.md` en une page, avec statut et preuves par point, résultat N1 réel, commande Move-Item et nettoyage seulement proposés. | À produire à la clôture d’une future exécution; ne pas déduire des preuves non recueillies. |

## CONTRAINTES ET PREUVES

- L’index Git, les fichiers locaux, les manifestes de build et les essais sont des états distincts. Une condition non prouvée ne devient pas une réussite.
- La garde Rust niveau 1 n’autorise pas les deux commandes de formatage demandées; aucune exécution directe hors garde n’est conforme aux règles du dépôt.
- `ci/guard-native.ps1` exige la cible Cargo canonique relative au worktree, déjà existante. Un N1 avec `-FreshTarget` n’établit pas cette précondition. Ne pas créer de copie, jonction ou mécanisme alternatif.
- Sources vérifiées dans le worktree à `267e820f` : `AGENTS.md`; `ci/guard-level1.ps1`; `ci/guard-native.ps1`; `ci/build-native.ps1`; `native/build-config.json`; manifestes de `C:/wb/native-qualif/reprise-2/cpu/qualif-r2-cpu1,cpu2` et `reprise-5/gpu/qualif-r5-gpu1,gpu2`; recherche de références `deliverables/` dans les chemins demandés.
- Aucun Cargo, test, build, script de garde ou construction native n’a été exécuté pendant ce cadrage.

## RISQUES / DÉCISIONS

- **R1 — Formatage impossible sous les permissions actuelles.** La garde refuse les commandes exactes et sa modification est exclue. Selon l’arrêt par point, CA-03 est abandonné. Une future demande de changement de garde exigerait un mandat séparé; ce cadrage ne le propose pas comme action autorisée.
- **R2 — N1 non prêt au préflight.** Les sources du registre Cargo requises ne sont pas présentes dans ce nouveau worktree; c’est la précondition immédiate du N1 complet. La cible canonique requise par le SelfTest natif manque aussi, mais le SelfTest de `guard-level1` suit une voie distincte et doit recevoir son propre préflight et résultat. Ne pas copier des caches depuis le dépôt principal ni utiliser `fetch` sans accord précis. Le N1 final est `NON EXÉCUTÉ` si les prérequis ne sont pas satisfaits.
- **R3 — Pas de commits vides.** CA-01 est déjà conforme; CA-02 et CA-03 sont abandonnés sur preuve/voie refusée. Aucun commit de point ne doit être fabriqué sans changement admissible.
- **R4 — Commande de déplacement.** À proposer avec `Move-Item -LiteralPath 'C:\dev\whisper\deliverables' -Destination 'C:\wb\deliverables'` derrière un test de non-existence de destination; ne pas l’exécuter. Le préflight devra confirmer l’existence de la source, la destination et le bon refus de collision.

## PRÉPARATION DU PLAN

Le plan peut organiser un préflight réversible, les arrêts par point, les validations admissibles et le bilan à `docs/architecture/PETITS-LOTS-1-BILAN.md`. La relecture de configuration des points 1 et 2 est générique; celle du point 3 relève de `rust_review_coordinator` et se limite à vérifier que le diff ne change que la mise en forme. Le statut « prêt à exécuter » demeure conditionné aux prérequis de CA-04; les critères utilisateur n’autorisent aucun contournement des gardes. Aucune construction native ni N2 n’est dans le périmètre. La planification ne vaut pas autorisation d’exécuter de nouveaux mécanismes ou d’élargir le périmètre.
