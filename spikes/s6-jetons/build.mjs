import StyleDictionary from 'style-dictionary';
import { rustFormat, cssFormat, dartFormat } from './formats.mjs';

/** Génère les trois sorties sous `root` (par défaut : la racine du spike). */
export async function build(root = '.') {
  const sd = new StyleDictionary({
    usesDtcg: true,
    source: ['tokens/*.tokens.json'],
    log: { verbosity: 'default', warnings: 'error' },
    hooks: {
      formats: { 'atelier/rust': rustFormat, 'atelier/css': cssFormat, 'atelier/dart': dartFormat },
    },
    platforms: {
      all: {
        transforms: ['name/kebab', 'color/hex'],
        buildPath: `${root}/`,
        files: [
          { destination: 'atelier-tui/src/generated.rs', format: 'atelier/rust' },
          { destination: 'generated/tokens.css', format: 'atelier/css' },
          { destination: 'generated/tokens.dart', format: 'atelier/dart' },
        ],
      },
    },
  });
  await sd.buildAllPlatforms();
}

if (import.meta.url === `file://${process.argv[1]}`) await build();
