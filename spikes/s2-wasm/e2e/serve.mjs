// Serveur statique minimal pour build/web, avec ou sans isolation cross-origin.
// Usage : node serve.mjs <dossier> <port> [--no-isolation]
import { createServer } from 'node:http';
import { readFile, stat } from 'node:fs/promises';
import { extname, join, normalize } from 'node:path';

const TYPES = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.wasm': 'application/wasm',
  '.json': 'application/json',
  '.png': 'image/png',
  '.ttf': 'font/ttf',
  '.otf': 'font/otf',
  '.symbols': 'text/plain',
};

export function startServer(root, port, { isolation = true } = {}) {
  const server = createServer(async (req, res) => {
    try {
      let path = decodeURIComponent(new URL(req.url, 'http://x').pathname);
      if (path.endsWith('/')) path += 'index.html';
      const file = join(root, normalize(path));
      if (!file.startsWith(root)) throw new Error('hors racine');
      await stat(file);
      const body = await readFile(file);
      const headers = {
        'Content-Type': TYPES[extname(file)] ?? 'application/octet-stream',
        'Cache-Control': 'no-store',
      };
      if (isolation) {
        headers['Cross-Origin-Opener-Policy'] = 'same-origin';
        headers['Cross-Origin-Embedder-Policy'] = 'require-corp';
      }
      res.writeHead(200, headers);
      res.end(body);
    } catch {
      res.writeHead(404);
      res.end('404');
    }
  });
  return new Promise((resolve) => server.listen(port, '127.0.0.1', () => resolve(server)));
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const [root, port, flag] = process.argv.slice(2);
  await startServer(root, Number(port), { isolation: flag !== '--no-isolation' });
  console.log(`http://127.0.0.1:${port}/ (isolation=${flag !== '--no-isolation'})`);
}
