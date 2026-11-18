import test from 'node:test';
import assert from 'node:assert/strict';
import { highlight } from './highlight.ts';

test('highlight colours C and Python and keeps the text exact', () => {
  const c = '#include <stdio.h>\nint main(void) { // say hi\n  printf("a < b %d\\n", 42);\n}';
  const out = highlight(c, 'c');
  assert.ok(out.includes('<span class="hl-k">#include</span>'));
  assert.ok(out.includes('<span class="hl-k">int</span> <span class="hl-f">main</span>'));
  assert.ok(out.includes('<span class="hl-c">// say hi</span>'));
  assert.ok(out.includes('<span class="hl-s">&quot;a &lt; b %d\\n&quot;</span>'));
  assert.ok(out.includes('<span class="hl-n">42</span>'));
  assert.equal(out.replace(/<[^>]+>/g, '').replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&quot;/g, '"').replace(/&#39;/g, "'").replace(/&amp;/g, '&'), c);
  const py = highlight("def f(x):\n    return 'it''s' # done", 'python');
  assert.ok(py.includes('<span class="hl-k">def</span> <span class="hl-f">f</span>') && py.includes('<span class="hl-c"># done</span>'));
});
