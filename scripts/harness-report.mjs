import { spawnSync } from "node:child_process";
import { chmodSync, mkdirSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
const directory = join(tmpdir(), "harnessy-harness-reports");
mkdirSync(directory, { recursive: true, mode: 0o700 });
chmodSync(directory, 0o700);
const result = spawnSync(
  process.execPath,
  ["node_modules/harness-score/dist/cli.js", ".", "--config", ".harness-score.json", "--json"],
  { cwd: root, encoding: "utf8" },
);
if (result.error) throw result.error;
if (result.stderr) process.stderr.write(result.stderr);
if (result.stdout) {
  const report = JSON.parse(result.stdout);
  const destination = join(directory, "current.json");
  writeFileSync(destination, `${JSON.stringify(report, null, 2)}\n`, { mode: 0o600 });
  chmodSync(destination, 0o600);
  console.log(destination);
  console.log(JSON.stringify({ score: report.score, level: report.level, scopes: report.scopes }));
}
process.exitCode = result.status ?? 2;
