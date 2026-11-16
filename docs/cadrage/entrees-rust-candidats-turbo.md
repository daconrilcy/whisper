# Entrées Rust des candidats Turbo

Avancée : parcours statique des dépendances `--extern` des deux compilations documentées.
Blocage : l'inclusion effective de chaque composant et les notices finales restent à établir ; les PE CPU/bootstrap finaux ne sont pas identifiés.
Décision : utiliser ces listes pour instruire les droits avant assemblage L3.

Depuis les commandes `rustc -vv` des builds frais identifiés dans [le relevé de compilation](dependances-compilees-turbo.md), on part de l'unité binaire et on suit récursivement les `--extern` vers des `.rlib` ou `.rmeta`, en associant chaque artefact à son unité par nom et suffixe ; les macros procédurales `.dll` sont exclues de ce parcours. Les licences proviennent des métadonnées Cargo gardées et sont déclaratives. Ce graphe identifie des entrées de compilation ou de liaison potentielles ; les optimisations et le linker peuvent éliminer du code, et les bibliothèques natives, liens dynamiques, métadonnées, polices et ressources requièrent un contrôle séparé. Il ne prouve pas l'inclusion effective de chaque paquet dans le fichier final.

Desktop SHA-256 `d052adba43459ce525d57c8da2d1eb45c4c189cf1871229e803a39baf66ce344`. GPU SHA-256 `757a7bd4c874fe076ed109dd176702a11ec3bd7ea0bc532be954a64ed92bcef8`. Ces identités sont celles des candidats observés, pas celles d'un paquet final figé.

## Composants et notices vérifiés séparément

Le 16 novembre 2026, une recherche en lecture seule des octets complets des polices du paquet `epaint_default_fonts 0.35.0` dans le desktop identifié ci-dessus a trouvé les quatre fichiers, sans transformation, aux positions suivantes en octets depuis le début du PE :

| Police incorporée | Taille | Position | Notices présentes dans le paquet source |
|---|---:|---:|---|
| `Hack-Regular.ttf` | 309 408 | 11 417 085 | `fonts/Hack-Regular.txt` : MIT, domaine public DejaVu et licence Bitstream Vera. |
| `NotoEmoji-Regular.ttf` | 418 804 | 11 726 510 | `fonts/OFL.txt` ; rattacher aussi le copyright propre à cette police. |
| `Ubuntu-Light.ttf` | 361 676 | 12 145 314 | `fonts/UFL.txt` ; rattacher aussi le copyright propre à cette police. |
| `emoji-icon-font.ttf` | 324 132 | 12 506 990 | `fonts/emoji-icon-font-mit-license.txt` : MIT et copyright John Slegers. |

