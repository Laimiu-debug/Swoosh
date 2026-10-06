import { createRequire } from 'node:module';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';

const args = process.argv.slice(2);
if (args.length !== 2 || args[0] !== '--sharp-dir') {
  throw new Error('Usage: node scripts/generate-store-assets.mjs --sharp-dir /path/to/sharp');
}
const sharp = createRequire(import.meta.url)(resolve(args[1]));
const root = new URL('../', import.meta.url);
const icon = await readFile(new URL('design/brand/app-icon-square.svg', root));
const assets = new URL('packaging/windows/Assets/', root);
const listing = new URL('store/assets/', root);
await Promise.all([mkdir(assets, { recursive: true }), mkdir(listing, { recursive: true })]);
for (const [name, size] of [['StoreLogo', 50], ['Square44x44Logo', 44], ['Square150x150Logo', 150]]) {
  for (const scale of [100, 200, 400]) {
    const filename = scale === 100 ? `${name}.png` : `${name}.scale-${scale}.png`;
    await writeFile(new URL(filename, assets), await sharp(icon).resize(size * scale / 100).png().toBuffer());
  }
}
for (const scale of [100, 200, 400]) {
  const factor = scale / 100;
  const mark = await sharp(icon).resize(120 * factor).png().toBuffer();
  const png = await sharp({ create: { width: 310 * factor, height: 150 * factor, channels: 4, background: '#087F6B' } })
    .composite([{ input: mark, gravity: 'centre' }]).png().toBuffer();
  await writeFile(new URL(scale === 100 ? 'Wide310x150Logo.png' : `Wide310x150Logo.scale-${scale}.png`, assets), png);
}
await writeFile(new URL('store-logo-300.png', listing), await sharp(icon).resize(300).png().toBuffer());
console.log('Generated MSIX assets at 100%, 200%, 400% and a 300px Store listing logo.');
