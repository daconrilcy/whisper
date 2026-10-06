# Sources et règles — P-WHISPER-08

Base PLANS P05, digest `62b33f09e1840e9f95b6d745a10f0b598f75b3b822ce68a5a7da3f7065aa1833`, parent DESIGN D19. P05 reste non promu ; P02 demeure PLANS READY au checkpoint jusqu’à une revue CLEAN et promotion ultérieure. P03/P04/P05 sont immuables. Les sources D19 héritées dans `sources/design/` restent byte-identiques.

## Sources architecturales exactes

| Copie P06 | Origine sous docs/predev | SHA256 source contrôlé |
|---|---|---|
| `sources/architecture/DETAIL-P01-v3.md` | `transports/T-WHISPER-ARCH-P01-DETAIL-02/DETAIL-P01-raw.md` | `c554c748d888eb91aef8cff024ba4eb6b3b7d0317f333f0395111c084cae8086` |
| `sources/architecture/ARCH-L01-INTEGRATION-v2.md` | `transports/T-WHISPER-ARCH-L01-INTEGRATION-02/CHANGE-L01-INTEGRATION-01-raw.md` | `1c8675f6d1e0f18e05556e8e3118d824e1c95591aca37e8d4e6823733787ea27` |
| `sources/architecture/ARCH-L01-BOUNDS-v1.md` | `transports/T-WHISPER-ARCH-L01-BOUNDS-01/ARCH-L01-BOUNDS-raw.md` | `fee3f0dbcb5ecdbc9f70082aba311bca3daa34bf5efaa12d6b90b71a10f2efce` |
| `sources/architecture/D19-COMPAT-v2.md` | `transports/T-WHISPER-REVIEW-L01-COMPAT-01/R-WHISPER-D19-COMPAT-01-raw.md` | `34c8ce954c352eb42da575c2164f19959b144f39889d846d5d3b816eb02947ac` |
| `sources/architecture/Q05-answer.md` | `transports/T-WHISPER-Q05-ANSWER-01/Q-05-answer-raw.md` | `89538c0beebfe221883bb9e260d415818a992e58273bb870ba5dbaddea0f45c6` |

Progress copies preserve L00/L01 historical preflights, L00 ledger, active railguard and its activation. The coordinator calculates each destination hash and compares bytes to its source. Historical status is not current gate evidence.

## Règles effectives

P03/P04 ont relevé des divergences de copies. P05 a restauré les snapshots byte-identiques et ajouté `execution-evidence.md`; P06 les conserve après comparaison directe. `rules/pack-manifest.json` lie les règles effectivement relues. Tout changement ultérieur de source exige une nouvelle évaluation et un nouveau candidat. Les helpers copiés en `.txt` sont des preuves ; exécuter uniquement les helpers dont le hash correspond au pack courant.

## Continuité

Sources produit : `sources/USER_AUTHORIZATION_PLANS.md`, `sources/USER_REQUEST_P03.md`, `sources/design/02_REQUIREMENTS.md`, `07_COVERAGE_AND_ACCEPTANCE.md`, `08_STATE_AND_PORT_CONTRACTS.md`, `10_QUESTIONS_RISKS_AND_READINESS.md`, `11_RAILGUARD_PROPOSAL.md`, `32_USER_DECISIONS.md`, `47_USER_VALIDATION_D17.md`, `50_ARCH_DECISIONS_D18.md`, `51_CLOSURE_INSTALL_MODES_D19.md`. Les sources exactes restent incluses au manifeste. La demande humaine de correction et d’autorisation d’enregistrer les 21 chemins est capturée séparément ; source d’autorisation documentaire et de planification ne remplace pas cette autorisation de code.

Les transports DETAIL-P01/Q05/INTEGRATION/BOUNDS/COMPAT, contribution plan writer et review P03 sont conservés/ajoutés au checkpoint après vérification. La revue COMPAT ne remplace pas la revue indépendante exacte P06. Les rapports P04/P05 restent des antériorités et ne déclarent pas P06 CLEAN. Aucune source conversationnelle non exportée n’est qualifiée de copie brute exacte.


P05 a ajouté `rules/skills/rust-predev-design/references/execution-evidence.md.txt` ; P06 le conserve depuis `C:/Users/cyril/.codex/skills/rust-predev-design/references/execution-evidence.md`, SHA256 `02054e091e4fecf77719b06b2b6fbe07a2d002ca10278626c422cf97acfe8ab5`. La copie du contrat et les snapshots de règles sont encodés depuis les octets source exacts, sans normalisation de fins de ligne ; recalculer et comparer les 23 hashes source/destination avant revue. Le freeze de pack référencé est PACK-WHISPER-P05.


## Sources de la correction P07

