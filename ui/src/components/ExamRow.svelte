<script lang="ts">
  import { examPassed, examReady, stageLessons } from '../lib/progress';
  import { href } from '../lib/format';
  import type { Course } from '../lib/types';
  import Icon from './Icon.svelte';
  let { c, stage: s }: { c: Course; stage: number } = $props();
  let result = $derived(c.state.exams?.[s]), ok = $derived(examPassed(c, s));
  let finished = $derived(stageLessons(c, s).every(l => c.state.done[l.id]));
</script>
<div class="exam-row" class:done={ok}><span><Icon name="flag" /> <b>Stage {s + 1} exam</b>{#if result} · {result.score} of {result.total}{ok ? ' · Passed' : ''}{/if}</span>{#if examReady(c, s)}<a class={ok ? 'link' : 'btn'} href={`${href(c.id)}/exam/${s}`}>{ok ? 'Take it again' : result ? 'Try again' : 'Take the exam'}</a>{:else}<span class="small">{finished ? 'No exam yet: it needs at least five quiz questions from this stage’s lessons.' : 'Opens when every lesson of this stage is complete.'}</span>{/if}</div>
