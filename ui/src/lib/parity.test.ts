import test from 'node:test';
import assert from 'node:assert/strict';
import { mockIPC, clearMocks } from '@tauri-apps/api/mocks';
import { Channel } from '@tauri-apps/api/core';
import { api, chatStream, updateCommand, updateState } from './api.ts';
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

test('escape-first formatting is preserved', () => {
  assert.equal(esc('<img src=x onerror="x">&\''), '&lt;img src=x onerror=&quot;x&quot;&gt;&amp;&#39;');
  assert.equal(fmt('**bold** `code` <script>'), '<strong>bold</strong> <code>code</code> &lt;script&gt;');
  assert.equal(fmt('what they *would* pay, 2*3*4, a * b * c'), 'what they <em>would</em> pay, 2*3*4, a * b * c');
  const text = fmtTutor('video 2 at 3:45\n```html\n<img onerror=x>```');
  assert.ok(text.includes('data-seek="1" data-at="225"'));
  assert.ok(text.includes('&lt;img onerror=x&gt;'));
  assert.ok(!text.includes('<img'));
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


test('updater commands have no payload and background failures stay quiet', async () => {
  Object.defineProperty(globalThis, 'window', { value: { crypto: globalThis.crypto }, configurable: true });
  try {
    for (const command of ['update_status', 'update_check', 'update_install'] as const) {
      mockIPC((cmd, args) => { assert.equal(cmd, command); assert.deepEqual(args, {}); return null; });
      await updateCommand(command);
    }
    mockIPC(() => { throw new Error('404'); });
    await assert.rejects(updateCommand('update_check'), /404/);
    assert.equal(updateState('error', false), '');
    assert.equal(updateState('error', true), 'error');
    assert.equal(updateState('ready', false), 'ready');
  } finally { clearMocks(); Reflect.deleteProperty(globalThis, 'window'); }
});

test('video preferences and plan previews preserve IPC bodies and opaque plans', async () => {
  Object.defineProperty(globalThis, 'window', { value: { crypto: globalThis.crypto }, configurable: true });
  const prefs = { style: 'gems', length: 'long', lang: 'Hindi', recent: 'yes' };
  const plan = { phases: [{ title: 'Projects', lessons: [{ title: 'Build it' }] }], opaque: { keep: true } };
  const preview = { options: [{ label: 'Practical', note: '', added: [], dropped: [], changed: [], plan }], note: '' };
  try {
    for (const [method, path, body, response] of [
      ['PUT', 'api/video-prefs', prefs, {}],
      ['PUT', 'api/courses/c/prefs', { ...prefs, refresh: true }, {}],
      ['POST', 'api/courses/c/revise', { ask: 'More projects' }, preview],
      ...['swap', 'harder', 'easier', 'deeper', 'remove'].map(action => ['POST', 'api/courses/c/edit', { lesson: 'a', action }, preview]),
      ['POST', 'api/courses/c/apply', { plan, ask: 'More projects' }, { course: {}, change: {} }],
      ['POST', 'api/courses/c/apply', { plan }, { course: {}, change: {} }],
    ] as const) {
      mockIPC((cmd, args) => { assert.equal(cmd, 'api'); assert.deepEqual(args, { method, path, body }); return response; });
      assert.deepEqual(await api(method as 'PUT' | 'POST', path as string, body as object), response);
    }
    mockIPC(() => { throw { status: 400, message: 'This lesson has already been started.' }; });
    await assert.rejects(api('POST', 'api/courses/c/edit', { lesson: 'a', action: 'remove' }), /already been started/);
  } finally { clearMocks(); Reflect.deleteProperty(globalThis, 'window'); }
});
