import { test } from 'node:test';
import assert from 'node:assert/strict';
import { marks } from './format.ts';

test('marks every search word, longest first, ignoring case', () => {
  assert.deepEqual(marks('Compile a C program', 'c compile'), [{ t: 'Compile', m: true }, { t: ' a ', m: false }, { t: 'C', m: true }, { t: ' program', m: false }]);
});
test('marks treats regex characters as text', () => {
  assert.deepEqual(marks('a+b (c)', '(c)'), [{ t: 'a+b ', m: false }, { t: '(c)', m: true }]);
  assert.deepEqual(marks('plain', '  '), [{ t: 'plain', m: false }]);
});
