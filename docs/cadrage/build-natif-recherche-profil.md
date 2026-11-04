Avancée : recherche élargie en lecture seule dans C:/Users/cyril le 4 novembre 2026, sur demande utilisateur ; aucune chaîne spike02 retrouvée.
Blocage : sources et sauvegarde de la chaîne historique toujours indisponibles ; BUILD-NATIF reste arrêté en phase 0.
Décision : suppression antérieure plausible selon l'utilisateur, sans preuve technique de suppression ; aucune reconstruction engagée.

# BUILD-NATIF — complément au bilan de phase 0

Le [bilan figé](../architecture/BUILD-NATIF-BILAN.md) conserve son état de clôture. Ce complément consigne la recherche supplémentaire autorisée dans le profil Windows, sans modifier les sources, DLL ou permissions.

## Recherche et résultats

- Énumération par `rg --files --hidden --no-ignore C:/Users/cyril`, y compris les fichiers cachés et ignorés : environ 1,63 million de chemins accessibles. Les nombres varient légèrement pendant la recherche ; aucun instantané immuable n'est revendiqué.
- Filtrage des chemins pour spike02, parakeet-feasibility, nemo_speech et llama.cpp ; recherche de noms d'archives ZIP/7z/tar/tgz/gz associés à Whisper, native, NeMo, ggml, llama ou Parakeet : aucune sauvegarde identifiable de la chaîne. Le contenu de toutes les archives génériques du profil n'a pas été inspecté.
- Recherche de contenu par `rg -l --hidden --no-ignore -g CMakeLists.txt -g CMakeCache.txt -g '*.cmake' 'nemo_speech_asr|parakeet-feasibility|spike02' C:/Users/cyril` : aucun fichier correspondant dans le périmètre accessible. Une recherche de contenu plus large incluant C++/headers/PowerShell a été interrompue après plusieurs minutes sans résultat ; elle n'est pas présentée comme exhaustive.
- Copies binaires NeMo retrouvées dans `AppData/Local/Whisper/native/{cpu,gpu}/`, un sous-dossier `AppData/Local/Temp/dll-*`, le cache local du paquet OpenAI.Codex (installation et staging Whisper), ainsi que `OneDrive/dell xps/Desktop/Whisper-CPU-Windows-x64/Whisper-CPU/Profil/Whisper/native/cpu/`. Ces DLL ne sont pas les sources ou les caches de construction recherchés ; aucune équivalence entre copies n'a été vérifiée ici.
- Le cache Cargo contient les sources ggml de whisper-rs-sys/whisper.cpp : elles ne démontrent pas la provenance du spike NeMo/llama.cpp. Un fichier de résumé historique portant spike02 dans son nom a été repéré, sans lecture de son contenu ni utilisation comme preuve de présence des sources.

## Limites et suite

`rg` signale un accès refusé sous `C:/Users/cyril/AppData/Local/Packages/whisperl04g2fixture/AC/Temp/g2-fixture8/stage` (code 2). Aucune élévation ou modification d'ACL effectuée. Les liens vers d'autres emplacements n'ont pas été suivis explicitement ; les autres volumes, la corbeille et les sauvegardes externes ne font pas partie de cette recherche. L'absence de résultat ne prouve donc pas une suppression définitive.

Aucune copie exploitable de spike02 retrouvée dans le profil accessible. Conformément au point d'arrêt, aucune sauvegarde factice, téléchargement de sources supposées, compilation ou substitution de DLL. Une éventuelle récupération depuis l'historique des commandes ou une sauvegarde constituerait une suite à décider ; la provenance ne doit pas être reconstruite au hasard.
