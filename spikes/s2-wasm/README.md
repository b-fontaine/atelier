# Essai S2 — cœur Rust en WASM + SQLite sur OPFS

Prototype jetable, copié de `spikes/s1-pont/`. Résultats et décisions :
`docs/architecture/adr/S2-wasm-sqlite.md`.

## Prérequis

```bash
cargo install flutter_rust_bridge_codegen --version 2.13.0 --locked
cargo install wasm-pack --locked            # 0.15.1
rustup toolchain install nightly-2026-11-15 --profile minimal \
    --component rust-src --target wasm32-unknown-unknown
```

`clang` doit savoir cibler `wasm32` (SQLite est compilé depuis le C par `sqlite-wasm-rs`).

## Construire et prouver

```bash
cd app
flutter_rust_bridge_codegen generate          # après modification de crates/bridge/src/api
flutter_rust_bridge_codegen build-web --rust-root ../crates/bridge \
    --wasm-pack-rustup-toolchain nightly-2026-11-15 --release      # -> web/pkg/
flutter build web --no-wasm-dry-run                                # dart2js -> build/web/

cd ../e2e
node run.mjs              # Chrome headless (CDP), 11 vérifications
node run.mjs --firefox    # Firefox headless (geckodriver)

cd ../app
flutter test integration_test/s2_test.dart -d linux   # même scénario en natif
```

Mode développement :

```bash
flutter run -d chrome \
    --web-header=Cross-Origin-Opener-Policy=same-origin \
    --web-header=Cross-Origin-Embedder-Policy=require-corp
```

## Repères

- `crates/core/src/store.rs` : port `NoteStore`, adaptateur `rusqlite`, et (web) verrou
  d'onglet Web Locks + installation du VFS `sahpool`.
- `crates/bridge/src/api/store.rs` : acteur propriétaire de la connexion, dans un Worker
  retiré du pool FRB.
- `.cargo/config.toml` : `CFLAGS_wasm32_unknown_unknown` pour compiler SQLite en mémoire partagée.
- `app/lib/main.dart` : scénario piloté par l'URL (`?mode=opfs&add=…&lock=0|1&stress=n`),
  résultat publié dans `globalThis.s2Result`.
- `e2e/` : serveur statique avec COOP/COEP et pilotes de navigateur sans dépendance npm.
