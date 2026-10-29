export const DRAFT_KEY = 'harnessy:ui-draft:v1';
export const DRAFT_LIMIT = 20000;

export interface DraftStorage {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
}

export type DraftStatus = 'empty' | 'saved' | 'saving' | 'error' | 'blocked';
export interface DraftSnapshot {
  text: string;
  status: DraftStatus;
}

// A session owns persistence and memory. Navigating between screens cannot destroy it.
// Storage is deliberately resolved inside try/catch: even accessing localStorage can fail.
export function createDraftSession(storage: () => DraftStorage = () => localStorage) {
  let blocked = false;
  let dirty = false;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let snapshot: DraftSnapshot = { text: '', status: 'empty' };
  const listeners = new Set<() => void>();

  try {
    const raw = storage().getItem(DRAFT_KEY);
    if (raw !== null) {
      const value: unknown = JSON.parse(raw);
      if (typeof value !== 'object' || value === null || !('schemaVersion' in value)
        || value.schemaVersion !== 1 || !('text' in value) || typeof value.text !== 'string'
        || value.text.length > DRAFT_LIMIT) {
        throw new Error('Unreadable or incompatible draft');
      }
      snapshot = { text: value.text, status: 'saved' };
    }
  } catch {
    blocked = true;
    snapshot = { text: '', status: 'blocked' };
  }

  function publish(next: DraftSnapshot) {
    snapshot = next;
    listeners.forEach(listener => listener());
  }

  function flush() {
    clearTimeout(timer);
    timer = undefined;
    if (!dirty || blocked) return;
    try {
      storage().setItem(DRAFT_KEY, JSON.stringify({ schemaVersion: 1, text: snapshot.text }));
      dirty = false;
      publish({ ...snapshot, status: 'saved' });
    } catch {
      // Keep the only current copy in memory, including across failed retries.
      publish({ ...snapshot, status: 'error' });
    }
  }

  function edit(text: string) {
    dirty = true;
    clearTimeout(timer);
    publish({ text: text.slice(0, DRAFT_LIMIT), status: blocked ? 'blocked' : 'saving' });
    if (!blocked) timer = setTimeout(flush, 300);
  }

  return {
    getSnapshot: () => snapshot,
    subscribe(listener: () => void) {
      listeners.add(listener);
      return () => { listeners.delete(listener); };
    },
    edit,
    appendSuggestion(text: string) {
      // Preserve literal whitespace and never truncate already-entered content.
      edit(snapshot.text.length ? `${snapshot.text}\n\n${text}` : text);
    },
    flush,
    replaceUnreadableRecord() {
      // Called only by an explicit user action explaining the replacement.
      blocked = false;
      dirty = true;
      flush();
    },
  };
}

export type DraftSession = ReturnType<typeof createDraftSession>;
