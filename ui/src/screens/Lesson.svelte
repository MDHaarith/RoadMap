<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { appState, keyFor, save, toast, expandPhase, message, finishLesson } from '../lib/state.svelte';
  import { lessons, checked, passed, locked, stageLessons, examPool } from '../lib/progress';
  import { href, enc } from '../lib/format';
  import type { Course, Lesson } from '../lib/types';
  import Icon from '../components/Icon.svelte';
  import Formatted from '../components/Formatted.svelte';
    import Videos from '../components/Videos.svelte';
  import ProjectFiles from '../components/ProjectFiles.svelte';
  import Practice from '../components/Practice.svelte';
  import Tutor from '../components/Tutor.svelte';
  import Chart from '../components/Chart.svelte';
  import Waiting from '../components/Waiting.svelte';
  let { c, l }: { c: Course; l: Lesson } = $props();
  let key = $derived(keyFor(c, l)), ls = $derived(lessons(c)), i = $derived(ls.findIndex(x => x.id === l.id));
  let pn = $derived(c.phases.findIndex(p => p.lessons.some(x => x.id === l.id)));
  let step = $state('watch'), writing = $state(false), writeError = $state(''), achievement = $state('');
  let seek = $state<{ k: number; at: number }>();
  let alive = true;
  let isLocked = $derived(locked(c, l));
  let previousStage = $derived((c.phases[pn]?.stage ?? 1) - 1);
  let left = $derived(stageLessons(c, previousStage).filter(x => !c.state.done[x.id]).length);
  // title doubles as the Icon name; short is the stepper label; what says what to do in this step.
  let steps = $derived([
    { id: 'watch', title: 'Watch', short: 'Watch', what: 'Watch one video. Pick another from the list if this one doesn’t suit you.' },
    { id: 'learn', title: 'Learn first', short: 'Learn', what: 'Get comfortable with these ideas before you start making.' },
    ...(l.charts?.length ? [{ id: 'graphs', title: 'Study the graphs', short: 'Graphs', what: 'Read each graph before you start. Hover a point to see its value.' }] : []),
    ...(l.lookups?.length ? [{ id: 'lookup', title: 'Look it up yourself', short: 'Look up', what: 'These searches are **your job**. Open each phrase in your browser, compare what you find, and keep useful discoveries in your notes.' }] : []),
    ...(l.files?.length ? [{ id: 'read', title: 'Read the code', short: 'Read code', what: 'Read these files from your project. Use your notes to trace what happens before you change anything.' }] : []),
    { id: 'do', title: 'Do it yourself', short: 'Do it', what: 'Try it yourself first. Ask the tutor for a hint when you need one.' },
    { id: 'practice', title: 'Practise', short: 'Practise', what: 'A few questions to check what stuck. No rush, and no penalty for getting it wrong.' },
    { id: 'done', title: 'Done when', short: 'Finish', what: 'Tick each one when you can honestly say it’s true. Then mark the lesson complete.' },
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
    visit('watch'); window.scrollTo(0, 0);
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
  function visit(id: string) { const visited = appState.visited[key] ||= []; if (!visited.includes(id)) visited.push(id); }
  async function showStep(id: string, focus = true) {
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

{#if isLocked}<div class="locked-page"><div class="note"><strong><Icon name="lock" /> Stage {previousStage + 2} is locked</strong><p>{left ? `Finish Stage ${previousStage + 1} first: ${left} lesson${left === 1 ? '' : 's'} to go. Then pass its exam.` : `Pass the Stage ${previousStage + 1} exam to open this lesson. It takes about ${Math.min(10, examPool(c, previousStage).length)} minutes.`}</p><div class="row">{#if !left}<a class="btn" href={`${href(c.id)}/exam/${previousStage}`}>Take the exam <Icon name="arrow" /></a>{/if}<a class="btn secondary" href={href(c.id)}>Back to the roadmap</a></div></div></div>
{:else if l.stub || writing}<div class="writing-page">{#if writeError}<p class="err" role="alert">{writeError}</p><div class="row"><button class="btn" id="retry-phase" onclick={writePhase}>Try again</button><a class="btn ghost" href={href(c.id)}>Back to the roadmap</a></div>{:else}<p class="small">{c.title} · Phase {pn + 1}</p><h1>{l.title}</h1><Waiting title="Writing this part of your roadmap" hint="this usually takes about a minute"><p>The AI is writing the lessons of this phase now, using what you have done so far. The lesson opens by itself when it is ready.</p><p style="margin-top:12px"><a href={href(c.id)}>Back to the roadmap</a></p></Waiting>{/if}</div>
{:else}{@const nextL = ls[i + 1]}<div class="lesson-layout">
  <article class="lesson-article">
    <div class="lesson-crumb"><a href={href(c.id)}><Icon name="back" /> {c.title}</a><span aria-hidden="true">·</span><span>Phase {pn + 1}: {c.phases[pn].title}</span><span aria-hidden="true">·</span><span>Lesson {i + 1} of {ls.length}</span></div>
    <h1 class="lesson-title">{l.title}</h1><p class="goal"><Formatted text={l.goal} /></p><span id="completed" class="tag ok" hidden={!c.state.done[l.id]}><Icon name="check" /> Lesson complete</span>
    <nav class="lesson-steps" id="lesson-rail" aria-label="Lesson steps">{#each steps as s, j}<button aria-current={step === s.id ? 'step' : undefined} class:visited={appState.visited[key]?.includes(s.id)} title={s.title} onclick={() => showStep(s.id)}><span class="step-number">{j + 1}</span><span>{s.short}</span></button>{/each}</nav>
    {#key step}<div id="lesson-content" class="lesson-content"><div class="step-heading"><span class="small">Step {si + 1} of {steps.length}</span><h2 id="step-title" tabindex="-1"><Icon name={steps[si].title} /> {steps[si].title}</h2><p><Formatted text={steps[si].what} /></p></div><div class="step-body">
      {#if step === 'watch'}<Videos {c} {l} {seek} />
      {:else if step === 'learn'}<ol class="content-list">{#each l.concepts || [] as text, j}{@const recap = /^recap:\s*/i.test(text)}<li class:concept-known={!!l.known?.[j]}><div class="concept-copy">{#if recap}<span class="concept-recap">Recap</span>{/if}<Formatted text={recap ? text.replace(/^recap:\s*/i, '') : text} />{#if l.known?.[j]}<span class="tag">You learned this in {l.known[j]}</span>{/if}</div></li>{/each}</ol>
      {:else if step === 'graphs'}{#each l.charts || [] as chart}<Chart {chart} />{/each}
      {:else if step === 'lookup'}{#each l.lookups || [] as x, j}<div class="lookup"><a href={`https://duckduckgo.com/?q=${enc(x)}`}><Icon name="Look it up yourself" /> {x}</a><button class="btn secondary sm" aria-label={`Copy search phrase ${j + 1}`} onclick={e => copy(x, e.currentTarget)}><Icon name="copy" /> Copy</button></div>{/each}
      {:else if step === 'read'}<ProjectFiles {c} {l} />
      {:else if step === 'do'}{#if l.warn}<div class="note warn"><strong><Icon name="Safety" /> Before you begin</strong><p><Formatted text={l.warn} /></p></div>{/if}<ol class="content-list">{#each l.tasks || [] as x}<li><Formatted text={x} /></li>{/each}</ol>
      {:else if step === 'practice'}<Practice {c} {l} />
      {:else if step === 'done'}<div class="checks">{#each l.done || [] as x, j}<label><input type="checkbox" checked={!!c.state.checks[l.id + '#' + j]} onchange={e => { if (e.currentTarget.checked) c.state.checks[l.id + '#' + j] = true; else delete c.state.checks[l.id + '#' + j]; achievement = ''; save(c); }} /><span><Formatted text={x} /></span></label>{/each}</div><div class="finish"><div id="fstate" role="status" class={achievement ? 'celebrate' : 'small'}>{#if achievement}<strong>{achievement}</strong><p>{nextL ? 'Next up: ' + nextL.title : 'You have reached the end of this roadmap.'}</p>{:else}{c.state.done[l.id] ? 'Lesson complete.' : checked(c, l).every(Boolean) ? 'All checks ticked. You can mark the lesson complete.' : ok ? 'Your quiz score shows you know this. You can complete it without the tasks.' : 'Tick every check to unlock completion. Know this already? Score 80% on the quiz to skip it.'}{/if}</div></div>{/if}
    </div>
    <div class="step-footer">{#if si}<button class="btn ghost" onclick={() => showStep(steps[si - 1].id)}><Icon name="back" /> Back</button>{:else}<span></span>{/if}
      {#if si < steps.length - 1}<button class="btn" class:secondary={practiceOpen} onclick={() => showStep(steps[si + 1].id)}>Next: {steps[si + 1].title} <Icon name="arrow" /></button>
      {:else if c.state.done[l.id]}<div class="row"><button id="finish" class="btn ghost" onclick={() => achievement = finishLesson()}>Completed. Undo</button><a class="btn" href={nextL ? href(c.id, nextL.id) : href(c.id)}>{nextL ? 'Next lesson' : 'See the roadmap'} <Icon name="arrow" /></a></div>
      {:else}<button id="finish" class="btn" disabled={!ok} onclick={() => achievement = finishLesson()}><Icon name="check" /> Mark lesson complete</button>{/if}</div></div>
    {/key}<nav class="pager" aria-label="Lessons">{#if i}<a href={href(c.id, ls[i - 1].id)}>Previous: {ls[i - 1].title}</a>{:else}<span></span>{/if}{#if nextL}<a href={href(c.id, nextL.id)}>Next: {nextL.title}</a>{:else}<a href={href(c.id)}>Back to roadmap</a>{/if}</nav>
  </article><Tutor {c} {l} seek={seekVideo} />
</div>{/if}
