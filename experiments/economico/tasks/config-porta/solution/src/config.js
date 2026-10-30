/** Configuração do servidor a partir do ambiente. */
export function loadConfig(env) {
  return {
    host: env.HOST ?? "127.0.0.1",
    port: env.PORT === undefined ? 3000 : port(env.PORT),
  };
}

function port(raw) {
  const text = raw.trim();
  const value = /^\d+$/.test(text) ? Number(text) : Number.NaN;
  if (!(value >= 1 && value <= 65535)) {
    throw new Error(`PORT inválida: "${raw}" (use um inteiro de 1 a 65535)`);
  }
  return value;
}
