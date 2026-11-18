# ADR S2 — Cœur Rust en WASM et SQLite sur OPFS derrière `flutter_rust_bridge` 2.13.0

- **Date** : 2026-11-18
- **Statut** : essai terminé — **succès sur Chrome 157 et Firefox 159 (Linux)**, Safari et navigateurs mobiles non testés
- **Référence** : `ARCHITECTURE-AGENT.md` §3.5 (ADR-05), §3.7 (ADR-07), §5.4, §14 (S2)
- **Prototype** : `spikes/s2-wasm/` (jetable, copié de `spikes/s1-pont/`)
- **Recommandation** : **plan A**, sous quatre conditions décrites plus bas

## Question

Le cœur Rust, compilé en WASM et appelé depuis Flutter Web par FRB 2.13.0, peut-il persister
des données dans SQLite sur OPFS (`rusqlite` + `sqlite-wasm-rs` + VFS `sahpool`) ? Quel plan
de l'ADR-07 retenir : A (même adaptateur partout), B (web sans stockage durable) ou C
(stockage fourni par Dart) ?

## Versions réellement utilisées

| Outil | Version |
|---|---|
| Flutter | 3.50.0 stable, Dart 3.14.0 ; app web construite en **dart2js** (`flutter build web`) |
| Rust natif | 1.100.0 (épinglé, inchangé depuis S1) |
| Rust web | **`nightly-2026-11-15`** (`rustc 1.102.0-nightly 5f4effd57 2026-11-13`), composant `rust-src`, cible `wasm32-unknown-unknown` |
| `flutter_rust_bridge` / `_codegen` | 2.13.0 |
| `wasm-pack` | 0.15.1 (télécharge lui-même le `wasm-bindgen-cli` assorti et `wasm-opt` au premier build) |
| `wasm-bindgen` / `js-sys` / `web-sys` | 0.2.130 / 0.3.107 / 0.3.107 |
| `rusqlite` | 0.43.0 (features `bundled` + défauts, dont `ffi-sqlite-wasm-rs`) |
| `sqlite-wasm-rs` | 0.6.3 (SQLite 3.54.1, feature `wasm-bindgen`) |
| `sqlite-wasm-vfs` | 0.3.0 (feature `sahpool`), `rsqlite-vfs` 0.2.0 |
| `clang` | 21.1.8 (compile SQLite pour `wasm32`) |
| Navigateurs | Google Chrome 157.0.8103.78, Mozilla Firefox 159.0.1, tous deux headless |
| Hôte | Ubuntu 26.04, x86_64, 12 cœurs, Node 26.13.1 |

## Ce qui a été construit

```
spikes/s2-wasm/
├── .cargo/config.toml     # CFLAGS_wasm32_unknown_unknown = "-matomics -mbulk-memory"
├── crates/core/src/store.rs      # port NoteStore + adaptateur rusqlite ; web : Web Locks + sahpool
├── crates/bridge/src/api/store.rs  # acteur propriétaire de la connexion, dans un Worker dédié
├── app/lib/main.dart      # scénario piloté par l'URL, résultat dans globalThis.s2Result
├── app/integration_test/s2_test.dart   # le même scénario en natif Linux
└── e2e/                   # serve.mjs (COOP/COEP), cdp.mjs (Chrome), webdriver.mjs (Firefox), run.mjs
```

Le code SQL (`CREATE TABLE`, `INSERT`, `SELECT`, transaction) est **le même** en natif et sur le
web : un seul `SqliteNoteStore` derrière le trait `NoteStore`. Les seules parties propres au web
sont la préparation du VFS (module `web` du cœur, une soixantaine de lignes) et le lancement du
fil de stockage (une quarantaine de lignes dans le pont).

## Commandes exactes

