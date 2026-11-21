# Mise en place du dépôt — guide pour agent

> **Destinataire** : un agent de code chargé de créer le squelette du projet dans un dépôt vide.
> **But** : obtenir, par les commandes des CLI de chaque technologie, un monorepo qui compile, passe ses linters et prouve de bout en bout qu'une application Flutter appelle le cœur Rust.
> **Référence d'architecture** : `docs/architecture/ARCHITECTURE-AGENT.md` (les « § » ci-dessous y renvoient).
> **Version pour développeur** : `mise-en-place.html`, même dossier.

## 0. Mode d'emploi

### 0.1 Niveaux de preuve des commandes

| Marqueur | Signification |
|---|---|
| `[T]` | Commande rejouée à blanc le 2026-11-21 dans un dossier vide, sous Linux, avec les versions du §1. Le résultat observé est indiqué. |
| `[E]` | Commande validée par un essai du dépôt (`spikes/`, ADR dans `docs/architecture/adr/`). |
| `[NT]` | Non testée ici. Vérifie-la dans la documentation de l'outil avant de l'exécuter. |

### 0.2 Règles d'exécution

- Exécute les étapes dans l'ordre. Chaque étape se termine par une **vérification** : ne passe pas à la suivante si elle échoue.
- Toutes les commandes partent de la racine du dépôt, sauf mention `cd`.
- Remplace les deux variables partout :

```bash
PROJET=atelier          # nom court, en minuscules, sans tiret
ORG=dev.atelier         # identifiant inversé pour Android, iOS, macOS
```

- Ne modifie jamais un fichier généré (`frb_generated.*`, `*.g.dart`, `*.freezed.dart`, `*.config.dart`, `*_test.dart` issus des `.feature`).
- Les versions sont épinglées exactement. Ne les monte pas : plusieurs combinaisons plus récentes ne se résolvent pas (§20.2).
- Cibles non couvertes par ce guide parce que non testables sous Linux : iOS, macOS, Windows. Les dossiers de plateforme sont créés, pas construits.

---

## 1. Prérequis

### 1.1 Versions de référence

| Outil | Version | Preuve |
|---|---|---|
| Flutter / Dart | 3.50.1 / 3.14.0 | `[T]` |
| Rust stable | 1.100.0 | `[T]` |
| Rust nightly (build web seulement) | `nightly-2026-11-15` | `[E]` S2, `[T]` |
| `flutter_rust_bridge_codegen` | 2.13.0 | `[T]` |
| `wasm-pack` | 0.15.1 | `[T]` |
| Node / npm | 26.13.1 / 12.2.3 | `[E]` S6 |

### 1.2 Installation des outils

```bash
# Rust : toolchains et cibles
rustup toolchain install 1.100.0 --component rustfmt --component clippy          # [T]
rustup toolchain install nightly-2026-11-15 --profile minimal \
    --component rust-src --target wasm32-unknown-unknown                         # [E] S2

# Pont et web
cargo install flutter_rust_bridge_codegen --version 2.13.0 --locked              # [E] S1, environ 1 min
cargo install wasm-pack --version 0.15.1 --locked                                # [E] S5

# Outils de qualité Rust
cargo install cargo-nextest --locked                                             # [NT]
cargo install cargo-deny --locked                                                # [NT]
cargo install cargo-insta --locked                                               # [NT]
cargo install cargo-llvm-cov --locked                                            # [NT]
```

Ne lance jamais deux `rustup toolchain install` en parallèle : collision observée (`[E]` S2).

### 1.3 Vérification

```bash
flutter --version && cargo --version && flutter_rust_bridge_codegen --version && wasm-pack --version
flutter doctor
```

Attendu : Flutter 3.50.1, cargo 1.100.0, codegen 2.13.0, wasm-pack 0.15.1.

---

## 2. Dépôt et arborescence

```bash
git init -b main .                                                               # [T]
mkdir -p crates/adapters packages features design/tokens tool docs/architecture/adr .github/workflows
```

`.gitignore` à la racine :

```gitignore
# Rust
/target/
# Dart et Flutter
.dart_tool/
build/
# Pont : sortie du build web
packages/app/web/pkg/
# Node (jetons de design)
node_modules/
# IDE
.idea/
*.iml
```

`Cargo.lock` et `pubspec.lock` sont **versionnés**.

