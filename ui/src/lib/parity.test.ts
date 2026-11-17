import test from 'node:test';
import assert from 'node:assert/strict';
import { mockIPC, clearMocks } from '@tauri-apps/api/mocks';
import { Channel } from '@tauri-apps/api/core';
import { api, chatStream, updateCommand, updateState } from './api.ts';
import { esc, fmt, fmtTutor } from './format.ts';
import { checked, passed, locked, examReady, examPassed, nextLesson, quizSet } from './progress.ts';
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
      const payload = args as { body: object; onPiece: Channel<string>; onStatus: Channel<{ text: string; done: boolean }> };
      assert.deepEqual(payload.body, { stream: true });
      assert.ok(payload.onPiece instanceof Channel);
      payload.onStatus.onmessage({ text: 'Reading another lesson', done: false }); payload.onStatus.onmessage({ text: 'Read lesson 1.1', done: true });
      payload.onPiece.onmessage('hello '); payload.onPiece.onmessage('world'); return 'hello world';
    });
    const statuses: object[] = [];
    assert.equal(await chatStream({ stream: true }, p => pieces.push(p), s => statuses.push(s)), 'hello world');
    assert.equal(pieces.join(''), 'hello world');
    assert.deepEqual(statuses, [{ text: 'Reading another lesson', done: false }, { text: 'Read lesson 1.1', done: true }]);
  } finally { clearMocks(); Reflect.deleteProperty(globalThis, 'window'); }
});

test('escape-first formatting is preserved', () => {
  assert.equal(esc('<img src=x onerror="x">&\''), '&lt;img src=x onerror=&quot;x&quot;&gt;&amp;&#39;');
  assert.equal(fmt('**bold** `code` <script>'), '<strong>bold</strong> <code>code</code> &lt;script&gt;');
  assert.equal(fmt('what they *would* pay, 2*3*4, a * b * c'), 'what they <em>would</em> pay, 2*3*4, a * b * c');
  // Unbalanced emphasis: no raw markers, no lost text, code spans untouched.
  assert.equal(fmt('Short answer: **it depends on how you pay yourself*.'), 'Short answer: <strong>it depends on how you pay yourself</strong>.');
  assert.equal(fmt('*x** first'), '<strong>x</strong> first');
  assert.equal(fmt('then **名前*'), 'then <strong>名前</strong>');
  assert.equal(fmt('a stray** end and *start, lone * star, 2**3'), 'a stray end and start, lone * star, 2**3');
  assert.equal(fmt('`a*b **c*` **<b>*'), '<code>a*b **c*</code> <strong>&lt;b&gt;</strong>');
  assert.equal(fmt('* item'), '* item');
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
  // "New questions" appends to the bank: the quiz asks the newest set, the exam pool keeps all of it.
  assert.equal(quizSet(first).from, 0); assert.equal(quizSet(first).quiz.length, 5);
  first.practice.from = 5; first.practice.quiz!.push({ q: 'new', options: ['a', 'b'], answer: 1, why: '' });
  assert.deepEqual(quizSet(first), { from: 5, at: [5], quiz: [{ q: 'new', options: ['a', 'b'], answer: 1, why: '' }] });
  // Stage exam questions join the bank but stay out of the lesson quiz.
  first.practice.quiz!.push({ q: 'exam', options: ['a', 'b'], answer: 0, why: '', exam: true });
  assert.deepEqual(quizSet(first).at, [5]);
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

test('design text tokens retain AA contrast on their supported surfaces', async () => {
  const { readFile } = await import('node:fs/promises');
  const css = await readFile(new URL('../app.css', import.meta.url), 'utf8');
  const colors = Object.fromEntries([...css.matchAll(/--([\w-]+):\s*(#[\da-f]{6})\b/gi)].map(m => [m[1], m[2]]));
  function luminance(hex: string) {
    const linear = hex.slice(1).match(/../g)!.map(c => {
      const n = parseInt(c, 16) / 255;
      return n <= .04045 ? n / 12.92 : ((n + .055) / 1.055) ** 2.4;
    });
    return linear[0] * .2126 + linear[1] * .7152 + linear[2] * .0722;
  }
  const pairs = ['bg', 'bg-subtle', 'surface-2', 'hover', 'accent-soft', 'success-soft', 'danger-soft', 'warn-soft']
    .flatMap(bg => ['ink', 'ink-2', 'muted'].map(fg => [fg, bg]));
  pairs.push(['accent-ink', 'bg'], ['accent-ink', 'accent-soft'], ['on-accent', 'accent'], ['on-accent', 'accent-hover'], ['on-accent', 'ink'],
    ['success', 'success-soft'], ['danger', 'danger-soft'], ['warn', 'warn-soft'], ['player-fg', 'player'], ['player-link', 'player']);
  for (const [fg, bg] of pairs) {
    const a = luminance(colors[fg]), b = luminance(colors[bg]);
    const contrast = (Math.max(a, b) + .05) / (Math.min(a, b) + .05);
    assert.ok(contrast >= 4.5, `${fg} on ${bg}: ${contrast.toFixed(2)}:1`);
  }
});