```bash
# Outils (une fois)
rustup toolchain install nightly-2026-11-15 --profile minimal \
    --component rust-src --target wasm32-unknown-unknown
cargo install wasm-pack --locked                         # 36 s

# Build
cd spikes/s2-wasm/app
flutter create --platforms=web --org dev.atelier.spike --project-name s2_app .
flutter_rust_bridge_codegen generate
flutter_rust_bridge_codegen build-web --rust-root ../crates/bridge \
    --wasm-pack-rustup-toolchain nightly-2026-11-15 --release      # -> web/pkg/
flutter build web --no-wasm-dry-run                                # -> build/web/

# Preuve
cd ../e2e && node run.mjs && node run.mjs --firefox
cd ../app && flutter test integration_test/s2_test.dart -d linux

# Développement
flutter run -d chrome \
    --web-header=Cross-Origin-Opener-Policy=same-origin \
    --web-header=Cross-Origin-Embedder-Policy=require-corp
```

`build-web` lance en réalité (lu dans `flutter_rust_bridge-2.13.0/lib/src/cli/build_web/executor.dart`) :
`wasm-pack build -t no-modules -d web/pkg --no-typescript --out-name s2_bridge ../crates/bridge
-- -Z build-std=std,panic_abort`, avec `RUSTUP_TOOLCHAIN` (défaut `nightly`, surchargé ici par
la date épinglée) et `RUSTFLAGS=-C target-feature=+atomics,+bulk-memory,+mutable-globals
-C link-args=--shared-memory --max-memory=1073741824 --import-memory` plus les exports TLS.
`RUSTUP_TOOLCHAIN` prime sur le `rust-toolchain.toml` du workspace : le natif reste en 1.100.0.

## Résultat, étape par étape

### Étape 1 — fonction async sans stockage : succès

`delayed_sum` et `tick_stream` de S1 répondent dans Chrome (`42`, puis `0,1,2,3,4`), page servie
avec `COOP: same-origin` et `COEP: require-corp`. Deux corrections ont été nécessaires :

1. **`i64`/`u64` dans les DTO cassent la compilation Dart du web.** FRB traduit `i64` en `int`
   en natif mais en `BigInt` sur le web : le code Dart de S1 (`delayedSum(a: 40, …)`) ne compile
   plus (`The argument type 'int' can't be assigned to the parameter type 'BigInt'`). Les DTO du
   pont sont passés en `i32`/`u32`.
2. **`futures-timer` exige son feature `wasm-bindgen`** sur `wasm32-unknown-unknown` (ajouté en
   dépendance conditionnelle dans le cœur).

Le hook `native-assets` de S1 ne gêne pas `flutter build web`.

### Étape 2 — `rusqlite` en mémoire dans le build threadé : succès, avec un réglage

- Sans réglage, l'édition de liens échoue :
  `rust-lld: error: --shared-memory is disallowed by …-sqlite3.o because it was not compiled
  with 'atomics' or 'bulk-memory' features.`
- Correction (documentée dans le README de `sqlite-wasm-rs`) :
  `CFLAGS_wasm32_unknown_unknown="-matomics -mbulk-memory"`. Placée dans
  `.cargo/config.toml` (`[env]`), elle s'applique sans rien passer à `build-web` et n'a aucun
  effet en natif.
