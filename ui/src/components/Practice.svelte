<script lang="ts">
  import { tick } from 'svelte';
  import { api } from '../lib/api';
  import { appState, keyFor, save, loadPractice, adaptNow, ask, aiProblem } from '../lib/state.svelte';
  import AiSetup from './AiSetup.svelte';
  import type { Course, Lesson } from '../lib/types';
  import { quizSet } from '../lib/progress';
  import QuizQuestion from './QuizQuestion.svelte';
  import Exercise from './Exercise.svelte';
  import Icon from './Icon.svelte';
  import Waiting from './Waiting.svelte';
  import AnswerReview from './AnswerReview.svelte';
  import LoadError from './LoadError.svelte';
  let { c, l }: { c: Course; l: Lesson } = $props();
  let key = $derived(keyFor(c, l)), p = $derived(l.practice), s = $derived(appState.sessions[key]);
  // The quiz asks the newest set of the lesson's question bank; older sets stay for the stage exam and the daily review.
  let set = $derived(quizSet(l));
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
    const q = set.quiz[s.index];
    if (s.finished || s.selected !== null || !Number.isInteger(i) || i < 0 || i >= q.options.length) return;
    s.selected = i; s.picks ||= []; s.picks[s.index] = i;
    if (i === q.answer) s.score++;
    else api('POST', 'api/mistake', { course: c.id, lesson: l.id, i: set.from + s.index, picked: i }).catch(() => {});
    await tick(); document.getElementById('quiz-next')?.focus({ preventScroll: true });
  }
  async function quizNext() {
    if (s.selected === null) return;
    if (s.index === set.quiz.length - 1) {
      s.finished = true; s.total = set.quiz.length; c.state.quiz[l.id] = { score: s.score, total: s.total }; save(c);
    } else { s.index++; s.selected = null; }
    await tick(); (document.getElementById('question') || document.querySelector<HTMLElement>('.score'))?.focus({ preventScroll: true });
  }
  async function again() { delete appState.sessions[key]; delete c.state.quiz[l.id]; save(c); await tick(); document.getElementById('question')?.focus({ preventScroll: true }); }
</script>
<div id="practice">
  {#if !p && aiProblem()}<AiSetup why="Practice questions are written by an AI service. Add one in Settings." />
  {:else if appState.practiceBusy[key]}<Waiting skeleton="question" title={`Preparing questions${(/python|code|program|javascript/i.test(c.prompt || '')) ? ' and an exercise' : ''} for this lesson`} hint="this usually takes about 30 seconds"><p>Your notes and the tutor still work while you wait.</p></Waiting>
  {:else if appState.practiceErrors[key]}<LoadError id="retry-practice" what="The questions could not load." error={appState.practiceErrors[key]} retry={() => loadPractice(!!appState.practiceFresh[key])} />
  {:else if !p}<div class="practice-start"><h3>Ready to check your understanding?</h3><button class="btn" id="load-practice" onclick={() => loadPractice()}>Prepare questions <Icon name="arrow" /></button><p class="small">Written for this lesson by the AI. Takes about 30 seconds.</p></div>
  {:else}
    {#if fails >= 3 || low}<div class="note stuck"><strong>Stuck on this one?</strong><p>{fails >= 3 ? `The tests have failed ${fails} times.` : 'Some answers need another look.'} Take a smaller step on the same idea, then come back.</p><div class="row"><button class="btn secondary" onclick={() => ask('I’m stuck on this lesson. Give me a smaller warm-up problem on the same idea, and take me through it one step at a time.')}>Ask the tutor for a smaller step</button><button class="btn secondary" id="stuck-adapt" disabled={appState.adapting} title="The AI reads your results and may add a short practice lesson before the next one" onclick={() => adaptNow(true)}>Add practice to my roadmap</button></div>{#if appState.adapting}<Waiting skeleton="plan" title="Updating your roadmap" hint="this can take about a minute" />{/if}</div>{/if}
    {#if p.exercise}<div class="practice-nav" role="group" aria-label="Practice activities"><button aria-pressed={view === 'quiz'} onclick={() => appState.practiceViews[key] = 'quiz'}>Quiz{q ? ` · ${q.score}/${q.total}` : ''}</button><button aria-pressed={view === 'code'} onclick={() => appState.practiceViews[key] = 'code'}>Coding exercise</button></div>{/if}
    {#if p.exercise && view === 'code'}<Exercise {c} {l} ex={p.exercise} />
    {:else if s?.finished}{@const reviewing = s.picks?.length === set.quiz.length}<div class="score" tabindex="-1"><strong>{s.score} / {s.total ?? set.quiz.length}</strong><h3>Quiz finished</h3><p>{s.score === (s.total ?? set.quiz.length) ? 'You answered every question correctly. Try using these ideas.' : reviewing ? 'Go over the answers you missed below, then try again.' : 'Review the answers you missed, then try again.'}</p>{#if set.from > 0}<p class="small">This lesson now has {p.quiz?.length} questions. Older ones come back in the stage exam and the daily review.</p>{/if}<div class="row"><button class="btn" id="quiz-again" onclick={again}>Try again</button><button class="btn secondary" id="fresh-practice" onclick={() => loadPractice(true)}>New questions</button></div>{#if reviewing}<AnswerReview qs={set.quiz} picks={s.picks!} />{/if}</div>
    {:else if set.quiz.length && s}<QuizQuestion q={set.quiz[s.index]} session={s} total={set.quiz.length} {answer} next={quizNext} />
    {:else if !set.quiz.length}<p>No questions found for this lesson. Try preparing a new set.</p><button class="btn secondary" id="fresh-practice" onclick={() => loadPractice(true)}>New questions</button>{/if}
  {/if}
</div>
