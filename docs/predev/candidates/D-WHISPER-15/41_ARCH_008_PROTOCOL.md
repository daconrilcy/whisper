# ARCH-008-A2 v1 — apport du rust_architect

Acteur : `/root/architect_008`. Base D-WHISPER-13, digest `709b2e7ff87aef21cf5c77f0108b0c90f9d07fce8ac32eed758f41def9a8f9cb`. Proposition DRAFT, lecture seule. Aucun fichier ni essai modifié ou exécuté par ce spécialiste. La fermeture de 008 appartient au reviewer indépendant.

## Constats sur le banc actuel

- Le moteur accepte `cpu|gpu` et appelle `use_gpu(bool)`, sans protocole Auto, génération ou garantie GPU strict. Le fallback CPU silencieux est observé.
- Le stockage utilise Python et un MP3 fictif `b"MP3-demo"` : il soutient un protocole, sans démontrer la pile Rust et l'encodeur réel.
- Le banc d'encodage utilise `rusty_mp3 0.8.0`, un seul passage et `flush()` sans `sync_all()`. Media Foundation serait une autre option à éprouver.
- Le replay live appelle `tx.send()` sur une file de huit fenêtres : à saturation, sa source simulée ralentit. Ce comportement ne démontre pas une capture indépendante sans perte.
- Le paquet temporaire contient EXE, modèle et trois DLL CUDA. Installation, notices/licences livrées et premier lancement installé restent ouverts.

## Ordre des essais proposés hors dépôt produit

Racine nouvelle : `%TEMP%\whisper-spikes\closure-008\<run-id>`. Réutiliser seulement les WAV existants ; ne pas réécrire les bancs historiques. Les essais ci-dessous sont PROPOSED/NOT RUN à la remise de cet apport.

| Essai | Protocole | Attendu et preuve | Limite |
|---|---|---|---|
| E0 provenance | Versions OS/volume/toolchain ; `cargo metadata --locked`, `cargo tree --locked`, hashes sources, binaires, modèle et DLL ; révision/arbre natif. | Manifeste exact et tableau DLL→version/hash/origine/licence. | Le commit du wrapper `whisper-rs-sys` ne suffit pas à identifier `whisper.cpp` embarqué. |
| E1 installation | Installateur retenu ; acquisition modèle en staging ; interruption, hash faux et troncature, reprise ; exécution CPU/GPU installée avec PATH réduit et réseau refusé. | Aucun faux Ready ; modèle au hash attendu ; CPU/GPU offline installés ; fichiers, licences, modules chargés et traces réseau. | PC propre différé. PATH réduit n'exclut pas les DLL globales. Prouver CPU sans DLL CUDA si promis. |
| E2 modes/bascule | Harness Rust avec job_id/generation/segment_id/offset ; GPU présent/masqué, CPU forcé, GPU strict, Auto ; panne injectée après fenêtre 2 ; vieux résultat GPU. | Strict refuse/arrête sans bascule ; Auto reprend CPU au dernier confirmé ; backend effectif attesté ; absence de doublon/trou silencieux et rejet de génération périmée. | GPU masqué avant lancement et worker tué ne prouvent pas une panne réelle du pilote. |
| E3 archives | Trois passages existants, pause de groupe, silence interne ; encodeur Rust retenu ; MP3 par passage, TXT/SRT cumulés ; crash autour de finish/publication/marqueur ; double scan. | MP3 décodables, chronologie de groupe, anciens passages préservés ; interruption Recoverable ; Complete seulement après hashes de l'ensemble ; recovery idempotent. | Padding et qualité requièrent seuil explicite ; texte oracle ne prouve pas qualité ASR. |
| E4 confirmation/charge | Refaire journal/publication en Rust et primitives Windows retenues ; crash avant/après chaque frontière ; marqueur tronqué, hash faux, fichier absent/refus ; replay à horloge indépendante, writer/CPU ralentis. | Confirmation après sync ; récupération sans croissance illimitée, perte silencieuse ni faux Complete ; logs occupancy, backlog, RSS, sync/stop/recovery. | Arrêt de processus ≠ coupure électrique ; 500 ms reste le plafond de la fixture DEC-29. |
| E5 blocages/arrêt | Worker lent vivant, silence normal, heartbeat absent, worker et encodeur bloqués ; canal contrôle indépendant ; Stop, Quitter, libération et action manuelle simulée. | Entrée cessée, parent réactif, diagnostic et Recoverable ; parent reste ouvert sans choix manuel ; finalisation après résolution. | Borner la cessation d'entrée et le diagnostic, sans promettre la fin de Quitter si blocage arbitraire. |
| E6 VAD/UI | VAD Rust choisi/versionné sur mêmes échantillons ; annotations candidates corrigées humainement ; harness UI réellement rendu avec événements moteur et retard CPU. | Après vérification humaine, métriques VAD réelles ; backend et retard visibles dans widget consommateur, génération périmée rejetée, horodatages jusqu'à frame présentée. | Sans humain, proxies seulement ; harness ne prouve pas le futur produit ou micro EN/bruit courant absent. |

Budgets proposés : E0 30 min ; E1 2 h 30 ; E2 2 h 45 ; E3 2 h 30 ; E4 préparation 3 h puis campagnes 55 min ; E5 1 h 15 ; E6 2 h 30 hors vérification humaine. L'exécuteur doit fixer limite disque, durée et sortie d'arrêt. Le mandat historique SPIKE-01/02 est documenté en `19_SPIKE_RESULTS.md`.

Décisions techniques proposées : packaging CPU distinct ou CUDA chargé facultativement ; worker processus ou thread coopératif ; encodeur/version effectivement retenus ; journal Rust versionné et garanties natives. Ajouter risques techniques RISK-T-008-A..E reliés à R-01..05, sans remplacer ces risques.

Les preuves de faisabilité installation/moteur/archives/concurrence peuvent être obtenues avant code produit par banc fidèle. Qualification du produit final et mesure de capture physique vers rendu restent futures. L'annotation humaine et le seuil VAD ne peuvent être suppléés automatiquement. Résultat incomplet : DRAFT selon la décision utilisateur.

Sources primaires consultées par le spécialiste le 2026-10-05 : https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers ; https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw ; https://docs.nvidia.com/cuda/archive/12.8.0/eula/index.html . Ces primitives ne démontrent ni transaction multi-fichiers ni résistance universelle à une panne électrique.

Patch documentaire proposé : `41_ARCH_008_PROTOCOL.md` sur base D13, et deltas ciblés `09/33/10` après résultats. Ce document est une transcription structurée du rendu final du spécialiste par le coordinateur, non une copie verbatim de son paquet détaillé ARCH-008-A2.
