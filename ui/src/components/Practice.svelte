<script lang="ts">
  import { tick } from 'svelte';
  import { api } from '../lib/api';
  import { appState, keyFor, save, loadPractice, adaptNow, ask } from '../lib/state.svelte';
  import type { Course, Lesson } from '../lib/types';
  import QuizQuestion from './QuizQuestion.svelte';
  import Exercise from './Exercise.svelte';
  import Icon from './Icon.svelte';
  import Waiting from './Waiting.svelte';
  let { c, l }: { c: Course; l: Lesson } = $props();
  let key = $derived(keyFor(c, l)), p = $derived(l.practice), s = $derived(appState.sessions[key]);
  let view = $derived(appState.practiceViews[key] || 'quiz');
  let fails = $derived(appState.runFails[key] ?? (c.state.tests?.[l.id] && !c.state.tests[l.id].ok ? c.state.tests[l.id].runs : 0));
  let q = $derived(c.state.quiz[l.id]), low = $derived(!!q && q.total > 0 && q.score / q.total < .6);
  $effect(() => {
    if (p && !appState.sessions[key]) {
      const score = c.state.quiz[l.id];
      appState.sessions[key] = score ? { index: 0, selected: null, finished: true, ...score } : { index: 0, score: 0, selected: null, finished: false };
    }
  });
  async function answer(i: number) {
    const q = p!.quiz![s.index];
    if (s.finished || s.selected !== null || !Number.isInteger(i) || i < 0 || i >= q.options.length) return;
    s.selected = i;
    if (i === q.answer) s.score++;
    else api('POST', 'api/mistake', { course: c.id, lesson: l.id, i: s.index, picked: i }).catch(() => {});
    await tick(); document.getElementById('quiz-next')?.focus({ preventScroll: true });
  }
  async function quizNext() {
    if (s.selected === null) return;
    if (s.index === p!.quiz!.length - 1) {
      s.finished = true; s.total = p!.quiz!.length; c.state.quiz[l.id] = { score: s.score, total: s.total }; save(c);
    } else { s.index++; s.selected = null; }
    await tick(); (document.getElementById('question') || document.querySelector<HTMLElement>('.score'))?.focus({ preventScroll: true });
  }
  async function again() { delete appState.sessions[key]; delete c.state.quiz[l.id]; save(c); await tick(); document.getElementById('question')?.focus({ preventScroll: true }); }
</script>
<div id="practice">
  {#if !p && (!appState.config.base || !appState.config.model)}<div class="note setup"><div><strong>No AI service connected</strong><p>Practice questions are written by an AI service. Add one in Settings.</p></div><a class="btn secondary" href="#/settings">Open Settings</a></div>
  {:else if appState.practiceBusy[key]}<Waiting skeleton="question" title={`Preparing questions${(/python|code|program|javascript/i.test(c.prompt || '')) ? ' and an exercise' : ''} for this lesson`} hint="this usually takes about 30 seconds"><p>Your notes and the tutor still work while you wait.</p></Waiting>
  {:else if appState.practiceErrors[key]}<div class="err" role="alert"><strong>The questions could not load.</strong><p>{appState.practiceErrors[key]}</p><button id="retry-practice" class="btn secondary" style="margin-top:8px" onclick={() => loadPractice(!!appState.practiceFresh[key])}>Try again</button></div>
  {:else if !p}<div class="practice-start"><h3>Ready to check your understanding?</h3><button class="btn" id="load-practice" onclick={() => loadPractice()}>Prepare questions <Icon name="arrow" /></button><p class="small" style="margin:var(--s5) 0 0">Written for this lesson by the AI. Takes about 30 seconds.</p></div>
  {:else}
    {#if fails >= 3 || low}<div class="note stuck"><strong>Stuck on this one?</strong><p>{fails >= 3 ? `The tests have failed ${fails} times.` : 'Some answers need another look.'} Take a smaller step on the same idea, then come back.</p><div class="row"><button class="btn secondary" onclick={() => ask('I’m stuck on this lesson. Give me a smaller warm-up problem on the same idea, and take me through it one step at a time.')}>Ask the tutor for a smaller step</button><button class="btn secondary" id="stuck-adapt" disabled={appState.adapting} title="The AI reads your results and may add a short practice lesson before the next one" onclick={() => adaptNow(true)}>Add practice to my roadmap</button></div>{#if appState.adapting}<Waiting skeleton="plan" title="Updating your roadmap" hint="this can take about a minute" />{/if}</div>{/if}
    {#if p.exercise}<div class="practice-nav" role="group" aria-label="Practice activities"><button aria-pressed={view === 'quiz'} onclick={() => appState.practiceViews[key] = 'quiz'}>Quiz{q ? ` · ${q.score}/${q.total}` : ''}</button><button aria-pressed={view === 'code'} onclick={() => appState.practiceViews[key] = 'code'}>Coding exercise</button></div>{/if}
    {#if p.exercise && view === 'code'}<Exercise {c} {l} ex={p.exercise} />
    {:else if s?.finished}<div class="score" tabindex="-1"><strong>{s.score} / {s.total ?? p.quiz?.length ?? 0}</strong><h3>Quiz finished</h3><p>{s.score === (s.total ?? p.quiz?.length) ? 'You answered every question correctly. Try using these ideas.' : 'Review the answers you missed, then try again.'}</p><div class="row"><button class="btn" id="quiz-again" onclick={again}>Try again</button><button class="btn secondary" id="fresh-practice" onclick={() => loadPractice(true)}>New questions</button></div></div>
    {:else if p.quiz?.length && s}<QuizQuestion q={p.quiz[s.index]} session={s} total={p.quiz.length} {answer} next={quizNext} />
    {:else if !p.quiz?.length}<p>No questions found for this lesson. Try preparing a new set.</p><button class="btn secondary" id="fresh-practice" onclick={() => loadPractice(true)}>New questions</button>{/if}
  {/if}
</div>
