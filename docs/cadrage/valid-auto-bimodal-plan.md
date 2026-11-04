Avancée : main fusionné et poussé à 4f13bb37 ; préalable satisfait.
Blocage : aucun au préflight initial ; espace libre 492 395 466 752 octets.
Décision : exécuter le lot autorisé, sans changement de code ni de configuration.

# VALID-AUTO-BIMODAL — plan du 4 novembre 2026

Objectif : vérifier si le mode lent historique subsiste après d7e6445a et,
s'il apparaît, localiser son coût avec les traces déjà livrées. Le correctif
et l'instrumentation sont ancêtres de main 4f13bb37 ; push origin/main réussi.
Worktree neuf `C:/wt/valid-auto`, branche `valid-auto-bimodal`, depuis ce main.
Demande du lot et « ok push puis continue » autorisent ce périmètre précis.
Début : 18:42 locale ; budget une heure, échéance 19:42 locale ; au plus douze
corps Auto N2, aucune autre recette native. Le plafond spécifique remplace le
budget générique de diagnostic pour cette campagne ; aucun essai supplémentaire.

1. Préparer seulement `target/n2-native/` dans ce worktree : copier les entrées
   épinglées et le cache Cargo isolé du lot précédent, sans écrire dans sa racine.
   Paramètres PersonalAutoFallback identiques, nouveaux chemins locaux uniquement,
   `harnessTrace=1`. Source MP3 24 237 octets, SHA-256
   39048CD1482D7BCCE4052EE8027CF629FCC2F7C6714DCF6419238EE202836D8A.
   Aucun modèle, DLL, worker, profil, garde, banc ou code produit modifié.
2. Préflight par garde native existante et auto-tests de la garde dans cette
   racine ; CARGO_TARGET_DIR partagé canonique inchangé. Pas de Cargo direct,
   réseau, changement de priorité ou configuration machine. Relever les processus
   pertinents ; aucun arrêt autorisé. Les reçus gardent toutes les racines et
   variables canoniques ; les environnements sont isolés par la garde.
3. Lancer PersonalAutoFallback séquentiellement, au plus douze fois. Staging
   neuf à chaque essai, même état et ressources. Arrêter dès un corps >=12 s
   avec traces complètes, ou au budget/quota. Une violation de garde, un test
   en échec ou des traces inexploitables interrompent la suite, sans correction
   ni nouvelle exécution improvisée. Ne pas modifier les fichiers suivis pendant
   les essais. stdout/stderr/reçus et JSONL existants sont conservés.
4. Pour chaque essai, lire corps libtest, Cargo et garde séparément ; compter
   lectures complètes ET inspections/cache par modèle, avec durées lecture/SHA,
   identité, thread et phases. Vérifier unicité de trace/origine monotone,
   paires début/fin et résultats. Les traces existantes n'enregistrent pas de
   pile d'appels : n'attribuer un appelant que si phase/thread/code le prouvent,
   sinon NON DÉTERMINÉ. Aucun nouveau marqueur ni mélange d'horloges.
5. Lire les chemins de catalogue encore susceptibles d'inspecter Turbo ;
   distinguer atteignabilité dans ce banc, catalogue d'affichage et live.
   Si lent localisé, proposer au plus deux corrections sans les appliquer.
   Sinon conclure seulement NON OBSERVÉ SUR N ESSAIS. Pas de moyenne ou médiane
   globale, pas de preuve d'absence générale du mode lent.
6. Confier les conclusions à un reviewer indépendant en lecture seule, deux
   cycles revue/correction documentaire maximum. La session principale rédige
   `docs/architecture/VALID-AUTO-BIMODAL-BILAN.md`, une page figée à la clôture,
   avec tableau des essais et citations fichier:ligne. Signaler seulement la
   provenance DLL spike02, les ignore décalés et le coût SHA Parakeet restant.
7. Archiver les petites preuves hors de la racine supprimable, sous
   `target/valid-auto-bimodal/archive/`, arborescence préservée et comparaison
   nombre/tailles/SHA-256 de chaque entrée aux sources. Exclure runtimes/modèles
   recopiés et cache. Proposer les commandes de nettoyage, ne rien supprimer.
   Préserver caches partagés/persistants et les ressources/proofs historiques,
   notamment target/parakeet-feasibility et target/poll-preserved.
8. Committer localement le bilan et ce plan ; aucun nouveau push ou fusion de
   cette branche de validation implicite dans l'accord de push de main.

Le reviewer indépendant est également chargé de la lecture des chemins Turbo.
Pas de Rust à implémenter, pas de nouveau test à écrire ni campagne N1 à répéter
pour une modification documentaire seule. Toute limite de la trace sera rendue
explicitement ; le but est une observation attribuable, pas un verdict forcé.

## Exécution et clôture

