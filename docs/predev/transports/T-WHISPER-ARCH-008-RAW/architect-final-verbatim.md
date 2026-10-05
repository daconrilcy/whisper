**ARCH-008-A2 v1 — proposition DRAFT, lecture seule.** Le paquet brut complet vient d’être transmis au coordinateur. Persistance TRANSPORT et reçu vérifié attendus ; aucun fichier ni essai modifié/exécuté.

Base vérifiée : **D-WHISPER-13**, digest `709b2e7ff87aef21cf5c77f0108b0c90f9d07fce8ac32eed758f41def9a8f9cb`. Owner : `rust_architect` ; finding **008 OPEN**, fermeture réservée au reviewer indépendant.

Les écarts déterminants observés sont :

- Le moteur actuel accepte `cpu|gpu` et appelle `use_gpu(bool)` sans protocole Auto, génération ou garantie GPU strict. Le fallback CPU silencieux est déjà observé.
- Le stockage actuel utilise **Python** et un MP3 fictif `b"MP3-demo"` : il soutient le protocole, mais ne prouve pas la pile Rust avec encodeur réel.
- Le véritable encodeur du banc est **rusty_mp3 0.8.0**, avec un seul passage et `flush()` sans `sync_all()`. Media Foundation constitue une autre option.
- Le replay live appelle `tx.send()` sur une file de huit fenêtres : à saturation, il ralentit sa source simulée. Il ne prouve pas le comportement d’un callback capture indépendant.
- Le paquet temporaire contient EXE, modèle et trois DLL CUDA ; installation, licences livrées et premier lancement installé restent ouverts.

Ordre proposé, dans un dossier neuf `%TEMP%\whisper-spikes\closure-008\<run-id>`, uniquement avec les WAV existants et sans réécrire les bancs historiques :

| Essai | Fixture / protocole | Résultat attendu et preuves | Limite |
|---|---|---|---|
| **E0 : provenance** | Versions OS/volume/toolchain ; `cargo metadata --locked`, `cargo tree --locked`, hashes sources/binaire/modèle/DLL ; révision et arbre natif embarqués. | Manifest exact reproductible, tableau DLL→version/hash/origine/licence. | Le commit wrapper de `whisper-rs-sys` ne suffit pas à identifier `whisper.cpp` embarqué. |
| **E1 : installation** | Installer réellement retenu ; acquisition modèle staging ; interruption, hash faux/troncature, reprise ; exécution installée CPU/GPU avec PATH réduit et réseau refusé au processus. | Aucun faux Ready ; modèle SHA-256 attendu ; CPU/GPU offline installé ; logs, fichiers/licences, modules chargés et trace réseau. | PC propre différé. PATH réduit seul n’exclut pas une DLL globale. Si CPU est promis sans CUDA, le packaging doit le démontrer. |
| **E2 : modes et bascule** | Harness Rust avec `job_id/generation/segment_id/offset` ; GPU présent/masqué, CPU forcé, GPU strict, Auto ; failure injectée après fenêtre 2 ; ancien résultat GPU retardé. | Strict refuse/arrête sans bascule ; Auto reprend CPU au dernier confirmé ; backend effectif attesté ; aucun doublon/trou silencieux ; génération périmée rejetée. | Masquer un GPU avant lancement et tuer un worker prouvent des scénarios distincts. Ils ne prouvent pas une panne réelle du driver. |
| **E3 : archives** | Trois passages existants, pause de groupe déterministe, silence interne ; encodeur Rust retenu ; MP3 par passage, TXT/SRT cumulés ; crash autour de finish/publication/marqueur ; double scan. | MP3 décodables, chronologie de groupe correcte, anciens passages préservés ; interruption Recoverable ; Complete seulement après hashes de l’ensemble ; recovery idempotent. | Padding et qualité exigent un seuil explicite ; texte oracle prouve offsets, pas qualité ASR. |
| **E4 : confirmation/charge** | Refaire journal/publication en Rust avec primitives Windows retenues ; crash avant/après chaque frontière ; marqueur tronqué, hash faux, fichiers manquants/refus ; replay à horloge indépendante, writer/CPU ralentis. | Confirmation seulement après sync ; confirmations récupérées ; aucune croissance illimitée/perte silencieuse/faux Complete ; saturation explicite ; journal occupancy, backlog, RSS, sync/stop/recovery. | Arrêt de processus ≠ coupure électrique. Le plafond 500 ms reste celui de DEC-29, sur sa fixture actuelle. |
| **E5 : blocages/arrêt** | Worker lent vivant, silence normal, heartbeat absent, worker et encodeur bloqués ; canal contrôle distinct ; Stop, Quitter, libération et action manuelle simulée. | Entrée cessée, parent réactif, diagnostic et Recoverable ; parent reste ouvert conformément au choix accepté ; finalisation après résolution ; kill ciblé seulement sur action explicite. | Borner cessation d’entrée/diagnostic ; aucune promesse de Quitter toujours terminé si le produit doit rester ouvert lors d’un blocage. |
| **E6 : VAD/UI** | VAD Rust choisi/versionné sur mêmes samples ; annotations candidates corrigées humainement ; harness UI réellement rendu avec événements moteur et retard CPU. | Après annotation humaine : métriques VAD réelles. UI : backend/retard visibles, résultats obsolètes rejetés, timestamps jusqu’à frame présentée et preuve dans le widget consommateur. | Sans humain, garder proxies automatiques. Le replay/harness ne prouve ni micro EN/bruit réel absent ni rendu du futur produit. |

Budgets proposés : E0 30 min ; E1 2 h 30 ; E2 2 h 45 ; E3 2 h 30 ; E4 préparation 3 h puis campagnes 55 min ; E5 1 h 15 ; E6 2 h 30 hors vérification humaine. L’exécuteur doit annoncer limite disque, durée et sortie d’arrêt avant lancement. Le mandat historique SPIKE-01/02 est attesté dans `19` ; le coordinateur vérifie sa portée et désigne l’acteur séparé.

Décisions **techniques proposées**, à tracer avant essais :

- Packaging CPU distinct ou chargement CUDA facultatif.
- Worker processus versus thread coopératif, tranché par E2/E5.
- Encodeur/version réellement retenus, tranchés par E3 et licences.
- Journal Rust/publication versionnée et garanties natives, tranchés par E4.

Ces décisions ajoutent des risques `RISK-T-008-A..E` reliés aux `R-01..05` existants, sans remplacer leur registre.

Installation, moteur, archives et concurrence peuvent obtenir des preuves de faisabilité **avant code produit** par banc fidèle. Qualification du produit final et mesure capture physique→rendu restent futures. L’annotation humaine et le seuil VAD ne peuvent être suppléés par un nouveau calcul automatique. Un résultat incomplet maintient DRAFT selon la dernière décision utilisateur.

Les primitives Windows documentent flush et déplacement ; elles ne démontrent pas une transaction MP3/TXT/SRT ou une garantie universelle après panne électrique. Sources consultées le 2026-10-05 : [FlushFileBuffers, MAJ 2021-10-13](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-flushfilebuffers), [MoveFileExW, MAJ 2023-06-01](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw). Pour le packaging, mapper chaque binaire à son texte précis : [CUDA EULA 12.8, MAJ 2025-01-07](https://docs.nvidia.com/cuda/archive/12.8.0/eula/index.html).

Patch proposé : nouvelle entrée **`41_ARCH_008_PROTOCOL.md`**, owner `rust_architect`, base D13, v1 DRAFT ; deltas ciblés `09/33/10` après résultats, revue sur nouveau candidat manifesté. Le paquet brut contient les protocoles détaillés, hashes et questions restantes.

