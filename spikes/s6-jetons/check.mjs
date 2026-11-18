// Mode CI : régénère dans un dossier temporaire et échoue si les fichiers versionnés diffèrent.
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { build } from './build.mjs';

const FILES = ['atelier-tui/src/generated.rs', 'generated/tokens.css', 'generated/tokens.dart'];
const tmp = await mkdtemp(join(tmpdir(), 's6-'));
let bad = 0;
try {
  await build(tmp);
  for (const f of FILES) {
    const [a, b] = await Promise.all([readFile(f, 'utf8').catch(() => null), readFile(join(tmp, f), 'utf8')]);
    if (a !== b) { console.error(`DÉRIVE : ${f} diffère de la sortie régénérée (lancer npm run build)`); bad++; }
  }
} finally { await rm(tmp, { recursive: true, force: true }); }
if (bad) process.exit(1);
console.log('Jetons : fichiers générés à jour.');
