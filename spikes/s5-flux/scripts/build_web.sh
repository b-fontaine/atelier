#!/usr/bin/env bash
# Construit l'app web de l'essai S5 dans build/web-<mode>.
#   scripts/build_web.sh threade        # mode par défaut de FRB (COOP/COEP requis)
#   scripts/build_web.sh sans-entetes   # default_dart_async: false
#   S5_FRB_PATCHED=<dossier> scripts/build_web.sh threade
#       # idem, avec une copie de FRB Dart 2.13.0 corrigée (scripts/frb-stream-sink-sans-pause.patch),
#       # sortie dans build/web-threade-corrige
# Le code généré committé correspond au mode threadé : il est régénéré en fin de script.
set -euo pipefail
MODE=${1:-threade}
NIGHTLY=${S5_NIGHTLY:-nightly-2026-11-15}
ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT/app"

case "$MODE" in
  threade)      CFG=flutter_rust_bridge.yaml;               DEFINES=() ;;
  sans-entetes) CFG=flutter_rust_bridge.sans-entetes.yaml;  DEFINES=(--dart-define=S5_SANS_ENTETES=true) ;;
  *) echo "mode inconnu : $MODE" >&2; exit 2 ;;
esac

flutter_rust_bridge_codegen generate --config-file "$CFG"
flutter_rust_bridge_codegen build-web --dart-root . --rust-root ../crates/bridge --release \
  --wasm-pack-rustup-toolchain "$NIGHTLY"
OUT="$ROOT/build/web-$MODE"
if [ -n "${S5_FRB_PATCHED:-}" ]; then
  printf 'dependency_overrides:\n  flutter_rust_bridge:\n    path: %s\n' "$S5_FRB_PATCHED" > pubspec_overrides.yaml
  trap 'rm -f pubspec_overrides.yaml; flutter pub get >/dev/null' EXIT
  flutter pub get
  OUT="$OUT-corrige"
fi
flutter build web --release --no-web-resources-cdn "${DEFINES[@]}" --output "$OUT"

if [ "$MODE" != threade ]; then
  flutter_rust_bridge_codegen generate --config-file flutter_rust_bridge.yaml
  flutter_rust_bridge_codegen build-web --dart-root . --rust-root ../crates/bridge --release \
    --wasm-pack-rustup-toolchain "$NIGHTLY"
fi
