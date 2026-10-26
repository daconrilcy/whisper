Avancée : revue indépendante et re-revue terminées après un cycle de correction.
Blocage : aucun constat REQUIRED ouvert ; R1 et A1 fermés par le reviewer.
Décision : CLEAN sur 42c8c16d ; design prêt pour acceptation utilisateur.

# Revue du cadrage Parakeet unifié

## Identité et périmètre

- Reviewer : `/root/parakeet_design_review`, rôle `rust_design_reviewer`.
- Déclaration du reviewer : aucune participation à la rédaction ni aux corrections du corpus ; indépendance fondée sur cette absence de participation, pas sur le seul nom du profil.
- Auteur de ce fichier : session principale, transcription des résultats reçus du reviewer ; aucun jugement de fermeture par l'auteur.
- Corpus revu : `docs/cadrage/evo-parakeet-transcription-unifiee.md`, D-PARA-UNIFIED-01 v0.1.
- Revue initiale : Git `bb308b58e50c6cf2470256bfd07aebeef10433a9`, 26 octobre 2026. Source et empreinte transitoire vérifiées directement avant/après par le reviewer, inchangées. Aucun manifeste d'empreintes créé.
- Aucune écriture par le reviewer, aucun test ni binaire exécuté. Ce compte rendu reste hors du corpus qu'il évalue.

## Revue initiale — VERDICT : FINDINGS

### REQUIRED — DESIGN-PU-R1

**High / BLOQUANT — Garantir la progression des finalisations malgré l'aperçu actif.**

Preuve : §6.4 du candidat initial, ligne 371 : l'aperçu actif est prioritaire à chaque frontière d'appel. Aucune règle d'équité, quota ou occasion réservée au traitement final n'est définie.

Scénario : A est arrêtée et attend sa finalisation ; B capture sans plafond de durée. Si B maintient un aperçu en attente à chaque retour du worker, chaque nouvelle sélection peut reprendre l'aperçu. Le remplacement dans une file de capacité un borne la mémoire, mais ne garantit pas que la finalisation A obtienne un tour.

Conséquence produit : le contrat autorise une attente indéfinie de A tant que B continue. Cela compromet REQ-PU-02 / AC-PU-02, le scénario de fin de A pendant B de AC-PU-07, et peut bloquer l'admission suivante puisque les deux sessions demeurent non terminales.

Owner : `rust_architect`.

Condition de fermeture : préciser une politique vérifiable garantissant, sous réserve que chaque appel moteur engagé termine, un tour de traitement final après un nombre borné d'appels malgré un aperçu continuellement disponible. Décrire aussi l'arbitrage entre A et B et relier un scénario déterministe futur à AC-PU-02/07/10. Aucun benchmark préalable ni nouvelle contrainte de durée totale n'est demandé.

État : OPEN à la revue initiale ; amendement ARCH-PU v2 demandé au propriétaire.

### ADVISORY — DESIGN-PU-A1

**Low / MINEUR — Rendre explicite l'attente de l'aperçu pendant un appel final engagé.**

Les appels natifs ne sont pas instantanément interruptibles, tandis qu'une fenêtre finale peut représenter 120 secondes d'audio. La priorité à la prochaine frontière n'est donc pas une garantie de délai d'affichage. Le document conserve correctement Q-PU-03 ouverte ; mentionner ce compromis parmi ses dépendances et dans les futures mesures serait utile. Aucun seuil inventé demandé ; cette observation ne bloque pas à elle seule le design.

## Couverture et preuve de revue initiale

| Rubrique | Verdict initial du reviewer |
| --- | --- |
| Scope / sources | PASS : USER-01 à 06, aucun plafond applicatif, plusieurs WAV classiques et transcription unique. |
| Brief → REQ/UC/AC | PASS : CAP-PU-01 à 10 couvertes, différés identifiés. |
| REQ → domaine / architecture / risques | PASS avec R1 : liens présents, progression concurrente non garantie. |
| États / erreurs / clôture | PASS avec R1 : axes séparés, transitions, périmés, annulation et fermeture décrits. |
| Dépendances | PASS : frontières, composition/UI, stack et contrôles futurs. |
| Persistance / compatibilité | PASS : rotation, temps globaux, document paginé, engagement de référence, anciennes publications préservées ; pas d'atomicité multi-fichiers présumée. |
| Concurrence | GAP — R1 : équité manquante. |
| Plateforme / faisabilité | PASS documentaire, sans preuve d'implémentation ou performance. |
| Acceptation / risques | PASS avec R1 : qualité des raccords, ressources et Windows NOT RUN. |
| Railguard nouveau | N/A justifié : aucun railguard demandé ou activé. |

