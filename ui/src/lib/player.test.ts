import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';

test('saved lesson playback skips a gap, pauses at the final boundary and resumes in context', () => {
  const messages: Record<string, unknown>[] = [];
  const parent = { postMessage: (msg: Record<string, unknown>) => messages.push(msg) };
  const element = () => Object.assign(new EventTarget(), {
    classList: { toggle() {}, remove() {}, add() {} }, firstChild: { textContent: '' },
    style: {}, replaceWith() {}, setAttribute() {}
  });
  const video = Object.assign(element(), {
    duration: 300, currentTime: 0, paused: false, ended: false,
    play() { this.paused = false; return Promise.resolve(); },
    pause() { this.paused = true; }
  });
  const document = Object.assign(element(), { getElementById: element, createElement: () => video });
  const window = new EventTarget();
  let tick = () => {};
  runInNewContext(readFileSync(new URL('../../../src/player/player.js', import.meta.url), 'utf8'), {
    window, document, parent, URLSearchParams, location: { search: '?local=1&v=abcdefghijk&start=10&parts=10-20,100-120' },
    setTimeout() {}, clearTimeout() {}, setInterval(fn: () => void) { tick = fn; }
  });
  video.dispatchEvent(new Event('loadedmetadata'));
  assert.equal(video.currentTime, 10);
  video.currentTime = 20; tick();
  assert.equal(video.currentTime, 100);
  video.currentTime = 120; tick();
  assert.equal(video.paused, true);
  assert.ok(messages.some(m => m.roadmapPartDone));
  window.dispatchEvent(Object.assign(new Event('message'), { source: parent, data: { roadmapCommand: 'context' } }));
  assert.equal(video.currentTime, 60);
  assert.equal(video.paused, false);
  video.currentTime = 125; tick();
  assert.equal(video.paused, false);
});
