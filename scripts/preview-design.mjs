import { createReadStream } from "node:fs";
import { stat } from "node:fs/promises";
import { createServer } from "node:http";
import { dirname, extname, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const designRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../design");
const previewPort = Number(process.argv[2] ?? 4178);
if (!Number.isInteger(previewPort) || previewPort < 1 || previewPort > 65535) throw new Error("Port must be an integer between 1 and 65535");
const mime = { ".html": "text/html; charset=utf-8", ".css": "text/css; charset=utf-8", ".js": "text/javascript; charset=utf-8", ".json": "application/json; charset=utf-8", ".svg": "image/svg+xml", ".png": "image/png", ".ico": "image/vnd.microsoft.icon" };

const server = createServer(async (request, response) => {
  try {
    if (!["GET", "HEAD"].includes(request.method)) { response.writeHead(405); response.end(); return; }
    const pathname = decodeURIComponent(new URL(request.url, "http://127.0.0.1").pathname);
    const filePath = resolve(designRoot, `.${pathname === "/" ? "/preview.html" : pathname}`);
    if (!filePath.startsWith(designRoot + sep)) { response.writeHead(403); response.end(); return; }
    const details = await stat(filePath);
    if (!details.isFile()) { response.writeHead(404); response.end(); return; }
    response.writeHead(200, { "Content-Type": mime[extname(filePath)] ?? "application/octet-stream", "Cache-Control": "no-store", "X-Content-Type-Options": "nosniff" });
    if (request.method === "HEAD") response.end();
    else createReadStream(filePath).on("error", () => response.destroy()).pipe(response);
  } catch {
    response.writeHead(404); response.end();
  }
});
server.listen(previewPort, "127.0.0.1", () => console.log(`Swoosh design preview: http://127.0.0.1:${previewPort}`));
process.on("SIGINT", () => server.close(() => process.exit(0)));
