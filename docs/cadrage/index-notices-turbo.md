# Index des notices candidates Turbo

Avancée : attribution prudente des sources de licence aux entrées de build des trois exécutables L2.
Blocage : textes, copyrights et composants effectivement distribués restent à vérifier ; ce document n'est pas un jeu de notices à livrer.
Décision : conserver un ensemble conservateur pour préparer les notices de l'assemblage L3.

État au 16 novembre 2026. D = entrée du graphe du desktop `aca1b94b230b14d21ea73f89f10d464b241f5a5f9c28caed7849f9c701bba414` ; G = entrée du graphe du worker GPU `757a7bd4c874fe076ed109dd176702a11ec3bd7ea0bc532be954a64ed92bcef8` ; C? = nom rencontré dans le parcours conservateur du cache du worker CPU `03566517fa8580414770f346cc0a6be9846da45d8c928ead6a32cc10635f782a`. Les versions multiples CPU, dépendances de compilation et macros peuvent faire partie de C? sans entrer dans le binaire. Les graphes D et G identifient des entrées potentielles, pas la présence de chaque paquet après liaison. Voir les [méthodes et réserves](entrees-rust-candidats-turbo.md).

Le tableau présente les fichiers de licence repérés **à la racine** de chaque archive Cargo ; sauf indication contraire, ils sont relatifs à `target/cargo-home-shared/registry/src/index.crates.io-1949cf8c6b5b557f/<paquet>-<version>/` dans le worktree de reprise. `license_file` est également pris en compte. La présence d'un fichier n'établit ni le choix d'une licence parmi les alternatives, ni l'absence de notices, copyrights ou composants natifs supplémentaires. Pour les fichiers non trouvés à la racine, un lien vers la révision amont est fourni quand elle est connue. Il faut identifier les textes applicables, les lire et les joindre au produit ; un lien ou cet index ne les remplace pas.

Les lignes de copyright identifiées dans les licences racine, ainsi que des mentions pour certaines ressources natives et polices, figurent dans le [relevé complémentaire](copyrights-notices-turbo.md). Elles restent à vérifier et à réunir avec les textes complets pour le paquet.

Un [recueil de textes de licences candidats](recueil-licences-candidates-turbo.md) rassemble les fichiers disponibles, dont certains textes des polices et composants natifs ; il reste incomplet et inclut des alternatives ou outils pouvant ne pas être livrés.

Pour Symphonia, la [notice MPL 2.0](licence-mpl-symphonia.md) et l'[archive des sources 0.5.5](sources-symphonia-0.5.5.zip) sont préparées ; l'archive inclut la notice. Pour le runtime Rust, [notices-rust-1.98.1.zip](notices-rust-1.98.1.zip) contient l'intégralité du fichier `COPYRIGHT-library.html` de la toolchain utilisée, avec identité de contenu vérifiée. Ces archives sont des entrées de travail suivies dans le dépôt, à remettre ou rendre disponibles selon les conditions applicables et le paquet finalement retenu.

