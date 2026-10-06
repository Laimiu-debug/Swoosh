import { readFile, writeFile } from "node:fs/promises";

const sourceUrl = new URL("../design/tokens.json", import.meta.url);
const outputUrl = new URL("../design/tokens.css", import.meta.url);
const tokens = JSON.parse(await readFile(sourceUrl, "utf8"));
const all = new Map();

function walk(group, path = []) {
  for (const [name, value] of Object.entries(group)) {
    if (name.startsWith("$")) continue;
    const next = [...path, name];
    if (value && Object.hasOwn(value, "$value")) {
      if (!value.$type) throw new Error(`Missing type: ${next.join(".")}`);
      all.set(next.join("."), value);
    } else if (value && typeof value === "object" && !Array.isArray(value)) {
      walk(value, next);
    } else {
      throw new Error(`Invalid token group: ${next.join(".")}`);
    }
  }
}
walk(tokens);

function variable(path) {
  return "--sw-" + path.split(".").map(part => part.replace(/[A-Z]/g, letter => `-${letter.toLowerCase()}`)).join("-");
}

function resolve(path, chain = []) {
  if (chain.includes(path)) throw new Error(`Circular reference: ${[...chain, path].join(" -> ")}`);
  const token = all.get(path);
  if (!token) throw new Error(`Unknown token: ${path}`);
  const value = token.$value;
  const reference = typeof value === "string" && value.match(/^\{([^}]+)\}$/);
  if (reference) {
    const target = all.get(reference[1]);
    if (target?.$type !== token.$type) throw new Error(`Reference type mismatch: ${path}`);
    return resolve(reference[1], [...chain, path]);
  }
  if (token.$type === "cubicBezier" && Array.isArray(value) && value.length === 4 && value.every(Number.isFinite)) {
    return `cubic-bezier(${value.join(", ")})`;
  }
  if (typeof value === "string" || typeof value === "number") return String(value);
  throw new Error(`Unsupported token value: ${path}`);
}

function declaration(path, shortPath = path) {
  resolve(path);
  const value = all.get(path).$value;
  const reference = typeof value === "string" && value.match(/^\{([^}]+)\}$/);
  if (reference && reference[1].startsWith("themes.")) throw new Error(`Theme aliases must reference primitives: ${path}`);
  return `  ${variable(shortPath)}: ${reference ? `var(${variable(reference[1])})` : resolve(path)};`;
}

const base = [...all.keys()].filter(path => !path.startsWith("themes."));
const themeNames = Object.keys(tokens.themes);
if (themeNames.join(",") !== "light,dark") throw new Error("Expected light and dark themes");
const themePaths = themeNames.map(theme => [...all.keys()].filter(path => path.startsWith(`themes.${theme}.`)));
const semanticSets = themePaths.map((paths, index) => paths.map(path => path.replace(`themes.${themeNames[index]}.`, "")).sort());
if (JSON.stringify(semanticSets[0]) !== JSON.stringify(semanticSets[1])) throw new Error("Theme token sets differ");

let css = "/* Generated from design/tokens.json. Run node scripts/generate-tokens.mjs. */\n\n";
css += `:root {\n${base.map(path => declaration(path)).join("\n")}\n}\n\n`;
for (const [index, theme] of themeNames.entries()) {
  const prefix = `themes.${theme}.`;
  const selector = theme === "light" ? ":root,\n[data-theme=\"light\"]" : "[data-theme=\"dark\"]";
  css += `${selector} {\n  color-scheme: ${theme};\n${themePaths[index].map(path => declaration(path, path.slice(prefix.length))).join("\n")}\n}\n\n`;
}
css += "@media (prefers-color-scheme: dark) {\n  :root:not([data-theme]) {\n    color-scheme: dark;\n";
css += themePaths[1].map(path => "  " + declaration(path, path.slice("themes.dark.".length))).join("\n");
css += "\n  }\n}\n\n";
css += "@media (prefers-reduced-motion: reduce) {\n  :root {\n";
css += ["fast", "normal", "slow", "send"].map(name => `    --sw-motion-duration-${name}: var(--sw-motion-duration-reduced);`).join("\n");
css += "\n  }\n}\n";

if (process.argv.slice(2).some(arg => arg !== "--check")) throw new Error("Usage: node scripts/generate-tokens.mjs [--check]");
if (process.argv.includes("--check")) {
  const existing = await readFile(outputUrl, "utf8");
  if (existing !== css) throw new Error("design/tokens.css is out of date; regenerate it");
  console.log(`Token references and CSS verified: ${all.size} tokens, two matching themes.`);
} else {
  await writeFile(outputUrl, css, "utf8");
  console.log(`Generated design/tokens.css from ${all.size} tokens.`);
}
