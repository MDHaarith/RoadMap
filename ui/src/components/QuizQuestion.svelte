<script lang="ts">
  import type { Quiz, Session } from '../lib/types';
  import Formatted from './Formatted.svelte';
  import Icon from './Icon.svelte';
  let { q, session: s, total, answer, next, mode = 'quiz', retry }: { q: Quiz; session: Session; total: number; answer: (i: number) => void; next: () => void; mode?: 'quiz' | 'review' | 'exam'; retry?: () => void } = $props();
</script>
<div class="quiz-question"><div class="quiz-meta">{#if mode === 'quiz'}<span>Question {s.index + 1} of {total}</span>{/if}<span>{s.score} correct so far</span></div>
<h3 class="question" id="question" tabindex="-1"><Formatted text={q.q} /></h3>
<div class="options">{#each q.options as x, i}<button class="option" class:correct={s.selected !== null && i === q.answer} class:wrong={s.selected !== null && i !== q.answer && i === s.selected} disabled={s.selected !== null} onclick={() => answer(i)}><span class="letter">{String.fromCharCode(65 + i)}</span><span class="grow">{x}</span>{#if s.selected !== null && i === q.answer}<span class="answer-mark complete-mark"><Icon name="check" /><span class="sr">Correct answer</span></span>{:else if s.selected !== null && i === s.selected}<span class="answer-mark"><span aria-hidden="true">×</span><span class="sr">Incorrect answer</span></span>{/if}</button>{/each}</div>
{#if s.selected !== null}<div class="feedback" role="status"><strong>{s.selected === q.answer ? 'Correct.' : 'The correct answer is ' + q.options[q.answer] + '.'}</strong><p><Formatted text={q.why} /></p></div><div class="row" style="margin-top:var(--s5)"><button class="btn" id={`${mode}-next`} disabled={mode === 'review' && (!!s.pending || !!s.error)} onclick={next}>{mode === 'review' && s.pending ? 'Saving answer…' : s.index === total - 1 ? 'See score' : 'Next question'} <Icon name="arrow" /></button>{#if mode === 'review' && s.error}<button class="link" id="review-retry" onclick={retry}>Try saving again</button>{/if}</div>{#if mode === 'review' && s.error}<p class="err" role="alert">Your answer could not be saved. {s.error}</p>{/if}{/if}
</div>
