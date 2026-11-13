<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { api } from '../lib/api';
  import { appState, message } from '../lib/state.svelte';
  import type { ReviewData, Session } from '../lib/types';
  import Icon from '../components/Icon.svelte';
  import Formatted from '../components/Formatted.svelte';
  import QuizQuestion from '../components/QuizQuestion.svelte';
  import Progress from '../components/Progress.svelte';
  import Waiting from '../components/Waiting.svelte';
  let loading = $state(true), error = $state('');
  let s = $state<Session>({ index: 0, score: 0, selected: null, finished: false, pending: false, error: '' });
  let qs = $derived(appState.review.questions), q = $derived(qs[s.index]);
  let alive = true;
  onMount(() => { void review(); return () => { alive = false; }; });
  // The title-bar count is loaded on the home screen; answered questions are no longer due, so refresh it here.
  function refreshCount() { api<ReviewData>('GET', 'api/review').then(r => { if (alive) appState.review.count = r.count; }).catch(() => {}); }
  async function focus() { await tick(); (document.getElementById('review-next') || document.getElementById('question') || document.querySelector<HTMLElement>('.score'))?.focus({ preventScroll: true }); }
  async function review() {
    loading = true; error = '';
    try {
      const r = await api<ReviewData>('GET', 'api/review'); if (!alive) return;
      appState.review = r; s = { index: 0, score: 0, selected: null, finished: false, pending: false, error: '' };
    } catch (err) { error = message(err); } finally { loading = false; }
  }
  async function saveReviewAnswer() {
    if (s.pending) return; const session = s, item = q;
    session.pending = true; session.error = '';
    try { await api('POST', 'api/review', { course: item.course, lesson: item.lesson, i: item.i, right: session.selected === item.answer }); }
    catch (err) { session.error = message(err); }
    finally { session.pending = false; if (alive) void focus(); }
  }
  async function reviewAnswer(i: number) {
    if (s.finished || s.selected !== null || !Number.isInteger(i) || i < 0 || i >= q.options.length) return;
    s.selected = i; if (i === q.answer) s.score++; await saveReviewAnswer();
  }
  async function reviewCheck(blank = false) {
    if (s.pending || s.selected !== null) return;
    const code = q.kind === 'code', text = s.text ?? (code ? q.starter || '' : '');
    s.text = text; if (!blank && !text.trim()) return;
    const session = s, item = q; session.pending = true; session.error = '';
    try {
      const r = await api<NonNullable<Session['result']>>('POST', 'api/review/answer', { course: item.course, lesson: item.lesson, i: item.i, answer: blank && !code ? '' : text });
      session.result = r.missing ? { right: false, output: r.output } : r;
      session.selected = -1; if (r.right) session.score++;
    } catch (err) { session.error = message(err); }
    finally { session.pending = false; if (alive) void focus(); }
  }
  function reviewNext() {
    if (s.selected === null || s.pending || s.error || s.finished) return;
    if (s.index === qs.length - 1) { s.finished = true; appState.review.count = Math.max(0, appState.review.count - qs.length); refreshCount(); }
    else { s.index++; s.selected = null; s.text = undefined; s.result = null; }
    void focus();
  }
  function keyboard(e: KeyboardEvent) {
    if (loading || !q || s.finished || e.defaultPrevented || document.getElementById('switch-menu')?.hidden === false || (e.target as Element).closest('textarea,input,select') || e.ctrlKey || e.altKey || e.metaKey) return;
    if (s.selected === null && q.kind !== 'explain' && q.kind !== 'code') {
      const i = /^[1-9]$/.test(e.key) ? +e.key - 1 : /^[a-z]$/i.test(e.key) ? e.key.toUpperCase().charCodeAt(0) - 65 : -1;
      if (i >= 0 && i < q.options.length) { e.preventDefault(); void reviewAnswer(i); }
    } else if (e.key === 'Enter' && !s.pending && !s.error) { e.preventDefault(); reviewNext(); }
  }
