Avancée : planification autorisée ; proposition du rédacteur dédié en cours.
Blocage : aucun pour rédiger le plan ; implémentation et campagnes non autorisées.
Décision : partir du design accepté D-PARA-UNIFIED-01 v0.2, sans changer ses contrats.

# Plan d'implémentation — transcription Parakeet unifiée

## 1. Identité, autorité et base

- ID : P-PARA-UNIFIED-01 ; version : v0.1 ; statut : DRAFT.
- Design parent : `docs/cadrage/evo-parakeet-transcription-unifiee.md`, D-PARA-UNIFIED-01 v0.2, accepté pour planification au commit `d8d170f9`.
- Revue du design : `docs/cadrage/revue-parakeet-transcription-unifiee.md`, CLEAN sur `42c8c16d`, clôture documentaire `03506050` ; acceptation S-USER-07 ajoutée sans changement technique.
- Base produit : `a483e176759fcd2f2340f221470fe09f20ec8545`, branche `evo-startup-perf`.
- Accord utilisateur de cette mission : « ok tu peux maintenant rediger le plan d'implementation à partir de ces données de cadrage. »
- Autorisé : planification documentaire par l'agent dédié, revue indépendante, rédaction par la session principale et commits locaux dans `docs/cadrage/`.
- Non autorisé : code produit, compilation, tests, exécution audio/GPU/replay, nouvelle campagne, installation, élévation, écriture hors dépôt ou push. La validation du plan ne sera pas assimilée à une autorisation d'implémenter un lot.
- Livrables de cette mission : ce plan et `revue-plan-parakeet-transcription-unifiee.md`. Pas de nouveau railguard, outil de persistance des agents, manifeste d'empreintes ou campagne documentaire.
- Budget documentaire : dix tâches d'implémentation maximum au total, regroupées en lots ; deux cycles revue/correction maximum. Toute extension structurelle revient à l'architecte et aux autorités du design.
- État initial vérifié : fichiers suivis propres, `models/` et `sound example/` non suivis préservés ; 114,57 Gio libres sur C: ; aucune commande Cargo exécutée.
- Sources : AGENTS courant, design parent, revue, règles et contrats du pack central lus à cette reprise. Les anciennes dérogations de campagnes ne sont pas transférées à ce plan.

## 2. Contribution du rédacteur et carte du plan

Owner du contenu : `rust_plan_writer`, contribution attendue en lecture seule. Auteur des fichiers : session principale.

Le plan devra couvrir : lots et tâches, dépendances DOCUMENT/CODE/EXECUTION, chemins directs et indirects, scénarios et données, critères et mesures, préflight et commandes futures sous garde, détails à résoudre avant les lots dépendants, récupération proportionnée et limites NOT RUN.

## 3. Lots, tâches et vérification

Contribution attendue du rédacteur dédié.

## 4. Readiness

PENDING REVIEW. Le rédacteur ne déclare pas son propre CLEAN/READY. Acceptation du plan, autorisation de lot, revue de code, validation produit et publication resteront distinctes.