Plan commis avant exécution dans 298e6972. Ressources/cache copiés depuis
C:/wt/diag-banc/target/n2-native sans modification de l'origine ; paramètres
identiques avec chemins locaux. Auto-tests de garde PASS, reçus sous
target/n2-native/logs/guard-selftest-reusable-roots.json. Préflight PASS,
487 782 207 488 octets libres. Aucun Cargo direct ni réseau. Douze Auto
séquentiels PASS, aucun corps >=12 s, quota 12/12 clos à 18:53 environ.
Première compilation 31,20 s, premier Cargo 40,730 s, première garde 56,440 s ;
autres Cargo 9,310–9,575 s et gardes 24,785–25,117 s. Aucun temps de staging
isolé n'est déduit de ces mesures de garde complète.

Identifiants complets, corps Rust et durée de chaque lecture/SHA dans le
[bilan figé](../architecture/VALID-AUTO-BIMODAL-BILAN.md). Les 12 traces produit
ont chacune une seule origine monotone/PID, un seul thread d'inspection, les
phases début/fin complètes et un résultat final. Le plus grand intervalle est
toujours lookup→full_read Parakeet, 7,149–7,378 s, conforme au calcul lecture/SHA.
Chaque lecture : 713 975 456 octets, identité stable après lecture ; aucune
inspection Turbo observée. Deux bind_import, cinq inspections Parakeet par
corps : 1 lecture et 4 hits ; aucun rafraîchissement UI/live dans ce scénario.

### Comptage et appelants vérifiés

`target/n2-native/valid-auto-bimodal/series.jsonl` garde l'analyse initiale.
Son filtre Turbo utilisait par erreur LargeV3Turbo au lieu de WhisperLargeV3Turbo ;
les listes Reads conservaient néanmoins tous les modèles et le critère d'arrêt
ne dépendait que du corps Rust. Aucun effet sur l'exécution ou le quota.
Les comptes faisant foi ont été recalculés depuis les douze JSONL bruts par
groupement de TOUS les model_id, sans filtre présupposé, dans
`verified-series.jsonl` : seul ParakeetTdt06BV3 présent ; 60 inspections,
12 lectures complètes et 48 hits. Recalcul indépendant du reviewer concordant.
Les deux analyses et les traces brutes sont archivées sans réécriture des traces.

Mapping des cinq inspections, identique dans les douze essais ; citations
relatives à `crates/whisper-adapters/src/`, sauf mention explicite :

| Ordre / état | Phase et chemin appelant établi par source synchrone/thread |
|---|---|
| A / cold, lecture complète | premier bind : worker_ipc/backends/mod.rs:124 → model_catalog.rs:358,386 → inspect_file:571 |
| B / hit | native_cpu_route : worker_ipc/backends/native.rs:151 → parakeet_import.rs:143,214 (qualification_key) |
| C / hit | native_cpu_route : worker_ipc/backends/native.rs:152 → parakeet_import.rs:114 (verify_qualification) |
| D / hit | worker_ipc/runtime/import.rs:128 → archive/import.rs:321,325, modèle de la requête Parakeet |
| E / hit | worker_ipc/runtime/import.rs:732 (Bounded start_worker_effect) → second bind identique à A |

Les traces ne portent pas de pile d'appels : attribution par inclusion dans la
phase monotone du même thread et ordre des appels synchrones lu dans le code.
Un autre modèle/thread hors phase aurait été NON DÉTERMINÉ, pas attribué par
simple proximité. Les chemins Turbo encore possibles sont dans le bilan :
snapshot/inspect complet, file, reprise, live, composition UI ; non atteints
par ce banc StartImport direct. Le constructeur crée le catalogue sans l'inspecter.
Le banc demande Parakeet dans tests/parakeet_unified_native.rs:353,397 et
StartImport:410 ; Auto:1267, stratégie Bounded:220. La présence de Turbo y est
contrôlée par is_file:146-155, pas par SHA complet.
Le cache global modèle/artefacts est défini dans model_catalog.rs:16,219-251 ;
path/identité/taille/mtime:85-89 déterminent le hit, lecture:112 et revalidation
d'identité:143-157. La copie fraîche et le nouveau processus imposent un cache
froid initial ; aucune seconde lecture ou invalidation observée dans un corps.
Un catalogue concurrent pourrait faire attendre le mutex global pendant un SHA,
mais aucune telle concurrence n'est observée et le banc ne crée pas l'UI.

Les assertions natives restent inchangées ; douze TXT, douze SRT et douze WAV
ont respectivement une seule empreinte commune par type, identique au lot précédent :
TXT 0EDF3F33E020E532102A9795F978ED2E19EE89B506688FE141E4556881B9D5F5 ;
SRT 53EF0552F2D2117179FF9BE4172A0E99FEE0C85287E86B7DC0BF8C4E2327526C ;
WAV 8C72DE575B6F5E93812A6D5BEA73B7C0C928DCC73F769B64BC325ADF79A2219A.

### Verdict et revue

