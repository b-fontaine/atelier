// Vérifie le mode développement : `flutter run -d web-server` (DDC) avec les en-têtes COOP/COEP.
// Lancer d'abord, depuis app/ :
//   flutter run -d web-server --web-port 8770 \
//     --web-header=Cross-Origin-Opener-Policy=same-origin \
//     --web-header=Cross-Origin-Embedder-Policy=require-corp
import { launchChrome, newTab } from './cdp.mjs';
const chrome = await launchChrome();
try {
  const tab = await newTab(chrome);
  await tab.goto('http://localhost:8770/?mode=opfs&add=dev');
  console.log(chrome.version, await tab.waitFor('globalThis.s2Result', 120000));
} catch (e) {
  console.error('ÉCHEC', e.message.slice(0, 3000));
  process.exitCode = 1;
} finally {
  await chrome.close();
}
