// Harnais de l'essai S5 : exécute les scénarios de l'app dans un navigateur réel.
//   node run.mjs <chrome|firefox> <threade|sans-entetes> [on|off]
// `on|off` : en-têtes COOP/COEP servis ou non (défaut : on pour threade, off sinon).
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium, firefox } from 'playwright';
import { serve } from './serve.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const [browserName = 'chrome', mode = 'threade', headersArg] = process.argv.slice(2);
const headers = (headersArg ?? (mode.startsWith('threade') ? 'on' : 'off')) === 'on';
const root = path.resolve(here, '..', 'build', `web-${mode}`);
const tag = `web-${browserName}-${mode}-${headers ? 'entetes' : 'sans-entetes'}`;
const port = 8200 + Math.floor(Math.random() * 500);
const base = `http://127.0.0.1:${port}/`;

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

function watch(page, name) {
  const log = { lines: [], errors: [], results: [], live: [], done: false };
  page.on('console', (m) => {
    const t = m.text();
    log.lines.push(`[${name}] ${m.type()}: ${t}`);
    if (t.startsWith('S5RESULT ')) log.results.push(JSON.parse(t.slice(9)));
    else if (t.startsWith('S5LIVE ')) log.live.push({ at: Date.now(), ...JSON.parse(t.slice(7)) });
    else if (t.startsWith('S5DONE')) log.done = true;
    if (m.type() === 'error') log.errors.push(t);
  });
  page.on('pageerror', (e) => { log.errors.push(`pageerror: ${e.message} | ${(e.stack ?? '').split('\n').slice(0, 6).join(' <- ')}`); log.lines.push(`[${name}] pageerror: ${e.message}`); });
  page.on('crash', () => log.errors.push('crash'));
  return log;
}

async function waitFor(pred, ms) {
  const t0 = Date.now();
  while (Date.now() - t0 < ms) { if (pred()) return true; await sleep(100); }
  return false;
}

const out = { tag, browser: browserName, mode, headers, scenarios: [], externes: [], erreurs: [] };

const server = await serve(root, { port, headers });
const browser = browserName === 'firefox'
  ? await firefox.launch()
  : await chromium.launch({ channel: 'chrome' });
out.version = browser.version();
const context = await browser.newContext();

