import { vi, test } from 'vitest';
import assert from 'node:assert/strict';
import { createDraftSession, DRAFT_KEY, DRAFT_LIMIT } from '../src/lib/drafts.ts';

function memoryStorage(initial: string | null = null) {
  let raw = initial;
  let fail = false;
  let writes = 0;
  return {
    getItem: (_key: string) => raw,
    setItem(key: string, value: string) {
      assert.equal(key, DRAFT_KEY);
      if (fail) throw new Error('Storage unavailable');
      raw = value;
      writes++;
    },
    get raw() { return raw; },
    get writes() { return writes; },
    set fail(value: boolean) { fail = value; },
  };
}

test('opening and flushing a valid or absent record never writes without an edit', () => {
  for (const raw of [null, JSON.stringify({ schemaVersion: 1, text: 'Existing task' })]) {
    const storage = memoryStorage(raw);
    const session = createDraftSession(() => storage);
    session.flush();
    assert.equal(storage.writes, 0);
    assert.equal(storage.raw, raw);
  }
});

test('corrupt, incompatible and oversized records survive opening, editing and flushing', () => {
  for (const raw of ['{invalid-json', JSON.stringify({ schemaVersion: 2, text: 'Future task' }), JSON.stringify({ schemaVersion: 1, text: 'x'.repeat(DRAFT_LIMIT + 1) })]) {
    const storage = memoryStorage(raw);
    const session = createDraftSession(() => storage);
    assert.equal(session.getSnapshot().status, 'blocked');
    session.edit('New task in memory');
    session.flush();
    assert.equal(storage.raw, raw);
    assert.equal(storage.writes, 0);
    assert.equal(session.getSnapshot().text, 'New task in memory');
    session.replaceUnreadableRecord();
    assert.deepEqual(JSON.parse(storage.raw!), { schemaVersion: 1, text: 'New task in memory' });
  }
});

test('failed reads cannot silently overwrite the original record', () => {
  let stored = 'Original inaccessible data';
  const session = createDraftSession(() => ({
    getItem() { throw new Error('Read failure'); },
    setItem(_key, value) { stored = value; },
  }));
  session.edit('New text');
  session.flush();
  assert.equal(stored, 'Original inaccessible data');
  assert.equal(session.getSnapshot().status, 'blocked');
});

test('failed writes preserve memory across unsubscribe/remount and allow retry', () => {
  const storage = memoryStorage();
  storage.fail = true;
  const session = createDraftSession(() => storage);
  const unsubscribe = session.subscribe(() => {});
  session.edit('Task not yet saved');
  session.flush();
  assert.equal(session.getSnapshot().status, 'error');
  unsubscribe();
  const unsubscribeAgain = session.subscribe(() => {});
  assert.equal(session.getSnapshot().text, 'Task not yet saved');
  storage.fail = false;
  session.flush();
  assert.equal(session.getSnapshot().status, 'saved');
  assert.equal(JSON.parse(storage.raw!).text, 'Task not yet saved');
  unsubscribeAgain();
});

test('suggestions preserve literal whitespace, existing text and length limit', () => {
  const session = createDraftSession(() => memoryStorage());
  session.edit(' \n ');
  session.appendSuggestion('Review');
  assert.equal(session.getSnapshot().text, ' \n \n\nReview');
  session.edit('x'.repeat(DRAFT_LIMIT));
  session.appendSuggestion('Review');
  assert.equal(session.getSnapshot().text, 'x'.repeat(DRAFT_LIMIT));
  session.flush();
});

test('edits are debounced and a navigation flush writes the latest version only', () => {
  vi.useFakeTimers();
  try {
    const storage = memoryStorage();
    const session = createDraftSession(() => storage);
    session.edit('First');
    vi.advanceTimersByTime(200);
    session.edit('Latest');
    vi.advanceTimersByTime(299);
    assert.equal(storage.writes, 0);
    vi.advanceTimersByTime(1);
    assert.equal(storage.writes, 1);
    assert.equal(JSON.parse(storage.raw!).text, 'Latest');
    session.flush();
    assert.equal(storage.writes, 1);
  } finally {
    vi.useRealTimers();
  }
});
