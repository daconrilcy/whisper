Avancée : six essais Auto PASS, tous rapides ; attente dominante située avant l'observation de Prepared.
Blocage : aucun essai lent obtenu ; origine des 15 s bimodales et cause interne non déterminées.
Décision : quota atteint, arrêt des essais ; instrumentation conservée sur sa branche en attente du choix utilisateur.

# DIAG-BANC-15S — 4 novembre 2026

Branche `diag-banc-15s`, instrumentation `d6482e95`, base `9943493d`.
Auto sans candidat → CPU, MP3 **24 237 octets**, ressources épinglées identiques.
Produit/garde/configuration machine inchangés ; descripteur de traces Auto autorisé.

**Six corps Rust, tous R (<30 s), sans moyenne globale :**
`fab3b20d` 25,26 s ; `07b8d734` 25,10 s ; `0b1e1300` 25,16 s ;
`a55b7d0c` 25,23 s ; `cfbb4133` 25,16 s ; `7eae9edf` 25,12 s.
Préfixes des dossiers `PersonalAutoFallback-*`. Aucun intermédiaire/lent observé ;
cette absence ne démontre pas la disparition du mode lent historique.

| Phase visible, secondes | Rapide `fab3b20d` | Lent / écart |
|---|---:|---|
| Début → retour StartImport : préparation externe, validations CPU/GPU, service | 0,0022 | Non mesuré / ND |
| Retour StartImport → observation « Source vérifiée » (Prepared) | **24,1051** | Non mesuré / ND |
| Prepared observé → Running CPU | 0,6100 | Non mesuré / ND |
| Running → Finalizing | 0,4596 | Non mesuré / ND |
| Finalizing → résultat observé | 0,0766 | Non mesuré / ND |
| Vérifications archive/exports/WAV/source/index et drop des handles | 0,0022 | Non mesuré / ND |
| Corps total, durée libtest arrondie | **25,26** | Non mesuré / ND |

Six attentes StartImport→Prepared observé : **23,866–24,105 s**. Refresh maximum :
0,1012 ms ; intervalle maximal entre événements : 26,49 ms. Ces frontières
délimitent l'attente asynchrone jusqu'au traitement de Prepared dans Refresh,
pas le traitement interne du service ; granularité du polling : environ 25 ms.

**Verdict : NON DÉTERMINÉ pour la cause BANC / GARDE / PRODUIT (installation neuve).**
La construction et les contrôles externes du banc n'expliquent pas la durée
dominante. L'écart apparent ~19 s avec l'application normale ne mesure aucune
phase commune : profil, compilation et installation diffèrent. Les 15 s bimodales
ne sont pas reproduites. Aucune attribution SHA-256/Defender/priorité/installation
neuve. Au prochain lot, proposer trois
marqueurs produit : entrée `prepare_effect`, fin `bind_import_request`, émission
Prepared, avec identité/horloge monotone ; **aucun ajouté ici**.

**Temps distincts :** Cargo 49,825 s au premier essai (compilation 23,57 s),
puis ~26 s. Garde complète, essais 3–6 : 41,442–41,601 s. Ses ~15,5 s hors
Cargo sont distinctes des 15 s bimodales du corps et du staging seul.

**Deux options futures, aucune correction de performance démontrée :**
1. Mesures premier import/état chaud séparées, chauffe hors mesure : coût faible ;
   répétabilité de l'état prêt, risque de masquer le premier lancement.
2. Série dédiée avec staging conservé/vérifié : coût moyen ; préparation plus
   stable, risque de perdre le cas installation neuve, effet sur le corps inconnu.

N1 : **705 PASS /128 ignorés /0 échec**, nombres inchangés ; format, build et
Clippy PASS (avertissements), Native SelfTest et compilation native-tests sans
exécution PASS. Instrumentation **CLEAN**, deux revues/une correction ; conclusions
relues indépendamment **CLEAN**. Incidents
et arrêt VCTIP autorisé : [plan](../cadrage/diag-banc-15s-plan.md). Drop des handles ≠
fin des threads détachés ; surcoût des traces non isolé par A/B.
Provenance DLL/ignore signalés, non traités. Aucun push/fusion.

Traces, archivage vérifié et commandes de nettoyage utilisateur : voir le plan.
**Choix demandé : garder l'instrumentation désactivée par défaut (recommandé pour
le diagnostic suivant), ou la jeter ; aucune fusion automatique.**
