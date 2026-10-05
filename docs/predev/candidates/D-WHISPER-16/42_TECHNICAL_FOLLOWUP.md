# Suite technique de 008 — D14

**Statut : PARTIAL ; aucun PASS pour 008.** DEC-32 maintient DESIGN DRAFT et demande des preuves techniques supplémentaires. Le protocole proposé par rust_architect est dans `41` et son transport attribué `T-WHISPER-ARCH-008`. Les essais ci-dessous ont été effectués dans `%TEMP%\whisper-spikes\closure-008`, hors dépôt produit.

## E0 — inventaire reproductible

`evidence/e0_inventory.py.txt` a relevé 17 fichiers du banc existant avec tailles et SHA-256 dans `evidence/e0-inventory.json`. Le modèle de 1 624 555 275 octets a le SHA-256 attendu `1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69`. `rustc -Vv`, `cargo -V`, puis `cargo metadata --locked --format-version 1 --no-deps` et `cargo tree --locked` pour engine, mp3-encode et formats : huit commandes, huit codes retour zéro. Ces contrôles identifient la pile Cargo et les octets présents ; ils ne fixent pas encore la révision de l'arbre natif whisper.cpp embarqué, ni une matrice licence binaire→texte, ni les DLL réellement chargées au premier lancement installé. E0 est donc PARTIAL.

## E3 — smoke multi-passages limité

La source Rust `evidence/e3-multipass-main.rs.txt` emploie hound et rusty_mp3 0.8.0 du lock copié, écrit puis `sync_all()` chaque MP3 et les deux fichiers cumulés. Trois WAV FLEURS existants sont lus en 16 kHz mono (f32 converti en i16) ; un passage par WAV ; pause de groupe déterministe de 1 500 ms ; texte SRT synthétique de fixture. `cargo run --release --locked` s'est terminé avec code 0 après correction du format d'entrée f32.

| Passage | Échantillons source | Départ groupe | Fin SRT | MP3 | Décodage Symphonia |
|---|---:|---:|---:|---:|---:|
| FR 1 | 231 360 | 0 ms | 14 460 ms | 116 064 octets | 231 552 échantillons |
| EN 2 | 104 640 | 15 960 ms | 22 500 ms | 52 704 octets | 104 832 échantillons |
| FR 3 | 96 000 | 24 000 ms | 30 000 ms | 48 384 octets | 96 192 échantillons |

Chaque MP3 se décode ; l'excédent décodé est de 192 échantillons, soit 12 ms par passage. `evidence/e3-report.json` contient les hashes des sources et des artefacts ; `evidence/e3-decode.log.txt` est la sortie du décodeur ; `evidence/e3-cumulative.srt.txt` montre les offsets du groupe. Les MP3 binaires restent dans le banc temporaire et ne sont pas copiés dans le dépôt. Le corpus D14 ne démontre donc pas un rejeu autonome des binaires MP3 si le temp disparaît ; scripts et hashes permettent de régénérer.

Ce smoke prouve la production et le décodage de trois fichiers séparés ainsi que le calcul simple d'offsets cumulés. Il ne teste pas crash, scan idempotent, marqueur Complete, récupération, silence interne, alignement corrigé, qualité audio, sortie réelle Whisper ou application. E3 reste PARTIAL.

## Suite exacte

E1 installateur local/version retenue et premier lancement offline ; E2 backend effectif GPU strict et Auto→CPU en cours de job ; E3 récupération/alignement/qualité ; E4 journal Rust, points de confirmation, charge et saturation ; E5 blocage worker/encodeur et arrêt ; E6 VAD Rust sur annotations humainement vérifiées, puis widget rendu et retard CPU visible. Les tests du futur produit restent NOT RUN. Le PC propre reste différé selon CHANGE-001. Aucune précision/rappel VAD ou seuil VAD n'est accepté.

## Contrôle indépendant des décodeurs

La revue D14 a confirmé la décodabilité avec FFmpeg 8.1 en mémoire. Pour les mêmes MP3, Symphonia rapporte +192 échantillons (12 ms) par passage et FFmpeg +768 échantillons (48 ms). La mesure dépend donc du décodeur et de son traitement du padding ; aucun alignement corrigé ni seuil de qualité/alignement accepté ne peut être déduit de ce smoke. Voir le paquet brut `44_REVIEW_RAW_D14.json`.
