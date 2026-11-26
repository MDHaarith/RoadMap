<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { appState, keyFor, save, toast, expandPhase, message, finishLesson } from '../lib/state.svelte';
  import { lessons, checked, passed, locked, stageLessons, examPool } from '../lib/progress';
  import { href, enc } from '../lib/format';
  import type { Course, Lesson } from '../lib/types';
  import LessonActions from '../components/LessonActions.svelte';
  import Icon from '../components/Icon.svelte';
  import Formatted from '../components/Formatted.svelte';
  import Videos from '../components/Videos.svelte';
  import ProjectFiles from '../components/ProjectFiles.svelte';
  import WorkCheck from '../components/WorkCheck.svelte';
  import Practice from '../components/Practice.svelte';
  import Milestone from '../components/Milestone.svelte';
  import Tutor from '../components/Tutor.svelte';
  import Chart from '../components/Chart.svelte';
  import Waiting from '../components/Waiting.svelte';
  let { c, l }: { c: Course; l: Lesson } = $props();
  let key = $derived(keyFor(c, l)), ls = $derived(lessons(c)), i = $derived(ls.findIndex(x => x.id === l.id));
  let pn = $derived(c.phases.findIndex(p => p.lessons.some(x => x.id === l.id)));
  let step = $state('watch'), writing = $state(false), writeError = $state(''), achievement = $state('');
  let direction = $state('step-forward');
  let seek = $state<{ k: number; at: number }>();
  let alive = true;
  let isLocked = $derived(locked(c, l));
  let previousStage = $derived((c.phases[pn]?.stage ?? 1) - 1);
  let left = $derived(stageLessons(c, previousStage).filter(x => !c.state.done[x.id]).length);
  // Keep tooltip names for existing keyboard and screenshot selectors; instructions are learner-facing.
  let steps = $derived([
    { id: 'watch', title: 'Watch', short: 'Watch', what: 'Watch one video to see the ideas in use.', next: 'Watch a video' },
    { id: 'learn', title: 'Learn first', short: 'Learn', what: 'Read the key ideas so you can use them.', next: 'Read the lesson' },
    ...(l.charts?.length ? [{ id: 'graphs', title: 'Study the graphs', short: 'Graphs', what: 'Compare the graphs to see what changes.', next: 'Study the graphs' }] : []),
    ...(l.lookups?.length ? [{ id: 'lookup', title: 'Look it up yourself', short: 'Look up', what: 'Compare sources and save useful details in your notes.', next: 'Look up more' }] : []),
    ...(l.files?.length ? [{ id: 'read', title: 'Read the code', short: 'Read code', what: 'Trace how the code works before you change it.', next: 'Read the code' }] : []),
    { id: 'do', title: 'Do it yourself', short: 'Do it', what: 'Try the steps yourself to put the ideas into use.', next: 'Try it yourself' },
    { id: 'practice', title: 'Practice', short: 'Practice', what: 'Answer questions or solve the exercise to check your understanding.', next: 'Practice' },
    { id: 'studio', title: 'Milestone studio', short: 'Studio', what: 'Demonstrate the skill and get feedback.', next: 'Open milestone studio' },
    { id: 'done', title: 'Done when', short: 'Finish', what: 'Check what you can do before completing the lesson.', next: 'Finish the lesson' },
  ]);
  let si = $derived(steps.findIndex(s => s.id === step));
  let ok = $derived(checked(c, l).every(Boolean) || passed(c, l));
  // While practice is unfinished it owns the primary button, so the footer's Next steps back to secondary.
  let practiceOpen = $derived(step === 'practice' && !appState.sessions[key]?.finished);
  onMount(() => {
    if (!isLocked) { if (l.stub) void writePhase(); else enter(); }
    return () => { alive = false; };
  });
  function enter() {
    if (c.state.last !== l.id) { c.state.last = l.id; save(c); }
    appState.activeStep = 'watch';
    visit('watch'); if (location.hash.split('/')[4] === 'studio') void showStep('studio', false); window.scrollTo(0, 0);
  }
  async function writePhase() {
    writing = true; writeError = '';
    try {
      await expandPhase(c, l.id);
      if (lessons(c).find(x => x.id === l.id)?.stub) throw new Error('The AI didn’t finish writing this lesson.');
      if (alive) enter();
    } catch (error) { if (alive) writeError = message(error); }
    finally { writing = false; }
  }
  function visit(id: string) { if (!appState.visited[key]?.includes(id)) appState.visited[key] = [...(appState.visited[key] || []), id]; }
  async function showStep(id: string, focus = true) {
    direction = steps.findIndex(s => s.id === id) < si ? 'step-back' : 'step-forward';
    step = id; seek = undefined; achievement = ''; appState.activeStep = id; visit(id);
    if (focus) { await tick(); document.getElementById('step-title')?.focus({ preventScroll: true }); const content = document.getElementById('lesson-content'); if (content && content.getBoundingClientRect().top < 120) content.scrollIntoView({ block: 'start' }); }
  }
  async function seekVideo(k: number, at: number) {
    if (!l.videos?.[k]) return;
    await showStep('watch', false); seek = { k, at };
  }
  async function copy(phrase: string, button: HTMLButtonElement) {
    try {
      if (navigator.clipboard?.writeText) await navigator.clipboard.writeText(phrase);
      else {
        const input = document.createElement('textarea'); input.value = phrase; input.style.position = 'fixed'; input.style.top = '-1000px'; document.body.append(input); input.select();
        const success = document.execCommand('copy'); input.remove(); button.focus(); if (!success) throw new Error('Copy unavailable');
      }
      toast('Search phrase copied.');
    } catch { toast('Copy unavailable. Select the phrase and copy it with Ctrl+C.'); }
  }
