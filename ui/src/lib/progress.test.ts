import { test } from 'node:test';
import assert from 'node:assert/strict';
import { whereAmI } from './progress.ts';
import type { Course } from './types.ts';

const course = (state: Partial<Course['state']>) => ({
  id: 'c', phases: [{ title: 'P', lessons: [
    { id: '1', title: 'Ownership', goal: 'g1' },
    { id: '2', title: 'Borrowing', goal: 'g2' },
    { id: '3', title: 'Lifetimes', goal: 'g3', checkpoint: { task: 'Build a cache', criteria: 'c', preparation: '' } },
  ] }],
  state: { checks: {}, done: {}, notes: {}, quiz: {}, code: {}, last: '', ...state },
}) as unknown as Course;

test('a shaky earlier lesson is recommended before new material', () => {
  const w = whereAmI(course({ done: { '1': true }, quiz: { '1': { score: 1, total: 5 } } }))!;
  assert.equal(w.next.id, '2');
  assert.equal(w.why.l.id, '1');
  assert.deepEqual(w.shaky.map(s => s.l.id), ['1']);
  assert.equal(w.milestone?.id, '3');
});

test('with nothing shaky the next lesson is recommended and shown work is listed', () => {
  const w = whereAmI(course({ done: { '1': true, '2': true }, quiz: { '1': { score: 5, total: 5 } } }))!;
  assert.equal(w.why.l.id, '3');
  assert.match(w.why.text, /milestone in this lesson/);
  assert.deepEqual(w.shown.map(s => s.text), ['Quiz 5 of 5']);
});
