# SPIKE-02 — encodage MP3 Rust borné

Le 2026-10-05, sous le mandat SPIKE-02, `rusty_mp3 0.8.0` (Apache-2.0) et `hound 3.5.1` ont été compilés en release avec Rust 1.98.1. Le code d'essai est `encoder-main.rs.txt`, les versions figées dans `Cargo.lock.txt`.

Entrée : `C:\WhisperLive\micro_test.wav`, SHA-256 `716e788c790b600fe472d0ebb868094c9d9b97fdf1d581133dd3df46a30de467`, 16 kHz mono, 233 472 échantillons. Sortie : MP3 mono 16 kHz CBR demandé 64 kb/s, 117 216 octets, 407 paquets, SHA-256 `ae665aecbd179f0b4879d4ba570eed11b7cbee7f7bbc325ef7bb983cf6af5c8d19`. FFprobe identifie codec MP3 et durée 14,652 s ; le décodage FFmpeg se termine sans erreur. Audio PCM source 14,592 s : 60 ms de padding/retard selon cette mesure.

Il s'agit d'un **smoke de validité de format sur un seul WAV**. Ni qualité perçue, ni comparaison d'alignement exact, ni interruption d'encodeur, ni publication multi-passages ou licence de paquet complet ne sont démontrés. Le finding 008 reste OPEN. Source du crate : [rusty_mp3 0.8.0](https://crates.io/crates/rusty_mp3/0.8.0).
