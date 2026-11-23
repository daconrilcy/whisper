Avancée : l’installation P9 et son certificat de test ont été retirés après la recette.
Blocage : aucun pour ce nettoyage ; les artefacts de test restent conservés dans la cible.
Décision : P8, ses données utilisateur, les transcriptions et le bilan P9 sont préservés.

# Suivi de nettoyage P9 — 23 novembre 2026

- Package `WhisperTurbo.NvidiaPrototype_0.1.0.0_x64__yxmgx5c9e80fr` désinstallé du compte courant. Vérifications : package absent, aucun processus encore lancé depuis son `InstallLocation`.
- Le `LocalState` P9 a été supprimé par la désinstallation normale, après confirmation de l’utilisateur. `-PreserveApplicationData` n’est pas compatible avec ce package de production/test signé installé, Windows a refusé cette option avant la désinstallation.
- Certificat `CN=WhisperTurbo MSIX NVIDIA Prototype`, thumbprint `2669EF242D2202EBE7D4496913FB84C38B34D154`, retiré de `Cert:\LocalMachine\TrustedPeople` après vérification de son identité. La P9 n’est donc plus approuvée localement.
- P8 `WhisperTurbo.CpuPrototype` reste installé. Le bilan P9 hors `target/` et les transcriptions TXT/SRT dans Documents sont toujours présents.
- Ce nettoyage n’a pas supprimé les fichiers d’assemblage dans `target/` : le MSIX signé de test et le PFX privé associé y restent conservés. Ils ne sont pas des livrables Store et ne doivent pas être distribués. Aucun autre certificat n’a été retiré.

Le bilan P9 figé est complété par cette note uniquement : [bilan P9](bilan-prototype-msix-nvidia-20261123.md).
