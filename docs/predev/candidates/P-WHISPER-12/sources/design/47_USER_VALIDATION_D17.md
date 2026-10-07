# Réponses utilisateur après D16 — 2026-10-05

Sources : messages directs de l'utilisateur dans ce chat. Ces citations transcrivent une attestation et deux décisions produit ; elles ne sont ni une revue indépendante ni un test du produit final.

| ID | Réponse exacte | Effet accepté |
| --- | --- | --- |
| DEC-33 | « j'ai fait les verifications manuelles des intervalles tout est ok » | L'utilisateur atteste avoir écouté et validé les intervalles exportés pour les 26 WAV. L'export exact est `evidence/human-annotations.json`; aucune borne n'a changé par rapport aux propositions automatiques. |
| DEC-34 | « Accepter les 26 WAV pour le gate DESIGN (recommandé) » | Ce corpus borné suffit comme corpus de preuve de conception VAD. La cible initiale micros réels FR/EN en calme et bruit courant (DEC-26) reste une qualification future du produit ; les WAV micro EN et bruit courant réel ne sont pas présents. DEC-28 reste applicable aux essais de ce cadrage. |
| DEC-35 | « Oui, fixer ces seuils (recommandé) » | Pour le gate DESIGN sur ces 26 WAV et WebRTC mode 1, précision ≥ 0,80 et rappel ≥ 0,95 agrégés ; pour chacun des quatre groupes FR FLEURS, EN FLEURS, micro FR et WAV public EN, précision ≥ 0,65 et rappel ≥ 0,90. Trames de 20 ms ; voir `48`. |

Les valeurs sont des critères acceptés pour ce corpus et cette méthode. Elles ne prouvent ni la version Rust choisie, ni le VAD dans l'application, ni la qualité sur un nouveau microphone ou sous bruit courant réel. Le finding 008 reste soumis au reviewer.