- Base immuable P06 : digest `5759cb207f39fe62fee921321dd90275a67a8d687133cf6fd23afd6ac09e3a93`; parent D19 inchangé.
- Mandat utilisateur couvrant les quinze chemins : `sources/progress/USER_AUTHORIZATION_L02.md`; capture directe, hashée dans le manifeste P07.
- Réponse Q-07 attribuée : `sources/architecture/Q-07-answer-v1.md`, copie octet-identique de `transports/T-WHISPER-Q07-ANSWER-01/Q-07-architecture-answer.md`; manifeste TRANSPORT `transports/T-WHISPER-Q07-ANSWER-01/transport-manifest.json`.
- Progression L01 : ledger et `transports/T-WHISPER-L01-CLOSURE-02/execution-evidence.json`; sorties hashées fournissent l’API et la base du préflight L02.
- Source des exigences/frontières : DESIGN D19/08, railguard actif et contrats L01 hérités de P06. Les sources de code pour l’extension sont `crates/whisper-core/src/application.rs`, `crates/whisper-core/src/ports.rs`, `crates/whisper-adapters/src/archive.rs`, `crates/whisper-adapters/src/worker_ipc.rs`, `crates/whisper-worker-cpu/src/decoder.rs` et root/UI ; leurs hashes courants doivent être recalculés au préflight, pas figés comme hashes de sortie future.
- Les nouveaux rapports de tests, builds et qualification restent futurs / NOT RUN ; ils ne figurent pas comme preuves de PASS.


## P08 baseline et L01

Les snapshots `sources/code/` sont byte-exacts depuis HEAD `5fee290b8e74205fca94d4d2b478d430680132d4`; détails/provenance et chemins futurs dans `09_L02_SOURCE_BASELINE.md`. Preuves texte L01-CLOSURE-02 incorporées sous `sources/progress/L01-CLOSURE-02/`; les captures UI binaires restent référencées à la source originale et leurs SHA y sont conservés. Ces copies ne valent ni revue P08 ni validation L02.

- `crates/whisper-adapters/src/archive.rs` SHA256 `28ea92cd2e06a4c4206d42c78078c95ac77ae474869976d9045ec2c6aabb195a` (L02_PRESENT)
- `crates/whisper-adapters/src/decoder.rs` SHA256 `548e111e4abb4aaab1170bf2e80cbf8c0e2603006e2f9fe80943feb450ae0288` (L02_PRESENT)
- `crates/whisper-adapters/src/lib.rs` SHA256 `bd3ea83de0ea78e06fd7dc679590f731e1a4656bdcba814ca2757583dfc26699` (L02_PRESENT)
- `crates/whisper-adapters/src/worker_ipc.rs` SHA256 `979431ac8b21ad4b81647c7fac2f9806481efa2585b6019fe2a0e5035b38e160` (L02_PRESENT)
- `crates/whisper-core/src/application.rs` SHA256 `5ec3deb57c98e842e1bab592d9217c95b810cb6ad71717467f4a6489ea7abb8b` (L02_PRESENT)
- `crates/whisper-core/src/ports.rs` SHA256 `94cb3ba7210ad755cf5273bdb2a8057ab2cb65f18983a499ea26be1e5c4adcb7` (L02_PRESENT)
- `crates/whisper-desktop/src/root.rs` SHA256 `9a3713112e3dfc3872e15e3cb581c7c6a53222e5f62864144ba4d111caa98599` (L02_PRESENT)
- `crates/whisper-desktop/src/ui.rs` SHA256 `67eac6aded566e0a0e3d5ed101321bb4134246d76af339b54271ae984302c7f8` (L02_PRESENT)
- `crates/whisper-worker-cpu/src/decoder.rs` SHA256 `bda03e8b8561f7912f5c8eaa21fc62215ea359eab047cb8771db9f9aea6f613a` (L02_PRESENT)
- `Cargo.lock` SHA256 `821b32dc3bc5a3c6f60b997772902e1af5de8029cdf8171c5292050ec4bf82aa` (SUPPORT)
- `rust-toolchain.toml` SHA256 `6a0855ab5a6b75bd9449e9ac8363a52eacb9ae8b3a7c80bfce0378d013cb16db` (SUPPORT)
- `crates/whisper-adapters/Cargo.toml` SHA256 `917120ec0edf7725143a2d9d0c8405dbf82238ce87a3134b3df86c6af2f6011e` (SUPPORT)
- `crates/whisper-worker-cpu/Cargo.toml` SHA256 `c1181281797bf037e9ca35d6ee78927e6fc7d18af26b7445b76e54be0247fbe9` (SUPPORT)
- `crates/whisper-core/src/domain.rs` SHA256 `3b45f9852965311d58b4aa3b53ec6db8a5936b3a8933e27dd46ee76a754f7acd` (SUPPORT)
- `crates/whisper-core/src/lib.rs` SHA256 `c5c83ee62d7609a8a8f01bfc433b3a7a3af8aec642c7ff5306bd9973134827e6` (SUPPORT)
- `crates/whisper-worker-cpu/src/main.rs` SHA256 `d04bd322d92501991d8be526b510a670c228161e6caa2c5e61940dd1894f2e9a` (SUPPORT)
- `crates/whisper-desktop/src/main.rs` SHA256 `9c6e5cbddbda59e41a610b187afb5f2e58dfbf1a01110e301c35c8ef9d5029a0` (SUPPORT)
- `crates/whisper-desktop/src/lib.rs` SHA256 `193360385cfea082ead338bf441ddf8ada5155b1d5113908fecd6988dd1c2db6` (SUPPORT)

Futurs absents sans hash : `crates/whisper-adapters/src/journal.rs`, `crates/whisper-adapters/src/queue_store.rs`, `crates/whisper-adapters/src/recovery.rs`, `crates/whisper-adapters/tests/durability.rs`, `crates/whisper-adapters/tests/import_mp3.rs`, `crates/whisper-core/tests/scheduler_contract.rs`.
