Avancée : mesures statiques actualisées et séparation production/tests explicitée.
Blocage : autres outils absents et exécution native [Non vérifié].
Décision : lignes physiques et relations lexicales, sans revendication de validation native.

# Méthode du 31 octobre 2026

Les membres proviennent de `Cargo.toml:1-11`, les dépendances des huit manifestes recopiés dans raw/. La collecte statique initiale ne lance que Git et rg et lit les TOML avec la bibliothèque standard. La mesure Cargo autorisée ensuite fournit metadata et les diagnostics Clippy séparément, sous la garde existante. Les commandes, sorties, erreurs et codes de sortie rg/Git sont conservés. Un code 1 rg signifie aucune correspondance, pas une absence sémantique.

## Sources et unité

`measure.py` lit chaque fichier .rs sous crates/. Les champs path/start/end des JSON et les colonnes de files.tsv et occurrences.tsv sont les références aux sources actuelles. Les comptes sont des lignes physiques, vides et commentaires inclus : total = production + tests. Le script de build partagé `scripts/recette-build.rs:1-7` est décrit séparément, sans le compter trois fois dans les mesures par crate.

Un fichier sous crate/tests/ est entièrement test. Les modules et fragments `#[cfg(test)]`, les fonctions `#[test]`, `cfg(all(test,...))` et leurs sous-modules externes sont classés tests. Les inclusions littérales et la forme concat avec CARGO_MANIFEST_DIR repérées sont rattachées au fichier déclarant ; les fichiers sous src/.../tests/ restent supports de tests. Les champs, paramètres et blocs cfg(test) sont repérés séparément. Les variantes `cfg(feature)`, `cfg(windows)` et `cfg(not(test))` restent hors tests ; les hooks opt-in ne sont donc pas assimilés à des tests exclus. Exemple : `crates/whisper-adapters/src/worker_ipc/runtime/mod.rs:110-137` ; `crates/whisper-adapters/src/worker_ipc/runtime/support.rs:51-102`.

[Non vérifié] Il s’agit d’une classification lexicale des sources, pas de lignes effectivement compilées. Les conditions complexes, les macros et attributs procéduraux ne sont pas évalués. Les limites des champs/blocs tests sont estimées par délimiteurs ; la précision de rustc n’est pas revendiquée.

## Fonctions et modules

Les commentaires Rust imbriqués, chaînes normales/raw et caractères sont masqués en préservant les positions pour retrouver les accolades. Les fonctions nommées avec corps sont relevées de fn à l’accolade finale, signature comprise. Les déclarations de traits sans corps, closures et expansions de macros ne figurent pas dans ce classement. Les variantes cfg restent distinctes. Le top production soustrait les fragments tests ; le top tests est séparé. Ces tailles diffèrent de clippy::too_many_lines.

Les déclarations mod externes/inline et leurs attributs sont inventoriés. La résolution des fichiers applique mod.rs et #[path] ; les include! explicites sont relevés séparément. Les modules inline et inclusions sont des sous-ensembles des fichiers physiques ; ils ne s’additionnent pas aux totaux. Une source incluse ou partagée par plusieurs cibles est comptée une fois physiquement. Exemples : `crates/whisper-desktop/src/ui/model.rs:37-42` ; `crates/whisper-desktop/src/ui/tests.rs:1-7`.

Les relations partent des imports use crate::, y compris arbres d’import multilignes. Elles rapprochent un préfixe du module déclaré le plus long ; cela n’établit pas la résolution des réexportations ou alias. Les relations sous tests sont séparées. [Non vérifié] Cet index ne couvre pas tous les usages via super/self ni les chemins hors use. Les types listés incluent structs, enums, traits et alias déclarés, sans résoudre leurs expansions. Les publics sont des déclarations textuelles ; leur visibilité effective dépend des modules parents et cfg.

## Motifs et frontières

Chaque occurrence demandée est enregistrée avec chemin, ligne, texte brut et contexte test. Les motifs cfg_feature, unsafe, unwrap, expect, TODO_FIXME_HACK, thread_spawn et command_new sont textuels, chaînes/commentaires inclus ; builder_spawn est un complément qui peut inclure des processus. Les sites I/O, processus, threads et GPU des fiches sont des repères positifs ; aucun motif absent ne prouve une absence d’effet indirect.

## Reproduction et contrôles

Scripts documentaires existants actualisés dans raw/, sans dépendance tierce ni installation. Python est lancé avec -B afin de ne pas créer de cache hors de ces sorties. Ordre : measure.py, collect.py, render_flows.py, render_map.py, validate_map.py. Le rendu utilise les responsabilités et fragments factuels stockés dans ce dossier. Aucun script ne modifie le produit. Le contrôle vérifie couverture des fichiers/modules, plages de sources, liens locaux, séparation des comptes et état Git. [Non vérifié] Un contrôle de plage ne suffit pas à prouver qu’une phrase est sémantiquement exacte ; les flux et observations sont fondés sur les corps consultés et conservent leurs limites.

## Mesures Cargo complémentaires

`cargo metadata --offline --locked --format-version 1 --no-deps` et `cargo clippy --offline --locked --workspace --all-targets --message-format=json -- -W clippy::too_many_lines -W clippy::cognitive_complexity` ont terminé avec code 0. Les stdout/stderr et les enregistrements de la garde sont conservés dans raw/. Les huit packages sont confirmés par metadata ; [Non vérifié] `--no-deps` ne donne pas une résolution transitive. Clippy compile/check les cibles de tests, sans les exécuter ; aucune feature supplémentaire de recette n’est activée par la commande et --all-features n’est pas utilisé. Les features effectives des artefacts sont conservées dans le JSON.

Les diagnostics émis sont conservés sans perte ; leur nombre inclut les répétitions de sources compilées dans plusieurs cibles. La déduplication documentaire utilise (lint, fichier normalisé, début, fin, message). Les plages tests sont déterminées par le même index lexical que les comptages, indépendamment de la cible compilée. Les scores Clippy ne sont pas déduits des longueurs physiques.

[Déduction] L’attribution d’un diagnostic à une compilation test/non-test relie les messages compiler-message à l’artefact compiler-artifact suivant de même package, nom et kind de cible dans le flux Cargo. L’artefact porte profile.test ; le diagnostic n’a pas ce champ. Cette attribution documentaire est explicitement distincte des spans et métriques fournis directement par Clippy. Le détail conserve les positions dans le flux et l’artefact associé pour examen. Les fonctions dont la source est hors tests mais dont la compilation est test ont un relevé séparé ; exemple : poll_unified_sessions, score 64 hors tests et 71 en compilation test, `crates/whisper-adapters/src/worker_ipc/runtime/poll_unified.rs:382-1373`.

L’auto-test initial et la première exécution Clippy ont été interrompus par la garde sur des événements .git, d’origine inconnue. Les reprises ont été explicitement autorisées avec la garde inchangée ; ces interruptions restent enregistrées et ne deviennent pas des PASS. L’analyse des mesures passe par analyze_cargo.py ; aucun Cargo supplémentaire n’est nécessaire pour régénérer les tableaux.
