Avancée : lot 1 validé fonctionnellement par l'utilisateur dans la fenêtre Windows le 23 octobre 2026.
Blocage : la confirmation progressive reste à réaliser ; le bruit parasite à la coupure reste une réserve du lot 1.
Décision : lot 1 clos avec réserves ; l'implémentation du lot 2 nécessite un GO distinct.

# Parakeet au microphone : texte provisoire et blocs confirmés

## Périmètre et réutilisation

Le micro Parakeet affiche rapidement une hypothèse **provisoire, révisable**, puis publie uniquement les blocs dont le texte et la continuité temporelle sont établis. Le préfixe confirmé reste ordonné et immuable. Le WAV continu est la source de vérité de l'audio capturé ; un échec de raccordement ne fait jamais avancer le curseur confirmé et ne supprime aucun échantillon. L'ancien banc FEAS-PM-01 et ses stratégies A/B sont clos ; leurs résultats servent de diagnostic, pas de validation de cette conception.

Réutiliser la capture et la conversion PCM 16 kHz de `capture.rs`, le WAV continu et la lecture bornée de `direct_live.rs`, la sélection du microphone et des calculs CPU/GPU/Auto, ainsi que le worker Parakeet CPU/GPU et son IPC `Transcribe` de fichiers. Chaque fenêtre est un WAV court distinct, créé dans le dossier temporaire de la séance, avec identité et SHA-256 propres ; l'adaptateur le soumet au worker existant puis reporte les temps de mots sur la position globale de la fenêtre. Cela évite un nouveau moteur et une nouvelle interface FFI. Le modèle et les DLL existants restent inchangés. Le traitement de plusieurs requêtes par une même instance du worker et l'admission du profil micro doivent être vérifiés dans le premier lot ; la qualification de l'import seul ne vaut pas admission du micro.

Le worker actuel rejette une requête dont les horodatages natifs sont invalides. Cette fenêtre est alors **non confirmable**, sans correction artificielle de ses temps ; l'audio correspondant reste dans le WAV et une fenêtre ultérieure ou le traitement de fin peut encore l'éclairer. Le texte de cette requête rejetée n'est pas disponible pour l'affichage provisoire avec l'IPC actuel. Cette limite doit être visible, sans inventer de mots ou d'horodatages.

## États et règle de publication

Maintenir, en échantillons 16 kHz sur une même chronologie : `audio_durable_end`, `confirmed_end`, les portions non résolues, les hypothèses de fenêtres avec leur intervalle source et les blocs confirmés. La mémoire de travail reste bornée ; le WAV conservé permet de relire tout intervalle non résolu, même lorsqu'il a quitté le recouvrement des dernières fenêtres. La capture continue pendant l'inférence ; une file saturée ou une capture interrompue produit un état incomplet explicite.

Après chaque résultat valide, afficher dans une zone « Texte provisoire » l'hypothèse la plus récente du suffixe non confirmé, avec indication qu'elle peut changer. Elle ne remplace ni ne prolonge silencieusement un bloc confirmé. Confirmer, dans l'ordre, uniquement une portion dont les mots concordent entre hypothèses possédant un recouvrement acoustique pertinent, avec des horodatages natifs valides ramenés au temps global, et dont les deux bords sont raccordés à la portion précédente sans mot manquant, répété ou tronqué. Garder un contexte de part et d'autre des frontières ; les bords incertains et la fin encore susceptible de changer restent provisoires. Le dernier mot confirmé n'a pas à réapparaître dans toute fenêtre suivante : la preuve de raccordement déjà conservée accompagne le curseur confirmé. Une similarité textuelle seule, deux erreurs identiques ou le seul écoulement du temps ne suffisent pas à confirmer.

Si une portion sort du recouvrement sans preuve suffisante, enregistrer une **lacune non résolue** avec ses bornes audio. Ne jamais confirmer au-delà de cette lacune. Une hypothèse ultérieure avec davantage de contexte peut la résoudre ; sinon elle reste provisoire ou sans texte. À l'arrêt, fermer la capture, conserver le WAV entier, traiter une dernière fois les portions non résolues avec un contexte borné, puis publier les nouveaux blocs uniquement si la même règle de preuve est satisfaite. Si une lacune subsiste, terminer en état **incomplet** : garder le WAV intégral, présenter la portion à revoir et produire des TXT/SRT clairement marqués incomplets, limités aux blocs confirmés. Ne jamais étiqueter le suffixe provisoire comme confirmé ni annoncer une transcription complète.

La vitesse d'apparition du provisoire, la précision des mots et le délai du premier bloc confirmé sont trois mesures distinctes. Cette conception ne garantit pas à elle seule la cible existante de 15 s pour le premier texte confirmé sur GPU.

## Fichiers et lots proposés

