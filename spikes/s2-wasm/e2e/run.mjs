// Preuve automatisée de l'essai S2 (sans dépendance npm).
// Usage : node run.mjs [--firefox] [--no-isolation] [--headed]
// Chrome est piloté par CDP ; Firefox par geckodriver (WebDriver classique).
// Variables : FIREFOX_BIN, GECKODRIVER (défaut : binaires du snap Firefox).
import assert from 'node:assert/strict';
import { startServer } from './serve.mjs';
import { launchChrome, newTab as newChromeTab } from './cdp.mjs';
import { launchFirefox } from './webdriver.mjs';

const isolation = !process.argv.includes('--no-isolation');
const PORT = 8765;
const ORIGIN = `http://127.0.0.1:${PORT}`;
const root = new URL('../app/build/web', import.meta.url).pathname;

const server = await startServer(root, PORT, { isolation });
const SNAP = '/snap/firefox/current/usr/lib/firefox';
let chrome;
if (process.argv.includes('--firefox')) {
  chrome = await launchFirefox({
    binary: process.env.FIREFOX_BIN ?? `${SNAP}/firefox`,
    geckodriver: process.env.GECKODRIVER ?? `${SNAP}/geckodriver`,
  });
} else {
  const c = await launchChrome({ headless: !process.argv.includes('--headed') });
  chrome = { version: c.version, newTab: () => newChromeTab(c), close: () => c.close() };
}
const results = [];
const step = (name, data) => {
  results.push(name);
  console.log(`✔ ${name}`, data === undefined ? '' : JSON.stringify(data));
};

async function scenario(tab, query) {
  await tab.goto(`${ORIGIN}/?${query}`);
  return JSON.parse(await tab.waitFor('globalThis.s2Result'));
}
async function reloadScenario(tab) {
  await tab.eval('globalThis.s2Result = undefined');
  await tab.reload();
  return JSON.parse(await tab.waitFor('globalThis.s2Result'));
}

let failed = false;
try {
  console.log(`${chrome.version} — isolation=${isolation}`);
  const tab1 = await chrome.newTab();

  // Étapes 1 et 2 : appel async + SQLite en mémoire.
  let r = await scenario(tab1, 'mode=memory&add=en-memoire');
  assert.equal(r.ok, true, JSON.stringify(r));
  assert.equal(r.sum, 42);
  assert.deepEqual(r.after, ['en-memoire']);
  step('étapes 1-2 : appel async et SQLite en mémoire', r);
  r = await reloadScenario(tab1);
  assert.deepEqual(r.before, [], 'la base en mémoire ne survit pas au rechargement');
  step('mémoire : rien ne survit au rechargement');

  // Étape 3 : persistance OPFS.
  const note = `note-${Date.now()}`;
  r = await scenario(tab1, `mode=opfs&add=${note}`);
  assert.equal(r.ok, true, JSON.stringify(r));
  assert.deepEqual(r.after, [note]);
  step('étape 3 : écriture sur OPFS (sahpool)', r);
  r = await scenario(tab1, 'mode=opfs');
  assert.deepEqual(r.before, [note], JSON.stringify(r));
  step('étape 3 : relue après navigation');
  r = await reloadScenario(tab1);
  assert.equal(r.ok, true, JSON.stringify(r));
  assert.deepEqual(r.before, [note]);
  step('étape 3 : relue après rechargement de page', { before: r.before });

  // Étape 5 : second onglet, pendant que le premier tient OPFS.
  const tab2 = await chrome.newTab();
  r = await scenario(tab2, 'mode=opfs&lock=0');
  assert.equal(r.ok, false, JSON.stringify(r));
  step('second onglet sans verrou : échec brut observé', r);
  r = await scenario(tab2, 'mode=opfs&lock=1');
  assert.equal(r.ok, false, JSON.stringify(r));
  assert.equal(r.errorKind, 'storeOpenInAnotherTab');
  step('second onglet avec verrou Web Locks : erreur typée', r);

  // Le premier onglet reste utilisable.
  r = await scenario(tab1, 'mode=opfs&add=encore');
  assert.deepEqual(r.after, [note, 'encore'], JSON.stringify(r));
  step("premier onglet intact après l'échec du second");

  // Fermeture du premier onglet : le second peut prendre la main.
  await tab1.close();
  await new Promise((res) => setTimeout(res, 500));
  r = await reloadScenario(tab2);
  assert.equal(r.ok, true, JSON.stringify(r));
  assert.deepEqual(r.before, [note, 'encore']);
  step('premier onglet fermé : le second prend la main et relit', { before: r.before });

  // Charge : 5000 insertions sur OPFS pendant que 16 tâches occupent les autres Workers.
  r = await scenario(tab2, 'mode=opfs&stress=5000');
  assert.equal(r.ok, true, JSON.stringify(r));
  assert.equal(r.count, 10002);
  step('charge : 2 × 5000 insertions OPFS, seules puis avec 32 appels concurrents', { seulMs: r.aloneMs, concurrentMs: r.insertMs, totalMs: r.stressMs, count: r.count });
  r = await reloadScenario(tab2);
  assert.equal(r.ok, true, JSON.stringify(r));
  assert.equal(r.stressTotal, 20002, 'les 10000 premières lignes ont survécu au rechargement');
  step('charge : relue après rechargement', { count: r.count });

  console.log(`\nRÉUSSITE : ${results.length} vérifications`);
} catch (e) {
  failed = true;
  console.error('\nÉCHEC :', e.message);
} finally {
  await chrome.close();
  server.close();
}
process.exit(failed ? 1 : 0);
