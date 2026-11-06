Avancée : code implémenté, revu CLEAN, commité localement et remise après vérifiée.
Blocage : mesures et validation Windows après attendues ; réserves N1/worker maintenues.
Décision : remettre le candidat et compléter T9 avant de figer ce bilan.

# INSPECT-MICRO-DEMARRAGE — bilan en préparation

**Non figé : T9 reste en attente.** Code livré dans `df81bf92` ; [plan et décisions](plan-inspect-micro-demarrage.md), [revue indépendante](revue-inspect-micro-demarrage.md).

Le démarrage et les quatre bindings micro inspectent le modèle demandé. Le catalogue distingue explicitement un modèle non inspecté. Le SHA et les lectures se font hors verrou global, avec coordination par clé et revalidation. Une commande nécessitant une inspection est refusée temporairement et doit être relancée explicitement ; aucune commande ni preuve GPU n'est conservée pour rejeu. CPU/GPU/Auto, capture, qualité et sorties gardent leurs contrats.

**Revue : CLEAN code**, epoch 1, même reviewer ; R1/R2 fermés, deux revues et un cycle de correction. Vérifications ciblées réussies : concurrence 11, catalogue 19 (inclut les 11), bindings 9, file/reprise 7, cœur 6, composition finale 34 avec 3 ignorés historiques, contrôleur 4, widget 11. Format, Clippy avec avertissements et build personnel réussis. Ces nombres ne sont pas additionnés comme tests distincts.

**Réserves acceptées :** N1 global avant correction : 767 réussis, 3 échecs d'ouverture du microphone Windows réel, 128 ignorés ; non relancé à environnement inchangé et jamais annoncé vert. Worker-common : 11 réussis, 1 ignoré, PE avant exécution réussi. Worker-cpu et worker-parakeet-cpu NON EXÉCUTÉS selon arbitrage. Aucun moteur natif suspendu ni worker livré reconstruit ; libfvad seule exception de compilation C autorisée.

**Mesures :** référence réduite acceptée : quatre essais Parakeet Auto et un dans chaque combinaison Parakeet/Turbo CPU/GPU ; interface environ 1 s, capture environ 1,5 s. Valeurs globales approximatives, source/cache non documentés. Après : en attente ; aucun gain revendiqué. Automatisation Windows bloquée par `GetCursorPos` accès refusé ; aucune mesure agent déduite. Texte, qualité, interruptions et TXT/SRT/WAV après restent à observer dans la fenêtre.

**Remise :** `deliverables/inspect-micro-demarrage/avant/` et `apres/`, trois EXE chacun. Même paire de workers Turbo avant/après, SHA des copies et PE des trois EXE après réussis. Modèles, workers Parakeet et DLL chargés depuis l'installation personnelle stable ; 20 ressources installées revérifiées taille/SHA, conformes. Desktop après : SHA-256 `2f6bce0608d167497028258ddd0aec5f2c3ae8881fd148a39585a22ed525fda7`.

**Preuves/rétention :** [archive durable](raw/inspect-micro-demarrage.zip), 138 journaux (1 519 901 octets sources), arborescence et chaque SHA vérifiés. ZIP 195 686 octets, SHA-256 `61b54043bb5ff96a32ba474da8a9f3453aaad02ca101bce8d0f1abe348a5ca29`. Aucune cible de compilation fraîche ou cache Cargo isolé créé à supprimer ; cible/cache partagés préservés, aucun nettoyage manuel. Espace libre après préparation : environ 427 Gio. Binaires non commités ; commits locaux uniquement, aucun push.
