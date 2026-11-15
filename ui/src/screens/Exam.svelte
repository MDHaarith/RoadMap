<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { api } from '../lib/api';
  import { save } from '../lib/state.svelte';
  import { examPool, examReady } from '../lib/progress';
  import { href, clock } from '../lib/format';
  import type { Course, Session, Quiz } from '../lib/types';
  import Icon from '../components/Icon.svelte';
  import Progress from '../components/Progress.svelte';
  import QuizQuestion from '../components/QuizQuestion.svelte';
  let { c, stage }: { c: Course; stage: number } = $props();
  let qs = $state<(Quiz & { lesson: string; i: number })[]>([]);
  let s = $state<Session>({ index: 0, score: 0, selected: null, finished: false });
  let started = $state(false), left = $state(60);
  let won = $derived(s.score / qs.length >= .7), last = $derived(c.state.exams?.[stage]);
  onMount(() => { exam(); });
  $effect(() => {
    if (!started || s.finished || s.selected !== null) return;
    const timer = setInterval(() => { left--; if (left <= 0) examAnswer(-1); }, 1000);
    return () => clearInterval(timer);
  });
  function exam() {
    if (!Number.isInteger(stage) || !c.stages?.[stage] || !examReady(c, stage)) { location.hash = href(c.id); return; }
    qs = examPool(c, stage).map(q => ({ order: Math.random(), q })).sort((a, b) => a.order - b.order).slice(0, 10).map(x => x.q);
    s = { index: 0, score: 0, selected: null, finished: false }; started = false; left = 60;
  }
  async function focus() { await tick(); (document.getElementById('exam-next') || document.getElementById('question') || document.querySelector<HTMLElement>('.score'))?.focus({ preventScroll: true }); }
  function examAnswer(i: number) {
    const q = qs[s.index]; if (!started || s.finished || s.selected !== null) return;
    s.selected = i; if (i === q.answer) s.score++;
    else api('POST', 'api/mistake', { course: c.id, lesson: q.lesson, i: q.i, ...(i >= 0 ? { picked: i } : {}) }).catch(() => {});
    void focus();
  }
  function examNext() {
    if (s.selected === null) return;
    if (s.index === qs.length - 1) {
      s.finished = true; const old = c.state.exams?.[stage];
      if (!old || s.score / qs.length >= old.score / old.total) { (c.state.exams ||= {})[stage] = { score: s.score, total: qs.length }; save(c); }
    } else { s.index++; s.selected = null; left = 60; }
    void focus();
  }
</script>
<div class="wrap narrow review-wrap screen-enter"><a class="back-map" href={href(c.id)}><Icon name="back" /> {c.title}</a><div class="page-head session-head"><div><div class="kicker">Stage {stage + 1} exam</div><h1>{c.stages?.[stage]?.title}</h1></div>{#if qs.length}<div class="row"><span class="session-count">{started && !s.finished ? `Question ${s.index + 1} of ${qs.length}` : `${qs.length} questions`}</span>{#if started && !s.finished}<span class="timer" class:low={left <= 10} role="timer"><span>Time left</span> <b id="exam-clock">{clock(Math.max(0, left))}</b></span>{/if}</div>{/if}</div>
  {#if qs.length}{#if !started}<div class="card exam-intro"><h2>{qs.length} questions, one minute each</h2><ul class="facts stagger"><li>They come from this stage’s lesson quizzes, in a new order.</li><li>Score 70% to pass{c.stages?.[stage + 1] ? ' and open Stage ' + (stage + 2) : ''}.</li><li>No notes and no tutor: this shows you what stuck.</li></ul>{#if last}<p class="small">Your best result so far: {last.score} of {last.total}.</p>{/if}<button class="btn big" id="exam-start" onclick={() => { started = true; void focus(); }}>Start the exam <Icon name="arrow" /></button></div>
  {:else if s.finished}<div class="score session-score" class:pass={won} tabindex="-1">{#if won}<span class="complete-mark"><Icon name="check" /></span>{/if}<span class="small">{won ? 'Stage passed' : 'Not passed yet'}</span><strong>{s.score} / {qs.length}</strong><p>{won ? 'Well earned. Carry on with the roadmap.' : 'You need 70%. The questions you missed are in your mistakes notebook and in today’s review. Go over them, then try again.'}</p><div class="row">{#if won}<a class="btn" href={href(c.id)}>Back to the roadmap <Icon name="arrow" /></a><a class="btn secondary" href="#/mistakes">Mistakes notebook</a>{:else}<button class="btn" id="exam-again" onclick={exam}>Try again</button><a class="btn secondary" href="#/mistakes">Mistakes notebook</a><a class="btn ghost" href={href(c.id)}>Back to the roadmap</a>{/if}</div></div>
  {:else}<div class="session-bar"><Progress n={s.index} total={qs.length} wide={true} /></div><section class="lesson-content"><div class="step-body">{#if s.selected === -1}<p class="err" role="alert">Time is up for this question.</p>{/if}<QuizQuestion q={qs[s.index]} session={s} total={qs.length} answer={examAnswer} next={examNext} mode="exam" /></div></section>{/if}{/if}
</div>
