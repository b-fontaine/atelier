// Client WebDriver classique minimal (geckodriver), sans dépendance.
import { spawn } from 'node:child_process';

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

export async function launchFirefox({ port = 4455, geckodriver, binary } = {}) {
  const proc = spawn(geckodriver, ['--port', String(port)], { stdio: 'ignore' });
  const base = `http://127.0.0.1:${port}`;
  const call = async (method, path, body) => {
    const res = await fetch(base + path, {
      method,
      headers: { 'Content-Type': 'application/json' },
      body: body === undefined ? undefined : JSON.stringify(body),
    });
    const json = await res.json();
    if (!res.ok) throw new Error(`${path}: ${json.value?.message ?? res.status}`);
    return json.value;
  };
  for (let i = 0; ; i++) {
    try {
      await call('GET', '/status');
      break;
    } catch (e) {
      if (i > 50) throw e;
      await sleep(100);
    }
  }
  const session = await call('POST', '/session', {
    capabilities: {
      alwaysMatch: { browserName: 'firefox', 'moz:firefoxOptions': { binary, args: ['-headless'] } },
    },
  });
  const s = `/session/${session.sessionId}`;
  let current = await call('GET', `${s}/window`);
  const focus = async (handle) => {
    if (current !== handle) {
      await call('POST', `${s}/window`, { handle });
      current = handle;
    }
  };
  const makeTab = (handle) => {
    const tab = {
      logs: [],
      async goto(u) {
        await focus(handle);
        await call('POST', `${s}/url`, { url: u });
      },
      async reload() {
        await focus(handle);
        await call('POST', `${s}/refresh`, {});
      },
      async eval(expression) {
        await focus(handle);
        return call('POST', `${s}/execute/sync`, { script: `return (${expression});`, args: [] });
      },
      async waitFor(expression, timeoutMs = 30000) {
        const t0 = Date.now();
        for (;;) {
          try {
            const v = await tab.eval(expression);
            if (v) return v;
          } catch {
            // page en cours de rechargement
          }
          if (Date.now() - t0 > timeoutMs) throw new Error(`délai dépassé : ${expression}`);
          await sleep(100);
        }
      },
      async close() {
        await focus(handle);
        await call('DELETE', `${s}/window`);
        current = null;
      },
    };
    return tab;
  };
  let first = true;
  return {
    version: `Firefox/${session.capabilities.browserVersion}`,
    async newTab() {
      if (first) {
        first = false;
        return makeTab(current);
      }
      const w = await call('POST', `${s}/window/new`, { type: 'tab' });
      return makeTab(w.handle);
    },
    async close() {
      await call('DELETE', s).catch(() => {});
      proc.kill('SIGTERM');
    },
  };
}
