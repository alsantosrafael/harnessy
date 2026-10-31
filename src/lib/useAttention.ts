import { useCallback, useEffect, useRef, useState } from "react";
import { type Attention, mergeArrivals, notificationFor } from "./attention";
import { notifyAttention, setAttentionCount } from "./desktop";

export interface AttentionSignals {
  /** O que chegou enquanto a janela estava sem foco, mostrado ao voltar. */
  digest: Attention[];
  dismissDigest: () => void;
}

/**
 * Leva o que pede o humano para fora da tela: o contador na bandeja, uma notificação
 * quando algo novo chega com a janela sem foco e o resumo ao voltar. Ao abrir o app ou
 * trocar de workspace, o que já estava pendente não conta como novidade.
 */
export function useAttentionSignals(
  attention: ReadonlyMap<number, Attention>,
  ready: boolean,
  resetKey: unknown,
): AttentionSignals {
  const [digest, setDigest] = useState<Attention[]>([]);
  const known = useRef<Map<number, string> | null>(null);
  const away = useRef<Attention[]>([]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: `resetKey` é o gatilho de recomeço.
  useEffect(() => {
    known.current = null;
    away.current = [];
    setDigest([]);
  }, [resetKey]);

  useEffect(() => {
    if (!ready) return;
    setAttentionCount(attention.size).catch(() => undefined);
    const before = known.current;
    known.current = new Map([...attention].map(([id, item]) => [id, item.key]));
    if (before === null) return;
    const arrived = [...attention.values()].filter((item) => before.get(item.taskId) !== item.key);
    if (arrived.length === 0 || document.hasFocus()) return;
    away.current = mergeArrivals(away.current, arrived);
    const text = notificationFor(arrived);
    if (text) notifyAttention(text.title, text.body).catch(() => undefined);
  }, [attention, ready]);

  useEffect(() => {
    const onFocus = () => {
      if (away.current.length === 0) return;
      setDigest(away.current);
      away.current = [];
    };
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, []);

  const dismissDigest = useCallback(() => setDigest([]), []);
  return { digest, dismissDigest };
}
