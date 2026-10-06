Contribution brute **ARCH-L01-BOUNDS v1 — DRAFT**, attribuée à `/root/l01_bounds_arch`, rôle `rust_architect`, datée du **2026-10-06**, owner technique architecte Rust ; suivi `AUTH-COORD`. Aucun fichier modifié, aucun banc exécuté. Destination proposée : nouvelle entrée documentaire du candidat P03 et TRANSPORT attribué, sans modifier D19 ou P02.

Base exacte :

- Checkout `C:\dev\whisper`, branche `main`, HEAD `e223166a08592048508865b480ecc53b101bdd0d` ; état observé : uniquement `?? target/`.
- DESIGN `D-WHISPER-19`, digest référencé par le checkpoint `903926ba27c70fc3a0cc6954d9e50914a64993e40c339683e557bc4c604d7529` ; SHA256 du fichier manifeste personnellement recalculé `7f6465f9254611d432da50ab64034c7ae92b088351396f2c3eb3249c71866cb7`.
- PLANS `P-WHISPER-02`, digest référencé `8113649a38976e0c14e1c82a10ed1a04dff7c1022aaff5587962a0d94b8b2bad` ; SHA256 du manifeste recalculé `7ae618d2b1149d5856ae9e080296af5470a509327d732ecec276eda77d3e96e9`.
- `state.json` lu : SHA256 `92924345e6dd3f133cfdd95a97d834d0a283342d04cd1a50153a58475eb5faed`.
- Source antérieure : `ARCH-L01-INTEGRATION v2`, attribuée à `/root/whisper_l01_integration_coordination/l01_architecture_contract`, retrouvée conversationnellement par le coordinateur. Le coordinateur m’a transmis exactement ses valeurs : **8 effets en attente, 64 événements persistables, 1 MiB/frame, bloc PCM limité par `max_samples` vérifié avant allocation**. Je n’ai pas personnellement contrôlé son export intégral ni son reçu. Cette limite de provenance doit rester explicite jusqu’à leur vérification.

La proposition numérique antérieure n’était présente dans aucun document consulté du corpus courant. Elle n’est donc pas absente de l’historique conversationnel ; elle reste à persister par l’hôte.

**Décision technique `DETAIL-L01-BOUNDS-v1`** : nature `reversible_detail`, validation technique déléguée par la mission et par la demande utilisateur « valider les limites proposées […] préciser la taille des fenêtres PCM ». J’accepte les trois valeurs antérieures pour L01 avec les contrats complémentaires suivants. Le document reste DRAFT jusqu’à persistance et revue ; cette validation ne ferme aucun finding de reviewer.

| Frontière / propriétaire | Limite validée | Effet et contrôle |
|---|---|---|
| Application → exécutant d’effets, application propriétaire des effets | **8 effets en attente** ; budget des représentations wire retenues **≤ 8 MiB** | L’effet en cours est compté séparément : **1 maximum**, sans seconde file cachée. Admission refuse explicitement si capacité indisponible ; aucun effet accepté puis perdu. |
| Exécutant → application, application propriétaire de l’admission des résultats | **64 événements persistables en attente** ; budget wire cumulé **≤ 64 MiB** | Événements de résultat retenus jusqu’à consommation ; rétropression hors UI. Aucune éviction silencieuse de segment, erreur, fin ou attestation. |
| Transport IPC, adaptateur `worker_ipc` et boucle worker | **1 MiB = 1 048 576 octets par frame**, enveloppe et métadonnées comprises | Vérifier taille avant allocation et désérialisation, dans les deux sens. Le framing doit borner aussi l’accumulation d’une frame incomplète ; lecture de ligne illimitée interdite. Refus typé pour frame trop grande, version incorrecte, corruption ou identité invalide. |
| Décodeur normalisé / worker CPU | **1 requête de décodage en cours** et **1 bloc retourné au maximum** | Le `Vec<DecodedPcmBlock>` actuel a une cardinalité de 0 ou 1 ; aucun cumul de tout le fichier dans ce `Vec`. `0` signifie EOF explicite selon le protocole. |
| Moteur / worker CPU | **1 fenêtre d’inférence en cours** | Pas de parallélisme moteur spéculatif. En L01, prochain bloc demandé seulement après admission du résultat précédent et disponibilité des capacités. |
| Contrôle critique / superviseur | État fini indépendant : **1 Stop en attente pour la génération active**, **1 Shutdown global** | Répétitions coalescées/idempotentes. Les contrôles ne dépendent pas d’une place disponible dans la file data ; canal et lecteur de contrôle doivent rester accessibles pendant `full()`. |
| Rendu provisoire / présentation | **1 dernier snapshot remplaçable** | Peut remplacer un rendu provisoire obsolète. N’accueille aucun résultat persistable ni commande critique. |

