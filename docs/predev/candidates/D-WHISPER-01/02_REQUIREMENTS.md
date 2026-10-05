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
| REQ-09 | Deux heures indicatives de parole, silences VAD exclus | Compteur exclut les silences détectés ; comportement au seuil reste à décider. |
| REQ-10 | Silence conservé dans MP3 ; SRT sur axe original | Archive audio complète et chronologie non compressée. |
| REQ-11 | 60 s sans progrès attendu : avertir/diagnostiquer | Le temps seul ne prouve pas une panne ni ne déclenche l’arrêt. |
| REQ-12 | Archives MP3/TXT/SRT cohérentes | Ne publier comme complet qu’après vérification de l’ensemble. |
| REQ-13 | Historique reconstruit depuis fichiers archivés | Pas de DB d’historique nécessaire. |
| REQ-14 | Fermer la fenêtre masque ; le travail continue dans tray | Live/import reste accessible après masquage. |
| REQ-15 | Quitter en live arrête, finalise, quitte | Échec : préserver récupérable, signaler, quitter sans faux statut complet. |
| REQ-16 | Quitter en import actif annule, préserve source, quitte | Fichier source intact. |
| REQ-17 | Imports non commencés conservés après crash ou Quitter | Au prochain lancement, signaler et proposer Traiter/Retirer ; rien ne démarre sans choix ; retrait ne supprime pas source. |
| REQ-18 | Reprise live conserve fragments confirmés | Mesurer perte maximale du fragment ouvert et buffers ; aucune borne en secondes fixée. |

À préciser : live répété, file multi-import, panne vs silence/retard, saturation, import actif interrompu par crash, VAD, formats précis, annulation bloquée. Ces critères ne sont pas des tests exécutés.