</script>
<svelte:window onkeydown={keyboard} />
<div class="wrap review-wrap"><a class="back-map" href="#/"><Icon name="back" /> Today</a><div class="session-head"><h1>Daily review</h1></div>
  {#if loading}<Waiting title="Opening your review" />{:else if error}<p class="err" role="alert">{error}</p><button class="btn secondary" onclick={review}>Try again</button>
  {:else if !qs.length}<div class="empty-state"><Icon name="check" /><h2>Nothing due today.</h2><p>You’re up to date. Your next lesson is waiting on Today.</p><a class="btn" href="#/">Back to Today</a></div>
  {:else if s.finished}<div class="score" tabindex="-1"><span class="small">Review complete</span><strong>{s.score} / {qs.length}</strong><p>Right answers return later. Missed ones come back tomorrow, so you can try them again while they’re fresh.</p>{#if appState.review.count > 0}<p class="small">{appState.review.count} more question{appState.review.count === 1 ? ' is' : 's are'} due today.</p>{/if}<div class="row"><a class="btn" href="#/">Back to Today <Icon name="arrow" /></a>{#if appState.review.count > 0}<button class="btn secondary" id="review-more" onclick={review}>Review another set</button>{/if}</div></div>
  {:else}<div class="session-bar"><Progress n={s.index} total={qs.length} wide={true} /></div><section class="lesson-content"><p class="review-context">{q.roadmap} · <b>{q.lesson_title}</b></p><div class="step-body">
    {#if q.kind === 'explain' || q.kind === 'code'}{@const code = q.kind === 'code'}<div class="quiz-meta"><span>{code ? 'Code from memory' : 'Explain from memory'}</span><span>{s.score} correct so far</span></div><h3 class="question" id="question" tabindex="-1">{code ? 'Write it again from the starter: ' + q.title : 'Explain this in your own words'}</h3><div class="recall"><Formatted text={code ? q.prompt || '' : q.q} /></div>
      <textarea id="review-text" class:codebox={code} rows={code ? 9 : 5} spellcheck={!code} readonly={s.selected !== null || s.pending} aria-label="Your answer" placeholder={code ? '' : 'Write what you remember. Short is fine.'} value={s.text ?? (code ? q.starter : '')} oninput={e => s.text = e.currentTarget.value} onkeydown={e => { if (code && e.key === 'Tab' && !e.shiftKey) { e.preventDefault(); const el = e.currentTarget; el.setRangeText('    ', el.selectionStart, el.selectionEnd, 'end'); s.text = el.value; } e.stopPropagation(); }}></textarea>
      {#if s.selected !== null}<div class="feedback" class:right={s.result?.right} class:wrong={!s.result?.right} role="status"><strong>{s.result?.right ? 'That’s right.' : 'Not yet.'}</strong>{#if s.result?.feedback}<p><Formatted text={s.result.feedback} /></p>{/if}{#if s.result?.output}<pre>{s.result.output}</pre>{/if}</div><div class="row answer-row"><button class="btn" id="review-next" onclick={reviewNext}>{s.index === qs.length - 1 ? 'See my score' : 'Next question'} <Icon name="arrow" /></button></div>
      {:else}<div class="row answer-row"><button class="btn" id="review-check" disabled={s.pending} onclick={() => reviewCheck()}>{s.pending ? 'Checking…' : code ? 'Run the tests' : 'Check my answer'}</button><button class="btn ghost" id="review-skip" disabled={s.pending} onclick={() => reviewCheck(true)}>I can’t remember</button>{#if s.pending}<Waiting inline title={code ? 'Running the tests' : 'Marking your answer'} />{/if}</div>{#if s.error}<p class="err" role="alert">{s.error}</p>{/if}{/if}
    {:else}<QuizQuestion {q} session={s} total={qs.length} answer={reviewAnswer} next={reviewNext} mode="review" retry={saveReviewAnswer} /><p class="keyboard-hint">Press A–{String.fromCharCode(64 + q.options.length)} or 1–{q.options.length} to answer, Enter to go on.</p>{/if}
  </div></section>{/if}
</div>