Preuves consultées par le reviewer : corpus complet et historique Git ; AGENTS, cinq règles centrales et références du skill ; Cargo.toml, Cargo.lock et whisper-desktop/Cargo.toml ; lectures ciblées de parakeet_windows.rs:150–271, worker_ipc.rs:4605–4648/5972–6010, direct_live.rs:1300–1395, native_engine.rs:1–116, worker main.rs:55–110, parakeet_ipc.rs, archive.rs autour de CURRENT/index, ui.rs:1668–1728 et bilan lot 3:80–99.

Le reviewer confirme que les limites historiques citées concordent avec les sources et qu'aucun SPIKE manquant n'est imposé par cette revue. Les validations produit futures ne sont pas exigées avant le code.

## Correction et re-revue

Cycle de correction 1 : amendement ARCH-PU v2 rendu par `/root/parakeet_architect`, base `bb308b58`, pour R1 et A1. Intégré par l'auteur documentaire : quota P = 1, tour de rôle final A/B, garantie conditionnelle d'un tour en quatre appels maximum, scénario V-PU-EQUITY-01 avec témoin négatif, attente non préemptible explicitée et lien Q-PU-03. L'auteur de la correction ne revendique pas la fermeture. Aucun troisième cycle ne sera ouvert ; maximum deux cycles revue/correction sur ce livrable.

### Re-revue — VERDICT : CLEAN

Même reviewer indépendant, sans participation à la correction. Candidat vérifié : `D-PARA-UNIFIED-01 v0.2`, Git `42c8c16d379a0acb9ea91416fbd0eb5a587a0270`. Revue ciblée du diff `bb308b58..42c8c16d` et des contrats environnants ; source et empreinte transitoire inchangées avant/après ; contrôle des espaces Git réussi. Aucun fichier modifié ni test/binaire exécuté par le reviewer.

| ID | Statut prononcé par le reviewer | Preuve de fermeture |
| --- | --- | --- |
| DESIGN-PU-R1 | CLOSED | DEC-T-PU-05 v2 impose au plus un aperçu entre deux appels finaux éligibles et des tours alternés A/B. Nouveaux aperçus sans réinitialisation du quota/ordre. Préparation dans le tour réservé, échec explicite. Avec deux sessions, la borne de quatre appels est cohérente sous les hypothèses de terminaison déclarées. |
| DESIGN-PU-A1 | CLOSED — conseil intégré | Limite de préemption et Q-PU-03 distinguent durée audio, exécution et attente de l'aperçu ; aucun délai utilisateur déduit de la borne d'appels. |

V-PU-EQUITY-01 couvre charge d'aperçus continue, alternance A/B, arrivée défavorable d'une session, identités, couverture, libération d'admission et fermeture. Son témoin négatif doit détecter l'ancienne priorité stricte. Il reste PRODUCT_VALIDATION — NOT RUN.

Aucune régression documentaire identifiée dans le diff. Les conclusions de couverture de la première revue restent valides ; la rubrique concurrence passe à PASS documentaire.

Conclusion du reviewer : **DESIGN READY pour acceptation utilisateur**, avec Q-PU-03/Q-RQ-01 ouverts avant les lots ou validations concernés. Performances, raccords réels, ressources et essais Windows restent NOT RUN.

Le reviewer permet à la session principale de finaliser uniquement les métadonnées d'en-tête, de carte des contenus et de readiness, ainsi que ce compte rendu. Aucun changement technique supplémentaire n'est couvert par cette clôture. Aucun verdict n'accepte le design au nom de l'utilisateur ni n'autorise la planification, le code ou le push.
