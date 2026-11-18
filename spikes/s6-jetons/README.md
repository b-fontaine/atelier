# Spike S6 — jetons DTCG → palette ratatui (prototype jetable)

Voir l'ADR : `docs/architecture/adr/S6-jetons-ratatui.md`.

```bash
cd spikes/s6-jetons
npm ci                          # Style Dictionary 5.8.0 exact
npm run build                   # régénère generated/* et atelier-tui/src/generated.rs
npm run check                   # CI : régénère en temporaire, échoue (code 1) si dérive
(cd atelier-tui && cargo test)  # 13 tests : rendu TestBackend, dégradation, NO_COLOR, COLORTERM
```

- `tokens/atelier.tokens.json` : source unique (primitives `color.*`, modes `mode.light|dark.*`).
- `formats.mjs` / `build.mjs` : formats personnalisés Rust, CSS, Dart.
- `check.mjs` : contrôle de dérive.
