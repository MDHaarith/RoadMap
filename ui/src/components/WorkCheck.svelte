<script lang="ts" module>
  import type { WorkCheck as Result } from '../lib/types';
  // Results are kept per lesson while the app is open, so leaving the step and coming back keeps them.
  const results = new Map<string, Result>();
</script>
<script lang="ts">
  import { untrack } from 'svelte';
  import { api } from '../lib/api';
  import { message } from '../lib/state.svelte';
  import { enc } from '../lib/format';
  import type { Course, Lesson, WorkCheck } from '../lib/types';
  import Icon from './Icon.svelte';
  import Formatted from './Formatted.svelte';
  import Waiting from './Waiting.svelte';
  import LoadError from './LoadError.svelte';
  import BusyLabel from './BusyLabel.svelte';
  let { c, l }: { c: Course; l: Lesson } = $props();
  let busy = $state(false), error = $state(''), result = $state<WorkCheck | undefined>(untrack(() => results.get(c.id + '/' + l.id)));
  const VERDICT = { done: 'Your project shows this lesson done', partly: 'Partly there', not_yet: 'Not there yet' };
  async function check() {
    busy = true; error = '';
    try { result = await api<WorkCheck>('POST', 'api/courses/' + enc(c.id) + '/check', { lesson: l.id }); results.set(c.id + '/' + l.id, result); }
    catch (e) { error = message(e); } finally { busy = false; }
  }
</script>
<section class="work-check" aria-labelledby="work-check-title">
  <div class="row between"><h3 id="work-check-title">Check my work</h3><button type="button" class="btn secondary sm" id="check-work" disabled={busy} onclick={() => void check()}><BusyLabel {busy} idle={result ? 'Check again' : 'Check my project'} label="Reading your project…" /></button></div>
  <p class="small">The AI reads your project folder and your changes since the last commit, and says which tasks it can see done. Parts of your files are sent to your AI service. It ticks nothing for you.</p>
  {#if busy}<Waiting skeleton="text" inline title="Reading your project" hint="usually under a minute" />{/if}
  {#if error}<LoadError what="Your project could not be checked." {error} retry={() => void check()} />{/if}
  {#if result && !busy}<div class="note" class:ok={result.verdict === 'done'}><strong>{VERDICT[result.verdict]}</strong><p><Formatted text={result.summary} /></p>
    <ul>{#each result.items as x}<li><span class="point-mark" class:complete-mark={x.met} aria-hidden="true">{#if x.met}<Icon name="check" />{/if}</span><div><span class="sr">{x.met ? 'Done:' : 'Not done yet:'}</span><b><Formatted text={x.task} /></b><p><Formatted text={x.note} /></p></div></li>{/each}</ul>
    {#if result.next}<p><strong>Next:</strong> <Formatted text={result.next} /></p>{/if}</div>{/if}
</section>