---

## 3. Workspace Rust

### 3.1 Fichiers racine

`Cargo.toml` — la liste `members` part vide : `cargo new` l'alimente tout seul `[T]`.

```toml
[workspace]
resolver = "3"
members = []

[workspace.package]
edition = "2024"
version = "0.1.0"
publish = false

[workspace.lints.rust]
unsafe_code = "deny"
unexpected_cfgs = { level = "warn", check-cfg = ['cfg(frb_expand)'] }

[workspace.lints.clippy]
all = { level = "deny", priority = -1 }
pedantic = { level = "warn", priority = -1 }
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
```

`rust-toolchain.toml` — version concrète obligatoire (`stable` est refusé par le hook du pont, `[E]` S1) :

```toml
[toolchain]
channel = "1.100.0"
components = ["rustfmt", "clippy"]
targets = [
  "x86_64-unknown-linux-gnu",
  "aarch64-linux-android",
  "armv7-linux-androideabi",
  "x86_64-linux-android",
  "wasm32-unknown-unknown",
]
```

Ajoute les cibles Apple et Windows quand ces plateformes sont construites.

**Piège `[T]`** : n'utilise pas `members = ["crates/*", "crates/adapters/*"]`. Le motif `crates/*` attrape le dossier `crates/adapters`, qui n'a pas de manifeste, et toute commande cargo échoue ; l'exclure par `exclude` retire aussi les crates qu'il contient.

### 3.2 Création des crates

```bash
for c in domain application sync-protocol core test-support; do                  # [T]
  cargo new --lib --vcs none --name "$PROJET-$c" "crates/$c"
done
cargo new --lib --vcs none --name "$PROJET-storage-sqlite" crates/adapters/storage-sqlite   # [T]
cargo new --lib --vcs none --name "$PROJET-sync-http"      crates/adapters/sync-http        # [NT] même forme
cargo new --bin --vcs none --name "$PROJET-tui"            crates/tui                       # [T]
```

Observé `[T]` : chaque crate est ajouté à `members`, et son `Cargo.toml` hérite déjà de `edition`, `version`, `publish` et `[lints] workspace = true`.

### 3.3 Remplacer le code du gabarit

Le `lib.rs` généré (`pub fn add…`) échoue sous `clippy::pedantic` (`must_use_candidate`) `[T]`. Remplace-le dans chaque bibliothèque :

```bash
for d in crates/domain crates/application crates/sync-protocol crates/core \
         crates/test-support crates/adapters/storage-sqlite crates/adapters/sync-http; do
  echo "//! $(basename "$d")." > "$d/src/lib.rs"                                 # [T]
done
```

### 3.4 Dépendances

```bash
cargo add -p "$PROJET-application"    --path crates/domain                       # [T]
cargo add -p "$PROJET-core"           --path crates/application                  # [T]
cargo add -p "$PROJET-storage-sqlite" --path crates/application                  # [T]
cargo add -p "$PROJET-storage-sqlite" rusqlite@0.43.0 --features bundled         # [T]
cargo add -p "$PROJET-tui"            --path crates/core                         # [T]
cargo add -p "$PROJET-tui"            ratatui@0.30.2 crossterm@0.29.0            # [T]
cargo add -p "$PROJET-tui" --dev      insta@1.49.0                               # [T]
cargo add -p "$PROJET-application" --dev cucumber@0.23.0                         # [T]
cargo add -p "$PROJET-application" --dev tokio --features macros,rt-multi-thread # [T]
cargo add -p "$PROJET-application" --dev --path crates/test-support              # [T]
```

Règles de dépendance à respecter (§5.1) : `domain` ne dépend de rien du workspace ; `application` ne dépend que de `domain` ; aucun crate hors `bridge` ne dépend de `flutter_rust_bridge` ; `tui` ne dépend jamais de `bridge` ; `test-support` n'apparaît qu'en `--dev`.

### 3.5 Vérification

```bash
cargo fmt --all --check                                                          # [T] sans sortie
cargo clippy --workspace --all-targets -- -D warnings                            # [T] Finished
cargo test --workspace                                                           # [T] ok
```

Attendu `[T]` : cargo signale `unused dependency` pour les dépendances pas encore utilisées. Ce sont des avertissements de manifeste, qui ne font pas échouer `-D warnings` ; ils disparaissent quand le code les utilise.

