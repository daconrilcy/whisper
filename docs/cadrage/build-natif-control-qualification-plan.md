Avancée : complément de qualification autorisé le 5 novembre 2026 après le candidat 237323b3.
Blocage : débit supérieur à une seconde ; cadence réelle et SelfTest non qualifiés.
Décision : une heure supplémentaire autorisée, borne 13:00 Paris / 12:00 UTC ; seuil et garde inchangés.

# Complément de qualification du contrôle NativeBuild

Objectif : lever les réserves du correctif de contrôle, sans consommer les deux compilations CPU avant qualification. L'accord utilisateur couvre ce complément borné et au plus deux cycles de revue/correction. Ce plan ne qualifie pas les DLL ni N2 par anticipation.

1. Mesurer séparément assemblage, classement et journaux à partir des données conservées ; diagnostic au plus trois essais ou trente minutes. Conserver les temps et erreurs avant toute assertion. Aucun watcher supplémentaire de diagnostic.
2. Optimiser `ci/native-build-control.cs` et son chargement dans `ci/guard-native.ps1` ; tests dans `ci/test-native-build-control.ps1` (et tests NativeBuild existants si nécessaire). Conserver tous les champs, décisions, frontières, erreurs fatales, nettoyage et la pause de 100 ms. `ci/native-directory-watch.cs` reste inchangé.
3. Gate hors compilation : parité complète de politique/journaux, traces/fixtures ambiguës et orphelines toujours refusées, write/flush/dispose et fermeture intégrée ; pointes de 55 414 records, assemblage/classement/journaux/flush ≤ 1 seconde et intervalle de drainage ≤ 1 seconde. Aucun préchauffage exclusif au test pour passer une première pointe trop lente.
4. Qualification du cycle réel avec le collecteur existant puis SelfTest NativeBuild complet, strictement séquentiels. Vérifier acquisition, nombres/identités, absence de pertes, cadence, refus immédiats. Aucun changement de capacité, délai, règle de renommage, chemin ou politique Temp. Tout échec arrête la campagne et appelle l'arbitrage prévu par AGENTS.md.
5. Revue indépendante au plus deux cycles ; archivage vérifié des preuves et nouveau bilan d'une page. Les anciens bilans et archives restent figés. Pas de suppression, push, fusion ou remplacement des DLL. Si les gates ne passent pas dans le budget : réserves restantes explicites, aucune compilation CPU.

Point d'arrêt à chaque étape en cas de blocage ; arrêt absolu à la borne budgétaire. La reprise des deux builds CPU conserve ses prérequis et son plafond antérieurs ; elle n'est jamais déduite d'un simple gain mesuré.
