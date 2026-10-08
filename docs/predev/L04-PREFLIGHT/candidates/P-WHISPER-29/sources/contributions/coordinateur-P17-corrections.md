# Contribution brute — coordination des corrections P17

Base de travail : P-WHISPER-16, digest `7dedd68a07954c89cbd6557a57c218d4dec1446b4b975a5361ad4bdc63770d40`; base canonique d’enregistrement : P-WHISPER-14, digest `2724e325c5377d725b4e4f73541b6079a7bf08a9ff8e4328b10768e6823ca55b`.

Owner : coordinateur AUTH-COORD. Sources : captures utilisateur P15/P16, P15 transport planificateur, P16 revue indépendante brute, sources consommateurs exactes au HEAD `c75ae195b4431abd8f8c1dfd7abb45e81f0ad79b`, contrôleur central et ses tests. IDs traités : P16-REQ-001..006.

Corrections proposées : unifier les références courantes sous P17, aligner l’allowlist à 13 paths à partir des consumers vérifiés, inclure les captures d’autorisation/revue et les snapshots sources, corriger la résolution de preuves du contrôleur et rafraîchir les 23 snapshots pack exacts. Les contrats produit D19 sont conservés; la revue indépendante P17 doit confirmer KEEP_D19.

Preuves : manifeste P16 `7dedd68a07954c89cbd6557a57c218d4dec1446b4b975a5361ad4bdc63770d40`; P16 revue transport `T-WHISPER-P16-REVIEW-RAW-01`; transport plan-writer `T-WHISPER-P15-PLAN-WRITER-01`; contrôleur SHA `7c674b7b0390b5d54f4ce5759e79336649d76631d1b8599a0f0ab71f8e996673`; suite contrôleur 30/30.

Limites : le brut exact de la revue P15 n’est pas disponible; ses IDs ne sont pas reconstruits. Aucun préflight `--lot`, aucune promotion, aucun build/test produit et aucune implémentation L04. Travail restant : revue exacte P17, checkpoint vérifié liant les transports utiles, puis action utilisateur distincte pour préflight et code.
