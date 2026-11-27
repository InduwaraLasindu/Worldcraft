// Check the actual deployable package, including project-path resolution.
import assert from 'node:assert/strict';
import { readFile, stat, readdir } from 'node:fs/promises';
import { resolve, join } from 'node:path';

const root = resolve('_site');
const index = await readFile(join(root, 'index.html'), 'utf8');
assert.match(index, /<html lang="en">/);
assert.match(index, /<title>Worldcraft/);
assert.match(index, /name="viewport"/);
assert.match(index, /Engineering alpha/i);
assert.match(index, /installer is unsigned/);
assert.match(index, /finite regional bake/);
assert.equal((index.match(/<h1\b/g) || []).length, 1);
const ids = [...index.matchAll(/\bid="([^"]+)"/g)].map(match => match[1]);
assert.equal(ids.length, new Set(ids).size, 'Duplicate element IDs');
const localUrls = [];
for (const match of index.matchAll(/\b(?:href|src)="([^"]+)"/g)) {
  const value = match[1];
  if (value.startsWith('#')) {
    assert(ids.includes(value.slice(1)), 'Missing anchor: ' + value);
  } else if (!/^(?:https?:|data:)/.test(value)) {
    const url = new URL(value, 'https://example.test/Worldcraft/');
    assert(url.pathname.startsWith('/Worldcraft/'), 'Escapes project path: ' + value);
    const filename = join(root, decodeURIComponent(url.pathname.slice('/Worldcraft/'.length)));
    await stat(filename);
    localUrls.push(value);
  }
}
for (const required of ['404.html', 'favicon.svg', 'contours.svg', 'robots.txt', 'sitemap.xml', '.nojekyll']) {
  await stat(join(root, required));
}
for (const image of await readdir(join(root, 'assets'))) {
  const bytes = await readFile(join(root, 'assets', image));
  assert.equal(bytes.subarray(1, 4).toString(), 'PNG', 'Expected a PNG: ' + image);
}
const js = await readFile(join(root, 'app.js'), 'utf8');
assert(!/\b(?:eval|fetch|XMLHttpRequest)\s*\(/.test(js), 'Unexpected remote runtime dependency');
console.log('Packaged HTML, anchors, three capture assets, metadata and project-relative links passed.');