Ces budgets portent sur les buffers explicitement gérés par L01. Ils **ne constituent pas une borne RSS globale** : modèle, contexte natif, buffers internes Symphonia, copies de sérialisation, allocations des objets Rust et buffers OS doivent être inventoriés et mesurés pendant l’implémentation. Un unique `sync_channel(64)` ne borne pas les octets, ni les objets déjà retirés de la file.

**Fenêtre PCM validée pour L01 : 5 secondes**, soit **80 000 échantillons mono à 16 000 Hz**.

| Représentation | Taille maximale par fenêtre |
|---|---:|
| PCM16 mono normalisé | 80 000 éléments `i16`, **160 000 octets** |
| PCM float interne pour `whisper-rs::WhisperState::full` | 80 000 éléments `f32`, **320 000 octets** |
| Durée portée | **5 000 ms** |
| Dernière fenêtre | **1 à 80 000 échantillons** ; aucune fenêtre vide envoyée au moteur |

Contrat concret :

- `1 ≤ max_samples ≤ 80_000` vérifié avant allocation et avant décodage. Les canaux/fréquence normalisés sont exactement **mono / 16 kHz**.
- `end_sample - start_sample = samples.len()` dans l’axe normalisé ; plages `[start_sample, end_sample)`, séquences monotones et contiguës pour le parcours import courant.
- Fenêtres successives sans chevauchement ni trou introduit par le découpage. Offsets de segments locaux convertis sur l’axe temporel original, puis vérifiés à l’intérieur de la plage réellement décodée. La dernière fenêtre conserve sa durée réelle ; aucun padding ne devient durée source ou SRT.
- La sérialisation actuelle en JSON de 80 000 valeurs PCM16 tient dans 1 MiB : borne conservatrice **560 000 octets** pour valeurs et séparateurs, hors métadonnées. L’émetteur doit néanmoins vérifier la taille réelle totale avant envoi. Aucun envoi en plusieurs frames implicite.
- Le décodeur ne charge pas le fichier entier. Buffers intermédiaires et paramètres source sont contrôlés avant allocations ; `max_samples` de sortie seul ne protège pas une allocation native effectuée auparavant.
- Le worker possède les buffers moteur et le contexte. Aucun audio brut dans les logs de diagnostic ; source en lecture seule et identité revalidée.
- Si le moteur refuse une dernière fenêtre courte, rendre une erreur explicite avec sa plage ; ne pas supprimer cette plage ni annoncer Complete.

Le choix **5 s** réutilise une taille déjà exercée par les essais D19, minimise le PCM conservé et fournit une progression granulaire. Il reste réversible après mesure du parcours réel. Il ne garantit ni qualité aux frontières de phrases, ni débit temps réel, ni délai d’annulation. Les essais D19 montrent précisément que le CPU peut accumuler du retard. Une éventuelle fenêtre différente doit être attribuée, justifiée et revue sur les contrats concernés.

**Saturation et arrêt.** Pour un import, le consommateur régule une source relisible : arrêter les demandes de blocs pendant la rétropression, sans modifier la source. L’UI reste non bloquante. File durablement indisponible, consommateur déconnecté, dépassement wire ou résultat invalide produisent une erreur explicite ; préserver les sorties déjà vérifiées et empêcher Complete tant que le publisher n’a pas confirmé les artefacts. Stop invalide la génération avant toute admission de nouveaux résultats et cesse la lecture/décodage suivant ; le contrôle coopératif peut demander l’abandon natif. Aucun délai arbitraire de retour de FFI n’est promis, aucun kill automatique ajouté. Quitter reste ouvert si l’enfant ne confirme pas son arrêt, conformément à D19.

**Résultats obsolètes.** Vérifier version, `job_id`, génération, requête, séquence et plage avant la mutation de l’application ou du publisher. Le simple remplissage d’une file ne constitue ni ACK durable ni confirmation de fragment. Les plafonds IPC n’ajoutent aucune limite au nombre total d’imports restaurables ou à la taille des archives ; ces politiques relèvent des lots suivants.

