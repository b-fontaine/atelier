// Client Chrome DevTools Protocol minimal, sans dépendance (Node >= 22 : fetch + WebSocket natifs).
import { spawn } from 'node:child_process';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

export async function launchChrome({ port = 9333, binary = 'google-chrome', headless = true } = {}) {
  const profile = await mkdtemp(join(tmpdir(), 's2-chrome-'));
  const args = [
    `--remote-debugging-port=${port}`,
    `--user-data-dir=${profile}`,
    '--no-first-run',
    '--no-default-browser-check',
    '--disable-gpu',
    '--enable-unsafe-swiftshader',
  ];
  if (headless) args.push('--headless=new');
  args.push('about:blank');
  const proc = spawn(binary, args, { stdio: 'ignore' });
  const base = `http://127.0.0.1:${port}`;
  for (let i = 0; i < 100; i++) {
    try {
      const v = await (await fetch(`${base}/json/version`)).json();
      return {
        base,
        version: v.Browser,
        async close() {
          proc.kill('SIGTERM');
          await sleep(500);
          await rm(profile, { recursive: true, force: true }).catch(() => {});
        },
      };
    } catch {
      await sleep(100);
    }
  }
  proc.kill('SIGKILL');
  throw new Error('Chrome ne répond pas sur le port de débogage');
}

/** Ouvre un nouvel onglet et renvoie une session CDP attachée. */
export async function newTab(browser, url = 'about:blank') {
  const res = await fetch(`${browser.base}/json/new?${encodeURIComponent(url)}`, { method: 'PUT' });
  const target = await res.json();
  const ws = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    ws.onopen = resolve;
    ws.onerror = reject;
  });
  let nextId = 1;
  const pending = new Map();
  const logs = [];
  ws.onmessage = (ev) => {
    const msg = JSON.parse(ev.data);
    if (msg.id && pending.has(msg.id)) {
      const { resolve, reject } = pending.get(msg.id);
      pending.delete(msg.id);
      msg.error ? reject(new Error(JSON.stringify(msg.error))) : resolve(msg.result);
    } else if (msg.method === 'Runtime.consoleAPICalled') {
      logs.push(`[${msg.params.type}] ` + msg.params.args.map((a) => a.value ?? a.description ?? '').join(' '));
    } else if (msg.method === 'Runtime.exceptionThrown') {
      const d = msg.params.exceptionDetails;
      logs.push(`[exception] ${d.text} ${d.exception?.description ?? ''}`);
    } else if (msg.method === 'Log.entryAdded') {
      logs.push(`[log:${msg.params.entry.level}] ${msg.params.entry.text}`);
    }
  };
  const send = (method, params = {}) =>
    new Promise((resolve, reject) => {
      const id = nextId++;
      pending.set(id, { resolve, reject });
      ws.send(JSON.stringify({ id, method, params }));
    });
  await send('Runtime.enable');
  await send('Log.enable');
  await send('Page.enable');
  const tab = {
    id: target.id,
    logs,
    send,
    async goto(u) {
      await send('Page.navigate', { url: u });
    },
    async reload() {
      await send('Page.reload', { ignoreCache: true });
    },
    async eval(expression) {
      const r = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
      if (r.exceptionDetails) throw new Error(r.exceptionDetails.exception?.description ?? r.exceptionDetails.text);
      return r.result.value;
    },
    /** Attend que `expression` renvoie une valeur vraie (ou expire). */
    async waitFor(expression, timeoutMs = 30000) {
      const t0 = Date.now();
      for (;;) {
        try {
          const v = await tab.eval(expression);
          if (v) return v;
        } catch {
          // contexte en cours de rechargement : on réessaie
        }
        if (Date.now() - t0 > timeoutMs) {
          throw new Error(`délai dépassé : ${expression}\n--- console ---\n${logs.join('\n')}`);
        }
        await sleep(100);
      }
    },
    async close() {
      ws.close();
      await fetch(`${browser.base}/json/close/${target.id}`).catch(() => {});
    },
  };
  return tab;
}
