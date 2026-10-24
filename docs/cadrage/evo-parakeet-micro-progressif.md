Avancée : lot 1 validé fonctionnellement ; la trace du lot 2 montre des mots ultérieurs confirmables malgré un préfixe bloqué.
Blocage : le lot 2 reste non validé fonctionnellement ; le bruit parasite à la coupure reste une réserve du lot 1.
Décision : confirmation par segments indépendants et lacunes explicites autorisée pour le lot 2, sans commencer le lot 3.

# Parakeet au microphone : texte provisoire et blocs confirmés

## Périmètre et réutilisation

Le micro Parakeet affiche rapidement une hypothèse **provisoire, révisable**, puis publie uniquement les segments dont le texte et les bornes temporelles possèdent leurs propres preuves suffisantes. Les segments confirmés restent ordonnés, sans chevauchement et immuables ; une lacune antérieure explicite ne bloque pas la confirmation indépendante d'un segment ultérieur. Le WAV continu est la source de vérité de l'audio capturé ; un échec de raccordement ne supprime aucun échantillon. L'ancien banc FEAS-PM-01 et ses stratégies A/B sont clos ; leurs résultats servent de diagnostic, pas de validation de cette conception.

Réutiliser la capture et la conversion PCM 16 kHz de `capture.rs`, le WAV continu et la lecture bornée de `direct_live.rs`, la sélection du microphone et des calculs CPU/GPU/Auto, ainsi que le worker Parakeet CPU/GPU et son IPC `Transcribe` de fichiers. Chaque fenêtre est un WAV court distinct, créé dans le dossier temporaire de la séance, avec identité et SHA-256 propres ; l'adaptateur le soumet au worker existant puis reporte les temps de mots sur la position globale de la fenêtre. Cela évite un nouveau moteur et une nouvelle interface FFI. Le modèle et les DLL existants restent inchangés. Le traitement de plusieurs requêtes par une même instance du worker et l'admission du profil micro doivent être vérifiés dans le premier lot ; la qualification de l'import seul ne vaut pas admission du micro.

Le worker actuel rejette une requête dont les horodatages natifs sont invalides. Cette fenêtre est alors **non confirmable**, sans correction artificielle de ses temps ; l'audio correspondant reste dans le WAV et une fenêtre ultérieure ou le traitement de fin peut encore l'éclairer. Le texte de cette requête rejetée n'est pas disponible pour l'affichage provisoire avec l'IPC actuel. Cette limite doit être visible, sans inventer de mots ou d'horodatages.

## États et règle de publication

Maintenir, en échantillons 16 kHz sur une même chronologie : `audio_durable_end`, les portions non résolues, les hypothèses de fenêtres avec leur intervalle source et les segments confirmés. La fin du dernier segment confirmé ne signifie plus que tout l'audio précédent est confirmé. La mémoire de travail reste bornée ; le WAV conservé permet de relire tout intervalle non résolu, même lorsqu'il a quitté le recouvrement des dernières fenêtres. La capture continue pendant l'inférence ; une file saturée ou une capture interrompue produit un état incomplet explicite.

Après chaque résultat valide, afficher dans une zone « Texte provisoire » l'hypothèse la plus récente de la portion non confirmée, avec indication qu'elle peut changer. Elle ne remplace ni ne prolonge silencieusement un segment confirmé. Confirmer uniquement les mots concordants entre hypothèses possédant un recouvrement acoustique pertinent, avec des horodatages natifs valides ramenés au temps global et sans conflit lexical ou temporel dans le segment. Garder un contexte de part et d'autre des frontières ; les bords incertains et la fin encore susceptible de changer restent provisoires. Les critères de preuve R1/R2 restent inchangés ; une similarité textuelle seule, deux erreurs identiques ou le seul écoulement du temps ne suffisent pas à confirmer.

Si une portion sort du recouvrement sans preuve suffisante, enregistrer une **lacune non résolue** avec ses bornes audio. Elle ne peut être résolue que par une couverture positive, concordante et temporellement valide de toute sa plage ; une hypothèse vide ou une couverture partielle ne prouve pas le silence ni la résolution. Une lacune antérieure n'interdit pas un segment ultérieur dont les propres preuves sont suffisantes, mais elle reste visible et n'est jamais franchie implicitement. Ne pas créer de lacune pour chaque silence ordinaire entre mots. À l'arrêt, fermer la capture, conserver le WAV entier, traiter les portions déjà engagées dans les limites prévues, puis publier seulement les nouveaux segments satisfaisant la même règle de preuve. Ne jamais promouvoir automatiquement le provisoire.

Associer au WAV un fichier compagnon JSON versionné, avec l'identité de la capture, sa durée en échantillons, les intervalles confirmés et non résolus en bornes entières d'échantillons, leur statut et la complétude globale. Les TXT/SRT restent chronologiques et contiennent uniquement le confirmé ; leur suffixe `-incomplet` signale toute lacune ou portion provisoire restante. Le JSON conserve les bornes des omissions que les seuls TXT/SRT ne peuvent exprimer. Ne jamais inventer de texte pour une lacune ni annoncer une transcription complète si une portion reste non résolue.

