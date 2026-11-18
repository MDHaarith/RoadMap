<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { api } from '../lib/api';
  import { autogrow } from '../lib/autogrow';
  import { appState, message, toast } from '../lib/state.svelte';
  import type { ReviewData, Session } from '../lib/types';
  import Icon from '../components/Icon.svelte';
  import Formatted from '../components/Formatted.svelte';
  import QuizQuestion from '../components/QuizQuestion.svelte';
  import Progress from '../components/Progress.svelte';
  import Waiting from '../components/Waiting.svelte';
  import Skeleton from '../components/Skeleton.svelte';
  import Elapsed from '../components/Elapsed.svelte';
  import BusyLabel from '../components/BusyLabel.svelte';
  import LoadError from '../components/LoadError.svelte';
  let loading = $state(true), error = $state('');
  let s = $state<Session>({ index: 0, score: 0, selected: null, finished: false, pending: false, error: '' });
  let qs = $derived(appState.review.questions), q = $derived(qs[s.index]);
  let alive = true;
  onMount(() => { void review(); return () => { alive = false; }; });
  // The title bar loads its own count; answered questions are no longer due, so refresh it here.
  function refreshCount() { api<ReviewData>('GET', 'api/review').then(r => { if (alive) appState.review.count = r.count; }).catch(() => {}); }
  async function focus() { await tick(); (document.getElementById('review-next') || document.getElementById('question') || document.querySelector<HTMLElement>('.score'))?.focus({ preventScroll: true }); }
  async function review() {
    loading = true; error = '';
    try {
      const r = await api<ReviewData>('GET', 'api/review'); if (!alive) return;
      appState.review = r; s = { index: 0, score: 0, selected: null, finished: false, pending: false, error: '' };
    } catch (err) { error = message(err); } finally { loading = false; }
  }
  // The question was rewritten or removed after this list loaded; the server marked nothing, so load the current list.
  function reload(text = '') { toast(text); void review(); }
  async function saveReviewAnswer() {
    if (s.pending) return; const session = s, item = q;
    session.pending = true; session.error = '';
    try { const r = await api<{ changed?: boolean; message?: string }>('POST', 'api/review', { course: item.course, lesson: item.lesson, i: item.i, q: item.q, right: session.selected === item.answer }); if (r.changed) reload(r.message); }
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
      const r = await api<NonNullable<Session['result']> & { changed?: boolean; message?: string }>('POST', 'api/review/answer', { course: item.course, lesson: item.lesson, i: item.i, q: code ? item.prompt : item.q, answer: blank && !code ? '' : text });
      if (r.changed) return reload(r.message);
      session.result = r.missing ? { right: false, output: r.output } : r;
      session.selected = -1; if (r.right) session.score++;
    } catch (err) { session.error = message(err); }
    finally { session.pending = false; if (alive) void focus(); }
  }
  // A card from the learner's notes: they recall the answer, see it, and say whether they knew it.
  let revealed = $state(false);
  async function cardAnswer(right: boolean) {
    if (s.pending || s.selected !== null) return;
    const session = s, item = q; session.pending = true; session.error = '';
    try {
      const r = await api<{ changed?: boolean; message?: string }>('POST', 'api/review', { course: item.course, lesson: item.lesson, i: item.i, kind: 'card', q: item.q, right });
      if (r.changed) return reload(r.message);
      session.selected = right ? 1 : 0; session.result = { right }; if (right) session.score++;
    } catch (err) { session.error = message(err); }
    finally { session.pending = false; if (alive) void focus(); }
  }
  function reviewNext() {
    if (s.selected === null || s.pending || s.error || s.finished) return;
    if (s.index === qs.length - 1) { s.finished = true; appState.review.count = Math.max(0, appState.review.count - qs.length); refreshCount(); }
    else { s.index++; s.selected = null; s.text = undefined; s.result = null; revealed = false; }
    void focus();
  }
  function keyboard(e: KeyboardEvent) {
    if (loading || !q || s.finished || e.defaultPrevented || document.getElementById('switch-menu')?.hidden === false || (e.target as Element).closest('textarea,input,select') || e.ctrlKey || e.altKey || e.metaKey) return;
    if (q.kind === 'card') { if (e.key === 'Enter' && s.selected !== null && !s.pending) { e.preventDefault(); reviewNext(); } return; }
    if (s.selected === null && q.kind !== 'explain' && q.kind !== 'code') {
      const i = /^[1-9]$/.test(e.key) ? +e.key - 1 : /^[a-z]$/i.test(e.key) ? e.key.toUpperCase().charCodeAt(0) - 65 : -1;
      if (i >= 0 && i < q.options.length) { e.preventDefault(); void reviewAnswer(i); }
    } else if (e.key === 'Enter' && !s.pending && !s.error) { e.preventDefault(); reviewNext(); }
  }
