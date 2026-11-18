# ADR S1 — Pont `flutter_rust_bridge` 2.13.0 + backend `native-assets` dans un Cargo workspace

- **Date** : 2026-11-18
- **Statut** : essai terminé — **succès sur Linux et Android (émulateur x86_64)**, Apple non testé
- **Référence** : `ARCHITECTURE-AGENT.md` §3.3 (ADR-03), §3.4 (ADR-04), §5.5, §14 (S1)
- **Prototype** : `spikes/s1-pont/` (jetable, pas la structure finale)

## Question

Le backend `native-assets` de FRB 2.13.0 fonctionne-t-il quand le crate du pont vit dans un
Cargo workspace (répertoire `target` et `Cargo.lock` partagés, cf. issue FRB #2460), avec la
version stable actuelle de Flutter ? Faut-il se replier sur `cargokit` ?

## Versions réellement utilisées

| Outil | Version |
|---|---|
| Flutter | 3.50.0 stable (rév. ab2912035e, 2026-11-09), Dart 3.14.0 |
| Rust | 1.100.0 (épinglé dans `rust-toolchain.toml`), cargo 1.100.0, rustup 1.29.3 |
| `flutter_rust_bridge` / `_codegen` / `_hooks` | 2.13.0 exactement (Cargo, pubspec, binaire) |
| `native_toolchain_rust` (transitif) | 1.1.1 |
| `hooks` (transitif) | 2.2.0 |
| Android | SDK 36.1.0, NDK 28.2.13676358 (`flutter.ndkVersion`), JDK 21, émulateur « Medium Phone API 36.1 » x86_64 |
| Hôte | Ubuntu 26.04.1 LTS, x86_64, clang 21.1.8 |

## Ce qui a été construit

```
spikes/s1-pont/
├── Cargo.toml            # workspace : crates/core, crates/bridge ; lints (unwrap/expect/panic = deny)
├── Cargo.lock            # unique, à la racine du workspace
├── rust-toolchain.toml   # channel = "1.100.0" + cibles Linux et Android
├── crates/core/          # s1_core : delayed_sum (async) + ticks (Stream), aucune dépendance FRB
├── crates/bridge/        # s1_bridge : seul crate dépendant de FRB, crate-type = ["staticlib", "cdylib"]
│   └── rust-toolchain.toml -> ../../rust-toolchain.toml   (lien symbolique, voir « Contournements »)
└── app/                  # app Flutter (linux, android), hook/build.dart, integration_test/s1_test.dart
```

API du pont (`crates/bridge/src/api/demo.rs`) : `delayed_sum(a, b, delay_ms) -> Result<i64, BridgeError>`
(async) et `tick_stream(sink: StreamSink<u32>, count, period_ms) -> Result<(), BridgeError>`.
`BridgeError` est une struct `{ kind: BridgeErrorKind, message }` ; côté Dart elle devient une
classe qui implémente `FrbException`, levée par le `Future`.

## Commandes exactes

```bash
cargo install flutter_rust_bridge_codegen --version 2.13.0 --locked   # 1 min 10 s
cd spikes/s1-pont
flutter create --platforms=linux,android --org dev.atelier.spike --project-name s1_app app
# (workspace + crates/core écrits à la main, crates/bridge absent à ce stade)
cd app
flutter_rust_bridge_codegen integrate --rust-crate-dir ../crates/bridge \
    --rust-crate-name s1_bridge --integration-backend native-assets
# puis : crates/bridge rattaché au workspace (voir Contournements), API remplacée
flutter_rust_bridge_codegen generate
flutter test integration_test/s1_test.dart -d linux
flutter build apk --debug
```

La syntaxe a été vérifiée dans les sources du codegen 2.13.0 (`src/binary/commands.rs`) :
`--integration-backend` accepte `cargokit` (défaut) et `native-assets` ; `--rust-crate-dir`
est relatif au projet Flutter et accepte `../`.

## Résultat

**Linux desktop : succès.** `flutter test integration_test/s1_test.dart -d linux` →
`All tests passed!` (4 tests) :

1. fonction async Rust → `42` ;
2. `Err` Rust → exception Dart `BridgeError` avec `kind == invalidInput` ;
3. flux `StreamSink` → `[0, 1, 2, 3, 4]` puis fermeture du flux ;
4. widget : somme et ticks affichés dans l'UI.

**Le problème du `target` partagé (#2460) ne se produit pas** avec `native-assets` :
`native_toolchain_rust` 1.1.1 lance `rustup run 1.100.0 cargo build --manifest-path
<crate>/Cargo.toml --package s1_bridge --target <triple> --target-dir <dossier>` et lit la
bibliothèque à un chemin qu'il a lui-même fixé. Il retrouve aussi le `Cargo.lock` du workspace
(remontée des dossiers parents) pour invalider le cache. Deux configurations testées, toutes
deux vertes :

- par défaut : `target` privé sous `app/.dart_tool/hooks_runner/shared/s1_app/build/<hash>/target` ;
- `CARGO_TARGET_DIR=<workspace>/target` : le hook écrit dans le `target` du workspace, et un
  `cargo build --release --target x86_64-unknown-linux-gnu -p s1_bridge` lancé à la main est
  ensuite réutilisé tel quel (0,02 s).

Une modification dans `crates/core` (dépendance de chemin, hors du crate du pont) est bien
détectée par le build incrémental : le `.d` de cargo liste les sources des dépendances de
chemin (vérifié en cherchant la chaîne modifiée dans le `.so`).

**Android : succès.** `flutter build apk --debug` → APK construit en **196 s** (premier build
Gradle + trois cibles Rust release à froid). Le hook compile une bibliothèque par ABI, toutes
embarquées : `arm64-v8a` 766 Kio, `armeabi-v7a` 489 Kio, `x86_64` 788 Kio. Aucun `cargo-ndk`
nécessaire. Le test d'intégration a ensuite été exécuté sur l'émulateur
(`flutter test integration_test/s1_test.dart -d emulator-5554`) : **4/4 verts**, 54 s
(Gradle incrémental 34 s + installation 4 s). Non testé sur appareil ARM physique.

**Apple (iOS, macOS) : non testé** — hôte Linux, pas de Xcode. Le critère « un appareil Apple »
du §14 reste ouvert.

### Mesures (Linux x86_64, `flutter test … -d linux`, temps mur)

| Scénario | Temps |
|---|---|
| Installation du codegen (`cargo install`, une fois) | 70 s |
| `integrate` | 21 s |
| `generate` (inclut un `cargo check`/expansion) | 42 s à froid, quelques s ensuite |
| Premier build + test après `flutter clean`, Rust à froid (target privé) | **25,2 s** |
| Premier build + test, `CARGO_TARGET_DIR` du workspace, Rust release à froid | 16,8 s |
| `flutter build apk --debug` à froid (3 ABI) | 196 s |
| Test d'intégration sur émulateur Android (Gradle incrémental) | 54 s |
| Relance sans changement | **3,8 s** |
| Après modification de `crates/core` | **4,5 s** |
| Après modification de `crates/bridge` | **4,4 s** |

Taille de la bibliothèque native Linux (`libs1_bridge.so`, **profil release**, non strippée) :
**1,08 Mio** (1 105 920 octets) ; **718 Kio** après `strip`. La `staticlib` produite à côté
fait 26,9 Mio (non embarquée). À titre de comparaison, `libflutter_linux_gtk.so` fait 41 Mio.

## Constats et contournements

1. **`integrate` génère un crate autonome**, pas un membre de workspace : il écrit
   `crates/bridge/Cargo.lock` et `crates/bridge/rust-toolchain.toml` (canal 1.93.1 et toutes
   les cibles). Correction manuelle : suppression du `Cargo.lock` local, ajout du crate dans
   `members`, `[package]` en `*.workspace = true`, ajout de la dépendance à `s1_core`.
2. **`rust-toolchain.toml` doit être dans le dossier du crate** :
   `native_toolchain_rust` lit `<cratePath>/rust-toolchain.toml` (pas de recherche dans les
   parents) et refuse un canal non concret (`stable`). Contournement : fichier unique à la
   racine du workspace, **lien symbolique** dans `crates/bridge/`. Fonctionne sous Linux ;
   sous Windows, préférer une copie vérifiée par la CI.
3. **Le hook compile toujours en `release`** (défaut de `FlutterRustBridgeNativeAssetsBuilder`,
   `buildMode: BuildMode.release`), même pour `flutter test`/`flutter build --debug`. C'est ce
   qui explique la petite taille du `.so` ; c'est réglable dans `hook/build.dart` si l'on veut
   des symboles de debug.
4. **Le code généré contient des `unwrap`** : avec `clippy::unwrap_used = "deny"` sur le crate,
   la compilation échoue (13 erreurs). Contournement conforme à l'ADR-03 :
   `#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::all)] mod frb_generated;`
   dans `lib.rs`. Le code écrit à la main reste sous les lints stricts.
5. **Les enums Rust à champs exigent `freezed`** côté Dart (`MissingDep: Please add freezed to
   your dev_dependencies`). Contournement : `BridgeError` en struct + enum C-like. À trancher
   pour la vraie API : accepter `freezed` (dépendance de build) ou s'en tenir à ce motif.
6. **`u64` devient `BigInt`** en Dart : ergonomie médiocre pour des durées/compteurs. Préférer
   `u32`/`i64` dans les DTO du pont.
7. **Épingler `1.100.0` installe une seconde toolchain** à côté de `stable` (même version
   pourtant), avec les cibles listées : téléchargement automatique par rustup au premier
   usage. À prévoir dans le cache de la CI.
8. `integrate` remplace `lib/main.dart` (l'original est conservé en commentaire) et ajoute
   `integration_test/simple_test.dart` ; les deux ont été réécrits. Le code généré est stable :
   `generate` relancé produit un `frb_generated.rs` identique (y compris après `cargo fmt`).
9. Le repli `cargokit` **n'a pas été essayé** : `native-assets` n'a pas échoué.

## Non couvert

- **Cibles Apple** (iOS, macOS) : impossible sur cet hôte Linux.
- **Windows** : non testé (lien symbolique du point 2 à revalider).
- **Android sur appareil ARM physique** : seul l'émulateur x86_64 a exécuté le test ; les `.so` ARM sont construits mais pas exécutés.
- Comportement avec plusieurs isolates, arrêt d'isolate (#2560), panics (#3067) : hors périmètre.

## Recommandation

**Garder `native-assets`** (ADR-04 confirmé pour Linux et Android). Le cas
« crate du pont dans un Cargo workspace » fonctionne sans le contournement #2460, car le hook
fixe lui-même `--target-dir`. Règles à reporter dans la structure finale :

- un seul `rust-toolchain.toml` à la racine, lien (ou copie contrôlée par la CI) dans `crates/bridge/` ;
- après `integrate`, rattacher le crate au workspace et supprimer son `Cargo.lock` ;
- `CARGO_TARGET_DIR` pointé vers le `target` du workspace en CI pour partager le cache avec
  `cargo test`/`clippy` (gain mesuré : 25 s → 17 s à froid sur ce prototype minuscule) ;
- `#[allow(...)]` limité au module `frb_generated` ;
- DTO sans `u64` et, sauf décision contraire, sans enums à champs.

Le repli `cargokit` reste documenté (§3.4) mais n'a pas été nécessaire. Le critère de sortie
du §14 n'est **pas entièrement rempli** tant qu'un appareil Apple n'a pas été testé.
