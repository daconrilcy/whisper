# Décisions utilisateur du 2026-10-05 — reprise après D9

Source : réponses directes de l'utilisateur dans le présent chat, capturées fidèlement ci-dessous. Cette capture documente l'autorité produit ; elle ne constitue pas une mesure.

| ID | Question et réponse exacte | Effet |
|---|---|---|
| DEC-26 | Pour les conditions d'usage des critères V1 de VAD, perte et délai live : « Micro réel FR/EN, calme et bruit courant (recommandé) » | Cible d'évaluation acceptée ; aucune mesure représentative ni aucun seuil accepté par cette réponse. |
| DEC-27 | Pour le live CPU : « CPU disponible avec retard visible, sans promesse de temps réel (recommandé) » | CPU reste dans V1 ; UI doit montrer le retard effectif. La mesure GPU ne vaut pas promesse CPU. |
| DEC-28 | Pour constituer le corpus : « Réutiliser seulement les WAV existants » | Aucun nouvel enregistrement micro autorisé dans ce cadrage. Le WAV FR micro existant et les WAV publics EN peuvent être réutilisés ; ils ne couvrent pas la cible FR/EN en calme et bruit courant. |
| DEC-29 | Proposition explicite de plafonds provisoires perte/GPU et de retard CPU affiché ; réponse exacte : « Oui, comme seuils provisoires sur ces essais (recommandé) » | Seuils et méthode limités aux essais WAV existants, détaillés dans `38_PROVISIONAL_THRESHOLDS.md`. Aucune validation VAD ni UI. |
| DEC-30 | Pour le VAD : « Annoter les WAV existants avant le seuil (recommandé) » | Fixer un seuil après annotation des WAV existants ; aucun chiffre accepté. |
| DEC-31 | Pour la méthode après exposition de la limite d’écoute : « Annotations automatiques provisoires à vérifier ensuite (recommandé) » | Préparer des intervalles automatiques, les vérifier humainement ensuite ; aucun seuil VAD accepté. |
| DEC-32 | Pour le gate DESIGN face aux preuves manquantes de 008 : « Conserver DRAFT et compléter les preuves techniques (recommandé) » | DRAFT maintenu ; aucune acceptation du risque 008, aucune réduction du périmètre V1 et aucune autorisation de coder le produit. |

Ces réponses ne modifient pas CHANGE-001 : le PC propre reste hors du gate actuel et l'installation locale reste incluse. Le seuil VAD reste ouvert ; les plafonds perte/GPU sont provisoires et bornés par DEC-29. Aucun fichier audio n'est copié dans ce candidat.