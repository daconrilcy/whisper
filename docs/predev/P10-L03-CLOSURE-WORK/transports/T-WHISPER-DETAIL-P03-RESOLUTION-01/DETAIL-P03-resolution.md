# DETAIL-P03 — résolution de coordination

Statut : ANSWERED_DETAIL, décision réversible, avant L04.

Pour un worker qui ne confirme pas son arrêt, la fenêtre de l’application reste ouverte. L’interface continue d’afficher l’attente et, si elle est disponible, le diagnostic du worker. Elle ne propose pas de geste de terminaison forcée : aucun kill automatique ou manuel n’est ajouté. Le délai de terminaison FFI/disque n’est pas garanti et la boucle UI ne bloque pas sur un join.

Cette résolution sélectionne l’option « attente/diagnostic » déjà prévue par DETAIL-P03. Elle applique la décision TECH-D18-04 approuvée dans `candidates/P-WHISPER-10/sources/design/50_ARCH_DECISIONS_D18.md` et le contrat d’intégration dans `candidates/P-WHISPER-10/sources/architecture/ARCH-L01-INTEGRATION-v2.md` : Quitter reste ouvert si l’enfant ne confirme pas son arrêt, l’attente/panne est affichée, aucune action de force n’est requise. L’effet visible reste ainsi celui déjà approuvé par l’utilisateur ; aucun nouveau choix produit n’est introduit.

Portée : débloque la précondition documentaire de L04 et L05. N’ajoute pas de code à L03 et ne qualifie pas les comportements runtime de L04/L05. L’implémentation et sa vérification restent à réaliser dans ces lots.
