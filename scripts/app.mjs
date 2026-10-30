// Abre, reconstrói ou instala o Harnessy.app deste projeto.
//   bun run app          abre o build; se não existir, gera antes
//   bun run app:build    gera o build de novo e abre
//   bun run app:install  gera o build e copia para ~/Applications
import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { homedir } from "node:os";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const bundle = join(root, "src-tauri/target/release/bundle/macos/Harnessy.app");
const mode = process.argv[2] ?? "open";

function run(command, args) {
  const result = spawnSync(command, args, { cwd: root, stdio: "inherit" });
  if (result.status !== 0) process.exit(result.status ?? 1);
}

function build() {
  run("bunx", ["--bun", "tauri", "build", "--bundles", "app"]);
}

if (process.platform !== "darwin") {
  console.error("O build local do Harnessy é só para macOS.");
  process.exit(1);
}
if (!["open", "build", "install"].includes(mode)) {
  console.error("Uso: bun scripts/app.mjs [open|build|install]");
  process.exit(1);
}

if (mode !== "open" || !existsSync(bundle)) build();

if (mode === "install") {
  const target = join(homedir(), "Applications");
  // Encerra a cópia em execução antes de substituí-la.
  spawnSync("osascript", ["-e", 'tell application "Harnessy" to quit']);
  run("mkdir", ["-p", target]);
  run("rm", ["-rf", join(target, "Harnessy.app")]);
  run("cp", ["-R", bundle, target]);
  console.log(`Instalado em ${join(target, "Harnessy.app")}`);
  run("open", [join(target, "Harnessy.app")]);
} else {
  console.log(`Abrindo ${bundle}`);
  run("open", [bundle]);
}
