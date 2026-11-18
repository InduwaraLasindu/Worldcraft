// Run only on a hosted runner; the native Rust application is not built.
import { cp, mkdir, copyFile } from 'node:fs/promises';
import { resolve } from 'node:path';

const output = resolve('_site');
await mkdir(output, { recursive: true });
await cp(resolve('website'), output, { recursive: true, filter: path => !path.endsWith('README.md') });
await mkdir(resolve(output, 'assets'), { recursive: true });
for (const name of ['cloud-alpine.png', 'cloud-coast.png', 'alpha2-detailed-linux.png']) {
  await copyFile(resolve('docs/media', name), resolve(output, 'assets', name));
}
console.log('Static site packaged at ' + output);
