import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { mockIPC, clearMocks } from '@tauri-apps/api/mocks';
import { Channel } from '@tauri-apps/api/core';
import { api, chatStream } from './api.ts';
import { esc, fmt, fmtTutor } from './format.ts';
import { checked, passed, locked, examReady, examPassed, nextLesson } from './progress.ts';
import type { Course, Lesson } from './types.ts';

test('IPC uses exact API paths, default bodies, structured errors, and a streaming Channel', async () => {
  Object.defineProperty(globalThis, 'window', { value: { crypto: globalThis.crypto }, configurable: true });
  try {
    mockIPC((cmd, args) => { assert.equal(cmd, 'api'); assert.deepEqual(args, { method: 'GET', path: 'api/settings', body: {} }); return { theme: 'dark' }; });
    assert.deepEqual(await api('GET', 'api/settings'), { theme: 'dark' });
    mockIPC((cmd, args) => { assert.equal(cmd, 'api'); assert.deepEqual(args, { method: 'PUT', path: 'api/theme', body: { theme: 'light' } }); throw { status: 403, message: 'Denied' }; });
    await assert.rejects(api('PUT', 'api/theme', { theme: 'light' }), (error: Error & { status?: number }) => error instanceof Error && error.message === 'Denied' && error.status === 403);
    const pieces: string[] = [];
    mockIPC((cmd, args) => {
      assert.equal(cmd, 'chat_stream');
      const payload = args as { body: object; onPiece: Channel<string> };
      assert.deepEqual(payload.body, { stream: true });
      assert.ok(payload.onPiece instanceof Channel);
      payload.onPiece.onmessage('hello '); payload.onPiece.onmessage('world'); return 'hello world';
    });
    assert.equal(await chatStream({ stream: true }, p => pieces.push(p)), 'hello world');
    assert.equal(pieces.join(''), 'hello world');
  } finally { clearMocks(); Reflect.deleteProperty(globalThis, 'window'); }
});

test('escape-first formatting and the complete original style layer are preserved', () => {
  assert.equal(esc('<img src=x onerror="x">&\''), '&lt;img src=x onerror=&quot;x&quot;&gt;&amp;&#39;');
  assert.equal(fmt('**bold** `code` <script>'), '<strong>bold</strong> <code>code</code> &lt;script&gt;');
  const text = fmtTutor('video 2 at 3:45\n```html\n<img onerror=x>```');
  assert.ok(text.includes('data-seek="1" data-at="225"'));
  assert.ok(text.includes('&lt;img onerror=x&gt;'));
  assert.ok(!text.includes('<img'));
  const original = readFileSync(new URL('../../../index.html', import.meta.url), 'utf8');
  assert.equal(readFileSync(new URL('../app.css', import.meta.url), 'utf8'), original.split('<style>')[1].split('</style>')[0]);
});

test('completion, stage locks, small quiz pools, and exam thresholds match the old frontend', () => {
  const first: Lesson = { id: 'a', title: 'First', goal: '', query: '', done: ['Build it'] };
  const second: Lesson = { id: 'b', title: 'Second', goal: '', query: '', done: ['Test it'] };
  const c: Course = { id: 'c', title: '', summary: '', prompt: '', phases: [{ title: '', blurb: '', stage: 0, lessons: [first] }, { title: '', blurb: '', stage: 1, lessons: [second] }], stages: [{ title: 'One' }, { title: 'Two' }], state: { checks: {}, done: {}, notes: {}, quiz: {}, code: {}, last: '' } };
  assert.deepEqual(checked(c, first), [false]);
  assert.equal(locked(c, second), true);
  c.state.checks['b#0'] = true; assert.equal(locked(c, second), false); delete c.state.checks['b#0'];
  c.state.done.a = true; assert.equal(locked(c, second), false); // A tiny pool must not deadlock the learner.
  first.practice = { quiz: Array.from({ length: 5 }, () => ({ q: '', options: ['a', 'b'], answer: 0, why: '' })) };
  assert.equal(examReady(c, 0), true); assert.equal(locked(c, second), true);
  c.state.exams = { 0: { score: 7, total: 10 } }; assert.equal(examPassed(c, 0), true); assert.equal(locked(c, second), false);
  c.state.quiz.b = { score: 2, total: 2 }; assert.equal(passed(c, second), false);
  c.state.quiz.b = { score: 4, total: 5 }; assert.equal(passed(c, second), true);
  assert.equal(nextLesson(c), second);
});
