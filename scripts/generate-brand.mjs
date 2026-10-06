import { mkdir, readFile, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { resolve } from "node:path";

const args = process.argv.slice(2);
const checkOnly = args.includes("--check");
const sharpIndex = args.indexOf("--sharp-dir");
const sharpPath = sharpIndex >= 0 ? args[sharpIndex + 1] : undefined;
const accepted = checkOnly ? ["--check"] : sharpPath ? ["--sharp-dir", sharpPath] : [];
if (JSON.stringify(args) !== JSON.stringify(accepted)) throw new Error("Usage: node scripts/generate-brand.mjs [--check | --sharp-dir /path/to/sharp]");

const projectUrl = new URL("../", import.meta.url);
const brandUrl = new URL("design/brand/", projectUrl);
const geometry = JSON.parse(await readFile(new URL("geometry.json", brandUrl), "utf8"));
const tokens = JSON.parse(await readFile(new URL("design/tokens.json", projectUrl), "utf8"));
const colors = {
  mint: tokens.palette.mint[700].$value,
  paper: tokens.palette.neutral[0].$value,
  wing: tokens.palette.mint[100].$value,
  beak: tokens.palette.apricot[200].$value
};
for (const name of ["body", "wing", "beak"]) {
  if (!/^[MLHVCQZmlhvcqz0-9.,\s-]+$/.test(geometry[name])) throw new Error(`Invalid SVG path: ${name}`);
}

function shapes(main = "currentColor", small = false, colored = false) {
  const { x, y } = geometry.eye;
  const radius = small ? geometry.eye.smallRadius : geometry.eye.radius;
  const eye = `M${(x - radius).toFixed(1)} ${y}a${radius} ${radius} 0 1 0 ${radius * 2} 0a${radius} ${radius} 0 1 0 -${radius * 2} 0Z`;
  // The eye is a transparent cutout, so the monochrome mark works on any background.
  return `<g transform="translate(${geometry.offset} ${geometry.offset})">\n<path d="${geometry.beak}" fill="${colored ? colors.beak : main}"/>\n<path d="${geometry.body} ${eye}" fill="${main}" fill-rule="evenodd"/>${colored && !small ? `\n<path d="${geometry.wing}" fill="${colors.wing}"/>` : ""}\n</g>`;
}

function mark(color) {
  return `<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="${geometry.viewBox}" color="${color}" role="img" aria-labelledby="title">\n<title id="title">Swoosh 飞行小信使标志</title>\n${shapes()}\n</svg>\n`;
}

function tile(square = false, small = false) {
  return `<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024" viewBox="${geometry.tile.viewBox}" role="img" aria-labelledby="title">\n<title id="title">Swoosh 应用图标</title>\n<rect width="256" height="256" rx="${square ? 0 : geometry.tile.radius}" fill="${colors.mint}"/>\n<g transform="translate(${geometry.tile.offset} ${geometry.tile.offset}) scale(${geometry.tile.scale})">\n${shapes(colors.paper, small, true)}\n</g>\n</svg>\n`;
}

const files = new Map([
  ["mark.svg", mark(colors.mint)],
  ["mark-white.svg", mark(colors.paper)],
  ["app-icon.svg", tile()],
  ["app-icon-square.svg", tile(true)],
  ["favicon.svg", tile(false, true)]
]);
const htmlUrl = new URL("design/preview.html", projectUrl);
const html = await readFile(htmlUrl, "utf8");
const marker = /<!-- swoosh-brand:start -->[\s\S]*?<!-- swoosh-brand:end -->/;
if (!marker.test(html)) throw new Error("Missing brand symbol markers in design/preview.html");
const symbol = `<!-- swoosh-brand:start -->\n      <symbol id="swoosh" viewBox="${geometry.viewBox}">\n${shapes()}\n      </symbol>\n      <!-- swoosh-brand:end -->`;
const nextHtml = html.replace(marker, () => symbol);

if (checkOnly) {
  for (const [name, svg] of files) {
    if (await readFile(new URL(name, brandUrl), "utf8") !== svg) throw new Error(`Outdated brand asset: ${name}`);
  }
  if (nextHtml !== html) throw new Error("Preview brand symbol is out of date");
  for (const [source, destination] of [["raster/swoosh-32.png", "32x32.png"], ["raster/swoosh-128.png", "128x128.png"], ["raster/swoosh-256.png", "128x128@2x.png"], ["icon.ico", "icon.ico"]]) {
    const sourceBytes = await readFile(new URL(source, brandUrl));
    const nativeBytes = await readFile(new URL(`src-tauri/icons/${destination}`, projectUrl));
    if (!sourceBytes.equals(nativeBytes)) throw new Error(`Outdated native icon: ${destination}`);
  }
  console.log("Brand geometry, SVG assets, inline symbol and Windows application icons are consistent.");
} else {
  await mkdir(brandUrl, { recursive: true });
  for (const [name, svg] of files) await writeFile(new URL(name, brandUrl), svg, "utf8");
  if (nextHtml !== html) await writeFile(htmlUrl, nextHtml, "utf8");
  console.log("Generated five SVG assets and synchronized the preview brand symbol.");
}

if (sharpPath) {
  const require = createRequire(import.meta.url);
  const sharp = require(resolve(sharpPath));
  const rasterUrl = new URL("raster/", brandUrl);
  await mkdir(rasterUrl, { recursive: true });
  const sizes = [16, 32, 48, 64, 128, 256, 512, 1024];
  const pngs = new Map();
  for (const size of sizes) {
    const png = await sharp(Buffer.from(files.get(size === 16 ? "favicon.svg" : "app-icon.svg"))).resize(size, size).ensureAlpha().png().toBuffer();
    pngs.set(size, png);
    await writeFile(new URL(`swoosh-${size}.png`, rasterUrl), png);
  }
  await writeFile(new URL("swoosh-square-1024.png", rasterUrl), await sharp(Buffer.from(files.get("app-icon-square.svg"))).ensureAlpha().png().toBuffer());
  const icoSizes = sizes.filter(size => size <= 256);
  const header = Buffer.alloc(6 + icoSizes.length * 16);
  header.writeUInt16LE(1, 2);
  header.writeUInt16LE(icoSizes.length, 4);
  let offset = header.length;
  icoSizes.forEach((size, index) => {
    const entry = 6 + index * 16;
    const png = pngs.get(size);
    header[entry] = size === 256 ? 0 : size;
    header[entry + 1] = size === 256 ? 0 : size;
    header.writeUInt16LE(1, entry + 4);
    header.writeUInt16LE(32, entry + 6);
    header.writeUInt32LE(png.length, entry + 8);
    header.writeUInt32LE(offset, entry + 12);
    offset += png.length;
  });
  const ico = Buffer.concat([header, ...icoSizes.map(size => pngs.get(size))]);
  await writeFile(new URL("icon.ico", brandUrl), ico);
  const nativeUrl = new URL("src-tauri/icons/", projectUrl);
  await mkdir(nativeUrl, { recursive: true });
  for (const [size, name] of [[32, "32x32.png"], [128, "128x128.png"], [256, "128x128@2x.png"]]) await writeFile(new URL(name, nativeUrl), pngs.get(size));
  await writeFile(new URL("icon.ico", nativeUrl), ico);
  console.log("Exported nine RGBA PNG assets, a six-size ICO and synchronized Windows application icons.");
}