Attention aux licences au niveau des fichiers : des sources des crates AccessKit retenues portent aussi des mentions Chromium et renvoient à `LICENSE.chromium`, non présent à la racine des archives Cargo examinées. Les textes correspondants sont disponibles dans les dépôts amont aux révisions [a55d3e1a](https://github.com/AccessKit/accesskit/blob/a55d3e1a18bb9ef0e4bccc9083fb13c3e0ad8969/LICENSE.chromium) et [1bbcf100](https://github.com/AccessKit/accesskit/blob/1bbcf100942bac96c2c3a4a91cb67b0b20201a24/LICENSE.chromium) ; les notices et les droits de ces sources doivent être traités en plus des déclarations MIT/Apache. `accesskit/src/geometry.rs` signale également des parties issues de kurbo. Ces crates sont des entrées potentielles du desktop ; leur inclusion effective n'est pas ici démontrée.

| Paquet | Version | PE candidats | Licence déclarée | Textes candidats à examiner |
|---|---|---|---|---|
| accesskit | 0.24.1 | D | MIT OR Apache-2.0 | [dépôt à la révision a55d3e1a](https://github.com/AccessKit/accesskit/tree/a55d3e1a18bb9ef0e4bccc9083fb13c3e0ad8969) : texte racine à vérifier |
| accesskit_consumer | 0.35.0 | D | MIT OR Apache-2.0 | [dépôt à la révision 1bbcf100](https://github.com/AccessKit/accesskit/tree/1bbcf100942bac96c2c3a4a91cb67b0b20201a24) : texte racine à vérifier |
| accesskit_windows | 0.32.1 | D | MIT OR Apache-2.0 | [dépôt à la révision 1bbcf100](https://github.com/AccessKit/accesskit/tree/1bbcf100942bac96c2c3a4a91cb67b0b20201a24) : texte racine à vérifier |
| accesskit_winit | 0.32.2 | D | Apache-2.0 | [dépôt à la révision 1bbcf100](https://github.com/AccessKit/accesskit/tree/1bbcf100942bac96c2c3a4a91cb67b0b20201a24) : texte racine à vérifier |
| adler2 | 2.0.1 | D | 0BSD OR MIT OR Apache-2.0 | `LICENSE-0BSD`, `LICENSE-APACHE`, `LICENSE-MIT` |
| ahash | 0.8.12 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| arboard | 3.6.1 | D | MIT OR Apache-2.0 | `LICENSE-APACHE.txt`, `LICENSE-MIT.txt` |
| arrayvec | 0.7.8 | D, G, C? | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| bindgen | 0.72.1 | C? | BSD-3-Clause | `LICENSE` |
| bitflags | 1.3.2 | D, G, C? | MIT/Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| bitflags | 2.13.2 | D, C? | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| block-buffer | 0.10.4 | D, G, C? | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| bytemuck | 1.25.2 | D, G, C? | Zlib OR Apache-2.0 OR MIT | `LICENSE-APACHE`, `LICENSE-MIT`, `LICENSE-ZLIB` |
| bytemuck_derive | 1.12.1 | C? | Zlib OR Apache-2.0 OR MIT | `LICENSE-APACHE`, `LICENSE-MIT`, `LICENSE-ZLIB` |
| byteorder-lite | 0.1.0 | D | Unlicense OR MIT | `LICENSE-MIT`, `UNLICENSE` |
| cc | 1.6.0 | C? | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| cexpr | 0.6.0 | C? | Apache-2.0/MIT | `LICENSE-APACHE`, `LICENSE-MIT` |
| cfg-if | 1.0.5 | D, G, C? | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| chrono | 0.4.45 | D | MIT OR Apache-2.0 | `LICENSE.txt` |
| clang-sys | 1.9.1 | C? | Apache-2.0 | `LICENSE.txt` |
| clipboard-win | 5.4.1 | D | BSL-1.0 | [dépôt à la révision 3b27cf2b](https://github.com/DoumanAsh/clipboard-win/tree/3b27cf2bfd1adcfa6e0264eb51c1025ddaf0f342) : texte racine à vérifier |
| cmake | 0.1.58 | C? | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| color | 0.3.3 | D | Apache-2.0 OR MIT | `LICENSE-APACHE`, `LICENSE-MIT` |
| core_detect | 1.0.0 | D, G, C? | MIT/Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| cpal | 0.18.2 | D | Apache-2.0 | `LICENSE` |
| cpufeatures | 0.2.17 | D, G, C? | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| crc32fast | 1.5.2 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| crypto-common | 0.1.7 | D, G, C? | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| cursor-icon | 1.2.0 | D | MIT OR Apache-2.0 OR Zlib | `LICENSE-APACHE`, `LICENSE-MIT`, `LICENSE-ZLIB` |
| dasp_sample | 0.11.0 | D | MIT OR Apache-2.0 | [dépôt à la révision 97c3bb9b](https://github.com/rustaudio/sample/tree/97c3bb9b2363c0b46ac1633858bf1054fd02a980) : texte racine à vérifier |
| digest | 0.10.7 | D, G, C? | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| dpi | 0.1.2 | D | Apache-2.0 AND MIT | `LICENSE`, `LICENSE-LIBM-MIT` |
| ecolor | 0.35.0 | D | MIT OR Apache-2.0 | [dépôt à la révision 6f15dc0e](https://github.com/emilk/egui/tree/6f15dc0e16b26edce1fc2a05212eaf7e749c1d05) : texte racine à vérifier |
| eframe | 0.35.0 | D | MIT OR Apache-2.0 | [dépôt à la révision 6f15dc0e](https://github.com/emilk/egui/tree/6f15dc0e16b26edce1fc2a05212eaf7e749c1d05) : texte racine à vérifier |
| egui | 0.35.0 | D | MIT OR Apache-2.0 | [dépôt à la révision 6f15dc0e](https://github.com/emilk/egui/tree/6f15dc0e16b26edce1fc2a05212eaf7e749c1d05) : texte racine à vérifier |
| egui-winit | 0.35.0 | D | MIT OR Apache-2.0 | [dépôt à la révision 6f15dc0e](https://github.com/emilk/egui/tree/6f15dc0e16b26edce1fc2a05212eaf7e749c1d05) : texte racine à vérifier |
| egui_glow | 0.35.0 | D | MIT OR Apache-2.0 | [dépôt à la révision 6f15dc0e](https://github.com/emilk/egui/tree/6f15dc0e16b26edce1fc2a05212eaf7e749c1d05) : texte racine à vérifier |
| either | 1.18.0 | D, C? | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| emath | 0.35.0 | D | MIT OR Apache-2.0 | [dépôt à la révision 6f15dc0e](https://github.com/emilk/egui/tree/6f15dc0e16b26edce1fc2a05212eaf7e749c1d05) : texte racine à vérifier |
| encoding_rs | 0.8.42 | D, G, C? | (Apache-2.0 OR MIT) AND BSD-3-Clause | `COPYRIGHT`, `LICENSE-APACHE`, `LICENSE-MIT`, `LICENSE-WHATWG` |
| epaint | 0.35.0 | D | MIT OR Apache-2.0 | [dépôt à la révision 6f15dc0e](https://github.com/emilk/egui/tree/6f15dc0e16b26edce1fc2a05212eaf7e749c1d05) : texte racine à vérifier |
| epaint_default_fonts | 0.35.0 | D | (MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0 | [dépôt à la révision 6f15dc0e](https://github.com/emilk/egui/tree/6f15dc0e16b26edce1fc2a05212eaf7e749c1d05) : texte racine à vérifier ; `fonts/OFL.txt`, `fonts/UFL.txt`, `fonts/Hack-Regular.txt`, `fonts/emoji-icon-font-mit-license.txt` |
| error-code | 3.4.0 | D | BSL-1.0 | `LICENSE` |
| euclid | 0.22.14 | D | MIT OR Apache-2.0 | `COPYRIGHT`, `LICENSE-APACHE`, `LICENSE-MIT` |
| extended | 0.1.0 | D, G, C? | MIT | `LICENSE.txt` |
| fdeflate | 0.3.7 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| fearless_simd | 0.4.1 | D | Apache-2.0 OR MIT | `LICENSE-APACHE`, `LICENSE-MIT` |
| find-msvc-tools | 0.1.14 | C? | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| flate2 | 1.1.10 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| foldhash | 0.2.0 | D | Zlib | `LICENSE` |
| font-types | 0.11.3 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| form_urlencoded | 1.2.2 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| fs_extra | 1.3.0 | C? | MIT | `LICENSE` |
| futures | 0.3.33 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| futures-channel | 0.3.33 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| futures-core | 0.3.33 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| futures-executor | 0.3.33 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| futures-io | 0.3.34 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| futures-sink | 0.3.34 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| futures-task | 0.3.33 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| futures-util | 0.3.33 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| generic-array | 0.14.7 | D, G, C? | MIT | `LICENSE` |
| glob | 0.3.4 | C? | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| glow | 0.17.0 | D | MIT OR Apache-2.0 OR Zlib | `LICENSE-APACHE`, `LICENSE-MIT`, `LICENSE-ZLIB` |
| glutin | 0.32.3 | D | Apache-2.0 | `LICENSE` |
| glutin-winit | 0.5.0 | D | MIT | `LICENSE` |
| glutin_egl_sys | 0.7.1 | D | Apache-2.0 | `LICENSE` |
| glutin_wgl_sys | 0.6.1 | D | Apache-2.0 | `LICENSE` |
| guillotiere | 0.7.0 | D | MIT/Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| harfrust | 0.7.0 | D | MIT | `LICENSE` |
| hashbrown | 0.16.1 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| hashbrown | 0.17.1 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| icu_collections | 2.3.0 | D | Unicode-3.0 | `LICENSE` |
| icu_locale_core | 2.3.0 | D | Unicode-3.0 | `LICENSE` |
| icu_normalizer | 2.3.0 | D | Unicode-3.0 | `LICENSE` |
| icu_normalizer_data | 2.3.0 | D | Unicode-3.0 | `LICENSE` |
| icu_properties | 2.3.0 | D | Unicode-3.0 | `LICENSE` |
| icu_properties_data | 2.3.0 | D | Unicode-3.0 | `LICENSE` |
| icu_provider | 2.3.1 | D | Unicode-3.0 | `LICENSE` |
| idna | 1.1.0 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| idna_adapter | 1.2.2 | D | Apache-2.0 OR MIT | `LICENSE-APACHE`, `LICENSE-MIT` |
| image | 0.25.10 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| itertools | 0.13.0 | C? | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| itertools | 0.14.0 | D, C? | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| itoa | 1.0.18 | D, G, C? | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| kurbo | 0.13.1 | D | Apache-2.0 OR MIT | `LICENSE-APACHE`, `LICENSE-MIT` |
| lazy_static | 1.5.1 | D, G, C? | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| libc | 0.2.190 | C? | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| libloading | 0.8.9 | D, C? | ISC | `LICENSE` |
| linebender_resource_handle | 0.1.1 | D | Apache-2.0 OR MIT | `LICENSE-APACHE`, `LICENSE-MIT` |
| litemap | 0.8.3 | D | Unicode-3.0 | `LICENSE` |
| lock_api | 0.4.14 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| log | 0.4.34 | D, G, C? | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| memchr | 2.8.3 | D, G, C? | Unlicense OR MIT | `COPYING`, `LICENSE-MIT`, `UNLICENSE` |
| memoffset | 0.9.1 | D | MIT | `LICENSE` |
| minimal-lexical | 0.2.1 | C? | MIT/Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT`, `LICENSE.md` |
| miniz_oxide | 0.8.9 | D | MIT OR Zlib OR Apache-2.0 | `LICENSE`, `LICENSE-APACHE.md`, `LICENSE-MIT.md`, `LICENSE-ZLIB.md` |
| miniz_oxide | 0.9.1 | D | MIT OR Zlib OR Apache-2.0 | `LICENSE`, `LICENSE-APACHE.md`, `LICENSE-MIT.md`, `LICENSE-ZLIB.md` |
| moxcms | 0.8.1 | D | BSD-3-Clause OR Apache-2.0 | `LICENSE-APACHE.md`, `LICENSE.md` |
| multiversion_no_op | 1.0.0 | C? | Apache-2.0 OR MIT | `COPYRIGHT`, `LICENSE-APACHE`, `LICENSE-MIT` |
| nohash-hasher | 0.2.0 | D | Apache-2.0 OR MIT | `LICENSE-APACHE`, `LICENSE-MIT` |
| nom | 7.1.3 | C? | MIT | `LICENSE` |
| num-complex | 0.4.6 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| num-integer | 0.1.47 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| num-traits | 0.2.19 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| once_cell | 1.21.4 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| parking_lot | 0.12.5 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| parking_lot_core | 0.9.12 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| peniko | 0.6.1 | D | Apache-2.0 OR MIT | `LICENSE-APACHE`, `LICENSE-MIT` |
| percent-encoding | 2.3.2 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| pin-project-lite | 0.2.17 | D | Apache-2.0 OR MIT | `LICENSE-APACHE`, `LICENSE-MIT` |
| png | 0.18.1 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| polycool | 0.4.0 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| potential_utf | 0.1.6 | D | Unicode-3.0 | `LICENSE` |
| prettyplease | 0.2.37 | C? | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| primal-check | 0.3.4 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| proc-macro2 | 1.0.107 | C? | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| profiling | 1.0.18 | D | MIT OR Apache-2.0 | [dépôt à la révision 82715511](https://github.com/aclysma/profiling/tree/8271551172eb6fa4cba47369aedd93790c623df9) : texte racine à vérifier |
| pxfm | 0.1.30 | D | BSD-3-Clause OR Apache-2.0 | `LICENSE-APACHE.md`, `LICENSE.md` |
| quote | 1.0.47 | C? | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| raw-window-handle | 0.6.2 | D | MIT OR Apache-2.0 OR Zlib | `LICENSE-APACHE.md`, `LICENSE-MIT.md`, `LICENSE-ZLIB.md` |
| read-fonts | 0.39.2 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| realfft | 3.5.0 | D | MIT | [déclaration MIT à la révision 3.5.0](https://github.com/HEnquist/realfft/blob/d0d4eee0525fd27c96c8a046d6d107acd5ed84a6/Cargo.toml) ; [texte MIT amont actuel](https://github.com/HEnquist/realfft/blob/master/LICENSE), attribution pour 3.5.0 à confirmer |
| regex | 1.13.1 | C? | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| regex-automata | 0.4.18 | C? | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| regex-syntax | 0.8.11 | C? | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| rfd | 0.17.2 | D | MIT | `LICENSE` |
| rtrb | 0.4.0 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| rubato | 0.16.2 | D | MIT | `LICENSE.txt` |
| rustc-hash | 2.1.3 | C? | Apache-2.0 OR MIT | `LICENSE-APACHE`, `LICENSE-MIT` |
| rustfft | 6.4.1 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| rustversion | 1.0.23 | C? | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| rusty_mp3 | 0.8.0 | G, C? | Apache-2.0 | `LICENSE` |
| ryu | 1.0.23 | D, G, C? | Apache-2.0 OR BSL-1.0 | `LICENSE-APACHE`, `LICENSE-BOOST` |
| same-file | 1.0.6 | D | Unlicense/MIT | `COPYING`, `LICENSE-MIT`, `UNLICENSE` |
| scopeguard | 1.2.0 | D, G, C? | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| self_cell | 1.3.0 | D | Apache-2.0 OR GPL-2.0-only | `LICENSE-APACHE`, `LICENSE-GPLv2` |
| semver | 1.0.28 | C? | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| serde | 1.0.228 | D, G, C? | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| serde_core | 1.0.228 | D, G, C? | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| serde_derive | 1.0.228 | C? | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| serde_json | 1.0.145 | D, G, C? | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| sha2 | 0.10.9 | D, G, C? | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| shlex | 1.3.0 | C? | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| shlex | 2.0.1 | C? | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| simd-adler32 | 0.3.10 | D | MIT | `LICENSE.md` |
| simdutf8 | 0.1.5 | D, G, C? | MIT OR Apache-2.0 | `LICENSE-Apache`, `LICENSE-MIT` |
| skrifa | 0.42.1 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| slab | 0.4.12 | D | MIT | `LICENSE` |
| smallvec | 1.16.2 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| smol_str | 0.2.2 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| stable_deref_trait | 1.2.1 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| static_assertions | 1.1.0 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| strength_reduce | 0.2.4 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| symphonia | 0.5.5 | D, G, C? | MPL-2.0 | [dépôt à la révision 6d533f26](https://github.com/pdeljanov/Symphonia/tree/6d533f26150953a882a6a111ebd13f0abf7129d5) : texte racine à vérifier |
| symphonia-bundle-mp3 | 0.5.5 | D, G, C? | MPL-2.0 | [dépôt à la révision 6d533f26](https://github.com/pdeljanov/Symphonia/tree/6d533f26150953a882a6a111ebd13f0abf7129d5) : texte racine à vérifier |
| symphonia-codec-pcm | 0.5.5 | D, G, C? | MPL-2.0 | [dépôt à la révision 6d533f26](https://github.com/pdeljanov/Symphonia/tree/6d533f26150953a882a6a111ebd13f0abf7129d5) : texte racine à vérifier |
| symphonia-core | 0.5.5 | D, G, C? | MPL-2.0 | [dépôt à la révision 6d533f26](https://github.com/pdeljanov/Symphonia/tree/6d533f26150953a882a6a111ebd13f0abf7129d5) : texte racine à vérifier |
| symphonia-format-riff | 0.5.5 | D, G, C? | MPL-2.0 | [dépôt à la révision 6d533f26](https://github.com/pdeljanov/Symphonia/tree/6d533f26150953a882a6a111ebd13f0abf7129d5) : texte racine à vérifier |
| symphonia-metadata | 0.5.5 | D, G, C? | MPL-2.0 | [dépôt à la révision 6d533f26](https://github.com/pdeljanov/Symphonia/tree/6d533f26150953a882a6a111ebd13f0abf7129d5) : texte racine à vérifier |
| syn | 2.0.119 | C? | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| syn | 3.0.6 | C? | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| thiserror | 2.0.21 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| tinystr | 0.8.4 | D | Unicode-3.0 | `LICENSE` |
| tracing | 0.1.44 | D | MIT | `LICENSE` |
| tracing-core | 0.1.36 | D | MIT | `LICENSE` |
| transpose | 0.2.3 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| typenum | 1.20.1 | D, G, C? | MIT OR Apache-2.0 | `LICENSE`, `LICENSE-APACHE`, `LICENSE-MIT` |
| unicode-general-category | 1.1.0 | D | Apache-2.0 | `LICENSE` |
| unicode-ident | 1.0.26 | C? | (MIT OR Apache-2.0) AND Unicode-3.0 | `LICENSE-APACHE`, `LICENSE-MIT`, `LICENSE-UNICODE` |
| unicode-segmentation | 1.13.3 | D | MIT OR Apache-2.0 | `COPYRIGHT`, `LICENSE-APACHE`, `LICENSE-MIT` |
| url | 2.5.8 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| utf8_iter | 1.0.4 | D | Apache-2.0 OR MIT | `COPYRIGHT`, `LICENSE-APACHE`, `LICENSE-MIT` |
| uuid | 1.27.0 | D | Apache-2.0 OR MIT | `LICENSE-APACHE`, `LICENSE-MIT` |
| vello_common | 0.0.9 | D | Apache-2.0 OR MIT | `LICENSE-APACHE`, `LICENSE-MIT` |
| vello_cpu | 0.0.9 | D | Apache-2.0 OR MIT | `LICENSE-APACHE`, `LICENSE-MIT` |
| version_check | 0.9.5 | C? | MIT/Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| web-time | 1.1.0 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| webbrowser | 1.2.4 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| webrtc-vad | 0.4.0 | D | MIT | `LICENSE` ; `resources/libfvad/LICENSE`, `resources/libfvad/PATENTS` |
| whisper-adapters | 0.1.0 | D | code propre | Code propre : droits confirmés, licence à préciser |
| whisper-core | 0.1.0 | D, G, C? | code propre | Code propre : droits confirmés, licence à préciser |
| whisper-desktop | 0.1.0 | D | code propre | Code propre : droits confirmés, licence à préciser |
| whisper-rs | 0.16.0 | G, C? | Unlicense | `LICENSE` |
| whisper-rs-sys | 0.15.0 | G, C? | Unlicense | `whisper.cpp/LICENSE` (MIT) ; texte Unlicense à recueillir |
| whisper-worker-common | 0.1.0 | G, C? | code propre | Code propre : droits confirmés, licence à préciser |
| whisper-worker-cpu | 0.1.0 | C? | code propre | Code propre : droits confirmés, licence à préciser |
| whisper-worker-gpu | 0.1.0 | G | code propre | Code propre : droits confirmés, licence à préciser |
| winapi-util | 0.1.11 | D | Unlicense OR MIT | `COPYING`, `LICENSE-MIT`, `UNLICENSE` |
| windows | 0.62.2 | D, G | MIT OR Apache-2.0 | `license-apache-2.0`, `license-mit` |
| windows-collections | 0.3.2 | D, G | MIT OR Apache-2.0 | `license-apache-2.0`, `license-mit` |
| windows-core | 0.62.2 | D, G | MIT OR Apache-2.0 | `license-apache-2.0`, `license-mit` |
| windows-future | 0.3.2 | D, G | MIT OR Apache-2.0 | `license-apache-2.0`, `license-mit` |
| windows-link | 0.2.1 | D, G, C? | MIT OR Apache-2.0 | `license-apache-2.0`, `license-mit` |
| windows-numerics | 0.3.1 | D, G | MIT OR Apache-2.0 | `license-apache-2.0`, `license-mit` |
| windows-result | 0.4.1 | D, G | MIT OR Apache-2.0 | `license-apache-2.0`, `license-mit` |
| windows-strings | 0.5.1 | D, G | MIT OR Apache-2.0 | `license-apache-2.0`, `license-mit` |
| windows-sys | 0.52.0 | D | MIT OR Apache-2.0 | `license-apache-2.0`, `license-mit` |
| windows-sys | 0.60.2 | D | MIT OR Apache-2.0 | `license-apache-2.0`, `license-mit` |
| windows-sys | 0.61.2 | D | MIT OR Apache-2.0 | `license-apache-2.0`, `license-mit` |
| windows-targets | 0.52.6 | D | MIT OR Apache-2.0 | `license-apache-2.0`, `license-mit` |
| windows-targets | 0.53.5 | D | MIT OR Apache-2.0 | `license-apache-2.0`, `license-mit` |
| windows-threading | 0.2.1 | D, G | MIT OR Apache-2.0 | `license-apache-2.0`, `license-mit` |
| windows_x86_64_msvc | 0.52.6 | D | MIT OR Apache-2.0 | `license-apache-2.0`, `license-mit` |
| windows_x86_64_msvc | 0.53.1 | D | MIT OR Apache-2.0 | `license-apache-2.0`, `license-mit` |
| winit | 0.30.13 | D | Apache-2.0 | `LICENSE` |
| wmi | 0.18.4 | D | MIT OR Apache-2.0 | `LICENSE-APACHE`, `LICENSE-MIT` |
| writeable | 0.6.4 | D | Unicode-3.0 | `LICENSE` |
| yoke | 0.8.3 | D | Unicode-3.0 | `LICENSE` |
| zerocopy | 0.8.59 | D | BSD-2-Clause OR Apache-2.0 OR MIT | `LICENSE-APACHE`, `LICENSE-BSD`, `LICENSE-MIT` |
| zerofrom | 0.1.8 | D | Unicode-3.0 | `LICENSE` |
| zerotrie | 0.2.5 | D | Unicode-3.0 | `LICENSE` |
| zerovec | 0.11.8 | D | Unicode-3.0 | `LICENSE` |

## Autres ressources

- Polices du desktop : quatre fichiers complets détectés dans le PE. Conserver les licences et copyrights spécifiques décrits dans le [relevé](entrees-rust-candidats-turbo.md) ; les options MIT/Apache du code egui ne se substituent pas à leurs conditions.
- Workers CPU/GPU : les sources natives `whisper.cpp` et GGML embarquées par `whisper-rs-sys` requièrent leurs propres licences et notices. Confirmer les objets natifs réellement retenus et les textes applicables ; le champ `Unlicense` de la crate ne suffit pas.
- Desktop : le build `webrtc-vad` utilise `libfvad` ; examiner la licence de ce dernier et `PATENTS` en plus de celle de la crate.
- CUDA : le GPU importe `cublas64_12.dll`, `cudart64_12.dll` et le pilote `nvcuda.dll` ; la DLL candidate cuBLAS importe `cublasLt64_12.dll`. Trois DLL CUDA sont des candidats locaux, le pilote est un prérequis externe. Vérifier les termes et notices des versions retenues et la fermeture des chargements dynamiques.
- Visual C++ : desktop et workers importent les DLL décrites dans l'[inventaire](inventaire-contrat-installation-turbo.md). Les fichiers candidats sont signés Microsoft ; déterminer le droit exact de les distribuer et les conserver non modifiés.
- Modèle : fourni séparément hors ligne ; voir le [rattachement de conversion](inventaire-contrat-installation-turbo.md#rattachement-de-la-conversion-du-modèle) et conserver ses mentions applicables.

Avant remise, vérifier également la licence du runtime Rust et des éventuels autres contenus embarqués, les notices `NOTICE` et droits d'auteur, les alternatives ou exigences cumulatives, ainsi que les obligations MPL de mise à disposition du code source couvert si applicable. Refaire l'index dès qu'un PE ou une dépendance change ; collecter et contrôler les notices avec le paquet L3.