---

## 4. Tests de comportement côté Rust

Dans `crates/application/Cargo.toml`, ajoute la cible de test sans harnais (`[E]` S4) :

```toml
[[test]]
name = "bdd"
harness = false
```

- Place les `.feature` dans `features/` à la racine, une `Fonctionnalité` par fichier, en français (`# language: fr`).
- Écris `crates/application/tests/bdd.rs` sur le modèle de `spikes/s4-gherkin/core/tests/bdd.rs`.
- Le lanceur DOIT se terminer par `run_and_exit` ou `filter_run_and_exit`, avec `fail_on_skipped()` : avec `.run()`, le processus rend 0 même si des scénarios échouent (`[E]` S4).

```bash
cargo test -p "$PROJET-application" --test bdd                                   # [E] S4
```

Dialecte Gherkin partagé avec Flutter : §8.3. Les points qui cassent le plus souvent : chaînes des tables de données entre guillemets, pas de forme optionnelle `note(s)`, un mot isolé n'est pas un paramètre.

---

## 5. Espace de travail Dart et paquets Flutter

### 5.1 Création des paquets

```bash
flutter create --template=package --project-name "${PROJET}_design_system" packages/design_system   # [T] 13 fichiers
flutter create --template=package --project-name "${PROJET}_core_bridge"   packages/core_bridge     # [T] 13 fichiers
flutter create --empty --platforms=android,ios,linux,macos,windows,web \
    --org "$ORG" --project-name "${PROJET}_app" packages/app                                         # [T] 130 fichiers
flutter create --empty --platforms=linux,web --project-name "${PROJET}_widgetbook" packages/widgetbook  # [T] 25 fichiers
```

Observé `[T]` : le gabarit de l'application importe déjà `package:material_ui/material_ui.dart` et déclare `material_ui: ^1.1.0`. Les dossiers `ios/` et `macos/` sont créés sous Linux.

### 5.2 Espace de travail pub

`pubspec.yaml` à la racine :

```yaml
name: atelier_workspace          # ${PROJET}_workspace
publish_to: none
environment:
  sdk: ^3.14.0
workspace:
  - packages/app
  - packages/core_bridge
  - packages/design_system
  - packages/widgetbook
```

Dans le `pubspec.yaml` de chaque paquet : ajoute `resolution: workspace` juste avant `environment:`, ajoute `publish_to: none` s'il manque, retire la ligne `homepage:` vide, et supprime le `pubspec.lock` local.

```bash
for p in app core_bridge design_system widgetbook; do                            # [T]
  sed -i 's/^environment:/resolution: workspace\nenvironment:/' "packages/$p/pubspec.yaml"
  rm -f "packages/$p/pubspec.lock"
done
sed -i '/^homepage:/d' packages/design_system/pubspec.yaml packages/core_bridge/pubspec.yaml
flutter pub get                                                                  # [T] un seul pubspec.lock, à la racine
dart pub add --dev melos                                                         # [T] melos 8.10.1
dart run melos list                                                              # [T] liste les 4 paquets
```

### 5.3 Vérification

`dart run melos list` affiche les quatre paquets. Il n'existe qu'un `pubspec.lock`, à la racine.

---

## 6. Le pont Rust ↔ Dart

Le pont vit dans son propre paquet, `packages/core_bridge`, et son crate dans `crates/bridge`. Cette disposition a été rejouée à blanc jusqu'au test d'intégration `[T]`.

### 6.1 Intégration

```bash
cd packages/core_bridge
flutter_rust_bridge_codegen integrate \
    --rust-crate-dir ../../crates/bridge \
    --rust-crate-name "${PROJET}_bridge" \
    --integration-backend native-assets                                          # [T] 6 s
cd ../..
```

La commande crée `crates/bridge/` (crate autonome), `packages/core_bridge/hook/build.dart`, `flutter_rust_bridge.yaml`, `lib/src/rust/`, et ajoute `flutter_rust_bridge: 2.13.0` et `flutter_rust_bridge_hooks: 2.13.0`.

### 6.2 Corrections obligatoires après `integrate`

`integrate` produit un crate isolé et du code de démonstration. Applique toutes ces corrections `[T]` :

