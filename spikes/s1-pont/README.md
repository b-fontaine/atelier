# Essai S1 — pont FRB 2.13.0 + native-assets dans un Cargo workspace

Prototype jetable. Résultats et décisions : `docs/architecture/adr/S1-pont-frb.md`.

```bash
cargo install flutter_rust_bridge_codegen --version 2.13.0 --locked
cd app
flutter_rust_bridge_codegen generate          # après modification de crates/bridge/src/api
flutter test integration_test/s1_test.dart -d linux
flutter build apk --debug
```