La vitesse d'apparition du provisoire, la précision des mots et le délai du premier bloc confirmé sont trois mesures distinctes. Cette conception ne garantit pas à elle seule la cible existante de 15 s pour le premier texte confirmé sur GPU.

## Fichiers et lots proposés

| Lot | Modification ciblée | Critère de sortie |
| --- | --- | --- |
| 1 — Audio et worker | `crates/whisper-adapters/src/direct_live.rs` : partager le WAV continu et sa lecture bornée ; nouveau `crates/whisper-adapters/src/parakeet_live.rs` : fenêtres courtes avec contexte, file bornée et temps globaux ; `crates/whisper-adapters/src/worker_ipc.rs` et `parakeet_import.rs` : admission micro, réemploi du worker et routage CPU/GPU/Auto. Conserver `crates/whisper-core/src/parakeet_ipc.rs` et le worker natif sans changement si le contrat fichier suffit. | Capture sans perte pendant des inférences successives ; résultats identifiés par séance/fenêtre et rejet des résultats tardifs ; erreurs de temps classées sans perte audio. |
| 2 — Confirmation et fin | Réconciliation dans `crates/whisper-adapters/src/parakeet_confirmation.rs` et `parakeet_live.rs` ; `crates/whisper-core/src/ports.rs` et `application.rs` : événements et états séparés provisoire/confirmé/lacune ; `direct_live.rs` : export TXT/SRT et fichier compagnon JSON associé au WAV. | Segments confirmés indépendants, ordonnés et immuables, sans chevauchement ni doublon ; lacunes conservées avec leurs bornes, sortie explicitement complète ou incomplète. |
| 3 — Interface et recette | `crates/whisper-desktop/src/ui.rs` : autoriser Parakeet micro seulement si le parcours est disponible, montrer provisoire, confirmé, lacunes et calcul effectif ; vérifications ciblées dans les modules touchés, puis essai réel dans la fenêtre Windows par l'utilisateur. | Parcours CPU, GPU, Auto et choix du micro compréhensibles ; imports Parakeet et micro Turbo inchangés ; sorties et messages vérifiés. |

### Clôture du lot 1 — 23 octobre 2026

L'utilisateur valide la recette manuelle du nouvel EXE Test (`SHA-256 F44FD01A5FAECD74FF1DC18C4BE25342A3AC909ED4A1DEA096530BE324C353B0`) : le worker Parakeet GPU démarre, le texte provisoire apparaît et évolue pendant la capture, les WAV enregistrés sont complets et corrects, et le texte provisoire n'est pas exporté comme confirmé. Cette validation porte sur le fonctionnement du lot 1, sans anticiper les blocs confirmés du lot 2.

Lot 1 **clos avec réserves** : la revue indépendante n'a pas pu être effectuée ; un bruit parasite à la coupure a été observé par l'utilisateur, sans cause établie dans cette recette. Aucune nouvelle campagne n'est demandée pour cette clôture.

Le protocole de mots, les profils de worker et les contraintes de durée du worker restent inchangés tant que le contrat fichier suffit. Si le réemploi du worker impose une extension IPC, la limiter au transport des fenêtres, sans modifier l'appel natif ni considérer le profil import comme une preuve de qualité micro. Le plan détaillé de ce point et les chemins exacts seront soumis avec le premier lot.

## Acceptation dans l'application Windows

1. En français et en anglais, avec pauses puis parole continue, un texte explicitement provisoire apparaît pendant la capture et peut changer ; les blocs confirmés restent inchangés après leur affichage et après l'arrêt.
2. Les échantillons annoncés capturés couvrent le WAV final sans trou. Un désaccord, un temps invalide, une file saturée ou une erreur de worker ne supprime pas de portion audio et ne transforme pas une lacune en confirmation.
3. À chaque frontière examinée à l'écoute, les segments confirmés respectent l'ordre, sans chevauchement, doublon ni mot tronqué introduit par le découpage. Une lacune antérieure reste explicitement non résolue même lorsqu'un segment ultérieur est confirmé. La qualité lexicale des mots confirmés est évaluée séparément de leur stabilité, avec une référence humaine ; un mot simplement répété dans deux hypothèses n'est pas réputé exact.
4. L'arrêt traite le reliquat sans promouvoir automatiquement le provisoire. S'il reste une lacune, la fenêtre indique « transcription incomplète », le WAV est accessible, les TXT/SRT sont identifiés incomplets et ne contiennent pas le provisoire comme texte validé ; le JSON associé au WAV garde les bornes en échantillons de chaque intervalle non résolu. Une séance complète produit les sorties attendues avec les temps globaux corrects.
5. Le calcul demandé et le moteur effectif CPU/GPU/Auto, la source micro choisie et les règles de repli existantes restent respectés. Les imports Parakeet et la dictée Turbo ne régressent pas.
6. Dans la fenêtre réelle, relever séparément le délai du provisoire et celui du premier bloc confirmé, notamment par rapport à l'exigence GPU de 15 s, ainsi que le retard de traitement. Un échec de cette cible est rapporté ; il n'est pas masqué par un provisoire rapide.

**Gate :** demander un GO écrit précisant le lot 2, les chemins autorisés et les vérifications. Ce document n'autorise ni modification du produit, ni compilation, ni campagne FEAS-PM-01.