</script>
<svelte:window onkeydown={keyboard} />
<div class="wrap narrow review-wrap screen-enter"><a class="back-map" href="#/"><Icon name="back" /> Your library</a><div class="page-head session-head"><h1>Daily review</h1>{#if !loading && !error}<span class="session-count">{s.finished ? `${qs.length} reviewed` : qs.length ? `Question ${s.index + 1} of ${qs.length}` : '0 due'}</span>{/if}</div>
  {#if loading}<Waiting skeleton="question" title="Opening your review" />{:else if error}<LoadError what="Your review could not load." {error} retry={review} />
  {:else if !qs.length}<div class="empty-state"><Icon name="empty-route" /><h2>Nothing due today.</h2><p>Questions return here when it’s time to practice them again.</p><a class="btn" href="#/">Your library</a></div>
  {:else if s.finished}<div class="score session-score" tabindex="-1"><span class="complete-mark"><Icon name="check" /></span><span class="small">Review complete</span><strong>{s.score} / {qs.length}</strong><p>Right answers return later. Missed ones come back tomorrow, so you can try them again while they’re fresh.</p>{#if appState.review.count > 0}<p class="small">{appState.review.count} more question{appState.review.count === 1 ? ' is' : 's are'} due today.</p>{/if}<div class="row"><a class="btn" href="#/">Your library <Icon name="arrow" /></a>{#if appState.review.count > 0}<button class="btn secondary" id="review-more" onclick={review}>Review another set</button>{/if}</div></div>
  {:else}<div class="session-bar"><Progress n={s.index} total={qs.length} wide={true} unit="questions answered" /></div><section class="lesson-content"><p class="review-context">{q.roadmap} · <b>{q.lesson_title}</b></p><div class="step-body">
    {#if q.kind === 'card'}<div class="quiz-meta"><span>Card from your notes</span><span>{s.score} correct so far</span></div><h3 class="question" id="question" tabindex="-1"><Formatted text={q.q} /></h3>
      {#if revealed}<div class="card-back"><Formatted text={q.back || ''} /></div>{/if}
      {#if s.selected !== null}<div class="feedback" class:right={s.result?.right} class:wrong={!s.result?.right} role="status"><strong>{s.result?.right ? 'Good.' : 'It comes back tomorrow.'}</strong></div><div class="row answer-row"><button class="btn" id="review-next" onclick={reviewNext}>{s.index === qs.length - 1 ? 'See score' : 'Next question'} <Icon name="arrow" /></button></div>
      {:else if !revealed}<div class="row answer-row"><button class="btn" id="review-reveal" onclick={() => { revealed = true; }}>Show the answer</button></div><p class="keyboard-hint">Answer in your head first, then check.</p>
      {:else}<div class="row answer-row"><button class="btn" id="card-knew" disabled={s.pending} onclick={() => void cardAnswer(true)}>I knew it</button><button class="btn secondary" id="card-missed" disabled={s.pending} onclick={() => void cardAnswer(false)}>I didn’t</button></div>{#if s.error}<p class="err" role="alert">{s.error}</p>{/if}{/if}
    {:else if q.kind === 'explain' || q.kind === 'code'}{@const code = q.kind === 'code'}<div class="quiz-meta"><span>{code ? 'Code from memory' : 'Answer from memory'}</span><span>{s.score} correct so far</span></div>{#if code}<h3 class="question" id="question" tabindex="-1">Write it again from the starter: {q.title}</h3><div class="recall"><Formatted text={q.prompt || ''} /></div>{:else}<h3 class="question" id="question" tabindex="-1"><Formatted text={q.q} /></h3>{/if}
      <textarea id="review-text" use:autogrow={{ max: code ? 0 : 260, value: s.text }} class:codebox={code} rows={code ? 9 : 5} spellcheck={!code} readonly={s.selected !== null || s.pending} aria-label="Your answer" placeholder={code ? '' : 'Write what you remember. Short is fine.'} value={s.text ?? (code ? q.starter : '')} oninput={e => s.text = e.currentTarget.value} onkeydown={e => { if (code && e.key === 'Tab' && !e.shiftKey) { e.preventDefault(); const el = e.currentTarget; el.setRangeText('    ', el.selectionStart, el.selectionEnd, 'end'); s.text = el.value; } e.stopPropagation(); }}></textarea>
      {#if s.selected !== null}<div class="feedback" class:right={s.result?.right} class:wrong={!s.result?.right} role="status"><strong>{s.result?.right ? 'Correct.' : 'Not yet.'}</strong>{#if s.result?.feedback}<p><Formatted text={s.result.feedback} /></p>{/if}{#if s.result?.output}<pre>{s.result.output}</pre>{/if}</div>{#if s.result?.points?.length}<section class="mark-scheme" aria-labelledby="mark-scheme-title"><h4 id="mark-scheme-title">What a full answer says</h4><ul>{#each s.result.points as point}<li class:hit={point.hit}><span class="point-mark" class:complete-mark={point.hit} aria-hidden="true">{#if point.hit}<Icon name="check" />{/if}</span><span class="sr">{point.hit ? 'Your answer made this point:' : 'Your answer missed this point:'}</span><span>{point.text}</span></li>{/each}</ul></section>{/if}<div class="row answer-row"><button class="btn" id="review-next" onclick={reviewNext}>{s.index === qs.length - 1 ? 'See score' : 'Next question'} <Icon name="arrow" /></button></div>
      {:else}{#if s.pending}<div class="feedback marking"><span class="sr" role="status">{code ? 'Running the tests' : 'Marking your answer'}</span><Skeleton rows={2} /></div>{/if}<div class="row answer-row"><button class="btn" id="review-check" disabled={s.pending} onclick={() => reviewCheck()}><BusyLabel busy={!!s.pending} idle={code ? 'Run the tests' : 'Check my answer'} label={code ? 'Running…' : 'Marking…'} /></button><button class="btn ghost" id="review-skip" disabled={s.pending} onclick={() => reviewCheck(true)}>I can’t remember</button>{#if s.pending}<span class="small" aria-hidden="true"><Elapsed /></span>{/if}</div>{#if s.error}<p class="err" role="alert">{s.error}</p>{/if}{/if}
    {:else}<QuizQuestion {q} session={s} total={qs.length} answer={reviewAnswer} next={reviewNext} mode="review" retry={saveReviewAnswer} /><p class="keyboard-hint">Press A–{String.fromCharCode(64 + q.options.length)} or 1–{q.options.length} to answer, Enter to go on.</p>{/if}
  </div></section>{/if}
</div>
