Avancée : protection préparée, N1 identique à main, stress A100/B1 synchronisé200 PASS ; cache relu, 16 auto-tests PASS.
Blocage : WHEA 19 corrigé « Internal parity error », APIC 24, le 05/11/2026 à 20:29:15,884 Paris ; CPU1 interrompu.
Décision : **BLOQUÉ — MATÉRIEL À REVÉRIFIER** ; arrêt des essais, fusion de protection seule recommandée.

# BUILD-NATIF-C1001 — bilan figé

Précondition A : **16,81 %** validés, aucun résiduel ciblé, MemWatch Ready/résultat 0, relevé <4 min, 455 Gio libres. Environnement MSVC **reconstruit**, original cpu8 absent ; commande exacte, essai-0 et prétraitement PASS.

| Série | Crashs / compilations achevées | Concurrence | Marge mémoire minimale observée |
|---|---:|---|---:|
| A, reprise bornée comprise | 0 / 100 | 1 | 83,0 % |
| B1 initial, non conforme | 0 / 200 | ≤3 unités ouvertes ; quatre non atteints | 82,9 % |
| B1 synchronisé | 0 / 200 | 4 PID en recouvrement, 50/50 lots ; minimum 614 ms | 82,4 % |
| B2 | NON EXÉCUTÉ | Aucun crash déclencheur | — |

**501 lancements, 500 succès, une interruption du lecteur MemWatch**, aucun C1001/dump/décalage c2 nouveau. Revue indépendante du stress CLEAN, sans conclusion causale.

CPU1 interrompu dans Protobuf hôte Release : événement System **527992**, détecté à 20:29:17,300, fermeture à 20:29:17,857 sans erreur. CSV précédent : **20:29:12, 17,6 %, OK** ; maximum observé du lot 20,6 %. Le contrôle direct WHEA complète MemWatch inchangé. Aucun survivant natif/du lot, cache non scellé, aucune nouvelle DLL.

| Critère du nouveau candidat | CPU | GPU |
|---|---|---|
| a — surfaces PE / CPU sans CUDA | NON EXÉCUTÉ | NON EXÉCUTÉ |
| b — TXT/SRT/WAV et assertions N2 | NON EXÉCUTÉ | NON EXÉCUTÉ |
| c — deux constructions identiques | NON EXÉCUTÉ | NON EXÉCUTÉ |

PE cpu7c4 5/5 : preuve antérieure. N1 mêmes sources Rust : **761 PASS, 149 ignorés, zéro échec**, 910 identités main/candidat identiques ; Structure 958/958. Aucun Rust produit modifié.

**DÉRIVE MÉMOIRE** : +5,04 Gio/+28 processus, de 18:08:18 (premier CIM complet) à 20:32:30 ; visible dès protection/N1. TopNombre : msedgewebview2 +13, svchost/conhost +3, RuntimeBroker/node +2, node_repl +1. Aucune fuite attribuée. `GGML_NATIVE=ON` : portabilité prioritaire proposée dans [NATIVE-BUILD](NATIVE-BUILD.md).

Preuves : [archive](raw/build-natif-c1001/evidence-20261105T194944Z.zip), **2 584 fichiers + manifeste**, 23 939 167 octets ; noms/tailles/SHA revérifiés. SHA-256 `3561105D56F7ACEB34847C204C195F58BC6C685C6D96F18CDDA9535A1F6C9ED9`. **14 DLL installées inchangées** ; aucun nettoyage.

**Décisions :** fusion de [protection](../cadrage/build-natif-protection-proposition.md) `1d9e7d60` recommandée, 18 fichiers ; cache/recettes C1001 restent hors proposition, qualification réelle inachevée. **Remplacement et suppression du spike : non recommandés.** Aucune fusion exécutée ; lien WHEA/C1001 historiques non établi.

Après nouvelle décision/préflight, nettoyage proposé seulement : `Remove-Item -LiteralPath 'C:\wt\build-natif\target\native-build\cpu\C1001-cpu1' -Recurse`. Retrait WER ciblé proposé dans le reçu archivé `wer-setup.json` ; aucune suppression exécutée. Espaces précédents, caches, archives et chemins protégés conservés.
