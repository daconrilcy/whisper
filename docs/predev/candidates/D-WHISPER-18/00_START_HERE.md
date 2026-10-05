# Cadrage Whisper Windows — DRAFT corrigé

**Candidate D-WHISPER-18 · V1 · périmètre 2 · 2026-10-05 · DRAFT**

Ce corpus reprend D-WHISPER-17 après R-WHISPER-DESIGN-17 : 001 et 014 sont fermés ; 008 reste ouvert jusqu’à revue indépendante de D18. D18 ajoute les preuves E0–E6 et les choix techniques de ARCH-D18 pour installation, backend, archives, durabilité, contrôle et VAD Rust. Les limites sont dans 49 et 50 ; aucun test du produit final n’est revendiqué. La décision DEC-32 conserve DRAFT et demande de compléter les preuves techniques. E0 et un smoke E3 partiel sont joints ; les autres essais du protocole 41 restent NOT RUN. D13 ajoute DEC-30/31 et des annotations automatiques provisoires sur les WAV existants, sans seuil VAD accepté. Les nouvelles decisions produit sont sourcees dans `32`. Les mesures FLEURS sont conservees. Le changement produit CHANGE-001 diffère l’essai sur PC propre ; les mesures demandées figurent dans `21`. Il conserve les décisions produit vérifiables, distingue les propositions techniques et les essais rapportés, et expose les preuves manquantes. Il ne constitue ni une conception READY ni une autorisation de coder.

## Carte du corpus

- `49_TECHNICAL_EVIDENCE_D18.md` : bancs exécutés, résultats, limites et correspondance avec 008.
- `50_ARCH_DECISIONS_D18.md` : décisions techniques attribuées ARCH-D18, contrats et risques.
- `evidence/d18/` : sources, locks, sorties, notices et MP3 encodés en base64.


- `47_USER_VALIDATION_D17.md` : verbatim, portée et autorité des décisions de reprise.
- `48_VAD_HUMAN_RESULT.md` : contrôle de l'export, métriques, seuils et limites.
- `evidence/human-annotations.json` et `evidence/vad-human-comparison.json` : données et résultats exacts.


- `01_PRODUCT_BRIEF.md` : utilisateur, objectifs, périmètre et exclusions.
- `02_REQUIREMENTS.md` : exigences et critères observables.
- `03_DOMAIN_ARCHITECTURE.md` : parcours, domaine, invariants, couches et contrats proposés.
- `04_TECHNICAL_EVIDENCE.md` : candidats, preuves et risques.
- `05_READINESS_AND_RAILGUARD.md` : garde-fous proposés et readiness.
- `06_SOURCES_AND_DECISIONS.md` : références exactes des messages utilisateur et limites de provenance.
- `07_COVERAGE_AND_ACCEPTANCE.md` : couverture bidirectionnelle, UC et AC.
- `08_STATE_AND_PORT_CONTRACTS.md` : transitions, durabilité, concurrence et ports.
- `09_TECHNICAL_SOURCES_AND_SPIKES.md` : sources primaires datées et preuves de faisabilité à produire.
- `10_QUESTIONS_RISKS_AND_READINESS.md` : questions classées, risques et rubriques DESIGN.
- `11_RAILGUARD_PROPOSAL.md` : proposition vérifiable de railguard non actif.
- `46_REVIEW_RAW_D15.json` : revue indépendante D15.
- `45_ARCH_008_RAW.md` : paquet détaillé exact du spécialiste, également transporté.
- `44_REVIEW_RAW_D14.json` : revue indépendante D14.
- `43_ARCH_PROVENANCE_AND_REVIEW14.md` : provenance du brut et correction de 014.
- `42_TECHNICAL_FOLLOWUP.md` : inventaire E0 et smoke MP3 E3 partiel.
- `41_ARCH_008_PROTOCOL.md` : protocole proposé et limites du finding 008.
- `40_REVIEW_RAW_D12.json` : revue indépendante D12.
- `39_AUTO_ANNOTATION.md` : préparation et limites des intervalles VAD candidats.
- `38_PROVISIONAL_THRESHOLDS.md` : seuils et limites de DEC-29.
- `37_REVIEW_RAW_D11.json` : revue indépendante D11.
- `36_REVIEW_RAW_D10.json` : resultat structure de la revue D10.
- `35_CORRECTION_001_D11.md` : correction des copies de sources apres revue D10.
- `34_REVIEW_RAW_D9.json` : revue indépendante brute de D9.
- `33_CLOSURE_MATRIX_008.md` : preuves et decisions encore necessaires pour 008.
- `32_USER_DECISIONS.md` : reponses utilisateur du present chat.
- `30_CORRECTION_013.md` : portee exacte des logs de preuve.
- `29_EXTENDED_MEASUREMENTS.md` : corpus FLEURS et replays FR/EN.
- `21_MEASUREMENTS.md` : résultats VAD, perte et délai live, avec scripts et logs.
- `26_CORRECTION_001.md` : snapshots de regles restaures apres revue D6.
- `25_CORRECTION_012.md` : calcul et portee corriges apres revue D5.
- `22_CHANGE-001.md` : décision utilisateur de différer le test sur PC propre.
- `12_SOURCE_INDEX.json` à `16_SNAPSHOT_INDEX.json`, `18_REVIEW_RAW.json`, `20_REVIEW_RAW_D3.json`, `23_REVIEW_RAW_D4.json`, `24_REVIEW_RAW_D5.json`, `27_REVIEW_RAW_D6.json`, `28_REVIEW_RAW_D7.json`, `31_REVIEW_RAW_D8.json` et `rules/` : copies de sources et règles fondatrices vérifiables.

Les rubriques sont regroupées par proximité. Aucun sujet applicable n’est déclaré N/A. Les nouveaux documents précisent les anciens lorsqu'une formulation courte y était ambiguë.

## Statut de preuve

Les décisions produit viennent des messages utilisateur référencés dans `06`. Les essais CPU/GPU Python sont rapportés par l’utilisateur ; commandes/logs non joints. Les essais Rust CPU/CUDA et les essais de récupération par arrêt de processus sont documentés dans `09` et `evidence/`. Leurs limites demeurent explicites : installateur, panne electrique, integration MP3 multi-passages, VAD et corpus qualite ne sont pas qualifies. Le manifeste prouve l’intégrité des octets ; il ne vaut pas revue indépendante CLEAN.