NON OBSERVÉ SUR 12 ESSAIS, corps 8,45–8,72 s. Aucun lent à comparer, aucune
cause des anciens lents établie et aucune correction spéculative proposée.
Le reviewer indépendant a recalculé les comptes des douze traces brutes, puis
relu le bilan : CLEAN en première revue, aucun constat ouvert, zéro correction.
Sa revue n'inclut pas archivage/nettoyage. Aucun code, garde, profil, machine,
modèle, worker ou DLL modifié ; aucun test N1 supplémentaire nécessaire pour
ces documents seuls. Limites : pas de GPU ni de fenêtre Windows, ni preuve
d'absence générale de mode lent ; lecture et SHA mesurés ensemble.

### Archive et rétention

`C:/wt/valid-auto/target/valid-auto-bimodal/archive/VALID-AUTO-BIMODAL-20261104.zip` :
445 entrées, 3 152 789 octets source, 1 856 909 octets ZIP.
Nombre, taille et SHA-256 de chaque entrée comparés aux sources : PASS.
SHA-256 ZIP : `C4C5F7BB70D4CD33C0D2DE8BA9D5DB3A8E2D8C8A9BBD444BF9FCD91019C8BCD1`.
Contenu : reçus et logs de garde, demandes d'exécution, traces complètes banc/
produit, sorties publiées et analyses initiale/vérifiée. Exclusion des caches,
temporaires et copies de runtime/modèles ; les sources/documents restent dans Git.

À 18:54:22 : target/n2-native = 42 203 260 487 octets apparents, deux jonctions
internes vérifiées (fixtures et INetCache). VCTIP 103004 créé à
18:47:34.925099 subsiste ; son usage des racines n'est pas exclu. Aucun arrêt
de processus autorisé ni exécuté. Aucun Cargo/rustc/Git actif relevé.
Nettoyage proposé uniquement à l'utilisateur, suspendu tant que cette instance
et tout processus/lot utilisant cette racine restent actifs. Aucun effacement
exécuté ; 449 578 602 496 octets libres relevés à 18:53:32.

Après vérification humaine de l'absence d'usage, commande PowerShell proposée
à l'utilisateur ; elle ne tue aucun processus, vérifie la racine exacte,
les redirections et l'archive avant toute suppression :

```powershell
$ErrorActionPreference = 'Stop'
if (Get-Process -Name vctip,cargo,rustc -ErrorAction SilentlyContinue) {
    throw 'Processus résiduels : vérifier identité et usage ; aucune suppression.'
}
$validCleanupRoot = 'C:\wt\valid-auto\target\n2-native'
if ((Resolve-Path -LiteralPath $validCleanupRoot).Path.TrimEnd('\') -ine $validCleanupRoot) {
    throw 'Racine inattendue.'
}
$validAncestor = $validCleanupRoot
while ($validAncestor) {
    if ((Get-Item -LiteralPath $validAncestor -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) {
        throw "Racine ou parent redirigé : $validAncestor"
    }
    $validParent = Split-Path -Parent $validAncestor
    if (-not $validParent -or $validParent -eq $validAncestor) { break }
    $validAncestor = $validParent
}
foreach ($validLink in Get-ChildItem -LiteralPath $validCleanupRoot -Recurse -Force -Attributes ReparsePoint) {
    $validTargets = @($validLink.Target)
    if (-not $validTargets.Count) { throw 'Lien sans cible vérifiable.' }
    foreach ($validTarget in $validTargets) {
        if (-not [IO.Path]::IsPathRooted($validTarget) -or
            -not ([IO.Path]::GetFullPath($validTarget)).StartsWith($validCleanupRoot + '\', [StringComparison]::OrdinalIgnoreCase)) {
            throw "Lien hors racine : $($validLink.FullName)"
        }
    }
}
$validArchive = 'C:\wt\valid-auto\target\valid-auto-bimodal\archive\VALID-AUTO-BIMODAL-20261104.zip'
if ((Get-FileHash -LiteralPath $validArchive -Algorithm SHA256).Hash -cne
    'C4C5F7BB70D4CD33C0D2DE8BA9D5DB3A8E2D8C8A9BBD444BF9FCD91019C8BCD1') {
    throw 'Archive absente ou différente : aucune suppression.'
}
Remove-Item -LiteralPath $validCleanupRoot -Recurse -Force
Get-PSDrive C | Select-Object Free
```

Cette racine contient seulement les copies/cache isolé propres au lot. Les
entrées canoniques du lot précédent ne sont pas supprimées ; cache Cargo
partagé, caches persistants, archives et target/parakeet-feasibility ainsi que
target/poll-preserved restent préservés. L'archive est hors de la racine supprimable.
Toute suppression ou validation après clôture sera un fait nouveau à documenter
séparément ; le bilan figé ne sera pas réécrit.
Syntaxe de la commande utilisateur contrôlée par le parseur PowerShell : PASS,
sans exécution ; git diff --check PASS. Clôture avant 19:42, dans le budget.
Seuls ce plan et le bilan sont modifiés sur la branche de validation ; main et
origin/main restent alignés sur 4f13bb37. Aucun nouveau push/fusion de cette branche.
