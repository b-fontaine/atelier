# Essai S5 — fiabilité des flux FRB 2.13.0 dans le navigateur

Prototype jetable copié depuis `spikes/s1-pont/`. Résultats et recommandation :
`docs/architecture/adr/S5-flux-web.md`.

- `crates/bridge/src/api/flux.rs` : émetteurs instrumentés (rafale, cadencé, `async` ou worker,
  erreur) et registre d'état par flux (`stream_info`, `stop_stream`).
- `crates/bridge/src/api/repli.rs` : repli « requête ponctuelle + notifications de version ».
- `app/lib/scenarios.dart` : scénarios partagés web / natif.
- `harness/` : serveur statique (avec ou sans COOP/COEP) et harnais Playwright.
- `resultats/` : sorties brutes des dernières exécutions.

```bash
# Outils (une fois)
rustup toolchain install nightly-2026-11-15 --profile minimal \
    --component rust-src --target wasm32-unknown-unknown
cargo install wasm-pack --version 0.15.1 --locked
(cd harness && npm install && npx playwright install firefox)

# Natif (référence)
(cd app && flutter test integration_test/s5_test.dart -d linux)

# Web threadé (COOP/COEP) puis sans en-têtes
scripts/build_web.sh threade
scripts/build_web.sh sans-entetes
(cd harness && node run.mjs chrome threade && node run.mjs firefox threade)
(cd harness && node run.mjs chrome sans-entetes && node run.mjs firefox sans-entetes)

# Rechargement / redémarrage à chaud
(cd harness && node hot.mjs web && node hot.mjs linux)

# Variante avec FRB Dart corrigé (scripts/frb-stream-sink-sans-pause.patch appliqué
# à une copie de ~/.pub-cache/hosted/pub.dev/flutter_rust_bridge-2.13.0)
S5_FRB_PATCHED=/chemin/vers/copie scripts/build_web.sh threade
(cd harness && node run.mjs chrome threade-corrige)
```

Variables du harnais : `S5_REPEAT` (passes de la série automatique, défaut 5),
`S5_ONLY` (expression régulière sur les noms de scénarios ; saute les séries B à D).