</script>

{#if isLocked}<div class="wrap narrow locked-page"><a class="back-map" href={href(c.id)}><Icon name="back" /> Back</a><div class="page-head"><h1>{l.title}</h1></div><div class="note"><strong><Icon name="lock" /> Stage {previousStage + 2} is locked</strong><p>{left ? `Finish Stage ${previousStage + 1} first: ${left} lesson${left === 1 ? '' : 's'} to go. Then pass its exam.` : `Pass the Stage ${previousStage + 1} exam to open this lesson. It takes about ${Math.min(10, examPool(c, previousStage).length)} minutes.`}</p><div class="row">{#if !left}<a class="btn" href={`${href(c.id)}/exam/${previousStage}`}>Take the exam <Icon name="arrow" /></a>{/if}<a class="btn secondary" href={href(c.id)}>Back to the roadmap</a></div></div></div>
{:else if l.stub || writing}<div class="wrap narrow writing-page"><a class="back-map" href={href(c.id)}><Icon name="back" /> Back</a><p class="overline">Phase {pn + 1}</p><div class="page-head"><h1>{l.title}</h1></div>{#if writeError}<p class="err" role="alert">{writeError}</p><div class="row"><button class="btn" id="retry-phase" onclick={writePhase}>Try again</button><a class="btn ghost" href={href(c.id)}>Back to the roadmap</a></div>{:else}<Waiting skeleton="plan" title="Writing this part of your roadmap" hint="this usually takes about a minute"><p>The AI is writing the lessons of this phase now, using what you have done so far. The lesson opens by itself when it is ready.</p><p class="waiting-link"><a href={href(c.id)}>Back to the roadmap</a></p></Waiting>{/if}</div>
{:else}{@const nextL = ls[i + 1]}<div class="lesson-layout">
  <article class="lesson-article">
    <div class="lesson-crumb"><a href={href(c.id)} title={c.title}><Icon name="back" /> Back</a><span aria-hidden="true">·</span><span title={c.phases[pn].title}>Phase {pn + 1}: {c.phases[pn].title}</span><span aria-hidden="true">·</span><span>Lesson {i + 1} of {ls.length}</span></div>
    <div class="row between"><div class="row lesson-title-row"><h1 class="lesson-title">{l.title}</h1><span id="completed" class="tag ok" hidden={!c.state.done[l.id]}><Icon name="check" /> Lesson complete</span></div><LessonActions {c} {l} /></div><p class="goal"><Formatted text={l.goal} /></p>
    <nav class="lesson-steps" id="lesson-rail" aria-label="Lesson steps">{#each steps as s, j}<button aria-current={step === s.id ? 'step' : undefined} class:visited={appState.visited[key]?.includes(s.id)} title={s.title} onclick={() => showStep(s.id)}><span class="step-label"><span class="step-number">{j + 1}</span><span>{s.short}</span></span></button>{/each}</nav>
    {#key step}<div id="lesson-content" class={`lesson-content ${direction}`}><div class="step-heading"><h2 id="step-title" tabindex="-1">{steps[si].what}</h2></div><div class="step-body">
      {#if step === 'watch'}<Videos {c} {l} {seek} />
      {:else if step === 'learn'}<ol class="content-list">{#each l.concepts || [] as text, j}{@const recap = /^recap:\s*/i.test(text)}<li class:concept-known={!!l.known?.[j]}><div class="concept-copy">{#if recap}<span class="concept-recap">Recap</span>{/if}<Formatted text={recap ? text.replace(/^recap:\s*/i, '') : text} />{#if l.known?.[j]}<span class="tag">You learned this in {l.known[j]}</span>{/if}</div></li>{/each}</ol>
      {:else if step === 'graphs'}{#each l.charts || [] as chart}<Chart {chart} />{/each}
      {:else if step === 'lookup'}{#each l.lookups || [] as x, j}<div class="lookup"><a href={`https://duckduckgo.com/?q=${enc(x)}`}><Icon name="Look it up yourself" /> {x}</a><button class="btn secondary sm" aria-label={`Copy search phrase ${j + 1}`} onclick={e => copy(x, e.currentTarget)}><Icon name="copy" /> Copy</button></div>{/each}
      {:else if step === 'read'}<ProjectFiles {c} {l} />
      {:else if step === 'do'}{#if l.warn}<div class="note warn"><strong><Icon name="Safety" /> Before you begin</strong><p><Formatted text={l.warn} /></p></div>{/if}<ol class="content-list">{#each l.tasks || [] as x}<li><div class="concept-copy"><Formatted text={x} /></div></li>{/each}</ol>{#if c.folder || c.work}{#key l.id}<WorkCheck {c} {l} />{/key}{/if}
      {:else if step === 'practice'}<Practice {c} {l} />
      {:else if step === 'studio'}{#key l.id}<Milestone {c} {l} />{/key}
      {:else if step === 'done'}<div class="checks finish-checks">{#each l.done || [] as x, j}<label><input class="sr" type="checkbox" checked={!!c.state.checks[l.id + '#' + j]} onchange={e => { if (e.currentTarget.checked) c.state.checks[l.id + '#' + j] = true; else delete c.state.checks[l.id + '#' + j]; achievement = ''; save(c); }} /><span class="finish-check" class:complete-mark={!!c.state.checks[l.id + '#' + j]} aria-hidden="true">{#if c.state.checks[l.id + '#' + j]}<Icon name="check" />{/if}</span><span><Formatted text={x} /></span></label>{/each}</div><div class="finish"><div id="fstate" role="status" class={achievement ? 'celebrate' : 'small'}>{#if achievement}<span class="completion-route" aria-hidden="true"><span class="completion-stop complete-mark"><Icon name="check" /></span><i class="completion-traveller"></i><span class="completion-next"></span></span><strong>{achievement}</strong><p>{nextL ? 'Next lesson: ' + nextL.title : 'You have reached the end of this roadmap.'}</p>{:else}{c.state.done[l.id] ? 'Lesson complete.' : checked(c, l).every(Boolean) ? 'All checks are ticked. You can complete the lesson.' : ok ? 'You passed the quiz. You can complete the lesson.' : 'Tick every check, or score at least 80% on the quiz, to complete the lesson.'}{/if}</div></div>{/if}
    </div>
    <div class="step-footer">{#if si}<button class="btn ghost" onclick={() => showStep(steps[si - 1].id)}><Icon name="back" /> Back</button>{/if}
      <nav class="pager" aria-label="Lessons">{#if i}<a href={href(c.id, ls[i - 1].id)} title={ls[i - 1].title}>Previous lesson: {ls[i - 1].title}</a>{:else}<span></span>{/if}{#if nextL}<a href={href(c.id, nextL.id)} title={nextL.title}>Next lesson: {nextL.title}</a>{:else}<a href={href(c.id)}>Back to the roadmap</a>{/if}</nav>
      {#if si < steps.length - 1}<button class="btn" class:secondary={practiceOpen} onclick={() => showStep(steps[si + 1].id)}>Next: {steps[si + 1].next} <Icon name="arrow" /></button>
      {:else if c.state.done[l.id]}<div class="row"><button id="finish" class="btn ghost" onclick={() => achievement = finishLesson()}>Undo completion</button><a class="btn" href={nextL ? href(c.id, nextL.id) : href(c.id)}>{nextL ? 'Next lesson' : 'Back to the roadmap'} <Icon name="arrow" /></a></div>
      {:else}<button id="finish" class="btn" disabled={!ok} onclick={() => achievement = finishLesson()}><Icon name="check" /> Mark lesson complete</button>{/if}</div></div>
    {/key}
  </article><Tutor {c} {l} studio={() => showStep('studio')} seek={seekVideo} />
</div>{/if}
