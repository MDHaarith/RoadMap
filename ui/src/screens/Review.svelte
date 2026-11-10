<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { api } from '../lib/api';
  import { appState, message } from '../lib/state.svelte';
  import type { ReviewData, Session } from '../lib/types';
  import Icon from '../components/Icon.svelte';
  import Formatted from '../components/Formatted.svelte';
  import QuizQuestion from '../components/QuizQuestion.svelte';
  import Progress from '../components/Progress.svelte';
  let loading = $state(true), error = $state('');
  let s = $state<Session>({ index: 0, score: 0, selected: null, finished: false, pending: false, error: '' });
  let qs = $derived(appState.review.questions), q = $derived(qs[s.index]);
  let alive = true;
  onMount(() => { void review(); return () => { alive = false; }; });
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
    if (s.index === qs.length - 1) s.finished = true;
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
<div class="wrap review-wrap"><div class="intro"><div><div class="kicker"><Icon name="Practise" /> A little recall goes a long way</div><h1>Daily review</h1></div><a href="#/">Back to Today</a></div>
  {#if loading}<div class="loading" role="status">Opening your review…</div>{:else if error}<p class="err" role="alert">{error}</p><button class="btn secondary" onclick={review}>Try again</button>
  {:else if !qs.length}<div class="note"><h2>Nothing due today.</h2><p>You’re up to date. Your next lesson is waiting on Today.</p><a class="btn" href="#/">Back to Today</a></div>
  {:else if s.finished}<div class="score" tabindex="-1"><strong>{s.score} / {qs.length}</strong><h3>Review complete.</h3><p>Right answers return later. Missed ones come back tomorrow, so you can try them again while they’re fresh.</p>{#if appState.review.count > qs.length}<p>{appState.review.count - qs.length} more questions are due today.</p><button class="btn secondary" id="review-more" onclick={review}>Review another set</button> {/if}<a class="btn" href="#/">Back to Today <Icon name="arrow" /></a></div>
  {:else}<div class="review-progress"><Progress n={s.index} total={qs.length} wide={false} /></div><section class="lesson-content"><div class="review-context"><p>{q.roadmap}</p><h2>{q.lesson_title}</h2></div><div class="step-body">
    {#if q.kind === 'explain' || q.kind === 'code'}{@const code = q.kind === 'code'}<div class="quiz-meta"><span>Question {s.index + 1} of {qs.length}</span><span>{s.score} correct so far</span></div><h3 class="question" id="question" tabindex="-1">{code ? 'Write it again from the starter: ' + q.title : 'Explain from memory, in your own words'}</h3><p class="recall">{#if code}{q.prompt}{:else}<Formatted text={q.q} />{/if}</p>
      <textarea id="review-text" class:codebox={code} rows={code ? 12 : 5} spellcheck={!code} readonly={s.selected !== null || s.pending} aria-label="Your answer" value={s.text ?? (code ? q.starter : '')} oninput={e => s.text = e.currentTarget.value} onkeydown={e => { if (code && e.key === 'Tab' && !e.shiftKey) { e.preventDefault(); const el = e.currentTarget; el.setRangeText('    ', el.selectionStart, el.selectionEnd, 'end'); s.text = el.value; } e.stopPropagation(); }}></textarea>
      {#if s.selected !== null}<div class="feedback" role="status"><strong>{s.result?.right ? 'That’s right.' : 'Not yet.'}</strong>{#if s.result?.feedback}<p><Formatted text={s.result.feedback} /></p>{/if}{#if s.result?.output}<pre>{s.result.output}</pre>{/if}</div><div class="row" style="margin-top:20px"><button class="btn" id="review-next" onclick={reviewNext}>{s.index === qs.length - 1 ? 'See my score' : 'Next question'} <Icon name="arrow" /></button></div>
      {:else}<div class="row" style="margin-top:16px"><button class="btn" id="review-check" disabled={s.pending} onclick={() => reviewCheck()}>{s.pending ? 'Checking…' : code ? 'Run the tests' : 'Check my answer'}</button><button class="link" id="review-skip" disabled={s.pending} onclick={() => reviewCheck(true)}>I can’t remember</button></div>{#if s.error}<p class="err" role="alert">{s.error}</p>{/if}{/if}
    {:else}<QuizQuestion {q} session={s} total={qs.length} answer={reviewAnswer} next={reviewNext} mode="review" retry={saveReviewAnswer} /><p class="keyboard-hint">A–{String.fromCharCode(64 + q.options.length)} or 1–{q.options.length} to answer · Enter to continue</p>{/if}
  </div></section>{/if}
</div>
