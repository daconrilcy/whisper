# Complément des notices tierces Turbo

Avancée : notices préparées pour les archives sans fichier de licence à la racine, dont AccessKit et Chromium.
Blocage : les éléments restent candidats tant que les binaires et les modalités de remise ne sont pas figés et revérifiés.
Décision : conserver ce complément avec le recueil et les archives de notices pour préparer L3.

Ce complément vise les entrées D de l'[index](index-notices-turbo.md), qui sont des dépendances potentielles du desktop et non une preuve individuelle d'incorporation. Pour les licences alternatives, le traitement proposé ici est Apache-2.0 pour AccessKit et MIT pour egui, dasp_sample, profiling et realfft ; il faut conserver les licences et notices additionnelles indiquées. Le texte complet Apache-2.0 figure dans le [recueil, texte 002](recueil-licences-candidates-turbo.md#texte-002). Le texte MIT commun figure ci-dessous avec les mentions particulières.

## AccessKit et sources dérivées

- `accesskit 0.24.1`, `accesskit_consumer 0.35.0`, `accesskit_windows 0.32.1` : Apache-2.0 proposé parmi les licences alternatives déclarées. `accesskit_winit 0.32.2` : Apache-2.0. Les sources portent des copyrights des auteurs AccessKit, datés selon les fichiers de 2021, 2022, 2023, 2024 et 2025.
- Certains fichiers portent les copyrights des auteurs Chromium pour 2018 ou 2021 et renvoient à `LICENSE.chromium`. La notice BSD complète et le copyright des auteurs Chromium pour 2015 figurent dans la section [Chromium du recueil](recueil-licences-candidates-turbo.md#chromium--licence-bsd-candidate-pour-accesskit) ; les fichiers concernés portent également leurs années propres.
- `accesskit 0.24.1/src/geometry.rs` mentionne les auteurs kurbo, copyright 2018, et indique Apache-2.0 ou MIT. Le texte Apache-2.0 du recueil est proposé pour cette partie.

Ces mentions proviennent des en-têtes des sources en cache ; les liens amont aux deux révisions et la réserve sur d'éventuels fichiers `NOTICE` sont dans l'index. Ne pas traiter la seule expression Cargo MIT/Apache comme une notice complète pour les parties Chromium.

## Autres archives sans licence à la racine

| Paquet | Traitement préparé |
|---|---|
| `clipboard-win 5.4.1` | BSL-1.0 ; texte complet dans le [recueil, texte 041](recueil-licences-candidates-turbo.md#texte-041), source amont référencée dans l'index. |
| `dasp_sample 0.11.0` | MIT ; copyright 2016 des développeurs RustAudio, selon la licence amont à la révision référencée. |
| `ecolor`, `eframe`, `egui`, `egui-winit`, `egui_glow`, `emath`, `epaint` et `epaint_default_fonts 0.35.0` | MIT pour le code ; copyright 2018–2021 Emil Ernerfeldt selon le fichier amont. Les licences et copyrights des polices sont cumulativement à conserver dans le recueil. |
| `profiling 1.0.18` | MIT ; copyright 2020 Philip Degarmo et autres contributeurs selon la licence amont. |
| `realfft 3.5.0` | MIT déclaré et HEnquist indiqué comme auteur par la version exacte. La licence amont actuelle nomme Henrik Enquist et les contributeurs RealFFT en 2020 ; ce fichier ultérieur est conservé comme indication, son rattachement exact à 3.5.0 restant non démontré. |
| Six crates `symphonia 0.5.5` | [Notice MPL et archive des sources](licence-mpl-symphonia.md). |
| `whisper-rs-sys 0.15.0` | Unlicense déclaré ; texte dans le [recueil, texte 017](recueil-licences-candidates-turbo.md#texte-017), en plus de la licence MIT de whisper.cpp et des notices natives applicables. |

Les sources amont exactes et réserves figurent dans le [relevé des copyrights](copyrights-notices-turbo.md). Les textes fournis par l'amont et toute notice supplémentaire exigée doivent être préservés dans la remise.

## Texte MIT commun

Le texte suivant provient du fichier `MIT.txt` de la toolchain Rust 1.98.1 ; la ligne générique de remplacement du copyright a été retirée, les mentions étant précisées ci-dessus.

````text
MIT License

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
````
