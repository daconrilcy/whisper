# Contrat checkpoint de lancement — L-WHISPER-04 / P28

Préconditions à préparer après CLEAN PLANS exact P28; ce document n’est pas un state exécuté. State canonique docs/predev, code root C:\dev\whisper.

## Lignée, reprise, lot

DESIGN D22 et PLANS P28 doivent être READY, liés aux manifests/revues CLEAN exacts; phase IMPLEMENTATION et pending séparé. L04 reste `planned`, avec exactement les quinze paths de 01_LOTS.md et requires_details DETAIL-P03, Q-04, Q-06, Q-09. Conserver L00–L03 completed, leurs sorties/ledgers/proofs, notamment le prérequis EXECUTION L03 avec preuve canonique courante. L05→L04 reste EXECUTION. Inclure les transports raw contributions triés et hashés.

## Autorisation

Référencer la capture exacte `USER_AUTHORIZATION_L04.md` (réponse « Oui, autoriser les deux »). Enregistrer `AUTH-IMPLEMENTER-L04`, rôle implementer, déléguée par AUTH-USER avec `delegation_source` et SHA de cette capture. `scope.lot_ids` contient L-WHISPER-04; `scope.paths` contient les quinze paths exacts triés. L’autorité lot et les paths doivent correspondre au lot et à `preconditions.authorization`; aucune portée globale ou path wildcard.

## Railguard et détails

Conserver railguard actif, proposition, attestation, copie active et SHA observés. preconditions.railguard.activation=`active`; référence/hash vers copie active. Ne pas déclarer not_applicable. DETAIL-P03/Q-04/Q-06/Q-09 restent answered, preuve/transport et autorité attribués; L03 demeure completed avec preuve qui existe sous docs/predev.

## Baseline D0

Le manifeste DESIGN D22 contient dix snapshots existants au HEAD D0 `ffd93125604be5dc6439587b7edad4f420f5cccc`; cinq créations futures sont absentes. `code_state.baseline` contient dix entrées `{path,sha256,snapshot,revision,dirty_hash}`; `snapshot` référence exactement l’entrée candidate D22 manifestée. `current` contient les mêmes dix paths et hashes lus sous `--code-root C:\dev\whisper`, égaux à baseline. `ledger` est vide au début de L04. Les cinq absences se prouvent dans repository_evidence, jamais par une entrée/hash inventée.

Capturer une preuve datée scoped : HEAD/branche, status/diff sur quinze paths, hashes dix fichiers, absences cinq, comparaison aux snapshots D22, changements hors scope séparément. Définir `dirty_hash` comme SHA-256 d’un JSON UTF-8 canonique (clés triées, séparateurs compacts, LF final) des observations précises; conserver payload et algorithme. Une reconstitution ultérieure doit comparer les dix hashes, les cinq absences et le diff scoped à D22 et le déclarer explicitement. Ne pas conclure que le worktree global est propre.

`preconditions.drift.classification=none`, `previous_lots=[]` signifie baseline L04 nouvellement capturée et confirmée après les lots antérieurs; joindre cette preuve d’égalité. repository_evidence inclut preuve inventaire, identité courante, absences, détails/railguard et freeze effectif. Toute dérive de contenu inexpliquée suspend le lancement.

## Racines et contrôles

Les chemins `current`/CODE se résolvent sous `--code-root`; snapshots DESIGN, manifests, ledgers et preuves sous `--root docs/predev`. La copie canonique du manifeste D22 sous `docs/predev/candidates/D-WHISPER-22/manifest.json` est byte-equivalente à l’officielle et doit vérifier avec cette racine. Ne pas modifier D22 pour raccorder les racines.

Après revue P28, valider manifests et state pending sans --lot, recapturer l’environnement, exécuter `check-state --lot L-WHISPER-04`, comparer le pending stable, promouvoir puis relancer le contrôle de lot. Aucun de ces résultats n’est revendiqué dans P28. Le helper `promote-checkpoint` ne remplace pas le contrôle explicite `--lot`. Préserver les états/travail/données sur erreur; aucun reset global.
