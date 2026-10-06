# Environnement L01 — P-WHISPER-05

Auteur : `/root/correct_p03`, rust_plan_writer, 2026-10-06 Europe/Paris. Nature : précondition documentaire d’exécution L-WHISPER-01. Statut : PROPOSED / NOT RUN ; aucun résultat d’environnement produit.

Matrice : `build-environment-L01.json`, schema `rust-predev-build-env/1`. Contrat : `rules/skills/rust-predev-design/references/execution-evidence.md.txt`. Owner du contrôle : exécutant L01 autorisé ; owner de persistance des preuves : hôte documentaire ; suivi : coordinateur.

DETAIL-P01 v3 fixe Rust/Cargo 1.98.1, édition 2024 et cible `x86_64-pc-windows-msvc`. Ledger L00 déclare MSVC, CMake et `LIBCLANG_PATH` pour le build CPU livré mais n’atteste pas leur disponibilité dans le futur processus. Exécuter la matrice dans le même Developer PowerShell MSVC et processus destinés aux commandes Cargo, cwd `C:\dev\whisper`, avant la première modification produit. Pas d’installation ou téléchargement implicite. Tout check obligatoire MISSING ou NOT CHECKED bloque L01 avant modification ; résoudre dans un mandat applicable puis refaire le contrôle et conserver la nouvelle preuve attribuée.

Le probe ne vérifie ni l’installation de la target Rust, ni la complétude Windows SDK/linkage, ni la chargeabilité/compatibilité de la DLL libclang ; vérifier ces éléments séparément au préflight. Modèle/données/hashes et contrat d’import sont des préconditions distinctes. CUDA n’est pas requis pour le parcours CPU L01. Un futur build GPU exige sa propre matrice incluant `nvcc`, `CUDA_PATH` et l’outil générateur prévu ; le PASS CPU ne qualifie pas GPU. Aucun PASS historique L00, cache target ou exit 0 d’outil ne remplace les builds/tests produit/qualification native.
