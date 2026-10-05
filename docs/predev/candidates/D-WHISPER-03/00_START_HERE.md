# Cadrage Whisper Windows — DRAFT corrigé

**Candidate D-WHISPER-03 · V1 · 2026-10-05 · DRAFT**

Ce corpus reprend D-WHISPER-02 après le rapport indépendant R-WHISPER-DESIGN-02 (quatre findings fermés, sept REQUIRED ouverts). Il conserve les décisions produit vérifiables, distingue les propositions techniques et les essais rapportés, et expose les preuves manquantes. Il ne constitue ni une conception READY ni une autorisation de coder.

## Carte du corpus

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
- `12_SOURCE_INDEX.json` à `16_SNAPSHOT_INDEX.json` et `rules/` : copies de sources et règles fondatrices vérifiables.

Les rubriques sont regroupées par proximité. Aucun sujet applicable n’est déclaré N/A. Les nouveaux documents précisent les anciens lorsqu'une formulation courte y était ambiguë.

## Statut de preuve

Les décisions produit viennent des messages utilisateur référencés dans `06`. Les essais CPU/GPU Python sont rapportés par l’utilisateur ; commandes/logs non joints. Les essais Rust CPU/CUDA et les essais de récupération par arrêt de processus sont documentés dans `09` et `evidence/`. Leurs limites demeurent explicites : installateur, panne électrique, encodeur MP3 de sortie, VAD et corpus qualité ne sont pas qualifiés. Le manifeste prouve l’intégrité des octets ; il ne vaut pas revue indépendante CLEAN.
