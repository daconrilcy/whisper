# Réponses attribuées — Q-04, Q-06 et Q-09

**Base** : DESIGN D-WHISPER-19 (`903926ba27c70fc3a0cc695d4e9e50914a64993e40c339683e557bc4c604d7529`) et PLANS P-WHISPER-10 (`9b9edfcf09e39bd6017fb818fd0b9784923b8eb89707b18a9e3a1fff4d7b8f4a`). Questions de classification `reversible_detail`. Apport `/root/q04_q06_q09_arch`, `rust_architect`, `ARCH-Q04-Q06-Q09-v1`, transcrit par le coordinateur. Réponse utilisateur directe demandant résolution des trois détails ; Q-09 ensuite explicitement accepté par « Rotation bornée (Recommended) ». `DESIGN_CHANGE_REQUIRED=false`.

## Q-04 — progrès attendu et diagnostic

La progression est monotone et spécifique à l’étape et à l’obligation. `Idle`, `Queued`, `AwaitingChoice` et les états terminaux stables n’ont aucune obligation de progrès ouverte. Le silence de capture est attendu ; le progrès de capture ne peut pas masquer un ACK durable du journal en attente. Un import suit la progression source décodée, les segments achevés et leurs reçus durables. L’inférence suit la plage soumise jusqu’à la plage terminée. `Draining`, `Finalizing` et `Quitting` suivent leurs obligations finies et reçus. La seule présence du processus ou un heartbeat ne remet pas la temporisation à zéro.

Sonder au plus chaque seconde. Après 60 secondes sans progression attendue sur une obligation ouverte, afficher l’avertissement et le diagnostic. Si le diagnostic est inchangé, le rafraîchir au plus toutes les 30 secondes ; le mettre à jour immédiatement à une transition d’état ou nouvelle erreur. Le temps seul ne peut pas arrêter/tuer le worker, déclencher le fallback ou publier un résultat. Cela précise REQ-11/AC-09 sans convertir le délai indicatif en arrêt.

## Q-06 — Quitter quand l’arrêt n’est pas confirmé

Réutiliser DETAIL-P03 : Quitter est latched et coopératif. Si le worker ne confirme pas `Stopped`, l’application reste ouverte, réactive, et montre l’attente ou le diagnostic ; aucun état `Complete` n’est affiché. Aucun geste de terminaison forcée, ni requête Stop concurrente répétée. Préserver les données dont la durabilité est confirmée afin qu’elles restent récupérables. Cela applique l’invariant utilisateur D19 « rester ouvert jusqu’à résolution ou action manuelle » et la résolution DETAIL-P03 acceptée ; cela ne propose aucun kill automatique.

Preuves : D19 `candidates/D-WHISPER-19/01_PRODUCT_BRIEF.md` et `06_SOURCES_AND_DECISIONS.md`; question/acceptation `candidates/D-WHISPER-19/10_QUESTIONS_RISKS_AND_READINESS.md` SHA-256 `6a0f5188178a942c6da82d5452e9dab0dfb32b733d8c66d6f58be692422227b6`; résolution DETAIL-P03 `P10-L03-CLOSURE-WORK/transports/T-WHISPER-DETAIL-P03-RESOLUTION-01/DETAIL-P03-resolution.md` SHA-256 `d80e2aa581926981f5ff138d9cec82940fcac626d46707c9b033e32aef766e98`. La conformité de l’implémentation et l’UI réelle restent à démontrer.

## Q-09 — rotation locale des journaux de diagnostic

Appliquer la rotation bornée acceptée par l’utilisateur : maximum 4 fichiers de 2 MiB chacun, fichier actif compris (8 MiB au total) ; âge maximal de 7 jours ; oldest-first ; aucune durée minimale de rétention garantie. Traiter uniquement les fichiers de noms connus sous un répertoire dédié de diagnostic. Ne jamais toucher aux sources importées, archives audio, textes de session ou données de reprise.

Les fichiers restent diagnostiques et locaux : IDs, erreurs filtrées et durées ; exclure audio, transcription, chemin source libre, chaîne d’erreur native non filtrée, réseau et cloud. Écriture best-effort via une file bornée à 128 enregistrements de 4 KiB au maximum chacun. Si la file sature, signaler la perte de diagnostic ; ne pas bloquer Stop et ne pas modifier l’ACK du journal durable. L’acceptation des limites et de l’option est consignée dans `user-acceptance-Q09.md`.

## Limites de preuve

Ces réponses définissent des contrats à implémenter et vérifier. Aucun test, build, essai runtime, scénario UI, ni qualification/campagne produit n’est déclaré exécuté ici ; ces preuves restent `NOT RUN`. P11 doit recevoir une revue indépendante sur son identité exacte avant promotion du checkpoint. Ce transport ne donne aucune autorisation d’implémenter L04/L05.
