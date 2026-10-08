# Contribution coordinator P18

Auteur : AUTH-COORD; base : P-WHISPER-17 exact `b8b3b10759ae0ba3f3c714e3b193d5abe867de740acb22d466bcb7eebef96459`, active PLANS P14; parent D19 `903926ba27c70fc3a0cc6954d9e50914a64993e40c339683e557bc4c604d7529`.

Corrections: (1) cadrer le worker GPU enfant opérable (protocole, identity/version, décodeur WAV/MP3, inférence, publication MP3/TXT/SRT, arrêt coopératif); ajouter GPU Cargo.toml et root Cargo.lock à l’allowlist; (2) corriger chemins et digest D19 courants; (3) inclure snapshot exact CPU native_engine, GPU Cargo.toml et Cargo.lock et clarifier count; (4) corriger le test central pour protéger deux preuves documentaires uniques; (5) conserver la réponse exacte du reviewer P17 avec limite d’export; (6) checkpoint P14 actif avec les raw transports et ce transport P18 référencés.

Limites : aucune modification du produit, DESIGN D19, promotion de P18, préflight lot ou implémentation. Les snapshots sont observations au HEAD indiqué, pas baseline D0.
