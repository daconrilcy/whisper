# Autorisation utilisateur — correction Q-07 / entrée L02

Source : message direct de HUMAN-USER reçu dans la conversation Codex active le 2026-10-06. Il signale que Q-07 bloque l’entrée L02 et demande les actions correctives nécessaires pour permettre le démarrage.

Portée de la mission : résoudre Q-07, détail technique réversible déjà requis par P-WHISPER-06, par une réponse attribuée à un architecte Rust ; transporter fidèlement cette contribution dans `transports/` ; mettre à jour le checkpoint de contrôle uniquement après vérification des références. Les limites mémoire, l’absence du contrôle de placement et l’exclusion du GPU restent telles que documentées. Cette autorisation porte sur la correction documentaire et le préflight ; elle ne constitue pas une autorisation d’implémenter les chemins de L-WHISPER-02.

Capture lisible de la demande :

> L02 ne peut pas encore démarrer : Q-07 est toujours ouvert, et le contrôle de transfert vers L02 échoue sur ce détail requis. Les limites mémoire, l’absence du contrôle de placement et le GPU hors périmètre sont documentés.
>
> Fais les actions correctives necessaires pour que L02 puisse demarrer

La capture conserve le contenu affiché dans la conversation ; elle ne prétend pas être un export octet à octet du protocole de chat.
