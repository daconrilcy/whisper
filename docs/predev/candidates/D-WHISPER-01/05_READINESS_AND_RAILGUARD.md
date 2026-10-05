# Readiness DESIGN et railguard — DRAFT

## Verdict

**DRAFT, non transférable à l’implémentation.** Corpus préparé pour revue ; aucun DESIGN READY n’est déclaré.

## Garde-fous projet proposés, non actifs

Le dépôt actuel n’a ni code Rust, ni Cargo.toml, ni railguard local. Proposition : conserver les couches constitutionnelles et leurs sens de dépendance ; aucune perte silencieuse, files bornées/rétropression ; persister file avant accusé ; source importée immuable ; file distincte de l’historique ; publier MP3/TXT/SRT uniquement après vérification ensemble ; versionner IPC et rejeter messages obsolètes ; isoler et justifier unsafe ; ne déclarer GPU actif que sur preuve. Vérifier frontières Cargo/imports, builds CPU/GPU, packaging modèle, crash/reprise, VAD/formats/charge. Le texte n’est pas actif sans revue/approbation et n’autorise aucun lot.

## Conditions avant READY

1. Revue indépendante du manifeste exact ; findings REQUIRED corrigés puis nouvelle revue CLEAN sur nouvelle version.
2. Résoudre ou borner en SPIKE les inconnues structurantes : moteur/backend GPU Rust, modèle et installateur, récupération/durabilité, import commencé après crash.
3. Définir file multi-imports, demandes live répétées, publication/annulation.
4. Revoir versions et sources primaires ; les essais Python manuels ne qualifient pas Rust.
5. Définir confidentialité, logs et données locales.
6. Établir critères/corpus de mesure : VAD, délai, perte, backpressure, formats et cohérence.
7. État contrôleur conforme au schéma avant toute gate READY ; le manifeste seul ne suffit pas.

## États actuels

Revue indépendante : NOT RUN. SPIKE/qualification technique : NOT RUN. Tests produit Rust : NOT RUN (aucun produit Rust dans le dépôt). Qualification native du workflow/hôte : NOT QUALIFIED pour la session danger-full-access ; ne pas en déduire une isolation. Plans/implémentation : non autorisés par ce corpus.
