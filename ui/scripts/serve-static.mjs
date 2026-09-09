#!/usr/bin/env node
/**
 * A static file server for the built catalogue, in forty lines and with no dependency.
 *
 * The browser tier runs against `storybook-static/`, not against the dev server, because
 * `build-storybook` *"fails on errors a dev server tolerates"* and a gate that ran against the
 * tolerant one would be testing a different artefact from the one CI produces. Serving it needs a
 * static server; adding a package for that would be a supply-chain entry and a build step to hold
 * a directory open on a port.
 *
 * Usage: `node scripts/serve-static.mjs <directory> <port>`
 */

import { createServer } from 'node:http';
import { createReadStream } from 'node:fs';
import { stat } from 'node:fs/promises';
import { extname, join, normalize, resolve, sep } from 'node:path';

const [, , directoryArgument = 'storybook-static', portArgument = '6007'] = process.argv;
const root = resolve(directoryArgument);
const port = Number(portArgument);

const types = new Map(
  Object.entries({
    '.html': 'text/html; charset=utf-8',
    '.js': 'text/javascript; charset=utf-8',
    '.mjs': 'text/javascript; charset=utf-8',
    '.css': 'text/css; charset=utf-8',
    '.json': 'application/json; charset=utf-8',
    '.png': 'image/png',
    '.jpg': 'image/jpeg',
    '.svg': 'image/svg+xml',
    '.woff2': 'font/woff2',
    '.map': 'application/json; charset=utf-8',
  }),
);

const server = createServer((request, response) => {
  const url = new URL(request.url ?? '/', 'http://localhost');
  const requested = decodeURIComponent(url.pathname);
  // Normalise before joining: a request for `/../../etc/passwd` must not escape the root, and this
  // server is pointed at a build directory on a developer's machine.
  const candidate = join(root, normalize(requested).replace(/^(\.\.[/\\])+/, ''));
  if (candidate !== root && !candidate.startsWith(root + sep)) {
    response.writeHead(403).end('forbidden');
    return;
  }

  void (async () => {
    let target = candidate;
    try {
      const info = await stat(target);
      if (info.isDirectory()) target = join(target, 'index.html');
      await stat(target);
    } catch {
      response.writeHead(404, { 'content-type': 'text/plain; charset=utf-8' }).end('not found');
      return;
    }
    response.writeHead(200, {
      'content-type': types.get(extname(target)) ?? 'application/octet-stream',
      'cache-control': 'no-store',
    });
    createReadStream(target).pipe(response);
  })();
});

server.listen(port, '127.0.0.1', () => {
  console.log(`serving ${root} on http://127.0.0.1:${String(port)}/`);
});