La comparaison démontre la présence de ces octets dans ce seul candidat, pas la complétude des notices. Le code source du paquet utilise `include_bytes!` pour ces quatre polices. Son expression de licence associe `(MIT OR Apache-2.0)` à `OFL-1.1` et `Ubuntu-font-1.0` ; une option pour le code ne remplace pas les obligations des polices. Recueillir les avis de copyright exacts et fournir les textes applicables avec le produit. La [licence OFL officielle](https://openfontlicense.org/open-font-license-official-text/) impose notamment de conserver le copyright et sa licence lors de la redistribution.

Pour le GPU, `whisper-rs-sys 0.15.0` est dans le graphe observé ; son source embarque `whisper.cpp` dont `CMakeLists.txt` annonce la version **1.8.3** et dont `LICENSE` est MIT, copyright des auteurs ggml. La licence `Unlicense` déclarée pour la crate ne suffit donc pas à couvrir les sources natives embarquées. L'attribution complète des objets natifs incorporés et des notices GGML reste à vérifier à partir des options et du résultat de liaison ; la version affichée ne démontre pas à elle seule une révision amont précise.

Les six crates Symphonia 0.5.5 listées pour chacun des candidats déclarent MPL-2.0, mais leur présence effective dans les PE n'est pas démontrée par ce graphe. Si elles sont distribuées sous forme exécutable, examiner les obligations applicables, y compris l'accès au code source couvert, à partir du [texte MPL 2.0](https://www.mozilla.org/en-US/MPL/2.0/). Ne pas déduire qu'il faut publier la totalité du produit ni que les seules déclarations Cargo suffisent à satisfaire la licence.

Les sources examinées sont dans le cache persistant du worktree, sous `target/cargo-home-shared/registry/src/index.crates.io-1949cf8c6b5b557f/` ; elles ne constituent pas des notices remises. Les notices définitives doivent être collectées et vérifiées pour les versions effectivement retenues, y compris les copyrights et les clauses `NOTICE` pertinentes, puis intégrées au livrable de L3.

### Complément : nouveau desktop et code natif de détection vocale

Le desktop reconstruit le 16 novembre porte le SHA-256 `aca1b94b230b14d21ea73f89f10d464b241f5a5f9c28caed7849f9c701bba414`. La comparaison en lecture seule retrouve les quatre fichiers de police complets dans ce PE également, aux offsets respectifs 11 416 509 (`Hack-Regular.ttf`), 11 725 934 (`NotoEmoji-Regular.ttf`), 12 144 738 (`Ubuntu-Light.ttf`) et 12 506 414 (`emoji-icon-font.ttf`). Les notices et copyrights à compléter restent ceux indiqués ci-dessus. Une comparaison des journaux des deux builds frais trouve 198 unités `rustc` distinctes dans chaque cas et les mêmes tuples nom de crate, type et références `--extern` avec leurs noms d'artefacts. Cela permet de réutiliser le graphe des 178 unités en amont du desktop ci-dessous comme inventaire indicatif du nouveau build ; cela ne prouve pas que les mêmes objets ont été conservés par le linker ni que les notices sont complètes.

`webrtc-vad 0.4.0` déclare MIT et son build compile `libfvad` ; le journal de compilation du nouveau desktop montre une recherche de bibliothèque native dans sa sortie de build. Le paquet source contient aussi `resources/libfvad/LICENSE`, qui porte les copyrights des auteurs WebRTC et de Daniel Pirch et des conditions de redistribution binaire, ainsi que `resources/libfvad/PATENTS`. Les conserver parmi les éléments à examiner pour les notices du desktop, sans déduire du seul graphe Cargo la présence de chaque objet C dans le PE. La licence MIT de la crate ne remplace pas ces textes. [Source amont libfvad à la révision citée par la crate](https://github.com/dpirch/libfvad/blob/9713e439efe24d77575cb5fbc63d545acc4e5b21/LICENSE).

Les métadonnées intégrées aux fichiers de police donnent aussi `Copyright 2013 Google Inc.` pour Noto Emoji et `Copyright 2011 Canonical Ltd.` pour Ubuntu Light. Reprendre ces mentions avec les textes OFL et Ubuntu Font Licence applicables. La lecture du cache des 182 couples nom/version distincts du graphe desktop/GPU repère 22 paquets tiers sans fichier de licence à la racine de leur archive Cargo ; cela ne signifie pas que leur licence est absente, mais interdit de prétendre que toutes les notices sont déjà collectées. Ces paquets incluent notamment AccessKit, egui/epaint et Symphonia ; il faut retrouver les notices adaptées à leurs révisions ou dans leurs ressources internes. Les champs de licence du tableau ne remplacent pas les textes ni les mentions de copyright.

Les fichiers amont suivants ont été retrouvés aux révisions inscrites dans les métadonnées `.cargo_vcs_info.json` des archives concernées : [egui MIT](https://github.com/emilk/egui/blob/6f15dc0e16b26edce1fc2a05212eaf7e749c1d05/LICENSE-MIT) et [Apache](https://github.com/emilk/egui/blob/6f15dc0e16b26edce1fc2a05212eaf7e749c1d05/LICENSE-APACHE), [AccessKit MIT révision 1](https://github.com/AccessKit/accesskit/blob/a55d3e1a18bb9ef0e4bccc9083fb13c3e0ad8969/LICENSE-MIT) et [révision 2](https://github.com/AccessKit/accesskit/blob/1bbcf100942bac96c2c3a4a91cb67b0b20201a24/LICENSE-MIT), [Symphonia MPL](https://github.com/pdeljanov/Symphonia/blob/6d533f26150953a882a6a111ebd13f0abf7129d5/LICENSE), [clipboard-win](https://github.com/DoumanAsh/clipboard-win/blob/3b27cf2bfd1adcfa6e0264eb51c1025ddaf0f342/LICENSE), [dasp_sample MIT](https://github.com/RustAudio/dasp/blob/97c3bb9b2363c0b46ac1633858bf1054fd02a980/LICENSE-MIT) et [profiling MIT](https://github.com/aclysma/profiling/blob/8271551172eb6fa4cba47369aedd93790c623df9/LICENSE-MIT). Ils constituent des sources à examiner et à inclure selon les licences choisies, avec les éventuelles notices et droits d'auteur complémentaires ; ils ne sont pas encore collectés dans un paquet de notices. Le fichier de licence `realfft` recherché à sa révision n'a pas été trouvé par le chemin essayé ; poursuivre cette recherche.

Pour `realfft 3.5.0`, le [manifeste de la révision exacte](https://github.com/HEnquist/realfft/blob/d0d4eee0525fd27c96c8a046d6d107acd5ed84a6/Cargo.toml) et son [README](https://github.com/HEnquist/realfft/blob/d0d4eee0525fd27c96c8a046d6d107acd5ed84a6/README.md) déclarent MIT ; le manifeste nomme HEnquist comme auteur. L'archive publiée ne fournit pas de fichier contenant le texte complet ni de mention explicite de copyright dans les fichiers examinés. Le [fichier LICENSE de la branche amont actuelle](https://github.com/HEnquist/realfft/blob/master/LICENSE), trouvé ensuite, contient le texte MIT et attribue le copyright 2020 à Henrik Enquist et aux contributeurs RealFFT. Ce fichier n'existe pas au chemin essayé de la révision 3.5.0 : sa présence ultérieure corrobore la notice candidate mais ne prouve pas à elle seule quelle notice accompagnait la version distribuée. `realfft` figure dans les entrées du desktop, sans preuve autonome d'incorporation dans le PE.

### Desktop

Unités rustc parcourues : 178 ; références rlib/rmeta non résolues : 0.

| Paquet en amont du PE | Version | Licence déclarée |
|---|---|---|
| accesskit | 0.24.1 | MIT OR Apache-2.0 |
| accesskit_consumer | 0.35.0 | MIT OR Apache-2.0 |
| accesskit_windows | 0.32.1 | MIT OR Apache-2.0 |
| accesskit_winit | 0.32.2 | Apache-2.0 |
| adler2 | 2.0.1 | 0BSD OR MIT OR Apache-2.0 |
| ahash | 0.8.12 | MIT OR Apache-2.0 |
| arboard | 3.6.1 | MIT OR Apache-2.0 |
| arrayvec | 0.7.8 | MIT OR Apache-2.0 |
| bitflags | 1.3.2 | MIT/Apache-2.0 |
| bitflags | 2.13.2 | MIT OR Apache-2.0 |
| block-buffer | 0.10.4 | MIT OR Apache-2.0 |
| bytemuck | 1.25.2 | Zlib OR Apache-2.0 OR MIT |
| byteorder-lite | 0.1.0 | Unlicense OR MIT |
| cfg-if | 1.0.5 | MIT OR Apache-2.0 |
| chrono | 0.4.45 | MIT OR Apache-2.0 |
| clipboard-win | 5.4.1 | BSL-1.0 |
| color | 0.3.3 | Apache-2.0 OR MIT |
| core_detect | 1.0.0 | MIT/Apache-2.0 |
| cpal | 0.18.2 | Apache-2.0 |
| cpufeatures | 0.2.17 | MIT OR Apache-2.0 |
| crc32fast | 1.5.2 | MIT OR Apache-2.0 |
| crypto-common | 0.1.7 | MIT OR Apache-2.0 |
| cursor-icon | 1.2.0 | MIT OR Apache-2.0 OR Zlib |
| dasp_sample | 0.11.0 | MIT OR Apache-2.0 |
| digest | 0.10.7 | MIT OR Apache-2.0 |
| dpi | 0.1.2 | Apache-2.0 AND MIT |
| ecolor | 0.35.0 | MIT OR Apache-2.0 |
| eframe | 0.35.0 | MIT OR Apache-2.0 |
| egui | 0.35.0 | MIT OR Apache-2.0 |
| egui-winit | 0.35.0 | MIT OR Apache-2.0 |
| egui_glow | 0.35.0 | MIT OR Apache-2.0 |
| either | 1.18.0 | MIT OR Apache-2.0 |
| emath | 0.35.0 | MIT OR Apache-2.0 |
| encoding_rs | 0.8.42 | (Apache-2.0 OR MIT) AND BSD-3-Clause |
| epaint | 0.35.0 | MIT OR Apache-2.0 |
| epaint_default_fonts | 0.35.0 | (MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0 |
| error-code | 3.4.0 | BSL-1.0 |
| euclid | 0.22.14 | MIT OR Apache-2.0 |
| extended | 0.1.0 | MIT |
| fdeflate | 0.3.7 | MIT OR Apache-2.0 |
| fearless_simd | 0.4.1 | Apache-2.0 OR MIT |
| flate2 | 1.1.10 | MIT OR Apache-2.0 |
| foldhash | 0.2.0 | Zlib |
| font-types | 0.11.3 | MIT OR Apache-2.0 |
| form_urlencoded | 1.2.2 | MIT OR Apache-2.0 |
| futures | 0.3.33 | MIT OR Apache-2.0 |
| futures-channel | 0.3.33 | MIT OR Apache-2.0 |
| futures-core | 0.3.33 | MIT OR Apache-2.0 |
| futures-executor | 0.3.33 | MIT OR Apache-2.0 |
| futures-io | 0.3.34 | MIT OR Apache-2.0 |
| futures-sink | 0.3.34 | MIT OR Apache-2.0 |
| futures-task | 0.3.33 | MIT OR Apache-2.0 |
| futures-util | 0.3.33 | MIT OR Apache-2.0 |
| generic-array | 0.14.7 | MIT |
| glow | 0.17.0 | MIT OR Apache-2.0 OR Zlib |
| glutin | 0.32.3 | Apache-2.0 |
| glutin-winit | 0.5.0 | MIT |
| glutin_egl_sys | 0.7.1 | Apache-2.0 |
| glutin_wgl_sys | 0.6.1 | Apache-2.0 |
| guillotiere | 0.7.0 | MIT/Apache-2.0 |
| harfrust | 0.7.0 | MIT |
| hashbrown | 0.16.1 | MIT OR Apache-2.0 |
| hashbrown | 0.17.1 | MIT OR Apache-2.0 |
| icu_collections | 2.3.0 | Unicode-3.0 |
| icu_locale_core | 2.3.0 | Unicode-3.0 |
| icu_normalizer | 2.3.0 | Unicode-3.0 |
| icu_normalizer_data | 2.3.0 | Unicode-3.0 |
| icu_properties | 2.3.0 | Unicode-3.0 |
| icu_properties_data | 2.3.0 | Unicode-3.0 |
| icu_provider | 2.3.1 | Unicode-3.0 |
| idna | 1.1.0 | MIT OR Apache-2.0 |
| idna_adapter | 1.2.2 | Apache-2.0 OR MIT |
| image | 0.25.10 | MIT OR Apache-2.0 |
| itertools | 0.14.0 | MIT OR Apache-2.0 |
| itoa | 1.0.18 | MIT OR Apache-2.0 |
| kurbo | 0.13.1 | Apache-2.0 OR MIT |
| lazy_static | 1.5.1 | MIT OR Apache-2.0 |
| libloading | 0.8.9 | ISC |
| linebender_resource_handle | 0.1.1 | Apache-2.0 OR MIT |
| litemap | 0.8.3 | Unicode-3.0 |
| lock_api | 0.4.14 | MIT OR Apache-2.0 |
| log | 0.4.34 | MIT OR Apache-2.0 |
| memchr | 2.8.3 | Unlicense OR MIT |
| memoffset | 0.9.1 | MIT |
| miniz_oxide | 0.8.9 | MIT OR Zlib OR Apache-2.0 |
| miniz_oxide | 0.9.1 | MIT OR Zlib OR Apache-2.0 |
| moxcms | 0.8.1 | BSD-3-Clause OR Apache-2.0 |
| nohash-hasher | 0.2.0 | Apache-2.0 OR MIT |
| num-complex | 0.4.6 | MIT OR Apache-2.0 |
| num-integer | 0.1.47 | MIT OR Apache-2.0 |
| num-traits | 0.2.19 | MIT OR Apache-2.0 |
| once_cell | 1.21.4 | MIT OR Apache-2.0 |
| parking_lot | 0.12.5 | MIT OR Apache-2.0 |
| parking_lot_core | 0.9.12 | MIT OR Apache-2.0 |
| peniko | 0.6.1 | Apache-2.0 OR MIT |
| percent-encoding | 2.3.2 | MIT OR Apache-2.0 |
| pin-project-lite | 0.2.17 | Apache-2.0 OR MIT |
| png | 0.18.1 | MIT OR Apache-2.0 |
| polycool | 0.4.0 | MIT OR Apache-2.0 |
| potential_utf | 0.1.6 | Unicode-3.0 |
| primal-check | 0.3.4 | MIT OR Apache-2.0 |
| profiling | 1.0.18 | MIT OR Apache-2.0 |
| pxfm | 0.1.30 | BSD-3-Clause OR Apache-2.0 |
| raw-window-handle | 0.6.2 | MIT OR Apache-2.0 OR Zlib |
| read-fonts | 0.39.2 | MIT OR Apache-2.0 |
| realfft | 3.5.0 | MIT |
| rfd | 0.17.2 | MIT |
| rtrb | 0.4.0 | MIT OR Apache-2.0 |
| rubato | 0.16.2 | MIT |
| rustfft | 6.4.1 | MIT OR Apache-2.0 |
| ryu | 1.0.23 | Apache-2.0 OR BSL-1.0 |
| same-file | 1.0.6 | Unlicense/MIT |
| scopeguard | 1.2.0 | MIT OR Apache-2.0 |
| self_cell | 1.3.0 | Apache-2.0 OR GPL-2.0-only |
| serde | 1.0.228 | MIT OR Apache-2.0 |
| serde_core | 1.0.228 | MIT OR Apache-2.0 |
| serde_json | 1.0.145 | MIT OR Apache-2.0 |
| sha2 | 0.10.9 | MIT OR Apache-2.0 |
| simd-adler32 | 0.3.10 | MIT |
| simdutf8 | 0.1.5 | MIT OR Apache-2.0 |
| skrifa | 0.42.1 | MIT OR Apache-2.0 |
| slab | 0.4.12 | MIT |
| smallvec | 1.16.2 | MIT OR Apache-2.0 |
| smol_str | 0.2.2 | MIT OR Apache-2.0 |
| stable_deref_trait | 1.2.1 | MIT OR Apache-2.0 |
| static_assertions | 1.1.0 | MIT OR Apache-2.0 |
| strength_reduce | 0.2.4 | MIT OR Apache-2.0 |
| symphonia | 0.5.5 | MPL-2.0 |
| symphonia-bundle-mp3 | 0.5.5 | MPL-2.0 |
| symphonia-codec-pcm | 0.5.5 | MPL-2.0 |
| symphonia-core | 0.5.5 | MPL-2.0 |
| symphonia-format-riff | 0.5.5 | MPL-2.0 |
| symphonia-metadata | 0.5.5 | MPL-2.0 |
| thiserror | 2.0.21 | MIT OR Apache-2.0 |
| tinystr | 0.8.4 | Unicode-3.0 |
| tracing | 0.1.44 | MIT |
| tracing-core | 0.1.36 | MIT |
| transpose | 0.2.3 | MIT OR Apache-2.0 |
| typenum | 1.20.1 | MIT OR Apache-2.0 |
| unicode-general-category | 1.1.0 | Apache-2.0 |
| unicode-segmentation | 1.13.3 | MIT OR Apache-2.0 |
| url | 2.5.8 | MIT OR Apache-2.0 |
| utf8_iter | 1.0.4 | Apache-2.0 OR MIT |
| uuid | 1.27.0 | Apache-2.0 OR MIT |
| vello_common | 0.0.9 | Apache-2.0 OR MIT |
| vello_cpu | 0.0.9 | Apache-2.0 OR MIT |
| web-time | 1.1.0 | MIT OR Apache-2.0 |
| webbrowser | 1.2.4 | MIT OR Apache-2.0 |
| webrtc-vad | 0.4.0 | MIT |
| whisper-adapters | 0.1.0 | non déclarée |
| whisper-core | 0.1.0 | non déclarée |
| whisper-desktop | 0.1.0 | non déclarée |
| winapi-util | 0.1.11 | Unlicense OR MIT |
| windows | 0.62.2 | MIT OR Apache-2.0 |
| windows-collections | 0.3.2 | MIT OR Apache-2.0 |
| windows-core | 0.62.2 | MIT OR Apache-2.0 |
| windows-future | 0.3.2 | MIT OR Apache-2.0 |
| windows-link | 0.2.1 | MIT OR Apache-2.0 |
| windows-numerics | 0.3.1 | MIT OR Apache-2.0 |
| windows-result | 0.4.1 | MIT OR Apache-2.0 |
| windows-strings | 0.5.1 | MIT OR Apache-2.0 |
| windows-sys | 0.52.0 | MIT OR Apache-2.0 |
| windows-sys | 0.60.2 | MIT OR Apache-2.0 |
| windows-sys | 0.61.2 | MIT OR Apache-2.0 |
| windows-targets | 0.52.6 | MIT OR Apache-2.0 |
| windows-targets | 0.53.5 | MIT OR Apache-2.0 |
| windows-threading | 0.2.1 | MIT OR Apache-2.0 |
| windows_x86_64_msvc | 0.52.6 | MIT OR Apache-2.0 |
| windows_x86_64_msvc | 0.53.1 | MIT OR Apache-2.0 |
| winit | 0.30.13 | Apache-2.0 |
| wmi | 0.18.4 | MIT OR Apache-2.0 |
| writeable | 0.6.4 | Unicode-3.0 |
| yoke | 0.8.3 | Unicode-3.0 |
| zerocopy | 0.8.59 | BSD-2-Clause OR Apache-2.0 OR MIT |
| zerofrom | 0.1.8 | Unicode-3.0 |
| zerotrie | 0.2.5 | Unicode-3.0 |
| zerovec | 0.11.8 | Unicode-3.0 |

### GPU

Unités rustc parcourues : 45 ; références rlib/rmeta non résolues : 0.

| Paquet en amont du PE | Version | Licence déclarée |
|---|---|---|
| arrayvec | 0.7.8 | MIT OR Apache-2.0 |
| bitflags | 1.3.2 | MIT/Apache-2.0 |
| block-buffer | 0.10.4 | MIT OR Apache-2.0 |
| bytemuck | 1.25.2 | Zlib OR Apache-2.0 OR MIT |
| cfg-if | 1.0.5 | MIT OR Apache-2.0 |
| core_detect | 1.0.0 | MIT/Apache-2.0 |
| cpufeatures | 0.2.17 | MIT OR Apache-2.0 |
| crypto-common | 0.1.7 | MIT OR Apache-2.0 |
| digest | 0.10.7 | MIT OR Apache-2.0 |
| encoding_rs | 0.8.42 | (Apache-2.0 OR MIT) AND BSD-3-Clause |
| extended | 0.1.0 | MIT |
| generic-array | 0.14.7 | MIT |
| itoa | 1.0.18 | MIT OR Apache-2.0 |
| lazy_static | 1.5.1 | MIT OR Apache-2.0 |
| log | 0.4.34 | MIT OR Apache-2.0 |
| memchr | 2.8.3 | Unlicense OR MIT |
| rusty_mp3 | 0.8.0 | Apache-2.0 |
| ryu | 1.0.23 | Apache-2.0 OR BSL-1.0 |
| scopeguard | 1.2.0 | MIT OR Apache-2.0 |
| serde | 1.0.228 | MIT OR Apache-2.0 |
| serde_core | 1.0.228 | MIT OR Apache-2.0 |
| serde_json | 1.0.145 | MIT OR Apache-2.0 |
| sha2 | 0.10.9 | MIT OR Apache-2.0 |
| simdutf8 | 0.1.5 | MIT OR Apache-2.0 |
| symphonia | 0.5.5 | MPL-2.0 |
| symphonia-bundle-mp3 | 0.5.5 | MPL-2.0 |
| symphonia-codec-pcm | 0.5.5 | MPL-2.0 |
| symphonia-core | 0.5.5 | MPL-2.0 |
| symphonia-format-riff | 0.5.5 | MPL-2.0 |
| symphonia-metadata | 0.5.5 | MPL-2.0 |
| typenum | 1.20.1 | MIT OR Apache-2.0 |
| whisper-core | 0.1.0 | non déclarée |
| whisper-rs | 0.16.0 | Unlicense |
| whisper-rs-sys | 0.15.0 | Unlicense |
| whisper-worker-common | 0.1.0 | non déclarée |
| whisper-worker-gpu | 0.1.0 | non déclarée |
| windows | 0.62.2 | MIT OR Apache-2.0 |
| windows-collections | 0.3.2 | MIT OR Apache-2.0 |
| windows-core | 0.62.2 | MIT OR Apache-2.0 |
| windows-future | 0.3.2 | MIT OR Apache-2.0 |
| windows-link | 0.2.1 | MIT OR Apache-2.0 |
| windows-numerics | 0.3.1 | MIT OR Apache-2.0 |
| windows-result | 0.4.1 | MIT OR Apache-2.0 |
| windows-strings | 0.5.1 | MIT OR Apache-2.0 |
| windows-threading | 0.2.1 | MIT OR Apache-2.0 |

### Worker CPU L1 : ensemble conservateur issu du cache

Le binaire CPU retenu est `03566517fa8580414770f346cc0a6be9846da45d8c928ead6a32cc10635f782a`. Dans sa cible L1, le graphe des fichiers Cargo `.fingerprint/*.json` part de `whisper-worker-cpu-467a86811100b567/bin-whisper-worker-cpu.json` et suit les noms de dépendances en retenant **tous** les dossiers de même nom ; les deux noms de scripts de build non résolus sont ceux des scripts internes des paquets traversés. Il en résulte 67 noms, incluant macros, scripts et outils de compilation comme bindgen et CMake, qui ne sont pas réputés distribués. Les versions ci-dessous proviennent des métadonnées de la résolution Cargo sous le même `Cargo.lock` (empreinte de ce fichier identique dans les deux worktrees) : lorsque plusieurs versions existent, elles sont toutes listées, sans attribution certaine à ce PE.

Cet ensemble facilite un traitement prudent des licences ; il ne démontre ni l'inclusion de chaque composant, ni l'exhaustivité des sources natives, ni l'absence de chargement dynamique. Les sources et notices effectives du CPU restent à rattacher au PE et à revoir. Les informations sous `target` restent temporaires jusqu'à archivage prévu du lot.

| Paquet candidat | Versions possibles | Licence déclarée |
|---|---|---|
| arrayvec | 0.7.8 | MIT OR Apache-2.0 |
| bindgen | 0.72.1 | BSD-3-Clause |
| bitflags | 1.3.2; 2.13.2 | MIT OR Apache-2.0; MIT/Apache-2.0 |
| block-buffer | 0.10.4 | MIT OR Apache-2.0 |
| bytemuck | 1.25.2 | Zlib OR Apache-2.0 OR MIT |
| bytemuck_derive | 1.12.1 | Zlib OR Apache-2.0 OR MIT |
| cc | 1.6.0 | MIT OR Apache-2.0 |
| cexpr | 0.6.0 | Apache-2.0/MIT |
| cfg-if | 1.0.5 | MIT OR Apache-2.0 |
| clang-sys | 1.9.1 | Apache-2.0 |
| cmake | 0.1.58 | MIT OR Apache-2.0 |
| core_detect | 1.0.0 | MIT/Apache-2.0 |
| cpufeatures | 0.2.17 | MIT OR Apache-2.0 |
| crypto-common | 0.1.7 | MIT OR Apache-2.0 |
| digest | 0.10.7 | MIT OR Apache-2.0 |
| either | 1.18.0 | MIT OR Apache-2.0 |
| encoding_rs | 0.8.42 | (Apache-2.0 OR MIT) AND BSD-3-Clause |
| extended | 0.1.0 | MIT |
| find-msvc-tools | 0.1.14 | MIT OR Apache-2.0 |
| fs_extra | 1.3.0 | MIT |
| generic-array | 0.14.7 | MIT |
| glob | 0.3.4 | MIT OR Apache-2.0 |
| itertools | 0.13.0; 0.14.0 | MIT OR Apache-2.0 |
| itoa | 1.0.18 | MIT OR Apache-2.0 |
| lazy_static | 1.5.1 | MIT OR Apache-2.0 |
| libc | 0.2.190 | MIT OR Apache-2.0 |
| libloading | 0.8.9 | ISC |
| log | 0.4.34 | MIT OR Apache-2.0 |
| memchr | 2.8.3 | Unlicense OR MIT |
| minimal-lexical | 0.2.1 | MIT/Apache-2.0 |
| multiversion_no_op | 1.0.0 | Apache-2.0 OR MIT |
| nom | 7.1.3 | MIT |
| prettyplease | 0.2.37 | MIT OR Apache-2.0 |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 |
| quote | 1.0.47 | MIT OR Apache-2.0 |
| regex | 1.13.1 | MIT OR Apache-2.0 |
| regex-automata | 0.4.18 | MIT OR Apache-2.0 |
| regex-syntax | 0.8.11 | MIT OR Apache-2.0 |
| rustc-hash | 2.1.3 | Apache-2.0 OR MIT |
| rustversion | 1.0.23 | MIT OR Apache-2.0 |
| rusty_mp3 | 0.8.0 | Apache-2.0 |
| ryu | 1.0.23 | Apache-2.0 OR BSL-1.0 |
| scopeguard | 1.2.0 | MIT OR Apache-2.0 |
| semver | 1.0.28 | MIT OR Apache-2.0 |
| serde | 1.0.228 | MIT OR Apache-2.0 |
| serde_core | 1.0.228 | MIT OR Apache-2.0 |
| serde_derive | 1.0.228 | MIT OR Apache-2.0 |
| serde_json | 1.0.145 | MIT OR Apache-2.0 |
| sha2 | 0.10.9 | MIT OR Apache-2.0 |
| shlex | 1.3.0; 2.0.1 | MIT OR Apache-2.0 |
| simdutf8 | 0.1.5 | MIT OR Apache-2.0 |
| symphonia | 0.5.5 | MPL-2.0 |
| symphonia-bundle-mp3 | 0.5.5 | MPL-2.0 |
| symphonia-codec-pcm | 0.5.5 | MPL-2.0 |
| symphonia-core | 0.5.5 | MPL-2.0 |
| symphonia-format-riff | 0.5.5 | MPL-2.0 |
| symphonia-metadata | 0.5.5 | MPL-2.0 |
| syn | 2.0.119; 3.0.6 | MIT OR Apache-2.0 |
| typenum | 1.20.1 | MIT OR Apache-2.0 |
| unicode-ident | 1.0.26 | (MIT OR Apache-2.0) AND Unicode-3.0 |
| version_check | 0.9.5 | MIT/Apache-2.0 |
| whisper-core | 0.1.0 | non declaree |
| whisper-rs | 0.16.0 | Unlicense |
| whisper-rs-sys | 0.15.0 | Unlicense |
| whisper-worker-common | 0.1.0 | non declaree |
| whisper-worker-cpu | 0.1.0 | non declaree |
| windows-link | 0.2.1 | MIT OR Apache-2.0 |