1. **Workspace** : supprime `crates/bridge/Cargo.lock` ; ajoute `"crates/bridge"` à `members` dans le `Cargo.toml` racine.
2. **Toolchain** : supprime `crates/bridge/rust-toolchain.toml` (canal 1.93.1, toutes les cibles) et remplace-le par un lien vers celui de la racine. Le hook lit ce fichier dans le dossier du crate, sans remonter (`[E]` S1).

```bash
rm crates/bridge/Cargo.lock crates/bridge/rust-toolchain.toml
ln -s ../../rust-toolchain.toml crates/bridge/rust-toolchain.toml
```

Sous Windows, remplace le lien par une copie vérifiée en CI.

3. **Manifeste** `crates/bridge/Cargo.toml` :

```toml
[package]
name = "atelier_bridge"          # ${PROJET}_bridge
edition.workspace = true
version.workspace = true
publish.workspace = true

[lib]
crate-type = ["cdylib", "staticlib"]

[dependencies]
flutter_rust_bridge = "=2.13.0"

[lints]
workspace = true
```

4. **Lints et formatage du code généré** dans `crates/bridge/src/lib.rs`. Le code généré contient des `unwrap` et de l'`unsafe`, et il est formaté selon l'édition 2021 alors que le workspace est en 2024 : sans `#[rustfmt::skip]`, `cargo fmt --check` échoue `[T]`.

```rust
//! Pont `flutter_rust_bridge` : seul crate du workspace qui en dépend.

pub mod api;

// Code généré : exempté des lints stricts, qui ne visent que le code écrit à la main.
#[allow(
    unsafe_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::all,
    clippy::pedantic
)]
#[rustfmt::skip]
mod frb_generated;
```

5. **Documentation** : `clippy::pedantic` exige une doc sur le code de démonstration de `crates/bridge/src/api/` et des accents graves autour des identifiants (`doc_markdown`) `[T]`. Ajoute `//!` en tête de `mod.rs` et `simple.rs`, `///` et `#[must_use]` sur `greet`.
6. **Dépendance au cœur** : `cargo add -p "${PROJET}_bridge" --path crates/core` `[T]`.
7. **Paquet Dart** : supprime `packages/core_bridge/lib/main.dart`, `test_driver/`, `integration_test/` et le test du gabarit ; retire `material_ui: any` de son `pubspec.yaml` ; remplace `lib/${PROJET}_core_bridge.dart` par :

```dart
/// Liaisons Dart du cœur Rust, générées par flutter_rust_bridge.
library;

export 'src/rust/api/simple.dart';
export 'src/rust/frb_generated.dart' show RustLib;
```

### 6.3 Génération et branchement

```bash
(cd packages/core_bridge && flutter_rust_bridge_codegen generate)                # [T] 7 s, « Done! »
(cd packages/app && flutter pub add "${PROJET}_core_bridge" --path ../core_bridge)   # [T]
(cd packages/app && flutter pub add dev:integration_test --sdk=flutter)          # [T]
```

Relance `generate` après toute modification de `crates/bridge/src/api/`.

### 6.4 Test de bout en bout

`packages/app/integration_test/pont_test.dart` :

```dart
import 'package:atelier_core_bridge/atelier_core_bridge.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(() async => RustLib.init());

  testWidgets('le cœur Rust répond à travers le pont', (tester) async {
    expect(greet(name: 'atelier'), 'Hello, atelier!');
  });
}
```

```bash
(cd packages/app && flutter test integration_test/pont_test.dart -d linux)       # [T] All tests passed, 23 s
```

