import { readFileSync } from "node:fs";
import assert from "node:assert/strict";

const pkg = JSON.parse(readFileSync(new URL("../package.json", import.meta.url)));
const config = JSON.parse(readFileSync(new URL("../src-tauri/tauri.conf.json", import.meta.url)));
assert.match(pkg.version, /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/);
assert.equal(config.version, "../package.json", "Tauri deve ler a versão de package.json");
console.log(`Versão centralizada: ${pkg.version}`);