- À l'exécution : table créée, ligne insérée et relue. Le même test passe en natif
  (`cargo test -p s2_core` et test d'intégration Linux).

**`SQLITE_THREADSAFE=0` dans un build `+atomics`** : aucun problème observé, à une condition qui
structure toute la solution — **tous les appels SQLite partent d'un seul Worker**. Les `malloc`
de SQLite passent par l'allocateur Rust, qui est verrouillé en build threadé. Vérifié sous
charge : 2 × 5 000 insertions pendant que 16 tâches allouent dans les autres Workers et que
16 appels async tournent sur le fil principal ; comptes exacts, relus après rechargement, sur
5 exécutions consécutives par navigateur.

### Étape 3 — persistance OPFS avec `sahpool` : succès

**Où s'exécute le code Rust** (mesuré par `globalThis.constructor.name`) :

| Forme de la fonction du pont | Contexte sur le web |
|---|---|
| `async fn` | **fil principal** (`Window`), via `wasm_bindgen_futures::spawn_local` |
| `fn` non async (mode normal) | un Worker du pool FRB (`DedicatedWorkerGlobalScope`) |
| `#[frb(sync)]` | fil principal, synchrone |

`sahpool` exige un Worker dédié (`createSyncAccessHandle` n'existe pas dans `Window`) et son
installation est asynchrone. Ni une `async fn` (mauvais fil) ni une `fn` simple (pas d'`await`)
ne conviennent directement. L'assemblage qui fonctionne est un **acteur** :

1. au premier appel, le pont envoie dans un Worker du pool (`flutter_rust_bridge::spawn_blocking_with`
   sur `FLUTTER_RUST_BRIDGE_HANDLER.thread_pool()`) une closure qui fait
   `wasm_bindgen_futures::spawn_local(acteur)` ; le futur vit ensuite dans la boucle
   d'événements de ce Worker ;
2. l'acteur prend le verrou d'onglet, installe `sahpool` comme VFS par défaut, ouvre l'unique
   `rusqlite::Connection` et sert des commandes reçues par `futures_channel::mpsc` ;
3. les `async fn` du pont (fil principal) envoient une commande et attendent la réponse par
   `oneshot`. Le réveil entre fils fonctionne (`wasm-bindgen-futures` en build `+atomics`).

Le même acteur sert en natif (un `std::thread` + `block_on`) : un seul chemin de code.

Résultat : note écrite, relue après navigation, **relue après rechargement de page**, relue par
un autre onglet après fermeture du premier (donc sans fermeture propre de la connexion).

**Le mode sans en-têtes n'a pas été retenu et n'a pas besoin de l'être**, le mode threadé
fonctionnant. Observation utile tout de même : le `.wasm` threadé servi sans COOP/COEP plante dès
`RustLib.init()` (`fail to create WorkerPool: DataCloneError … SharedArrayBuffer transfer
requires self.crossOriginIsolated`), en exception non typée. Le mode sans en-têtes de FRB
exécute tout sur le fil principal, où `sahpool` ne peut pas exister : **le plan A impose donc
COOP/COEP**, bien que `opfs-sahpool` lui-même ne les demande pas.

### Étape 5 — second onglet : succès

- **Échec brut** (sans verrou) : pas de plantage, `install` renvoie une erreur
  `OpfsSAHError::Opfs` portant `NoModificationAllowedError` (« Access Handles cannot be created
  if there is another open Access Handle… » sur Chrome, « No modification allowed » sur Firefox).
  Le message diffère selon le navigateur : inutilisable comme critère.
- **Détection propre** : avant d'installer `sahpool`, l'acteur demande le verrou Web Locks
  `navigator.locks.request(nom, { ifAvailable: true }, …)` depuis le Worker et le garde par une
  promesse jamais résolue. S'il n'est pas disponible : `StoreError::AlreadyOpenElsewhere` →
  `BridgeErrorKind::StoreOpenInAnotherTab` → l'UI affiche « déjà ouvert dans un autre onglet ».
- Le premier onglet reste intact. À sa fermeture (ou à son rechargement), le verrou et les
  poignées OPFS sont libérés : le second onglet, rechargé, prend la main et relit les données.
  Aucun faux positif sur rechargement dans l'ensemble des exécutions (étape 6).

### Étape 6 — automatisation

`e2e/run.mjs` enchaîne 11 vérifications, sans dépendance npm : Chrome par le protocole DevTools,
Firefox par `geckodriver`. Au fil des versions du script, **Chrome : 18 exécutions réussies
sur 18 ; Firefox : 9 sur 9**, dont 5 consécutives chacun dans la version finale. `flutter run -d web-server` (compilateur de
développement DDC) avec les deux `--web-header` a aussi été vérifié une fois dans Chrome.

Particularité de l'hôte : le Firefox du snap ne démarre pas depuis l'environnement de l'agent
(`snap-confine … cannot continue`). Le binaire a été lancé directement depuis
`/snap/firefox/current/usr/lib/firefox/` (`firefox` et `geckodriver`).

### Étape 7 — plan C : non prototypé

Le plan A réussit ; le rappel Rust→Dart du plan C n'a pas été écrit. Il reste la voie de repli
si Safari échoue (voir « Non couvert »).

## Obstacles et contournements

| # | Obstacle | Contournement | Fragilité |
|---|---|---|---|
| 1 | `i64`/`u64` → `BigInt` sur le web seulement | DTO en `i32`/`u32`/`f64`/`String` | Aucune ; règle à écrire |
| 2 | Édition de liens : SQLite C non compilé en mémoire partagée | `CFLAGS_wasm32_unknown_unknown` dans `.cargo/config.toml` | Faible ; documenté en amont |
| 3 | `async fn` du pont sur le fil principal, OPFS interdit | Acteur dans un Worker, commandes par canal | Moyenne : patron à respecter partout |
| 4 | **Le Worker de l'acteur retourne dans le pool FRB**, qui est une pile : la prochaine `fn` non async du pont s'y exécute et gèle SQLite | `reserve_current_worker()` masque un `postMessage` pour que le pool ne récupère jamais ce Worker | **Élevée : dépend d'un détail interne de FRB 2.13.0** |
| 5 | Registre de VFS de SQLite global à la mémoire partagée, mais état de `sahpool` (poignées JS) local au Worker | Un seul fil de stockage par page, jamais recréé ; `store_open` rouvre dans le même acteur | Moyenne : une seconde installation depuis un autre Worker n'a pas été testée et est à proscrire |
| 6 | `#[wasm_bindgen(inline_js)]` indisponible avec `--target no-modules` | Fonction JS construite par `js_sys::Function::new_with_args` | Faible |
| 7 | Collision entre deux `rustup toolchain install` simultanés (session S5) : `could not rename 'component' file … Directory not empty` | Attendre la fin de l'autre installation | Sans objet en usage normal ; ne pas paralléliser en CI |

Mesure de l'obstacle 4 (5 000 insertions OPFS en une transaction) :

| Situation | Chrome | Firefox |
|---|---|---|
| Seules | 18 à 26 ms | 10 à 19 ms |
| Pool occupé, **sans** réservation du Worker | **11 532 ms** | **11 909 ms** |
| Pool occupé, **avec** réservation du Worker | 109 ms | 384 ms |

Constat annexe : les 16 tâches d'allocation intensive mettent environ 11 s sur le web contre
moins d'une seconde en natif (allocateur partagé verrouillé). Les calculs lourds en `fn` non
async sont à surveiller sur le web, indépendamment du stockage.

## Mesures

| Mesure | Valeur |
|---|---|
| `build-web` à froid, sans SQLite (std reconstruit, premier téléchargement de `wasm-bindgen`) | 27,7 s |
| `build-web` à froid, avec SQLite | **29,7 s** (115 s CPU) |
| `build-web` sans changement / après modification du cœur | 2,0 s / 2,9 s |
| `flutter build web` (dart2js) | 19 à 27 s |
| `.wasm` sans SQLite (release, `wasm-opt`) | 110 Kio (112 915 octets) |
| `.wasm` avec SQLite + `sahpool` | **1,62 Mio** (1 693 904 octets) ; **507 Kio en brotli**, environ 667 Kio en gzip |
| Glu `s2_bridge.js` | 52 Kio |
| `main.dart.js` | 1,90 Mio ; 446 Kio en brotli |
| Workers créés au démarrage | 12 (`navigator.hardwareConcurrency`) ; le pool en recrée un au besoin quand le Worker de stockage lui est retiré |

## Non couvert

- **Safari (macOS, iOS)** : aucun moyen de test sur cet hôte. C'est le risque principal restant :
  `createSyncAccessHandle` et Web Locks y existent, mais ni le build threadé de FRB ni `sahpool`
  n'y ont été exécutés. Le critère de sortie du §14 n'est donc **pas entièrement rempli**.
- Chrome et Firefox sur Android ; navigation privée (OPFS y est restreint ou absent selon le
  navigateur) ; quota et éviction du stockage (`navigator.storage.persist()`).
- Montée de version du schéma, plusieurs bases, tailles réalistes, durabilité après arrêt brutal
  du navigateur en cours de transaction.
- Sortie Dart WebAssembly (`--wasm`), hors périmètre avec FRB 2.13.0.
- Service worker et mise en cache hors ligne de l'app elle-même.
- Fiabilité des flux sur le web : essai S5.

## Recommandation

**Retenir le plan A** : même adaptateur `rusqlite` sur toutes les cibles, VFS `sahpool` sur le
web, verrou d'onglet unique par Web Locks. Le critère de réussite est rempli sur Chrome et
Firefox, de façon répétable, y compris sous charge concurrente.

- **Plan B écarté** : le hors-ligne web fonctionne, il n'y a pas de raison d'y renoncer.
- **Plan C écarté pour l'instant** : il ajouterait un second adaptateur et du trafic sur le pont
  pour un problème qui ne s'est pas présenté. Il reste le repli désigné si Safari échoue.

Conditions du plan A :

1. **Valider Safari** (macOS et iOS) avec `e2e/run.mjs` adapté avant tout engagement public.
   En cas d'échec : plan C pour Safari seulement, ou pour tout le web.
2. **Encapsuler l'acteur de stockage** dans le crate adaptateur web, et interdire tout appel
   SQLite hors de lui. Une règle de revue ou un lint doit garantir qu'aucune `Connection` ne
   sort du Worker.
3. **Remplacer le contournement n° 4** par un Worker réellement dédié : soit demander en amont
   à FRB une API pour sortir un Worker du pool, soit créer ce Worker soi-même. Tant que ce n'est
   pas fait, épingler FRB en version exacte et garder le test de charge dans la CI.
4. **Figer la chaîne de build web** : date de nightly, `wasm-pack`, `CFLAGS` dans
   `.cargo/config.toml`, et cache CI des binaires que `wasm-pack` télécharge.

## Corrections à apporter au document d'architecture

**§3.5 (ADR-05)**

- La commande réelle est `wasm-pack build -t no-modules … -- -Z build-std=std,panic_abort`, la
  toolchain se choisit par `--wasm-pack-rustup-toolchain` (défaut : `nightly`). Passer ce fait
  de `[P]` à `[V]` et **épingler une date de nightly**.
- Ajouter : `flutter build web` signale un « wasm dry run » en avertissement ;
  `--no-wasm-dry-run` le supprime.
- Ajouter : sur le web, `async fn` s'exécute sur le fil principal, `fn` non async dans un Worker
  du pool, dont la taille est `navigator.hardwareConcurrency` `[V]`.
- Ajouter : servi sans COOP/COEP, le build threadé plante à `RustLib.init()` par une exception
  non typée. L'app doit tester `crossOriginIsolated` avant d'initialiser le pont et afficher
  un message clair.
- Ajouter une règle de DTO : pas de `i64`/`u64` dans l'API du pont (types Dart différents entre
  natif et web). Elle renforce le constat n° 6 de l'ADR S1.
- Remplacer « mode sans en-têtes possible mais déconseillé » par « incompatible avec le plan A
  de l'ADR-07 ».

**§3.7 (ADR-07)**

- Lever les deux `[NV]` : `sahpool` fonctionne dans un Worker du pool FRB, et SQLite
  `THREADSAFE=0` fonctionne dans un build `+atomics`, **à condition que tous les appels SQLite
  viennent d'un seul Worker** et que SQLite soit compilé avec
  `CFLAGS_wasm32_unknown_unknown="-matomics -mbulk-memory"`.
- Corriger « `opfs-sahpool` ne demande pas COOP/COEP » : vrai du VFS seul, mais l'assemblage
  avec FRB les exige.
- Préciser « un second onglet échoue à l'initialisation » : l'échec est une erreur Rust
  récupérable (`NoModificationAllowedError`), au message variable selon le navigateur ; la
  détection fiable est le verrou Web Locks pris **avant** l'installation du VFS, depuis le Worker.
- Décision : remplacer « trois plans, départagés par l'essai S2 » par « **plan A retenu**, sous
  réserve de Safari ; plan C en repli ; plan B abandonné ».
- Ajouter le patron imposé : acteur propriétaire de la connexion dans un Worker dédié, un seul
  par page, commandes par canal ; même patron en natif.
- Ajouter le poids : environ 1,6 Mio de `.wasm` (0,5 Mio en brotli) pour SQLite et son VFS.

**§14** : critère de S2 rempli pour Chrome et Firefox, ouvert pour Safari.
