// Install QA dependencies on a standard hosted runner, never on the owner's computer.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { createServer } from 'node:http';
import { readFile, stat, mkdir, writeFile } from 'node:fs/promises';
import { resolve, join, extname, relative, isAbsolute } from 'node:path';

const require = createRequire(join(process.env.SITE_QA_DEPS, 'package.json'));
const { chromium } = require('playwright');
const axePath = require.resolve('axe-core/axe.min.js');
const root = resolve('_site');
const qaOutput = resolve('_site-qa');
await mkdir(qaOutput, { recursive: true });
const mime = { '.html': 'text/html; charset=utf-8', '.css': 'text/css', '.js': 'text/javascript', '.svg': 'image/svg+xml', '.png': 'image/png', '.xml': 'application/xml', '.txt': 'text/plain' };
const server = createServer(async (request, response) => {
  try {
    const url = new URL(request.url, 'http://localhost');
    if (!url.pathname.startsWith('/Worldcraft/')) {
      response.writeHead(404); response.end('Not found'); return;
    }
    let path = resolve(root, decodeURIComponent(url.pathname.slice('/Worldcraft/'.length)));
    const rel = relative(root, path);
    if (rel.startsWith('..') || isAbsolute(rel)) {
      response.writeHead(403); response.end('Forbidden'); return;
    }
    try { if ((await stat(path)).isDirectory()) path = join(path, 'index.html'); }
    catch { response.writeHead(404, { 'Content-Type': 'text/html' }); response.end(await readFile(join(root, '404.html'))); return; }
    response.writeHead(200, { 'Content-Type': mime[extname(path)] || 'application/octet-stream' });
    response.end(await readFile(path));
  } catch (error) { response.writeHead(500); response.end(String(error)); }
});
await new Promise(done => server.listen(0, '127.0.0.1', done));
const url = 'http://127.0.0.1:' + server.address().port + '/Worldcraft/';
const browser = await chromium.launch();
const findings = [];
const errors = [];
async function checkImages(page) {
  for (const image of await page.locator('img').all()) {
    await image.scrollIntoViewIfNeeded();
    await image.evaluate(item => item.decode());
  }
  const images = await page.locator('img').evaluateAll(items => items.map(item => ({ src: item.getAttribute('src'), complete: item.complete, width: item.naturalWidth })));
  assert(images.every(item => item.complete && item.width > 0), 'An image failed to load: ' + JSON.stringify(images));
}
async function checkOverflow(page, width) {
  const dimensions = await page.evaluate(() => ({ width: document.documentElement.clientWidth, scroll: document.documentElement.scrollWidth }));
  assert(dimensions.scroll <= dimensions.width + 1, 'Horizontal overflow at ' + width + ': ' + JSON.stringify(dimensions));
}
async function accessibility(page, label) {
  await page.addScriptTag({ path: axePath });
  const results = await page.evaluate(async () => window.axe.run(document, { runOnly: { type: 'tag', values: ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa'] } }));
  const violations = results.violations.map(item => ({ id: item.id, impact: item.impact, description: item.description, nodes: item.nodes.map(node => ({ target: node.target, summary: node.failureSummary })) }));
  findings.push({ label, violations });
  assert.equal(violations.length, 0, label + ' accessibility violations: ' + JSON.stringify(violations));
}
try {
  const desktop = await browser.newContext({ viewport: { width: 1440, height: 1000 }, deviceScaleFactor: 1, reducedMotion: 'reduce' });
  const page = await desktop.newPage();
  page.on('pageerror', error => errors.push(String(error)));
  page.on('response', response => { if (response.status() >= 400) errors.push(response.status() + ' ' + response.url()); });
  assert.equal((await page.goto(url)).status(), 200);
  await page.locator('#download').scrollIntoViewIfNeeded();
  await page.waitForLoadState('networkidle');
  await checkImages(page);
  await page.locator('#tab-coast').click();
  assert.equal(await page.locator('#scene-title').textContent(), 'Find a different shore.');
  await page.locator('#scene-image').evaluate(image => image.decode());
  await page.locator('#tab-coast').press('ArrowRight');
  assert.equal(await page.locator('#tab-learned').getAttribute('aria-selected'), 'true');
  assert.equal(await page.locator('#scene-counter').textContent(), '03 / 03');
  await page.locator('#scene-image').evaluate(image => image.decode());
  await page.locator('#tab-learned').press('Home');
  assert.equal(await page.locator('#tab-alpine').getAttribute('aria-selected'), 'true');
  await page.locator('#scene-image').evaluate(image => image.decode());
  assert.equal(await page.locator('[data-download=installer]').getAttribute('href'), 'https://github.com/InduwaraLasindu/Worldcraft/releases/download/v0.2.0-alpha.2-build.2/Worldcraft-Setup-0.2.0-alpha.2.exe');
  assert((await page.locator('[data-download=portable]').getAttribute('href')).endsWith('.zip'));
  await accessibility(page, 'Desktop');
  for (const width of [1440, 1024, 768]) {
    await page.setViewportSize({ width, height: 1000 });
    await checkOverflow(page, width);
  }
  await page.setViewportSize({ width: 1440, height: 1000 });
  await page.evaluate(() => window.scrollTo(0, 0));
  await page.screenshot({ path: join(qaOutput, 'desktop.png'), fullPage: true });
  const mobile = await browser.newContext({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 1, isMobile: true, hasTouch: true, reducedMotion: 'reduce' });
  const phone = await mobile.newPage();
  phone.on('pageerror', error => errors.push(String(error)));
  await phone.goto(url);
  await phone.locator('.menu-toggle').click();
  assert.equal(await phone.locator('.menu-toggle').getAttribute('aria-expanded'), 'true');
  await phone.locator('#main-nav a[href="#features"]').click();
  assert.equal(await phone.locator('.menu-toggle').getAttribute('aria-expanded'), 'false');
  await phone.locator('.menu-toggle').click();
  await phone.locator('.menu-toggle').press('Escape');
  assert.equal(await phone.locator('.menu-toggle').getAttribute('aria-expanded'), 'false');
  await phone.locator('#download').scrollIntoViewIfNeeded();
  await phone.waitForLoadState('networkidle');
  await checkImages(phone);
  await accessibility(phone, 'Mobile');
  for (const width of [390, 360, 320]) {
    await phone.setViewportSize({ width, height: 844 });
    await checkOverflow(phone, width);
  }
  await phone.setViewportSize({ width: 390, height: 844 });
  await phone.evaluate(() => window.scrollTo(0, 0));
  await phone.screenshot({ path: join(qaOutput, 'mobile.png'), fullPage: true });
  const noJs = await browser.newContext({ javaScriptEnabled: false, viewport: { width: 390, height: 844 } });
  const fallback = await noJs.newPage();
  await fallback.goto(url);
  assert(await fallback.locator('[data-download=installer]').isVisible());
  assert(await fallback.locator('#main-nav a[href="#explore"]').isVisible());
  await checkOverflow(fallback, 390);
  assert.equal((await fallback.goto(url + 'missing-page')).status(), 404);
  await fallback.getByRole('link', { name: 'Return to Worldcraft' }).click();
  assert.equal(new URL(fallback.url()).pathname, '/Worldcraft/');
  assert.equal(errors.length, 0, 'Browser errors: ' + JSON.stringify(errors));
  console.log('PASS: gallery clicks and keyboard navigation; mobile menu and Escape; 320–1440px overflow; all images; downloads; JS-free navigation; project-path 404 recovery; desktop/mobile WCAG A/AA automated checks; no browser errors.');
} finally {
  await writeFile(join(qaOutput, 'accessibility.json'), JSON.stringify(findings, null, 2));
  await browser.close();
  await new Promise(done => server.close(done));
}
