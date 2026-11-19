Avancée : l’essai F4 CPU a abouti sur la VM Snapdragon X2 Elite du Qualcomm Device Cloud.
Blocage : aucun pour l’acceptation F4 ; une exécution native ARM64 n’a pas été démontrée.
Décision : F4 validé par l’utilisateur le 19 novembre 2026, avec une erreur de transcription acceptée.

# Bilan F4 Parakeet sur Snapdragon X2 — 19 novembre 2026

L’archive de retour `RETOUR-Snapdragon-X2-2306a8eda2ab4749b6ea38d5adf56160.zip` (SHA-256 `6AD4819890EE8F017E3B724F4054EF10088059AEC888501ABBFF9CC7FD92BC61`) rapporte `CONTROLES_OK` pour le cas F4, backend CPU. La machine est identifiée comme Snapdragon X2 Elite (X2E88100, Qualcomm Oryon), sous Windows 11 Enterprise ARM64, build 28000. Le worker Parakeet CPU x64 a terminé avec le code 0.

Le résultat IPC correspond au modèle attendu et au WAV de référence de 64 000 échantillons, soit 4 secondes à 16 kHz. Les mots horodatés concordent avec le texte retourné ; l’inférence native rapportée est de 721 ms. Cela valide l’essai de worker CPU sur cette VM Qualcomm. Ce temps est celui de l’inférence, pas une mesure du démarrage complet ni une mesure de performance sur un PC local.

Le PE du worker est x64 (`Machine=34404`) alors que la machine hôte est ARM64 (`nativeMachine=43620`) : cet essai ne démontre pas un binaire ARM64 natif. La sonde CPUID/XGETBV est indiquée comme non établie. Le résultat reste limité à une inférence CPU hors interface ; il ne qualifie pas l’interface, Auto, le microphone ou les exports.

Le texte produit contient « Haute Gold » au lieu de « Hong Kong ». Après en avoir pris connaissance, l’utilisateur valide néanmoins le contenu pour cet essai, car l’audio est difficile même pour un humain. Le statut utilisateur est donc **validé avec erreur de transcription acceptée** ; l’écart est conservé explicitement plutôt que présenté comme une transcription exacte.

L’archive cite une attestation F3 précédente dans `attestation-precedente.txt`. Le retour F4 n’embarque pas cette attestation, et ce bilan ne requalifie pas les résultats F1 à F3.
