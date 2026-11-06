Avancée : code livré, revue CLEAN et essais Windows après déclarés effectués.
Blocage : aucun sur la livraison ; réserves de validation maintenues.
Décision : clôturer le lot avec comparaison indicative et réserves explicites.

# INSPECT-MICRO-DEMARRAGE — bilan de clôture

**Clôturé le 6 novembre 2026, livré avec réserves de validation.** Code `df81bf92` ; [plan et décisions](plan-inspect-micro-demarrage.md), [revue indépendante](revue-inspect-micro-demarrage.md).

Inspections ciblées au démarrage et dans les quatre bindings micro ; état non inspecté explicite ; SHA hors verrou global, coordination par clé et revalidation. Une inspection sur commande exige une relance explicite, sans rejeu automatique ni preuve GPU conservée. Les contrats CPU/GPU/Auto, capture et sorties sont préservés.

**Code CLEAN**, epoch 1 : R1/R2 fermés, deux revues et un cycle de correction. Tests ciblés cache/bindings/file/reprise/cœur/UI réussis ; composition finale 34 réussis/3 ignorés historiques, contrôleur 4 réussis. Format, Clippy avec avertissements et build personnel réussis ; détail des comptes dans le plan.

**Réserves acceptées :** N1 antérieur 767 réussis/3 échecs microphone réel/128 ignorés, non relancé et jamais annoncé vert. Worker-common 11 réussis/1 ignoré, PE préalable réussi. Worker-cpu et worker-parakeet-cpu NON EXÉCUTÉS. Aucun moteur suspendu ni worker livré reconstruit ; libfvad seule compilation C autorisée. Qualité et TXT/SRT/WAV après non détaillés séparément dans le retour utilisateur.

**Mesures humaines :** recette réduite acceptée, quatre essais Parakeet Auto puis un par combinaison Parakeet/Turbo CPU/GPU. Après, l'utilisateur déclare les essais effectués, « Pour toutes les configurations env 0.5 seconde », retenu comme estimation commune des deux délais demandés.

| Borne | Avant | Après |
|---|---|---|
| Lancement → interface utilisable | ≈1 s | ≈0,5 s |
| Clic micro → compteur capturés qui progresse | ≈1,5 s | ≈0,5 s |

Amélioration indicative ; valeurs globales non ventilées, sans relevés individuels ni conditions de cache/source documentées. Aucun min/max ou gain statistique déduit. Automatisation Windows bloquée : ces mesures sont celles de l'utilisateur.

**Remise :** `deliverables/inspect-micro-demarrage/avant/` et `apres/`, trois EXE chacun ; mêmes workers Turbo, SHA/PE vérifiés. Ressources Parakeet/modèles dans l'installation personnelle stable, 20 fichiers revérifiés conformes. Desktop après SHA-256 `2f6bce0608d167497028258ddd0aec5f2c3ae8881fd148a39585a22ed525fda7`.

**Preuves/rétention :** [archive durable](raw/inspect-micro-demarrage.zip), 138 journaux, arborescence et chaque SHA vérifiés. ZIP SHA-256 `61b54043bb5ff96a32ba474da8a9f3453aaad02ca101bce8d0f1abe348a5ca29`. Aucun cache/cible de compilation isolé à supprimer ; partagés préservés, aucun nettoyage manuel, environ 427 Gio libres. Commits locaux, binaires non suivis, aucun push.