**Placement P03 nécessaire** : constantes/types purs et validations de contrats dans `whisper-core/src/{ipc,ports}.rs` ; admission/coordination dans application/import service ; framing et budgets dans `whisper-adapters/src/worker_ipc.rs` ; validation `max_samples` et flux dans le proxy `decoder.rs` et le décodeur enfant ; boucle CPU et conversion `i16 → f32` dans les modules worker concernés ; wiring des capacités dans `whisper-desktop/src/root.rs`, appelé par `main.rs`. L’UI consomme uniquement façade/vues. Les noms additionnels exacts sont à fixer par le plan writer sur l’assemblage complet, sans nouvelle écriture de cet acteur.

**Preuves et limites datées du 2026-10-06 :**

- D19/`08_STATE_AND_PORT_CONTRACTS.md`, concurrence et ports : contrôle distinct, données finies, aucune perte silencieuse, génération périmée et arrêt ouvert. SHA256 `ffa9efb206c42941a9c846cfdf6a1783b766781472c556e1aad4187958406240`.
- D19/`21_MEASUREMENTS.md` : fenêtres 5 s, CPU distinct et retard observé ; SHA256 `3c24cd11dcb0c0cd73c06a5a57a5e3fbcbf08ab0cf63c77d81693dc1d768ceba`.
- D19/`38_PROVISIONAL_THRESHOLDS.md` : seuils limités aux fixtures, aucun plafond temps réel CPU ; SHA256 `392f398a4d87be65ec997e3a5c33fecc6ed50eee618e73c1174b1915526a539b`.
- D19/`49_TECHNICAL_EVIDENCE_D18.md` : E5 exerce huit messages et saturation ; peak neuf inclut writer sans pipe OS, perte totale et mémoire non prouvées. SHA256 `fda812bf6f75c8e7a5fe284dd3830f9729c9d9dee4e434c414235a4456df8bb8`. **DESIGN_FEASIBILITY historique**, portée de la preuve inchangée.
- D19/`50_ARCH_DECISIONS_D18.md`, TECH-D18-04/08 : processus enfant et frontières ; SHA256 `bac81d0eca399cd7cef48a0e87eb778aa60c0981daf35e03a959dd265dec6b80`.
- Source locale primaire `whisper-rs 0.16.0/src/whisper_state/mod.rs:279,292` : `full` reçoit `f32`, mono, 16 kHz et refuse les données vides ; SHA256 `3bb8bda10aabe8bb46ba0c1cb5da68128284dfe33b4ef7cc5d8654acee63bdd9`.
- Source locale primaire `symphonia-core 0.5.5/src/audio.rs:725,730,735,850` : allocation `duration × channels`, contrôle capacité lors de copie ; SHA256 `1f2944423229cb974f28fae8c635a8b0137bff7301c4b7fd9aaf0fb1ed802265`. Les pages docs.rs de ces versions étaient inaccessibles dans cette consultation.
- [Rust `sync_channel`](https://doc.rust-lang.org/std/sync/mpsc/fn.sync_channel.html) et [`try_send`](https://doc.rust-lang.org/std/sync/mpsc/struct.SyncSender.html#method.try_send), consultés le 2026-10-06 : capacité par message, `send` bloquant, `try_send` immédiat avec refus si plein. La page courante affiche Rust 1.99.0 ; ces API stables depuis 1.0 ne remplacent pas la toolchain projet 1.98.1.

**Contrôles de fermeture à inscrire dans P03, tous `PRODUCT_VALIDATION / NOT RUN`** : saturation au neuvième effet et soixante-cinquième événement ; dépassement 1 MiB et frame incomplète ; `max_samples=0/80_001`, plage incohérente et PCM vide ; EOF court ; refus d’une génération précédente avant publication ; Stop accessible lorsque data est pleine ou inference bloquée ; vraie UI réactive ; mémoire observée par étage et absence de cumul du fichier entier. Exécution par un acteur distinct autorisé au lot, avec stimulus, branche, effet et témoin attribués.

Résultat technique : **Q-L01-BOUNDS-01 peut recevoir `answered` avec cette contribution comme preuve technique**, après persistance vérifiée. La revue indépendante doit encore statuer sur la compatibilité du paquet complet IPC v2/assemblage avec D19. Le contrôle `check-state`, la revue et les mesures ne sont pas exécutés par cet acteur. Avant handoff suivant, l’hôte doit retourner les références et reçus vérifiés du document DRAFT et de ce paquet brut en TRANSPORT.

<oai-mem-citation>
<citation_entries>
MEMORY.md:21-32|note=[prior product decisions and distinction between prototype evidence and Rust product validation]
</citation_entries>
<rollout_ids>
01a10a6a-418e-7cd2-ab69-162ffe16db34
01a108b7-9f64-7671-bd72-21db5f9b42d6
</rollout_ids>
</oai-mem-citation>