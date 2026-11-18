# ADR S5 — Fiabilité des flux `StreamSink` de FRB 2.13.0 dans le navigateur

- **Date** : 2026-11-18
- **Statut** : essai terminé — **critère de réussite NON atteint** : sur le web en mode threadé, on observe des pertes (intermittentes) et des blocages (systématiques dans un cas d'usage courant). Le repli prévu par l'étude fonctionne.
- **Référence** : `ARCHITECTURE-AGENT.md` §3.5 (ADR-05), §5.4, §14 (S5)
- **Prototype** : `spikes/s5-flux/` (jetable, copié depuis `spikes/s1-pont/`)

## Question

Les flux Rust → Dart de `flutter_rust_bridge` 2.13.0 (`StreamSink`), validés en natif par S1,
sont-ils fiables dans le navigateur : pas de perte, pas de désordre, pas de blocage, arrêt
propre de l'émetteur Rust à l'annulation, comportement sain au rechargement ? Faut-il le repli
« requête ponctuelle + notification de changement » prévu au §5.4 ?

## Réponse courte

**Non, pas tels quels.** Les rafales simples (1 000 et 10 000 émissions) passent, mais :

1. **Un consommateur qui fait `await` dans un `await for` bloque le flux, 100 % du temps**, sur
   Chrome et Firefox, dans les deux modes web. En natif, le même code passe. Cause établie
   (voir D1) : FRB écoute un `BroadcastChannel` via un abonnement DOM, et mettre cet
   abonnement en pause retire l'écouteur : les messages, dont celui de fermeture, sont perdus.
2. **Pertes intermittentes au début des flux émis depuis un worker** (Chrome seulement) :
   4 fois en 35 passes complètes, jusqu'à 170 événements manquants sur 10 000. Rust a bien
   tout émis. C'est le défaut que l'amont a mis en quarantaine (PR #3458, #3460).
3. **L'annulation côté Dart n'arrête jamais l'émetteur Rust sur le web** (fuite systématique) ;
   en natif, `sink.add` échoue et l'émetteur s'arrête.
4. **Deux onglets de la même app reçoivent les événements l'un de l'autre** (noms de canal
   déterministes partagés par l'origine) ; idem entre l'ancienne et la nouvelle instance au
   redémarrage à chaud, et brièvement au rechargement de page sur Chrome.

Le **repli** (requête ponctuelle + flux de numéros de version, désabonnement explicite) passe
partout, et ses pertes éventuelles de notifications sont sans effet sur le résultat. Il est
**nécessaire**, complété d'un petit correctif de FRB Dart (prototypé ici) ou d'une discipline
d'usage stricte (voir « Recommandation »).

## Versions réellement utilisées

| Outil | Version |
|---|---|
| Flutter / Dart | 3.50.0 stable (rév. ab2912035e) / 3.14.0 ; build web **dart2js** (`flutter build web --release`, sans `--wasm`) |
| `flutter_rust_bridge` (Cargo, pub) / `_codegen` | 2.13.0 exactement |
| Rust natif | 1.100.0 (`rust-toolchain.toml`) |
| Rust web | **`nightly-2026-11-15`** (rustc 1.102.0-nightly 5f4effd57 2026-11-13), composant `rust-src`, cible `wasm32-unknown-unknown` |
| wasm-pack | 0.15.1 (télécharge `wasm-bindgen` 0.2.130 selon `Cargo.lock`, et `wasm-opt`) |
| wasm-bindgen / wasm-bindgen-futures | 0.2.130 / 0.4.80 |
| futures-timer / gloo-timers | 3.0.4 (fonctionnalité `wasm-bindgen` en WASM) / 0.4.1 |
| Navigateurs | Google Chrome 157.0.8103.78 (système, piloté par Playwright `channel: 'chrome'`) ; Firefox 159.0 (**build Playwright**, pas le Firefox du système) |
| Playwright / Node | 1.65.1 / 26.13.1 |
| Hôte | Ubuntu 26.04.1 LTS, x86_64 |

Le Firefox du système est un snap qui ne démarre pas dans le bac à sable de cette session
(`snap-confine … cap_dac_override not found`) : le Firefox de Playwright (moteur Gecko
standard, quelques correctifs d'automatisation) a été utilisé à la place.

## Ce qui a été construit

```
spikes/s5-flux/
├── crates/core/                 # s5_core : sleep_ms portable (futures-timer, setTimeout en WASM)
├── crates/bridge/src/api/
│   ├── flux.rs                  # émetteurs instrumentés + registre (stream_info, stop_stream)
│   └── repli.rs                 # repli : repli_get (ponctuel) + repli_watch (versions) + repli_unwatch
├── app/
│   ├── lib/scenarios.dart       # scénarios partagés web / natif, résultats en JSON
│   ├── lib/main.dart            # ?auto=1 : série ; ?live=1 : flux infinis (rechargements)
│   ├── integration_test/s5_test.dart   # référence native
│   ├── flutter_rust_bridge.yaml             # mode threadé (code généré committé)
│   └── flutter_rust_bridge.sans-entetes.yaml # default_dart_async: false
├── scripts/build_web.sh         # generate + build-web + flutter build web, par mode
├── scripts/frb-stream-sink-sans-pause.patch # correctif expérimental de FRB Dart 2.13.0
├── harness/                     # serveur statique (COOP/COEP on/off), run.mjs, hot.mjs (Playwright)
└── resultats/                   # JSON et journaux bruts des exécutions
```

Émetteurs Rust testés : `burst_async` (fonction `async` : sur le web elle tourne **sur le fil
principal**, `wasm_bindgen_futures::spawn_local`), `burst_sync` (fonction normale : **dans le
pool de Web Workers** en mode threadé, synchrone sur le fil principal sans en-têtes),
`paced_async` / `paced_worker` (cadencés, infinis si `count == 0`), `error_stream`. Chaque
émetteur s'arrête dès que `sink.add` échoue ou que Dart appelle `stop_stream(id)`, et tient à
jour un registre (`running`, `emitted`, raison d'arrêt) lu par Dart.

## Commandes exactes

```bash
# Outils
rustup toolchain install nightly-2026-11-15 --profile minimal \
    --component rust-src --target wasm32-unknown-unknown
cargo install wasm-pack --version 0.15.1 --locked
cd spikes/s5-flux/harness && npm install && npx playwright install firefox

# Construction web (scripts/build_web.sh threade | sans-entetes), depuis app/ :
flutter_rust_bridge_codegen generate --config-file flutter_rust_bridge.yaml   # ou .sans-entetes.yaml
flutter_rust_bridge_codegen build-web --dart-root . --rust-root ../crates/bridge --release \
    --wasm-pack-rustup-toolchain nightly-2026-11-15
flutter build web --release --no-web-resources-cdn [--dart-define=S5_SANS_ENTETES=true] \
    --output ../build/web-<mode>

# build-web lance en réalité (vérifié dans sa sortie) :
#   RUSTUP_TOOLCHAIN=nightly-2026-11-15 RUSTFLAGS='-C target-feature=+atomics,+bulk-memory,+mutable-globals
#     -C link-args=--shared-memory -C link-args=--max-memory=1073741824 -C link-args=--import-memory
#     -C link-args=--export=__heap_base -C link-args=--export=__wasm_init_tls -C link-args=--export=__tls_size
#     -C link-args=--export=__tls_align -C link-args=--export=__tls_base'
#   wasm-pack build -t no-modules -d app/web/pkg --no-typescript --out-name s5_bridge ../crates/bridge \
#     -- -Z build-std=std,panic_abort

# Tests
(cd app && flutter test integration_test/s5_test.dart -d linux)        # natif
S5_REPEAT=10 node harness/run.mjs chrome threade                       # COOP/COEP servis
S5_REPEAT=10 node harness/run.mjs firefox threade
S5_REPEAT=5  node harness/run.mjs chrome sans-entetes                  # aucun en-tête
node harness/run.mjs chrome threade off                                # build threadé SANS en-têtes
node harness/hot.mjs web    # flutter run -d web-server --web-header=Cross-Origin-Opener-Policy=same-origin
                            #   --web-header=Cross-Origin-Embedder-Policy=require-corp, puis r et R
node harness/hot.mjs linux
```

Points vérifiés dans les sources des versions épinglées (pas seulement dans la doc) :
`build-web` est implémenté côté Dart (`flutter_rust_bridge/lib/src/cli/build_web/executor.dart`) ;
`--wasm-pack-rustup-toolchain` remplace `RUSTUP_TOOLCHAIN` (sinon `nightly` non épinglé) et
**ignore `rust-toolchain.toml`**. Le serveur de test sert `COOP: same-origin` et
`COEP: require-corp` (le choix de l'ADR-05). Durées : `build-web` 35 s à froid, puis
`flutter build web` 32 à 44 s. Tailles : `s5_bridge_bg.wasm` 145 Kio (release, `wasm-opt`),
colle JS 44 Kio, `main.dart.js` 2,18 Mio.

## Protocole

- **Série automatique** (`?auto=1`), répétée dans des pages neuves (5 ou 10 passes) : chaque
  scénario a un délai maximal (blocage = échec) et vérifie l'identifiant de flux, l'ordre et
  l'exhaustivité de chaque événement.
- **Annulation** : abonnement à un émetteur infini (5 ms), annulation après 20 événements,
  observation du registre Rust pendant 1 s. « Explicite » : l'abonnement passe par
  `cancellable()`, qui appelle `stop_stream(id)` dans `onCancel`.
- **Rechargement de page** : page `?live=1` (flux infinis), `page.reload()`, comptage des
  événements étrangers (identifiant d'un autre flux) et du désordre ; puis rechargement au
  milieu de la série automatique, qui doit se relancer et finir.
- **Deux onglets** de la même origine en `?live=1`.
- **Rechargement à chaud** : `flutter run` (debug, DDC), `r` après modification d'une fonction,
  puis `R`, avec les mêmes compteurs.

## Résultats

Légende : nombre de passes réussies / passes ; ✅ tout passe, ❌ échec systématique, ⚠️ échec
intermittent, — non applicable. Toutes les colonnes web sont en dart2js release.

| Scénario | Natif Linux (5) | Chrome threadé (10) | Firefox threadé (10) | Chrome sans en-têtes (5) | Firefox sans en-têtes (5) |
|---|---|---|---|---|---|
| Rafale 1 000, `async` (fil principal) | ✅ 5/5 | ✅ 10/10 (≈ 50 ms) | ✅ 10/10 | ✅ 5/5 | ✅ 5/5 |
| Rafale 1 000, worker | ✅ 5/5 | ⚠️ 10/10 ici ; **9/10** dans une autre série | ✅ 10/10 | ✅ 5/5 (synchrone) | ✅ 5/5 |
| Rafale 10 000, `async` | ✅ 5/5 (≈ 350 ms) | ✅ 10/10 (≈ 250 ms) | ✅ 10/10 | ✅ 5/5 | ✅ 5/5 |
| Rafale 10 000, worker | ✅ 5/5 | ⚠️ 10/10 ici ; **9/10** ailleurs (9 830/10 000, les 170 premiers manquants) | ✅ 10/10 | ✅ 5/5 | ✅ 5/5 |
| 4 flux simultanés (2 `async`, 2 worker) | ✅ 5/5 | ⚠️ 10/10 ici ; **4/5** ailleurs (197/200) ; 20/20 isolé | ✅ 10/10 | ✅ 5/5 (4 `async`) | ✅ 5/5 |
| Consommateur lent (`await for` + `await`) | ✅ 5/5 | ❌ **0/10, blocage** | ❌ **0/10, blocage** | ❌ 0/5, blocage | ❌ 0/5, blocage |
| Annulation sans arrêt explicite | ✅ arrêt Rust (`sendError`), 0 émission après | ❌ **fuite** : ≈ 190 émissions/s continuent | ❌ fuite | ❌ fuite | ❌ fuite |
| Annulation avec `stop_stream` (`onCancel`) | ✅ | ✅ 10/10 (⚠️ une perte de `seq 0` dans une autre série) | ✅ 10/10 | ✅ (`async` seul) | ✅ (`async` seul) |
| Réabonnement après annulation | ✅ | ✅ 10/10 (l'ancien émetteur fuit) | ✅ | ✅ | ✅ |
| Erreur `sink.add_error` puis suite | ❌ `0,1,2,E` : le flux se **ferme** après l'erreur | ❌ idem | ❌ idem | ❌ idem | ❌ idem |
| `Err` renvoyé par la fonction de flux | ❌ flux fermé « normalement », erreur **non interceptée** hors du flux | ❌ idem | ❌ idem | ❌ idem | ❌ idem |
| Repli (ponctuel + versions), 1 000 écritures | ✅ | ✅ 10/10 (une passe : 907/1 001 notifications reçues, résultat exact) | ✅ 10/10 | ✅ | ✅ |
| Rechargement de page pendant des flux | — | ⚠️ flux repartis, mais **≈ 90 événements étrangers** juste après | ✅ 0 étranger | ✅ | ✅ |
| Rechargement au milieu de la série | — | ✅ série relancée et complète | ✅ | ✅ | ✅ |
| Deux onglets | — | ❌ **≈ 290 événements étrangers** par onglet | ❌ ≈ 290 | ❌ ≈ 140 | ❌ ≈ 145 |
| Rechargement à chaud (`r`) | ✅ code v2, flux continus | ✅ code v2, flux continus, 0 étranger | non testé | non testé | non testé |
| Redémarrage à chaud (`R`) | ✅ anciens émetteurs arrêtés, 0 étranger | ❌ anciens émetteurs **continuent** (4 en cours au lieu de 2), **435 étrangers**, `Identifier 'wasm_bindgen' has already been declared` | non testé | non testé | non testé |

« Ici / ailleurs » : la colonne Chrome threadé agrège 4 séries (5 + 10 + 10 + 10 passes
complètes, plus 20 passes isolées de `simultanes_4`). Sur l'ensemble, **4 pertes en début de
flux worker** (`seq 0` ou un préfixe manquant ; Rust a émis la totalité) ; aucune perte sur
un flux `async`, aucune sur Firefox. `spikes/s5-flux/resultats/` ne garde que la dernière
exécution de chaque configuration (pour Chrome threadé : la série de 10 passes sans perte) ;
les pertes des séries antérieures sont reprises ici depuis leur sortie.

**Build threadé servi sans en-têtes** (Chrome et Firefox) : panique Rust à la première fonction
non `async` (`fail to create WorkerPool: DataCloneError … SharedArrayBuffer transfer requires
self.crossOriginIsolated`), puis plus rien ne répond (série bloquée). Le build
`default_dart_async: false`, lui, se charge sans en-têtes sur Chrome et Firefox
(`crossOriginIsolated=false`), malgré une mémoire WASM partagée.

### Variante « FRB Dart corrigé » (expérience)

Avec `scripts/frb-stream-sink-sans-pause.patch` appliqué à une copie de
`flutter_rust_bridge` 2.13.0 (Dart) via `pubspec_overrides.yaml` :

| Scénario | Chrome threadé corrigé (15) | Firefox threadé corrigé (5) |
|---|---|---|
| Rafales, simultanés | ✅ sauf **1 perte en début de flux worker** (808/1 000) sur 15 passes | ✅ 5/5 |
| Consommateur lent | ✅ 15/15 | ✅ 5/5 |
| `add_error` puis suite | ✅ `0,1,2,E,3,4` | ✅ `0,1,2,E,3,4` |
| Deux onglets, rechargement de page | ✅ 0 étranger | ✅ 0 étranger |
| Redémarrage à chaud | ✅ 0 étranger ; ❌ anciens émetteurs toujours en cours | non testé |
| Annulation sans `stop_stream`, `Err` renvoyé | ❌ inchangés (hors du périmètre du correctif) | ❌ |

Le correctif supprime les défauts D1 et D4, pas D2 ni D3. Il change aussi la sémantique de
`add_error` sur le web (le flux continue après l'erreur), qui diverge alors du natif (où le
flux se ferme) : à aligner en appliquant le même code aux deux plateformes si on le retient.

## Défauts constatés

**D1 — Pause = perte, et blocage si le message de fermeture est perdu (web).**
`RustStreamSink._setup` (FRB Dart, `lib/src/stream/stream_sink.dart`) lit le port avec
`await for` dans un générateur `async*`, puis `listenAndBuffer()`. Sur le web, le port est un
`BroadcastChannel` écouté par `EventStreamProvider` de `package:web` 1.1.2, dont
`pause()` appelle `removeEventListener` : un message arrivé pendant la pause est **jeté**. Or
toute contre-pression Dart (consommateur `await for` qui attend, `pause()` explicite) remonte
jusqu'à cet abonnement. En natif, `ReceivePort` met en tampon. Correctif prototypé : écouter le
port sans jamais le mettre en pause et tamponner dans un `StreamController`
(`scripts/frb-stream-sink-sans-pause.patch`, 30 lignes). Pas d'issue amont trouvée sur cette
cause précise ; l'amont a mis en quarantaine des tests de flux web intermittents
(PR [#3458](https://github.com/fzyzcjy/flutter_rust_bridge/pull/3458),
[#3460](https://github.com/fzyzcjy/flutter_rust_bridge/pull/3460) : `[2,3,4]` reçu au lieu de
`[0,1,2,3,4]`), et un correctif plus large a été fermé sans fusion le 2026-11-14
([#3435](https://github.com/fzyzcjy/flutter_rust_bridge/pull/3435)). La doc dit seulement que
les flux Rust → Dart « can be flaky on Web » (guides/types/translatable/stream).

**D2 — Perte au début des flux émis depuis un worker (Chrome, intermittent).** Avec le
correctif D1, une perte de préfixe subsiste (1 fois en 15 passes). Le worker poste dans un
`BroadcastChannel` créé par le fil principal juste avant l'appel ; tout se passe comme si
l'abonnement de ce récepteur n'était pas encore effectif côté navigateur pour les premiers
messages. Non observé sur Firefox, ni pour les émetteurs `async` (fil principal). Amont :
reproduction [#3356](https://github.com/fzyzcjy/flutter_rust_bridge/pull/3356) (« cross-worker
ordering loss »), correctif #3435 non fusionné. **Non corrigé ici.**

**D3 — Pas de signal d'annulation vers Rust sur le web.** `BroadcastChannel.postMessage` ne
peut pas échouer quand le récepteur est fermé : `sink.add` renvoie toujours `Ok`, l'émetteur
tourne jusqu'à la fermeture de la page (≈ 190 émissions/s dans le test). En natif,
`sink.add` échoue et l'émetteur s'arrête. Amont : demandes de `onCancel` / jeton d'annulation
pour `StreamSink` fermées sans suite
([#2917](https://github.com/fzyzcjy/flutter_rust_bridge/issues/2917),
[#2978](https://github.com/fzyzcjy/flutter_rust_bridge/issues/2978),
[#2339](https://github.com/fzyzcjy/flutter_rust_bridge/issues/2339)). Contournement validé :
désabonnement explicite (`stop_stream` / `repli_unwatch` appelé depuis `onCancel`).

**D4 — Noms de canal déterministes partagés par l'origine.** Les noms sont
`__frb_streamsink_RustStreamSink_<n>` avec un compteur qui part de 0 dans chaque contexte
Dart (`lib/src/utils/port_generator.dart`). Deux onglets, ou l'instance redémarrée à chaud et
les émetteurs de la précédente (D3), écrivent donc sur les mêmes canaux : chaque onglet reçoit
les données de l'autre, y compris un éventuel message de fermeture. Dans une vraie app, un
onglet afficherait le modèle de lecture de l'autre. Correctif prototypé : préfixe aléatoire
par contexte dans le même patch. Pas d'issue amont trouvée.

**D5 — Erreurs dans les flux (natif et web).** `sink.add_error` est toujours décodé côté Dart
en `AnyhowException` (une chaîne) : passer `BridgeError` donne une exception au contenu
illisible ; il faut encoder le code dans la chaîne. Le flux se **ferme** après la première
erreur (le générateur `async*` sort sur l'exception). Une fonction de flux qui renvoie `Err`
ferme le flux normalement et l'erreur part en **erreur asynchrone non interceptée**
(`unawaited(handler.executeNormal(...))` dans le code généré). Amont : erreurs typées dans les
flux demandées puis fermées ([#2756](https://github.com/fzyzcjy/flutter_rust_bridge/issues/2756)).
Voir aussi [#3473](https://github.com/fzyzcjy/flutter_rust_bridge/issues/3473) (ouverte) :
`cancel()` d'un flux généré inactif ne se termine jamais.

**D6 — Redémarrage à chaud sur le web.** `RustLib.init()` ré-injecte `pkg/s5_bridge.js`
(`Identifier 'wasm_bindgen' has already been declared`), l'instance WASM précédente reste en
place et ses émetteurs continuent (D3). Gêne de développement seulement ; le rechargement à
chaud (`r`) est sain.

**Constats annexes (non imputables aux flux).**

- En WASM threadé, un `std::sync::Mutex` **contendu** sur le fil principal appelle
  `memory.atomic.wait` et lève `Atomics.wait cannot be called in this context` (vu au premier
  essai, avec un registre partagé entre fil principal et worker). Contournement : `try_lock`
  en boucle pour de courtes sections critiques, ou pas d'état partagé bloquant.
- `futures-timer` 3 exige la fonctionnalité `wasm-bindgen` en WASM (sinon il démarre un fil).
- La colle `wasm-bindgen` 0.2.130 avertit « using deprecated parameters for the initialization
  function » à chaque worker : FRB 2.13 appelle l'initialisation avec l'ancienne signature.
  Sans effet observé.
- `flutter build web --no-web-resources-cdn` a été utilisé pour auto-héberger CanvasKit sous
  `COEP: require-corp` (conforme à l'ADR-05) ; le CDN n'a pas été essayé.

## Non couvert

- **Safari** (aucun moteur WebKit disponible sous Linux dans cette session) : les comportements
  de `BroadcastChannel` et `require-corp` y restent à mesurer, ce qui tombe dans S3.
- Le **Firefox du système** (snap) : remplacé par le Firefox de Playwright.
- Le rechargement et le redémarrage à chaud sur Firefox et en mode sans en-têtes.
- Le mode dart2wasm (`--wasm`) : hors périmètre tant que FRB stable est en 2.13.
- FRB 2.14.0-beta.2 : non essayé ; aucun correctif de flux web n'y a été fusionné d'après
  l'historique amont consulté (#3435 fermé sans fusion).
- Appareils mobiles (Chrome Android, Safari iOS).

## Recommandation

**Les flux FRB 2.13 ne sont pas utilisables tels quels sur le web.** Le critère du §14
(« aucune perte, aucun blocage ») échoue en mode threadé. Le repli de l'étude est
**nécessaire** et fonctionne ; on l'adopte comme **forme unique** des modèles de lecture, sur
toutes les cibles (un seul chemin à tester) :

1. **Requête ponctuelle, source de vérité** : `get_notes(query) -> NoteListViewDto` portant un
   numéro de `version`.
2. **Flux de notifications sans données** : `watch_notes_changes(watch_id, sink: StreamSink<u32>)`
   qui ne transporte que des numéros de version. Une notification perdue ou fusionnée est
   sans effet ; Dart relit l'instantané quand la version reçue dépasse la sienne.
3. **Désabonnement explicite** : `unwatch(watch_id)` appelé depuis `onCancel` (D3) ; Rust
   retire l'observateur. Jamais d'émetteur infini qui compte sur l'échec de `sink.add`.
4. **Resynchronisation** pour couvrir la perte de la *dernière* notification (D2) : relire à
   l'abonnement, au retour au premier plan (`visibilitychange`) et à faible fréquence quand la
   vue est visible. Le flux reste une accélération, pas une garantie.
5. **Consommer sans pause** : `listen` avec une fonction synchrone qui planifie la relecture,
   jamais `await for` avec des `await` dans la boucle sur un flux FRB (D1).
6. **Émettre depuis le fil principal sur le web** : notifier depuis des fonctions `async` ou
   `#[frb(sync)]`, pas depuis un worker (D2 n'a été observé que pour les workers).
7. **Erreurs hors du flux** : les fonctions de flux renvoient toujours `Ok(())` ; les erreurs
   passent par la requête ponctuelle (typée) ou, si besoin, comme donnée d'un DTO.

En complément, **appliquer le correctif D1 + D4** (`scripts/frb-stream-sink-sans-pause.patch`)
par une copie interne de `flutter_rust_bridge` Dart 2.13.0 référencée en
`dependency_overrides`, et proposer les deux changements à l'amont. C'est une décision de
l'utilisateur (maintenir une copie corrigée d'une dépendance) ; sans elle, les règles 3 à 6
ci-dessus suffisent pour le repli, mais **deux onglets de l'app se contamineront** (D4), ce qui
n'est acceptable que si l'on interdit les onglets multiples (détection par
`BroadcastChannel`/Web Locks, en lien avec S2).

Le mode sans en-têtes ne change rien aux défauts des flux : il ne justifie pas, à lui seul,
de renoncer à COOP/COEP. Il interdit en revanche tout travail Rust long (pas de worker).

## Corrections à apporter au §5.4 de l'étude

- Remplacer l'exemple `watch_notes(query, sink: StreamSink<NoteListViewDto>)` par la paire
  `get_notes(query) -> NoteListViewDto` (avec `version`) +
  `watch_notes_changes(watch_id, sink: StreamSink<u32>)` + `unwatch(watch_id)`. La requête
  ponctuelle passe de « DEVRAIT … en repli » à **DOIT, source de vérité**.
- Remplacer « la doc les signale potentiellement instables sur le web `[V]` » par les défauts
  mesurés `[E]` (S5) : pertes intermittentes en début de flux worker (Chrome), blocage dès que
  le consommateur met le flux en pause, aucune annulation propagée vers Rust, canaux partagés
  entre onglets.
- Ajouter : « Une fonction de flux renvoie toujours `Ok(())` ; `add_error` n'accepte qu'une
  chaîne (`AnyhowException`) et ferme le flux ; un `Err` renvoyé échappe au flux. » `[E]`
- Ajouter : « Tout flux a un désabonnement explicite appelé depuis `onCancel` ; aucun émetteur
  ne dépend de l'échec de `sink.add` pour s'arrêter. » `[E]`
- Ajouter : « Côté Dart, ne jamais consommer un flux FRB par `await for` avec des `await` dans
  la boucle, ni le mettre en pause. » `[E]`
- Ajouter : « En WASM threadé, pas de `Mutex` bloquant partagé entre fil principal et workers
  (`Atomics.wait` interdit sur le fil principal). » `[E]`
- Conserver « chaque flux DOIT avoir un test d'intégration web », en précisant : répété (au
  moins 10 passes), sur Chrome et Firefox, avec un consommateur lent et deux onglets.
- Au §14, ligne S5 : critère non atteint ; repli adopté (voir cette ADR). Au §3.5 : épingler
  la nightly par `--wasm-pack-rustup-toolchain nightly-AAAA-MM-JJ` (sinon `nightly` flottante,
  et `rust-toolchain.toml` est ignoré par `build-web`).
