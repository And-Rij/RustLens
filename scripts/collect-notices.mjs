import { readFile, writeFile, readdir, mkdir, copyFile } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const root = fileURLToPath(new URL('../', import.meta.url));
const target = path.join(root, 'src-tauri/licenses');
await mkdir(target, { recursive: true });
const sections = ['RustLens dependency notices\nGenerated from installed packages.\nIncludes Rust packages resolved for Windows and installed frontend/build packages.\nExtra platform/build packages may be listed. Each package retains its own license.'];
const missing = [];
const fetchMissing = process.argv.includes('--fetch-missing');
const cache = path.join(root, 'scripts/license-cache');
await mkdir(cache, { recursive: true });

async function upstream(repository) {
  const match = repository?.replace(/^git\+/, '').match(/^https:\/\/github\.com\/([\w.-]+)\/([\w.-]+)/);
  if (!match) return '';
  const repo = `${match[1]}/${match[2].replace(/\.git$/, '')}`;
  const filename = path.join(cache, repo.replace('/', '--') + '.txt');
  try { return await readFile(filename, 'utf8'); } catch { /* No cached notice yet. */ }
  if (!fetchMissing) return '';
  if (repo === 'servo/stylo') {
    const url = 'https://www.mozilla.org/media/MPL/2.0/index.815ca599c9df.txt';
    const response = await fetch(url, { signal: AbortSignal.timeout(30_000) });
    if (!response.ok) throw new Error(`MPL license: HTTP ${response.status}`);
    const text = `Source: https://github.com/servo/stylo (MPL-2.0)\nLicense text: ${url}\n${await response.text()}`;
    await writeFile(filename, text);
    return text;
  }
  const response = await fetch(`https://api.github.com/repos/${repo}/license`, { headers: { 'User-Agent': 'RustLens-notices' }, signal: AbortSignal.timeout(30_000) });
  if (!response.ok) throw new Error(`License for ${repo}: HTTP ${response.status}`);
  const result = await response.json();
  const text = `Upstream license source: ${result.html_url}\n${Buffer.from(result.content, 'base64').toString('utf8')}`;
  await writeFile(filename, text);
  return text;
}

async function notice(name, license, directory, extra = [], repository) {
  const files = (await readdir(directory)).filter(file => /^(licen[sc]e|copying|copyright|notice)([._-]|$)/i.test(file));
  const content = [];
  for (const file of new Set([...files, ...extra])) {
    try { content.push(`${file}\n${await readFile(path.join(directory, file), 'utf8')}`); }
    catch { /* Directory or license file supplied in metadata not included in package. */ }
  }
  if (!content.length) {
    const remote = await upstream(repository);
    if (remote) content.push(remote);
    else missing.push(name);
  }
  sections.push(`\n===== ${name} =====\nDeclared license: ${license ?? 'See package source'}\nSource repository: ${repository ?? 'See package registry'}\n${content.join('\n\n') || 'No standalone license text in installed package; see upstream source and package license metadata.'}`);
}

const metadata = spawnSync('cargo', ['metadata', '--manifest-path', 'src-tauri/Cargo.toml', '--locked', ...(process.argv.includes('--offline') ? ['--offline'] : []), '--format-version', '1', '--filter-platform', 'x86_64-pc-windows-msvc'], { cwd: root, encoding: 'utf8', maxBuffer: 20 * 1024 * 1024 });
if (metadata.status !== 0) throw new Error(metadata.stderr || 'cargo metadata failed');
const cargo = JSON.parse(metadata.stdout);
const resolved = new Set(cargo.resolve.nodes.map(node => node.id));
for (const pkg of cargo.packages.filter(pkg => pkg.source && resolved.has(pkg.id)).sort((a, b) => a.name.localeCompare(b.name))) {
  await notice(`${pkg.name} ${pkg.version} (Rust)`, pkg.license, path.dirname(pkg.manifest_path), pkg.license_file ? [pkg.license_file] : [], pkg.repository);
}

const store = path.join(root, 'node_modules/.pnpm');
const directories = (await readdir(store, { withFileTypes: true })).filter(entry => entry.isDirectory() && entry.name !== 'node_modules').sort((a, b) => a.name.localeCompare(b.name));
const seen = new Set();
async function npmPackages(directory) {
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    if (!entry.isDirectory()) continue; // Symlinked dependencies are enumerated in their own store directory.
    const dest = path.join(directory, entry.name);
    if (entry.name.startsWith('@')) { await npmPackages(dest); continue; }
    let pkg;
    try { pkg = JSON.parse(await readFile(path.join(dest, 'package.json'), 'utf8')); } catch { continue; }
    const key = `${pkg.name}@${pkg.version}`;
    if (seen.has(key)) continue;
    seen.add(key);
    await notice(`${key} (JavaScript)`, pkg.license, dest, [], typeof pkg.repository === 'string' ? pkg.repository : pkg.repository?.url);
  }
}
for (const entry of directories) await npmPackages(path.join(store, entry.name, 'node_modules'));

await writeFile(path.join(target, 'DEPENDENCIES.txt'), sections.join('\n') + '\n');
await writeFile(path.join(target, 'SOURCE-OFFER.txt'), 'Unmodified MPL-2.0 selectors dependency source is available from:\nhttps://crates.io/crates/selectors\nhttps://github.com/servo/stylo/tree/main/selectors\nThe precise version is recorded in Cargo.lock and DEPENDENCIES.txt. Download the matching crate archive from crates.io for the distributed source version.\n');
await copyFile(path.join(root, 'LICENSE'), path.join(target, 'RUSTLENS-MIT.txt'));
console.log(`Collected ${sections.length - 1} dependency notices.`);
if (missing.length) console.log(`Packages with license metadata but no standalone license text (${missing.length}): ${missing.join(', ')}`);
