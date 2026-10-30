import { parseNumber } from "./parse.js";

/** Configuração do servidor a partir do ambiente. */
export function loadConfig(env) {
  return {
    host: env.HOST ?? "127.0.0.1",
    port: env.PORT === undefined ? 3000 : parseNumber(env.PORT),
  };
}
