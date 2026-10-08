# Autorisation utilisateur — extension L-WHISPER-04 et CUDA 12.9

Date : 2026-10-08. Source : réponse utilisateur dans la présente conversation, après présentation des six chemins manquants et du contrôle d'environnement.

Réponse exacte :

> oui j'autorise  
> il faut passer aussi en Cuda 12.9

L'autorisation d'implémenter L-WHISPER-04 accordée le 2026-10-07 dans `USER_AUTHORIZATION_L04.md` est étendue aux six chemins suivants, et seulement à eux en complément des quinze chemins déjà autorisés :

```text
crates/whisper-adapters/src/archive.rs
crates/whisper-adapters/tests/live_archive.rs
crates/whisper-core/src/ipc.rs
crates/whisper-core/src/ports.rs
crates/whisper-core/tests/live_contract.rs
crates/whisper-worker-cpu/src/main.rs
```

L'union couvre exactement les 21 chemins de `candidates/P-WHISPER-29/17_L04_D28_PLAN.md`, section 2. Le préflight L04 reste autorisé. Aucun autre chemin produit, commit, push ou publication n'est autorisé par cette extension.

L'utilisateur demande aussi de passer à CUDA 12.9. Ce choix remplace la version 12.8 exigée par la matrice d'environnement P29 pour le futur lancement L04. Il doit être intégré dans un successeur documentaire attribué et revu, avec impact sur les preuves techniques, les commandes et la qualification. Cette réponse ne modifie pas rétroactivement les candidats DESIGN D28 et PLANS P29 immuables, et ne vaut pas à elle seule checkpoint IMPLEMENTATION ou `check-state --lot` réussi.
