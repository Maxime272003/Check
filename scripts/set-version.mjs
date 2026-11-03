// Usage : node scripts/set-version.mjs 0.1.42
// Aligne la version dans package.json, tauri.conf.json et Cargo.toml (utilisé par la CI).
import { readFileSync, writeFileSync } from 'node:fs';

const v = process.argv[2];
if (!/^\d+\.\d+\.\d+$/.test(v ?? '')) {
  console.error('Version invalide :', v);
  process.exit(1);
}
const json = (p, fn) => {
  const o = JSON.parse(readFileSync(p, 'utf8'));
  fn(o);
  writeFileSync(p, JSON.stringify(o, null, 2) + '\n');
};
json('package.json', (o) => (o.version = v));
json('src-tauri/tauri.conf.json', (o) => (o.version = v));
const cargo = readFileSync('src-tauri/Cargo.toml', 'utf8').replace(/^version = ".*"$/m, `version = "${v}"`);
writeFileSync('src-tauri/Cargo.toml', cargo);
console.log('Version', v);
