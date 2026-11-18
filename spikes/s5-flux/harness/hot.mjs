// Rechargement à chaud (r) et redémarrage à chaud (R) pendant des flux infinis.
//   node hot.mjs [web|linux]
// web : `flutter run -d web-server` (en-têtes COOP/COEP) + Chrome piloté par Playwright ;
// linux : `flutter run -d linux`, lecture des lignes S5LIVE sur la sortie standard.
import { spawn } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

const here = path.dirname(fileURLToPath(import.meta.url));
const appDir = path.resolve(here, '..', 'app');
const mainDart = path.join(appDir, 'lib', 'main.dart');
const target = process.argv[2] ?? 'web';
const port = 8700 + Math.floor(Math.random() * 200);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const original = fs.readFileSync(mainDart, 'utf8');
const live = [];
const errors = [];
let stdout = '';

function onLine(line) {
  const i = line.indexOf('S5LIVE ');
  if (i >= 0) live.push({ at: Date.now(), ...JSON.parse(line.slice(i + 7)) });
  if (/error|exception|panicked/i.test(line) && !line.includes('S5LIVE')) errors.push(line.trim());
}

const args = target === 'web'
  ? ['run', '-d', 'web-server', '--web-port', String(port), '--web-hostname', '127.0.0.1',
     '--web-header=Cross-Origin-Opener-Policy=same-origin',
     '--web-header=Cross-Origin-Embedder-Policy=require-corp',
     '--dart-define=S5_MODE=live']
  : ['run', '-d', 'linux', '--dart-define=S5_MODE=live'];
const proc = spawn('flutter', args, { cwd: appDir, stdio: ['pipe', 'pipe', 'pipe'] });
let buf = '';
const feed = (d) => {
  stdout += d; buf += d;
  const lines = buf.split('\n'); buf = lines.pop();
  for (const l of lines) onLine(l);
};
proc.stdout.on('data', (d) => feed(d.toString()));
proc.stderr.on('data', (d) => feed(d.toString()));

async function waitFor(pred, ms) {
  const t0 = Date.now();
  while (Date.now() - t0 < ms) { if (pred()) return true; await sleep(100); }
  return false;
}
const send = (k) => proc.stdin.write(k);
const out = { target, etapes: [] };
let browser;

try {
  const ready = await waitFor(() => (target === 'web' ? /is being served at|served at/i.test(stdout) : live.length > 0), 300_000);
  if (!ready) throw new Error(`flutter run n'a pas démarré :\n${stdout.slice(-2000)}`);
  if (target === 'web') {
    browser = await chromium.launch({ channel: 'chrome' });
    const page = await browser.newPage();
    page.on('console', (m) => onLine(m.text()));
    page.on('pageerror', (e) => errors.push(`pageerror: ${e.message}`));
    await page.goto(`http://127.0.0.1:${port}/?live=1`);
    out.crossOriginIsolated = await page.evaluate(() => self.crossOriginIsolated);
  }
  await waitFor(() => live.length >= 4, 120_000);
  const l0 = live.at(-1);
  out.etapes.push({ etape: 'depart', ...l0 });

  // Rechargement à chaud : le code change, l'état Dart et les abonnements doivent survivre.
  fs.writeFileSync(mainDart, original.replace("String codeVersion() => 'v1';", "String codeVersion() => 'v2';"));
  const n1 = live.length;
  send('r');
  const reloaded = await waitFor(() => live.length > n1 && live.at(-1).code === 'v2', 60_000);
  await sleep(3000);
  const l1 = live.at(-1);
  out.etapes.push({ etape: 'rechargement_a_chaud', pris_en_compte: reloaded, ...l1 });

  // Redémarrage à chaud : l'état Dart repart de zéro ; que deviennent les émetteurs Rust ?
  const n2 = live.length;
  send('R');
  const restarted = await waitFor(() => live.length > n2 && JSON.stringify(live.at(-1).ids) !== JSON.stringify(l1.ids), 120_000);
  await sleep(4000);
  const l2 = live.at(-1);
  out.etapes.push({ etape: 'redemarrage_a_chaud', pris_en_compte: restarted, ...l2 });
} catch (e) {
  out.exception = String(e);
} finally {
  fs.writeFileSync(mainDart, original);
  send('q');
  await sleep(3000);
  proc.kill('SIGTERM');
  if (browser) await browser.close();
}
out.erreurs = [...new Set(errors)].slice(0, 20);
const dir = path.resolve(here, '..', 'resultats');
fs.writeFileSync(path.join(dir, `hot-${target}.json`), JSON.stringify(out, null, 2));
fs.writeFileSync(path.join(dir, `hot-${target}.log`), stdout);
console.log(JSON.stringify(out, null, 2));