try {
  // A. Série automatique, répétée S5_REPEAT fois (défaut 5) dans des pages neuves.
  {
    const repeat = Number(process.env.S5_REPEAT ?? 5);
    const agg = new Map();
    out.passes = [];
    for (let i = 0; i < repeat; i++) {
      const page = await context.newPage();
      const log = watch(page, `auto#${i + 1}`);
      await page.goto(`${base}?auto=1${process.env.S5_ONLY ? `&only=${encodeURIComponent(process.env.S5_ONLY)}` : ''}`);
      const ok = await waitFor(() => log.done, 240_000);
      out.crossOriginIsolated = await page.evaluate(() => self.crossOriginIsolated).catch(() => null);
      const results = [...log.results];
      if (!ok) results.push({ name: 'serie_auto', status: 'echec', detail: `blocage : ${log.results.length} scénario(s) terminés, pas de S5DONE en 240 s` });
      out.passes.push(results);
      for (const r of results) {
        const a = agg.get(r.name) ?? { name: r.name, ok: 0, echec: 0, nt: 0, info: 0, details: [] };
        a[r.status] = (a[r.status] ?? 0) + 1;
        if (!a.details.includes(r.detail)) a.details.push(r.detail);
        agg.set(r.name, a);
      }
      out.erreurs.push(...log.errors.map((e) => `auto#${i + 1}: ${e}`));
      if (i === 0) out.consoleAuto = log.lines.slice(0, 400);
      await page.close();
    }
    out.scenarios = [...agg.values()].map((a) => ({
      name: a.name,
      status: a.echec === 0 ? (a.ok > 0 ? 'ok' : (a.nt > 0 ? 'nt' : 'info')) : 'echec',
      detail: `${a.ok}/${repeat} ok${a.echec ? `, ${a.echec} échec(s)` : ''} — ${a.details.slice(0, 3).join(' | ')}`,
    }));
  }

  const skipExternal = Boolean(process.env.S5_ONLY); // séries B à D sautées

  // B. Rechargement de page pendant des flux infinis.
  if (!skipExternal) {
    const page = await context.newPage();
    const log = watch(page, 'live');
    await page.goto(`${base}?live=1`);
    const started = await waitFor(() => log.live.length >= 3, 30_000);
    const before = log.live.at(-1);
    const n = log.live.length;
    await page.reload();
    const after1 = await waitFor(() => log.live.length >= n + 2 && log.live.at(-1).ids?.async !== before?.ids?.async, 30_000);
    await sleep(2000);
    const after = log.live.at(-1);
    const keys = Object.keys(after?.recv ?? {});
    const growing = keys.length > 0 && keys.every((k) => after.recv[k] > 10);
    const ok = started && after1 && growing && after.foreign === 0 && after.disorder === 0;
    out.externes.push({
      name: 'rechargement_page_live', status: ok ? 'ok' : 'echec',
      detail: `avant : ${JSON.stringify(before?.recv)} ; après rechargement : reçus ${JSON.stringify(after?.recv)}, étrangers ${after?.foreign}, désordre ${after?.disorder}`,
    });
    out.erreurs.push(...log.errors.map((e) => `live: ${e}`));
    await page.close();
  }

  // C. Rechargement au milieu de la série automatique, puis série complète.
  if (!skipExternal) {
    const page = await context.newPage();
    const log = watch(page, 'reload-auto');
    await page.goto(`${base}?auto=1`);
    await waitFor(() => log.results.length >= 3, 60_000);
    const partial = log.results.length;
    log.results = []; log.done = false;
    await page.reload();
    const ok = await waitFor(() => log.done, 240_000);
    const bad = log.results.filter((r) => r.status === 'echec').map((r) => r.name);
    const expectedBad = out.scenarios.filter((r) => r.status === 'echec').map((r) => r.name);
    const same = JSON.stringify(bad) === JSON.stringify(expectedBad);
    out.externes.push({
      name: 'rechargement_page_en_cours_de_serie', status: ok && same ? 'ok' : 'echec',
      detail: `rechargé après ${partial} scénarios ; série relancée ${ok ? 'complète' : 'BLOQUÉE'} ; échecs ${JSON.stringify(bad)} (série A : ${JSON.stringify(expectedBad)})`,
    });
    out.erreurs.push(...log.errors.map((e) => `reload-auto: ${e}`));
    await page.close();
  }

  // D. Deux onglets de la même origine (BroadcastChannel partagé par l'origine).
  if (!skipExternal) {
    const a = await context.newPage();
    const b = await context.newPage();
    const la = watch(a, 'onglet-A');
    const lb = watch(b, 'onglet-B');
    await a.goto(`${base}?live=1`);
    await b.goto(`${base}?live=1`);
    await waitFor(() => la.live.length >= 6 && lb.live.length >= 6, 30_000);
    const ea = la.live.at(-1); const eb = lb.live.at(-1);
    const ok = ea && eb && ea.foreign === 0 && eb.foreign === 0 && ea.disorder === 0 && eb.disorder === 0;
    out.externes.push({
      name: 'deux_onglets', status: ok ? 'ok' : 'echec',
      detail: `A : reçus ${JSON.stringify(ea?.recv)}, étrangers ${ea?.foreign}, désordre ${ea?.disorder} ; B : reçus ${JSON.stringify(eb?.recv)}, étrangers ${eb?.foreign}, désordre ${eb?.disorder}`,
    });
    out.erreurs.push(...la.errors.map((e) => `A: ${e}`), ...lb.errors.map((e) => `B: ${e}`));
    await a.close(); await b.close();
  }
} finally {
  await browser.close();
  server.close();
}

const dir = path.resolve(here, '..', 'resultats');
fs.mkdirSync(dir, { recursive: true });
fs.writeFileSync(path.join(dir, `${tag}.json`), JSON.stringify(out, null, 2));
console.log(`${tag} — ${out.browser} ${out.version}, crossOriginIsolated=${out.crossOriginIsolated}`);
for (const r of [...out.scenarios, ...out.externes]) console.log(`  ${r.status.padEnd(5)} ${r.name.padEnd(36)} ${r.detail}`);
const uniq = [...new Set(out.erreurs)];
if (uniq.length) console.log(`  erreurs console (${out.erreurs.length}, ${uniq.length} distinctes) :\n    ${uniq.slice(0, 15).join('\n    ')}`);