**Limite établie `[T]`** : le même test lancé par `flutter test test/…` (test unitaire sur l'hôte) échoue avec `Failed to load dynamic library 'libatelier_bridge.so'`. Le cœur réel n'est accessible que par `integration_test` ; les tests unitaires et de widgets utilisent des passerelles factices (§8.5).

### 6.5 Règles de l'API du pont

Rappels à appliquer dès la première fonction (§5.4) : toute fonction renvoie `Result` ; ni `i64` ni `u64` (types Dart différents entre natif et web) ; pas d'enum à champs sans décision explicite sur `freezed` ; un modèle de lecture se transmet par une requête ponctuelle plus un flux de numéros de version, jamais par un flux de données.

### 6.6 Vérification

```bash
cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings   # [T]
(cd packages/app && flutter test integration_test/pont_test.dart -d linux)         # [T]
```

---

## 7. Pile Flutter de l'application

### 7.1 Dépendances épinglées

```bash
cd packages/app
flutter pub add flutter_bloc:9.1.1 get_it:9.3.0 injectable:3.0.0                 # [T]
flutter pub add dev:build_runner:2.16.1 dev:injectable_generator:3.1.3 \
    dev:bdd_widget_test:3.0.5 dev:very_good_analysis:11.0.0 dev:bloc_test        # [T] bloc_test 10.0.0
cd ../..
```

Versions résolues `[T]` : `build` 4.0.11, `build_runner` 2.16.1, `injectable_generator` 3.1.3, `bdd_widget_test` 3.0.5, `material_ui` 1.11.1.

Contraintes à connaître :

| Contrainte | Preuve |
|---|---|
| `bdd_widget_test` 3.1.0 exige `build` 5 et ne se résout pas avec `injectable_generator` : rester en **3.0.5** | `[E]` S7 |
| `freezed` 4.0.3 exige `analyzer` 14 ; `widgetbook_generator` 3.24.0 exige `analyzer` < 14. **Les deux ne se résolvent pas dans le même espace de travail.** | `[T]` |
| Sans `freezed`, l'ajout de `widgetbook_generator` fait descendre `analyzer` à 13.3.0. L'essai S7 a validé la génération en `analyzer` 14.1.0 seulement. | `[T]`, `[E]` S7 |

Décision par défaut de ce guide : **ne pas installer `widgetbook_generator`** (cas d'usage Widgetbook écrits à la main) et n'ajouter `freezed` que sur décision explicite. On reste ainsi sur l'ensemble validé par S7.

Si `freezed`, `json_serializable` ou `retrofit` sont décidés, versions validées ensemble (`[E]` S7) :

```bash
flutter pub add freezed_annotation json_annotation dio retrofit
flutter pub add dev:freezed:4.0.3 dev:json_serializable:6.15.2 dev:retrofit_generator:10.3.3
```

### 7.2 Analyse

`packages/app/analysis_options.yaml` :

```yaml
include: package:very_good_analysis/analysis_options.yaml
analyzer:
  exclude:
    - "**/*.g.dart"
    - "**/*.freezed.dart"
    - "**/*.config.dart"
    - "test/features/*_test.dart"
    - build/**
```

Retire `flutter_lints` des `dev_dependencies` (`flutter pub remove flutter_lints`) `[NT]`.

### 7.3 Génération de code et BDD

`packages/app/build.yaml` — `lib/**` est obligatoire : sans lui, les générateurs ignorent `lib/` sans erreur (`[E]` S7).

```yaml
targets:
  $default:
    sources:
      - $package$
      - pubspec.yaml
      - lib/$lib$
      - lib/**
      - test/**
    builders:
      bdd_widget_test|featureBuilder:
        options:
          literalParameters: true
```

`packages/app/dart_test.yaml` :

```yaml
tags:
  core:
  gui:
  e2e:
```

Copie l'outil de synchronisation des `.feature` depuis l'essai :

```bash
mkdir -p packages/app/tool packages/app/test/features
cp spikes/s4-gherkin/app/tool/sync_features.dart packages/app/tool/              # [E] S4
```

Adapte dans ce fichier le chemin du dossier source (`features/` à la racine, soit `../../features` depuis `packages/app`).

Commandes du quotidien (`[E]` S7) :

```bash
cd packages/app
dart run tool/sync_features.dart             # copie les .feature portant @gui
dart run tool/sync_features.dart --check     # code 1 en cas d'écart : à lancer en CI
dart run build_runner build                  # 33 s la première fois, 9 s ensuite
dart analyze                                 # obligatoire après une création de fichiers d'étapes
flutter test --tags gui
```

- `--delete-conflicting-outputs` n'existe plus en `build_runner` 2.16.1 (`[E]` S7).
- Filtre toujours par inclusion (`--tags gui`), jamais par `--exclude-tags core` (`[E]` S4).
- La première génération des fichiers d'étapes peut produire un fichier corrompu : relis-les et lance `dart analyze` avant de continuer (`[E]` S4, S7).

### 7.4 Arborescence Feature-Sliced Design

```bash
mkdir -p packages/app/lib/{app,pages,widgets,features,entities,shared}           # [NT]
```

Chaque tranche de `features/` contient `presentation/`, `domain/`, `data/` et un fichier d'API publique. Seul le segment `data` importe `${PROJET}_core_bridge` (§6.2).

### 7.5 Vérification

```bash
(cd packages/app && dart run build_runner build && dart analyze && flutter test)
```

---

## 8. Design system, Widgetbook et jetons

```bash
(cd packages/design_system && flutter pub add material_ui)                       # [T] 1.11.1
(cd packages/design_system && flutter pub add dev:alchemist)                     # [T] 0.15.0
(cd packages/widgetbook && flutter pub add widgetbook widgetbook_annotation)     # [T] 3.25.0 / 3.11.0
(cd packages/widgetbook && flutter pub add "${PROJET}_design_system" --path ../design_system)   # [NT] même forme qu'au §6.3
mkdir -p packages/design_system/lib/src/{tokens,atoms,molecules,organisms}
```

- Épingle ensuite `material_ui` à la version exacte résolue.
- Tout fichier Dart importe `package:material_ui/…`, jamais `package:flutter/material.dart` (déprécié, fait échouer `--fatal-infos`).
- La résolution de `widgetbook` 3.25.0 et `alchemist` 0.15.0 avec `material_ui` 1.11.1 est établie `[T]`. Leur fonctionnement à l'exécution ne l'est pas `[NT]`.

Jetons de design (`[E]` S6) :

```bash
cd design
npm init -y
npm install --save-dev --save-exact style-dictionary@5.8.0
cd ..
```

Reprends `spikes/s6-jetons/` : `build.mjs`, `formats.mjs`, `check.mjs` et le fichier de jetons. Règles : pas de `$extends` ni de `$ref` dans les jetons ; modes sous `mode.<nom>.*` ; transformation `name/kebab` ; `log.warnings: 'error'`. Sorties à versionner : palette Rust dans un crate `crates/design-tokens`, constantes Dart dans `packages/design_system/lib/src/tokens/`, variables CSS pour le site public.

```bash
cargo new --lib --vcs none --name "$PROJET-design-tokens" crates/design-tokens   # [NT] même forme qu'au §3.2
cargo add -p "$PROJET-tui" --path crates/design-tokens                           # [NT]
```

---

## 9. Cible web

Préalable : étape 6 terminée.

```bash
cd packages/app
flutter_rust_bridge_codegen build-web --dart-root . --rust-root ../../crates/bridge \
    --wasm-pack-rustup-toolchain nightly-2026-11-15 --release                    # [T] 32 s, crée web/pkg/
flutter build web --no-wasm-dry-run --no-web-resources-cdn                       # [T] « Built build/web », 21 s
cd ../..
```

Observé `[T]` : `web/pkg/` contient `atelier_bridge.js` et `atelier_bridge_bg.wasm`. Ce dossier est une sortie de build : il est dans `.gitignore`.

Développement, avec les en-têtes d'isolation obligatoires (`[E]` S2) :

```bash
flutter run -d chrome \
    --web-header=Cross-Origin-Opener-Policy=same-origin \
    --web-header=Cross-Origin-Embedder-Policy=require-corp
```

À ajouter avant d'introduire SQLite dans le cœur (`[E]` S2), dans `.cargo/config.toml` à la racine :

```toml
[env]
CFLAGS_wasm32_unknown_unknown = "-matomics -mbulk-memory"
```

Règles (§3.5, §3.7, §5.4) : pas de `--wasm` tant que le pont est en 2.13 ; l'application teste `crossOriginIsolated` avant `RustLib.init()` ; tous les appels SQLite partent d'un seul Worker.

Non rejoué dans cette disposition `[NT]` : l'exécution dans un navigateur. Les essais S2 et S5 l'ont validée avec le pont intégré directement à l'application, pas dans un paquet séparé. Reprends le harnais de `spikes/s2-wasm/e2e/` pour le vérifier.

---

## 10. Site public

Non testé ici `[NT]` : `cargo-leptos` n'est pas installé sur la machine de référence. Vérifie les commandes dans la documentation de Leptos avant de les lancer.

```bash
cargo install cargo-leptos --locked                                              # [NT]
cargo new --bin --vcs none --name "$PROJET-web-public" crates/web-public         # [NT] même forme qu'au §3.2
cargo add -p "$PROJET-web-public" --path crates/core                             # [NT]
```

Le site ne dépend jamais de `bridge`. Il est servi sur une origine distincte de l'application (§3.6).

---

## 11. Commande de vérification unique

`just` n'est pas installé sur la machine de référence. Crée `tool/check.sh`, exécutable, qui enchaîne :

```bash
#!/usr/bin/env bash
set -euo pipefail
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
dart format --output=none --set-exit-if-changed packages
(cd packages/app && dart run tool/sync_features.dart --check)
(cd packages/app && dart run build_runner build && git diff --exit-code)
dart analyze --fatal-infos packages
(cd packages/app && flutter test)
(cd packages/core_bridge && flutter_rust_bridge_codegen generate && git diff --exit-code)
```

Statut : chaque commande a été rejouée séparément `[T]` ou validée par un essai `[E]`, sauf `dart format … packages` et `dart analyze --fatal-infos packages` à la racine de l'espace de travail `[NT]`. Le script dans son ensemble n'a pas été exécuté.

Ajoute ensuite `cargo nextest run`, `cargo deny check` et le test d'intégration Linux une fois les outils installés (§11.2 de l'architecture).

---

## 12. Critère de fin et premier commit

Le squelette est en place quand :

1. `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings` et `cargo test --workspace` passent.
2. `dart run melos list` affiche les quatre paquets, avec un seul `pubspec.lock`.
3. `flutter test integration_test/pont_test.dart -d linux` passe dans `packages/app`.
4. `dart run build_runner build` et `dart analyze` passent dans `packages/app`.
5. `flutter build web` produit `build/web`.

```bash
git add -A && git commit -m "chore: squelette du monorepo (workspace Rust, paquets Flutter, pont)"
```

---

## 13. Récapitulatif des pièges

| Piège | Symptôme | Parade | Preuve |
|---|---|---|---|
| Motifs dans `members` | `failed to load manifest for workspace member …/crates/adapters` | Liste explicite, alimentée par `cargo new` | `[T]` |
| Code du gabarit `cargo new --lib` | `clippy::must_use_candidate` | Remplacer `lib.rs` | `[T]` |
| `rust-toolchain.toml` absent du crate du pont, ou canal `stable` | Le hook refuse de construire | Lien symbolique, version concrète | `[E]` S1 |
| Code généré du pont en édition 2021 | `cargo fmt --check` échoue | `#[rustfmt::skip]` sur `mod frb_generated;` | `[T]` |
| Lints stricts sur le code généré | 13 erreurs clippy | `#[allow(…)]` ciblé sur le module | `[E]` S1 |
| Test unitaire qui appelle le cœur | `Failed to load dynamic library` | `integration_test`, ou passerelle factice | `[T]` |
| `i64`/`u64` dans l'API du pont | Ne compile plus sur le web | `i32`, `u32`, `f64`, `String` | `[E]` S2 |
| `build.yaml` sans `lib/**` | Aucun fichier généré, aucune erreur | Lister `lib/**` et `test/**` | `[E]` S7 |
| `--delete-conflicting-outputs` | Option ignorée | La retirer | `[E]` S7 |
| `bdd_widget_test` 3.1.0 | Résolution impossible avec `injectable_generator` | Rester en 3.0.5 | `[E]` S7 |
| `freezed` + `widgetbook_generator` | Résolution impossible (`analyzer`) | Pas de `widgetbook_generator` | `[T]` |
| `flutter test --exclude-tags core` | Aucun test exécuté | `--tags gui` | `[E]` S4 |
| Lanceur cucumber en `.run()` | Code de sortie 0 malgré des échecs | `run_and_exit` | `[E]` S4 |
| Deux installations `rustup` simultanées | `could not rename 'component' file` | Les lancer l'une après l'autre | `[E]` S2 |
| Flutter 3.50.0 | `.env` exclu des paquets Android | 3.50.1 | veille 2026 |

---

## 14. Ce que ce guide ne couvre pas

- Construction pour iOS, macOS et Windows ; SwiftPM avec le backend `native-assets` ; signature et distribution.
- Exécution de l'application web dans un navigateur avec le pont en paquet séparé ; Safari.
- Site public Leptos, CI GitHub Actions, empaquetage : voir §3.6, §11.4 et §12 de l'architecture.
- Moteur de synchronisation et stockage SQLite : ce guide crée les crates, pas leur contenu.
