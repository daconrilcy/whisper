Avancée : dette BUILD-NATIF inscrite le 4 novembre 2026 pour préserver la capacité de reconstruire les DLL natives du produit.
Blocage : chaîne de compilation reproductible non établie ; `target/parakeet-feasibility/` absent du checkout inspecté à cette date.
Décision : priorité supérieure aux renommages L4/L04 ; conservation du chemin inscrite dans AGENTS.md ; lot d'implémentation à cadrer et à autoriser.

# BUILD-NATIF — provenance et reconstruction des DLL ggml/NeMo

## Constat et risque produit

L'utilisateur signale que les DLL natives installées proviennent d'une compilation de spike de faisabilité sous `target/`, sans script versionné connu pour les reproduire. La [note Perso](note-parakeet-l6-perso.md) décrit une copie des DLL depuis `target/parakeet-tests/l2/l2b/native/` ; les [fiches R6](fiche-parakeet-r6.md) et [Perso](fiche-parakeet-l6-perso.md) consignent des identités binaires. Ces empreintes identifient les fichiers, mais ne suffisent pas à reconstruire leurs sources, options et outils de compilation.

La recherche ciblée dans `ci/` et `scripts/` n'a pas identifié de `ci/build-native.ps1`. Elle ne démontre pas l'absence de toute procédure ailleurs. Le dossier `target/parakeet-feasibility/` est absent de ce checkout : sa conservation effective et la disponibilité de la chaîne historique ne sont donc pas établies. Aucune suppression ni reconstruction n'a été effectuée pour cette inscription.

La perte des sources, modifications locales ou paramètres du spike empêcherait de reconstruire, corriger ou remettre à jour les DLL utilisées pour la transcription locale. Cette dette passe avant les renommages L4/L04.

## Futur lot proposé

Prévoir `ci/build-native.ps1` versionné pour construire les DLL ggml/NeMo CPU et GPU nécessaires au produit. Le cadrage devra identifier les dépôts réellement utilisés, figer leurs révisions exactes (dont llama.cpp), préserver les éventuels correctifs locaux et préciser versions des outils, options, prérequis et sorties. La version de llama.cpp seule ne suffit pas à fixer la provenance de NeMo.

Critères proposés pour le futur plan :

- Reconstruire depuis des sources figées et les outils documentés, sans dépendre du spike sous `target/`.
- Publier les SHA-256 des DLL produites avec leurs révisions sources et paramètres de compilation ; distinguer cette traçabilité d'une reproductibilité binaire, qui reste à démontrer.
- Vérifier les dépendances natives et la compatibilité avec les workers et profils CPU/GPU existants avant toute substitution aux DLL installées.
- Établir la disponibilité de la nouvelle chaîne avant de proposer la levée de la conservation du spike à l'utilisateur.

Le présent document inscrit la dette et le périmètre proposé ; il n'autorise ni création du script, ni téléchargement, ni compilation, ni remplacement des DLL. La règle de conservation figure dans [AGENTS.md](../../AGENTS.md), conformément à la séparation entre consignes et suivi de dette.
