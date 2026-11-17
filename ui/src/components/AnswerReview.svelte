<script lang="ts">
  import type { Quiz } from '../lib/types';
  import Formatted from './Formatted.svelte';
  import Icon from './Icon.svelte';
  // picks[i] is the option chosen for qs[i]; -1 means the time ran out.
  let { qs, picks }: { qs: Quiz[]; picks: number[] } = $props();
  const letter = (i: number) => String.fromCharCode(65 + i);
</script>
<section class="answer-review" aria-labelledby="answer-review-title"><h3 id="answer-review-title">Review answers</h3><ol class="answer-list">
  {#each qs as q, i}{@const pick = picks[i] ?? -1}
    {#if pick === q.answer}<li class="answer-item"><details><summary><span class="answer-mark"><Icon name="check" /><span class="sr">Correct.</span></span><span class="answer-q"><Formatted text={q.q} /></span></summary><div class="answer-why"><Formatted text={q.why} /></div></details></li>
    {:else}<li class="answer-item missed"><p class="answer-head"><span class="answer-mark" aria-hidden="true">×</span><span class="sr">Missed.</span><span class="answer-q"><Formatted text={q.q} /></span></p>
      <div class="answer-body">{#if pick >= 0}<p class="option wrong"><span class="letter">{letter(pick)}</span><span class="grow"><span class="sr">Your answer: </span>{q.options[pick]}</span></p>{:else}<p class="answer-none">No answer. Time ran out.</p>{/if}
      <p class="option correct"><span class="letter">{letter(q.answer)}</span><span class="grow"><span class="sr">Correct answer: </span>{q.options[q.answer]}</span></p>
      {#if q.why}<div class="answer-why"><Formatted text={q.why} /></div>{/if}</div></li>{/if}
  {/each}
</ol></section>
