// Abre, reconstrói ou instala o Harnessy.app deste projeto.
//   bun run app          abre o build; se não existir, gera antes
//   bun run app:build    gera o build de novo e abre
//   bun run app:install  gera o build e copia para ~/Applications
//   bun run app:dogfood  gera e abre uma SEGUNDA instância, "Harnessy Dogfood", com dados, ícone
//                        e branches próprios, para testar o código desta pasta sem tocar no seu
//                        Harnessy principal (use `-- --build-only` para só gerar)
//   bun run app:preview  roda o Harnessy desta pasta em modo de desenvolvimento, isolado
//                        (identificador próprio e porta própria); é o comando do "Rodar localmente"
//                        quando o projeto é o próprio Harnessy
import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { homedir } from "node:os";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const bundle = join(root, "src-tauri/target/release/bundle/macos/Harnessy.app");
const mode = process.argv[2] ?? "open";

function run(command, args, env = {}) {
  const result = spawnSync(command, args, {
    cwd: root,
    stdio: "inherit",
    env: { ...process.env, ...env },
  });
  if (result.status !== 0) process.exit(result.status ?? 1);
}

function build() {
  run("bunx", ["--bun", "tauri", "build", "--bundles", "app"]);
}

if (process.platform !== "darwin") {
  console.error("O build local do Harnessy é só para macOS.");
  process.exit(1);
}
if (!["open", "build", "install", "dogfood", "preview"].includes(mode)) {
  console.error("Uso: bun scripts/app.mjs [open|build|install|dogfood|preview]");
  process.exit(1);
}

// Instâncias de teste: o identificador decide onde ficam os dados (~/Library/Application Support/<id>),
// então um identificador próprio separa banco, preferências e WebView da instância principal.
if (mode === "dogfood") {
  const name = "Harnessy Dogfood";
  const config = JSON.stringify({ identifier: "app.harnessy.desktop.dogfood", productName: name });
  // Alvo de compilação próprio: alternar entre o build normal e este não joga o cache do outro fora.
  const target = join(root, "src-tauri/target/dogfood");
  const app = join(target, "release/bundle/macos", `${name}.app`);
  run("bunx", ["--bun", "tauri", "build", "--bundles", "app", "--config", config], {
    CARGO_TARGET_DIR: target,
  });
  if (process.argv.includes("--build-only")) {
    console.log(`Gerado em ${app}`);
    process.exit(0);
  }
  // Encerra a instância de teste anterior; a principal não é tocada.
  spawnSync("osascript", ["-e", `tell application "${name}" to quit`]);
  console.log(`Abrindo ${app}`);
  run("open", [app]);
  process.exit(0);
}

if (mode === "preview") {
  // A porta vem do "Rodar localmente" (marcador {port}) ou do ambiente; sem nenhuma, uma fixa.
  const port = process.argv[3] ?? process.env.PORT ?? "5391";
  if (!/^\d{4,5}$/.test(port)) {
    console.error(`Porta inválida: ${port}`);
    process.exit(1);
  }
  const config = JSON.stringify({
    identifier: "app.harnessy.desktop.preview",
    productName: "Harnessy Preview",
    build: {
      devUrl: `http://127.0.0.1:${port}`,
      beforeDevCommand: `bunx --bun vite --host 127.0.0.1 --port ${port} --strictPort`,
    },
  });
  run("bunx", ["--bun", "tauri", "dev", "--config", config]);
  process.exit(0);
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
