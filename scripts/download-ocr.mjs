import { mkdir, readFile, rename, writeFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import { fileURLToPath } from "node:url";

const directory = fileURLToPath(new URL("../src-tauri/models/", import.meta.url));
const manifestPath = new URL("../ocr-models.json", import.meta.url);
const models = JSON.parse(await readFile(manifestPath, "utf8"));
await mkdir(directory, { recursive: true });
for (const model of models) {
  const destination = `${directory}/${model.file}`;
  let data;
  try { data = await readFile(destination); } catch { /* Download missing assets. */ }
  const hash = value => createHash("sha256").update(value).digest("hex");
  if (data && (!model.sha256 || hash(data) === model.sha256)) {
    console.log(`${model.file}: cached, SHA-256 ${hash(data)}`);
    continue;
  }
  console.log(`Downloading ${model.file}…`);
  const response = await fetch(model.url, { signal: AbortSignal.timeout(180_000) });
  if (!response.ok) throw new Error(`${model.file}: HTTP ${response.status}`);
  data = Buffer.from(await response.arrayBuffer());
  const digest = hash(data);
  if (model.sha256 && digest !== model.sha256) throw new Error(`${model.file}: unexpected SHA-256; cached file was not replaced`);
  await writeFile(`${destination}.partial`, data);
  await rename(`${destination}.partial`, destination);
  console.log(`${model.file}: ${data.length} bytes, SHA-256 ${digest}`);
}
