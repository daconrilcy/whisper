# ARCH-L04-CHANGE-v3 — apport brut DRAFT

Auteur : `/root/d23_arch_delta` ; rôle : `rust_architect`, lecture seule ; date : 2026-10-07. Owner technique : `AUTH-ARCH-L04-CHANGE`. Aucune écriture ni exécution. Cette contribution remplace mes propositions v1/v2.

Base : **D-WHISPER-23 DRAFT**, digest `cd90c9360820705405dfa870e160c14f2ab273b7f55851a551c878902fe277bb`, héritage D22/P28/TECH-D18 et Q04/Q06/Q09. Aucun READY/CLEAN n’est revendiqué.

## Alignement produit

Réponses acceptées explicitement transmises par le coordinateur, à relier aux sources humaines exactes : Auto GPU→CPU : la capture live continue avec le même handle, sous les bornes établies. GPU forcé défaillant : couper le microphone, drainer le PCM conservé et attendre. Le choix humain CPU termine **le même passage**, sans rouvrir le microphone. Un fallback ne constitue pas Stop+Reprise et ne crée aucun passage supplémentaire.

## Identités et checkpoint

Conserver `(job_id, storage_generation)` pour passage, fichiers, queue et archive. Distinguer `(job_id, attempt_generation, instance_id)` pour moteur, événements et Stop. L’application possède la décision et la tentative courante ; l’adaptateur exécute.

Réserver durablement une tentative monotone avant lancement, avec addition contrôlée et journal technique versionné dans l’archive. Préserver les formats historiques ; suffixe incomplet ignoré, aucun lancement automatique au redémarrage.

Sur panne explicite corrélée, fermer l’admission des vieux résultats, traiter les persistences déjà acceptées puis délivrer leurs ACK. Ensuite seulement produire le checkpoint autoritatif : dernier segment confirmé, offset local PCM, préfixe audio durable vérifié, modèle/configuration et identité source import. Une réception/soumission ne vaut jamais confirmation. Convertir explicitement offsets groupe/local.

Attendre `Stopped` corrélé ou sortie effectivement constatée de l’ancien enfant avant CPU. Un enfant bloqué reste diagnostiqué ; aucun kill ou délai maximal native promis.

## Reprise, IPC, archive et bornes

Auto conserve capture, convertisseur, VAD, PCM writer et compteurs. Les capacités proviennent des contrats L03/P28 et des constantes effectives recapturées ; inventorier capacité, payload/wire, allocations et backlog par étage. Aucun nouveau seuil de durée, mémoire ou disque n’est inventé. Atteinte effective d’une borne, refus d’admission ou erreur IO déclenche cessation concernée, perte non confirmée mesurée et Recoverable ; aucune croissance ou perte silencieuse.

GPU forcé n’admet aucun CPU automatique. Après clic CPU, utiliser uniquement le PCM scellé ; si aucun audio durable existe, signaler cette absence sans faux Complete.

Versionner explicitement l’IPC et livrer parent/CPU/GPU ensemble. Commandes de reprise : checkpoint, identité de tentative, séquences/plages bornées existantes et mode `ContinueCapture` ou `FinishPreservedPassage`.

Reconstruire le MP3 pending depuis PCM durable : préfixe confirmé pour encodage seul, suffixe pour encodage/inférence. Chaque échantillon est encodé une fois ; texte confirmé inchangé, nouveaux segments après son dernier numéro. Ne concaténer aucun flux encodeur défaillant.

Archive valide tentative, séquence et couverture PCM avant persistance/ACK. Reconstruction limitée au pending du même passage ; PCM, fragments confirmés et anciennes publications préservés. Publication : encodeur fini, fichiers synchronisés/vérifiés, manifeste/pointeur dernier. Coupure ⇒ réconciliation idempotente et Recoverable si incohérence ; aucune transaction multi-fichiers supposée.

Import Auto garde source intacte et identité durable de queue/archive ; revérifier hash/durée puis reprendre au checkpoint. Aucun import Interrupted restauré ne redémarre automatiquement. Stop/Quit latched avant reprise interdit lancement. Après CPU lancé, arrêt corrélé courant ; UI reste ouverte si confirmation manque.

Supervision : obligations indépendantes capture/persistance/inférence/finalisation/arrêt, progrès réel uniquement, état/backend attesté. Valeurs Q04 conservées ; aucun timer ne déclenche fallback, Stop, kill ou publication. Dégradation/pertes diagnostiques visibles selon Q09.

## Portée exacte proposée : 21 chemins

```text
Cargo.lock
crates/whisper-adapters/src/lib.rs
crates/whisper-adapters/src/archive.rs
crates/whisper-adapters/src/supervisor.rs
crates/whisper-adapters/src/worker_ipc.rs
crates/whisper-adapters/tests/live_archive.rs
crates/whisper-adapters/tests/worker_control.rs
crates/whisper-core/src/application.rs
crates/whisper-core/src/compute_policy.rs
crates/whisper-core/src/ipc.rs
crates/whisper-core/src/lib.rs
crates/whisper-core/src/ports.rs
crates/whisper-core/tests/compute_policy.rs
crates/whisper-core/tests/live_contract.rs
crates/whisper-desktop/src/root.rs
crates/whisper-desktop/src/ui.rs
crates/whisper-worker-cpu/src/main.rs
crates/whisper-worker-cpu/src/native_engine.rs
crates/whisper-worker-gpu/Cargo.toml
crates/whisper-worker-gpu/src/main.rs
crates/whisper-worker-gpu/src/native_engine.rs
```

D19 demeure DESIGN_FEASIBILITY limitée ; reprise produit, MP3 réel, saturation, événements obsolètes et vraie UI restent PRODUCT_VALIDATION NOT RUN. Package homogène et cible native restent DELIVERY_QUALIFICATION NOT RUN. Revue successor DESIGN, PLANS, autorisation scoped et préflight courant requis avant code.