| Lot | Modification ciblée | Critère de sortie |
| --- | --- | --- |
| 1 — Audio et worker | `crates/whisper-adapters/src/direct_live.rs` : partager le WAV continu et sa lecture bornée ; nouveau `crates/whisper-adapters/src/parakeet_live.rs` : fenêtres courtes avec contexte, file bornée et temps globaux ; `crates/whisper-adapters/src/worker_ipc.rs` et `parakeet_import.rs` : admission micro, réemploi du worker et routage CPU/GPU/Auto. Conserver `crates/whisper-core/src/parakeet_ipc.rs` et le worker natif sans changement si le contrat fichier suffit. | Capture sans perte pendant des inférences successives ; résultats identifiés par séance/fenêtre et rejet des résultats tardifs ; erreurs de temps classées sans perte audio. |
| 2 — Confirmation et fin | Nouveau module de réconciliation dans `crates/whisper-adapters/src/parakeet_live.rs` ou un module voisin si la taille l'exige ; `crates/whisper-core/src/ports.rs` et `application.rs` : événements et états séparés provisoire/confirmé/lacune ; réutiliser l'export TXT/SRT/WAV et l'état incomplet de `direct_live.rs`. | Préfixe confirmé immuable, aucun saut ni doublon aux raccords, lacunes conservées et fin de séance explicitement complète ou incomplète. |
| 3 — Interface et recette | `crates/whisper-desktop/src/ui.rs` : autoriser Parakeet micro seulement si le parcours est disponible, montrer provisoire, confirmé, lacunes et calcul effectif ; vérifications ciblées dans les modules touchés, puis essai réel dans la fenêtre Windows par l'utilisateur. | Parcours CPU, GPU, Auto et choix du micro compréhensibles ; imports Parakeet et micro Turbo inchangés ; sorties et messages vérifiés. |

### Clôture du lot 1 — 23 octobre 2026

L'utilisateur valide la recette manuelle du nouvel EXE Test (`SHA-256 F44FD01A5FAECD74FF1DC18C4BE25342A3AC909ED4A1DEA096530BE324C353B0`) : le worker Parakeet GPU démarre, le texte provisoire apparaît et évolue pendant la capture, les WAV enregistrés sont complets et corrects, et le texte provisoire n'est pas exporté comme confirmé. Cette validation porte sur le fonctionnement du lot 1, sans anticiper les blocs confirmés du lot 2.

Lot 1 **clos avec réserves** : la revue indépendante n'a pas pu être effectuée ; un bruit parasite à la coupure a été observé par l'utilisateur, sans cause établie dans cette recette. Aucune nouvelle campagne n'est demandée pour cette clôture.

Le protocole de mots, les profils de worker et les contraintes de durée du worker restent inchangés tant que le contrat fichier suffit. Si le réemploi du worker impose une extension IPC, la limiter au transport des fenêtres, sans modifier l'appel natif ni considérer le profil import comme une preuve de qualité micro. Le plan détaillé de ce point et les chemins exacts seront soumis avec le premier lot.

## Acceptation dans l'application Windows

1. En français et en anglais, avec pauses puis parole continue, un texte explicitement provisoire apparaît pendant la capture et peut changer ; les blocs confirmés restent inchangés après leur affichage et après l'arrêt.
2. Les échantillons annoncés capturés couvrent le WAV final sans trou. Un désaccord, un temps invalide, une file saturée ou une erreur de worker ne supprime pas de portion audio et ne transforme pas une lacune en confirmation.
3. À chaque frontière examinée à l'écoute, les blocs confirmés respectent l'ordre, sans perte, doublon ni mot tronqué introduit par le découpage. La qualité lexicale des mots confirmés est évaluée séparément de leur stabilité, avec une référence humaine ; un mot simplement répété dans deux hypothèses n'est pas réputé exact.
4. L'arrêt traite le reliquat. S'il reste une lacune, la fenêtre indique « transcription incomplète », le WAV est accessible, les TXT/SRT sont identifiés incomplets et ne contiennent pas le provisoire comme texte validé. Une séance complète produit les trois sorties attendues avec les temps globaux corrects.
5. Le calcul demandé et le moteur effectif CPU/GPU/Auto, la source micro choisie et les règles de repli existantes restent respectés. Les imports Parakeet et la dictée Turbo ne régressent pas.
6. Dans la fenêtre réelle, relever séparément le délai du provisoire et celui du premier bloc confirmé, notamment par rapport à l'exigence GPU de 15 s, ainsi que le retard de traitement. Un échec de cette cible est rapporté ; il n'est pas masqué par un provisoire rapide.

**Gate :** demander un GO écrit précisant le lot 2, les chemins autorisés et les vérifications. Ce document n'autorise ni modification du produit, ni compilation, ni campagne FEAS-PM-01.
