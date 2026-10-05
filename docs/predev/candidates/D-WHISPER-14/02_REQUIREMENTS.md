# Exigences et critères d’acceptation — proposés

| ID | Exigence | Critère observable |
|---|---|---|
| REQ-01 | Live micro seulement, une session | Une seconde capture ne démarre pas. |
| REQ-02 | Texte live progressif ; délai mesuré par l’app | Mesurer capture, disponibilité du texte et premier rendu ; aucun plafond produit. |
| REQ-03 | Import WAV/MP3 ; demande pendant live en attente | Pas de démarrage avant fin du live ; variantes exactes à qualifier. |
| REQ-04 | Live demandé pendant import actif refusé | Inviter à relancer après import ; aucune mise en attente de live. |
| REQ-05 | Modèle téléchargé pendant installation | Disponible au premier lancement ou erreur/reprise expliquée. |
| REQ-06 | Langue manuelle prioritaire, sinon automatique | Langue effective visible. |
| REQ-07 | Auto peut replier sur CPU et le signaler | Mode effectif visible ; reprise sans duplications/pertes silencieuses. |
| REQ-08 | GPU forcé en échec s’arrête et propose CPU | Attendre le choix explicite avant bascule. |
| REQ-09 | Deux heures indicatives de parole, silences VAD exclus ; option arrêt à durée choisie ou sans limite | Compteurs parole et total distincts ; l'arrêt automatique n'agit que si activé. |
| REQ-10 | Silence conservé dans MP3 ; SRT sur axe original | Archive audio complète et chronologie non compressée. |
| REQ-11 | Environ 60 s sans progrès attendu : avertir/diagnostiquer | Silence attendu distingué du blocage ; arrêter et alerter seulement sur panne établie, jamais sur le temps seul. |
| REQ-12 | Archives MP3/TXT/SRT cohérentes | Ne publier comme complet qu’après vérification de l’ensemble. |
| REQ-13 | Historique reconstruit depuis fichiers archivés | Pas de DB d’historique nécessaire. |
| REQ-14 | Fermer la fenêtre masque ; le travail continue dans tray | Live/import reste accessible après masquage. |
| REQ-15 | Quitter en live arrête, finalise, quitte | Échec : préserver récupérable, signaler, quitter sans faux statut complet. |
| REQ-16 | Quitter en import actif annule, préserve source, quitte | Fichier source intact. |
| REQ-17 | Imports non commencés conservés après crash ou Quitter | Au prochain lancement, signaler et proposer Traiter/Retirer ; rien ne démarre sans choix ; retrait ne supprime pas source. |
| REQ-18 | Reprise live conserve fragments confirmés | Mesurer perte maximale du fragment ouvert et buffers ; aucune borne en secondes fixée. |
| REQ-19 | Démarrer avec Windows en tray, sans capture ni fenêtre | Au login une seule instance inactive ; option désactivable et erreur d'autostart visible. |
| REQ-20 | Réglages persistants pour langue, calcul, micro, dossier, durée, autostart et raccourci | Valeurs valides conservées après redémarrage ; job actif garde son snapshot. |
| REQ-21 | Raccourci global configurable Start/Stop | Conflit signalé ; aucun démarrage caché dans un état incompatible. |
| REQ-22 | Choix et changement du dossier d'archives | Accès validé ; chemin effectif explicite ; indisponibilité signalée sans publication ailleurs. |
| REQ-23 | Historique des archives et suppression manuelle | Suppression confirmée efface archives MP3/TXT/SRT visées, jamais la source importée. |
| REQ-24 | Mode CPU forcé même si un GPU compatible est disponible | Le job utilise CPU et affiche CPU ; aucun contexte GPU activé pour ce job. |
| REQ-25 | Stop + Reprise démarre un nouveau passage dans la même transcription sur impulsion utilisateur | Le passage précédent est finalisé avant le suivant ; identités et chronologie du groupe cohérentes, sans départ automatique. |

Les UC/AC identifiés, variantes et méthodes futures sont dans `07`; questions classées et statuts dans `10`. Ces critères ne sont pas des tests exécutés. Les réponses de 2026-10-05 ferment l'ordre multi-import, la confirmation avant reprise, la suppression d'archives, le comportement de Quitter bloqué et le contenu des logs ; la faisabilité technique reste à démontrer. Les sorties obligatoires dépendent du type de job : live = MP3/TXT/SRT ; import = TXT/SRT et lien vers la source, sans MP3 archivé par défaut.

## Précisions D10

- `REQ-02` : sur CPU, afficher le retard mesuré et le mode effectif ; aucun plafond de temps réel n'est promis. Le retard doit rester diagnostiquable lorsque la file augmente (DEC-27).
- `REQ-09/18` : corpus cible micro FR/EN, calme et bruit courant (DEC-26). Les WAV existants sont la seule matière disponible pour ce cadrage (DEC-28) ; la valeur cible et sa preuve restent ouvertes.

## Seuils provisoires D12

`REQ-02` : pour le replay GPU sur WAV existants, P95 fin de fenêtre de 5 s→console ≤ 1 s ; le vrai délai capture→premier rendu reste à mesurer dans l’application. `REQ-18` : avec confirmation toutes les 0,5 s, perte maximale du fragment ouvert ≤ 0,5 s dans les arrêts de processus simulés. Le CPU affiche le retard sans plafond temps réel ; aucun seuil VAD encore accepté. Voir `38_PROVISIONAL_THRESHOLDS.md`.

## Préparation VAD D13

`REQ-09` : les intervalles d’activité acoustique calculés automatiquement sur les WAV existants sont des candidats à vérifier humainement ; ils ne fournissent pas encore une précision ou un rappel VAD. Aucun seuil V1 de VAD n’est accepté par DEC-30/31. Voir `39_AUTO_ANNOTATION.md`.
