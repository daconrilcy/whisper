Avancée : perte du banc spike02 consignée le 6 novembre 2026 ; note figée.
Blocage : sources de travail, scripts et preuves non versionnés du banc introuvables ; cause non établie.
Décision : conserver les ressources survivantes ; aucun build natif ni remplacement de DLL autorisé par cette note.
# Perte de `target/parakeet-feasibility/spike02/`
Existence attestée le 15/10/2026 par les bilans de faisabilité versionnés ; clôture historique le 16/10.
Absence constatée le 06/11 selon l'enquête utilisateur : disque entier, AppData, Corbeille et cliché VSS du 05/11 à 14:39 monté et contrôlé.
Cette session ne reproduit pas l'enquête disque/VSS ; elle vérifie les sources Git et les ressources indiquées ci-dessous.
Ce n'est pas un `cargo clean` global dans `C:\dev\whisper` : `target\release` du 13/10 subsiste (création vérifiée), sans que cela explique la perte.
Absence déjà mentionnée dans `11f96fc8` (21/10, evo-parakeet-micro-live.md) et `9943493d` (04/11, dette-build-natif.md).
Ces mentions ont possiblement été écrites depuis un worktree ; elles ne datent pas la disparition sur tout le disque.
Subsistent : DLL installées, `docs/architecture/raw/native-build/installed-inventory.json` et `native/runtime-pins/reference.json` de `1d9e7d60`, sources épinglées et bilans versionnés.
Leur présence ne rétablit ni le banc original ni sa qualification ; aucun impact produit n'est signalé.
Cause non établie ; les bilans figés gardent leurs références historiques, même mortes.
