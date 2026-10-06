# Sources et règles — P-WHISPER-06

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
