# Sources et règles figées — P-WHISPER-02

Le manifeste PLANS couvre toutes les copies ci-dessous en plus des documents du plan et du manifeste DESIGN parent. SHA256 des octets exacts ; aucune règle du corpus D19 n’est modifiée.

Mandat PLANS : `sources/USER_AUTHORIZATION_PLANS.md`, export direct du message utilisateur de la session `rollout-2026-10-05T20-35-48-01a10d59-bfe5-7913-bc7c-308ec7dc4210.jsonl` ligne 12. Il autorise préparation et revue des plans, pas le code.

Avis indépendant KEEP_D19 : `transports/T-D19-IMPACT-01` ; plan writer exact : `T-PW-WHISPER-01-EXACT` ; findings P1 : `T-WHISPER-PLANS-REVIEW-01`. Ces résultats restent hors manifeste PLANS et dans le checkpoint.

| Copie dans P2 | Origine | SHA256 |
| --- | --- | --- |
| `sources/design/02_REQUIREMENTS.md` | `candidates/D-WHISPER-19/02_REQUIREMENTS.md` | `ac6df6a29aa6d6fca09512af26adb0b1c0418cd94ede08c57526d4a49e52ec31` |
| `sources/design/07_COVERAGE_AND_ACCEPTANCE.md` | `candidates/D-WHISPER-19/07_COVERAGE_AND_ACCEPTANCE.md` | `03e88ece134bfa7f388c60468e31046b5d591238c24769a4a37f6f356c61e0cf` |
| `sources/design/08_STATE_AND_PORT_CONTRACTS.md` | `candidates/D-WHISPER-19/08_STATE_AND_PORT_CONTRACTS.md` | `ffa9efb206c42941a9c846cfdf6a1783b766781472c556e1aad4187958406240` |
| `sources/design/10_QUESTIONS_RISKS_AND_READINESS.md` | `candidates/D-WHISPER-19/10_QUESTIONS_RISKS_AND_READINESS.md` | `6a0f5188178a942c6da82d5452e9dab0dfb32b733d8c66d6f58be692422227b6` |
| `sources/design/11_RAILGUARD_PROPOSAL.md` | `candidates/D-WHISPER-19/11_RAILGUARD_PROPOSAL.md` | `c59b913296117ee949f66349db8e821e2921a663589c3be743b31a2a757b8a01` |
| `sources/design/32_USER_DECISIONS.md` | `candidates/D-WHISPER-19/32_USER_DECISIONS.md` | `6654eda0f8e467e852ce82a10561250bb5a35b6633d8b4073a161d21a6d72508` |
| `sources/design/47_USER_VALIDATION_D17.md` | `candidates/D-WHISPER-19/47_USER_VALIDATION_D17.md` | `996cde451b5d9b6394f8b3e59717b8efbae914cdef16aaa9b225bf2f8277c90b` |
| `sources/design/50_ARCH_DECISIONS_D18.md` | `candidates/D-WHISPER-19/50_ARCH_DECISIONS_D18.md` | `bac81d0eca399cd7cef48a0e87eb778aa60c0981daf35e03a959dd265dec6b80` |
| `sources/design/51_CLOSURE_INSTALL_MODES_D19.md` | `candidates/D-WHISPER-19/51_CLOSURE_INSTALL_MODES_D19.md` | `b601f6b7d3266f5f501b47b583e4ed6faea765fed889259606e860323897b81b` |
| `rules/agent-rules/ARCHITECTURE_CONSTITUTION.md.txt` | `C:/Users/cyril/.codex/agent-rules/ARCHITECTURE_CONSTITUTION.md` | `74865dcb4e69173d43a2e38f3dad134fe0816c9da4a7a50029c7445ca3d85b38` |
| `rules/agent-rules/IMPLEMENTATION_RULES.md.txt` | `C:/Users/cyril/.codex/agent-rules/IMPLEMENTATION_RULES.md` | `04ad5f6cdbc09df8fdbdb9d0ef6d59157f37289872966f60cac892172ba66dc0` |
| `rules/agent-rules/REQUIREMENTS_RULES.md.txt` | `C:/Users/cyril/.codex/agent-rules/REQUIREMENTS_RULES.md` | `f6e393569a59fe7f910f863f7753ccd3c9d64c82a7bfb28bd30ebdb48a22e0e6` |
| `rules/agent-rules/REVIEW_RULES.md.txt` | `C:/Users/cyril/.codex/agent-rules/REVIEW_RULES.md` | `92be7cd942eadc25876a2d3b80a200ed55bb962d0567f2b53da69f65b419d621` |
| `rules/agent-rules/RUST_RULES.md.txt` | `C:/Users/cyril/.codex/agent-rules/RUST_RULES.md` | `ed8789932c0ad8abdc6422c7730a802f160a27b210ed0dafc6cd6318d90751fe` |
| `rules/agents/rust_architect.toml.txt` | `C:/Users/cyril/.codex/agents/rust_architect.toml` | `2357cee1a032948fc58f575fee3c2c851d5089220115e845d4a4430d8b9cbc26` |
| `rules/agents/rust_design_reviewer.toml.txt` | `C:/Users/cyril/.codex/agents/rust_design_reviewer.toml` | `630ba050ffe3101c2c168e2caac923f81092191c0d1e99ad696984adfe793e7d` |
| `rules/agents/rust_domain_architect.toml.txt` | `C:/Users/cyril/.codex/agents/rust_domain_architect.toml` | `2289439762ec7d7980d09149872c4a14af4c5585d7b3c7388ecb7ca86be0faeb` |
| `rules/agents/rust_plan_writer.toml.txt` | `C:/Users/cyril/.codex/agents/rust_plan_writer.toml` | `26aa5d8421735b1a5c22061a779177a0f4a097fd6133754b5d2590bfd9de8e62` |
| `rules/agents/rust_predev_orchestrator.toml.txt` | `C:/Users/cyril/.codex/agents/rust_predev_orchestrator.toml` | `3557a48740537fa63cc041052e89d85073fa032c59257bda64b381d0ba607301` |
| `rules/agents/rust_product_framer.toml.txt` | `C:/Users/cyril/.codex/agents/rust_product_framer.toml` | `2e76b521c3e9870b084f3d5fb688c2766bceb92581853c0bcbd434676c278e83` |
| `rules/agents/rust_requirements_analyst.toml.txt` | `C:/Users/cyril/.codex/agents/rust_requirements_analyst.toml` | `59d7e2fc29078e262c22a5104e86bb96ea3aebc69a738454ae1869cbdc4e5d72` |
| `rules/skills/rust-predev-design/SKILL.md.txt` | `C:/Users/cyril/.codex/skills/rust-predev-design/SKILL.md` | `a186c12b971b97c6af2be33f792605750001c17e5103060c2505646e0295eeae` |
| `rules/skills/rust-predev-design/references/deliverable-quality.md.txt` | `C:/Users/cyril/.codex/skills/rust-predev-design/references/deliverable-quality.md` | `b376331863fe099e5361a6fb686bd720b5977d339657c7ff53c085e0943660ef` |
| `rules/skills/rust-predev-design/references/engineering-contract.md.txt` | `C:/Users/cyril/.codex/skills/rust-predev-design/references/engineering-contract.md` | `1f5af0cc54a55339ed2f8946860b008383015c9301d311266104da895e94c5ed` |
| `rules/skills/rust-predev-design/references/handoff-contract.md.txt` | `C:/Users/cyril/.codex/skills/rust-predev-design/references/handoff-contract.md` | `ece19ba4f02299aa774868a1811046638c7f5f296e6485e4c9daeb1bae9d1138` |
| `rules/skills/rust-predev-design/references/runtime-qualification.md.txt` | `C:/Users/cyril/.codex/skills/rust-predev-design/references/runtime-qualification.md` | `ed28c253cb94d7911a3709c012f086302faa8dff779f032c9173c15ec016fa08` |
| `rules/skills/rust-predev-design/references/workflow-schema.md.txt` | `C:/Users/cyril/.codex/skills/rust-predev-design/references/workflow-schema.md` | `526b015ae3a6a6451f684b4484ee849b7f8344aaf6be8e5dace09eb0c728bba6` |
| `rules/skills/rust-predev-design/scripts/predev_control.py.txt` | `C:/Users/cyril/.codex/skills/rust-predev-design/scripts/predev_control.py` | `6727732e36edc5f3c05821f61549ab92ba1462d47ba928ca12663c8e36a7d83f` |
| `rules/skills/rust-predev-design/scripts/predev_host.py.txt` | `C:/Users/cyril/.codex/skills/rust-predev-design/scripts/predev_host.py` | `041da69893a814c86337863cd41a22637b4754c381ef59c026e6917361bfe031` |
| `rules/skills/rust-predev-orchestration/SKILL.md.txt` | `C:/Users/cyril/.codex/skills/rust-predev-orchestration/SKILL.md` | `df8e53491a42433557319056f61c7e26f4acb7da0871e8448c1964dd9859af97` |
| `rules/skills/rust-predev-design/scripts/verify_raw_handoff.py.txt` | `C:/Users/cyril/.codex/skills/rust-predev-design/scripts/verify_raw_handoff.py` | `f31851b7477ea43b66d66de36c104b24e76e6ffd3e38106289513d2606729f35` |

Le pack courant est `rules/pack-manifest.json`, digest `0ca0e6b14fd0d3473611d91fde0d4afbd6322ff3b4964b40746c4860232486ed`. Comparer les copies à ces sources avant reprise ; une divergence future est une réévaluation explicite, non une retouche de P2.
