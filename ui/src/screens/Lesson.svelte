<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { appState, keyFor, save, toast, expandPhase, message, finishLesson } from '../lib/state.svelte';
  import { lessons, checked, passed, locked, stageLessons, examPool } from '../lib/progress';
  import { href, enc } from '../lib/format';
  import type { Course, Lesson } from '../lib/types';
  import Icon from '../components/Icon.svelte';
  import Formatted from '../components/Formatted.svelte';
  import Progress from '../components/Progress.svelte';
  import Videos from '../components/Videos.svelte';
  import ProjectFiles from '../components/ProjectFiles.svelte';
  import Practice from '../components/Practice.svelte';
  import Tutor from '../components/Tutor.svelte';
  let { c, l }: { c: Course; l: Lesson } = $props();
  let key = $derived(keyFor(c, l)), ls = $derived(lessons(c)), i = $derived(ls.findIndex(x => x.id === l.id));
  let pn = $derived(c.phases.findIndex(p => p.lessons.some(x => x.id === l.id)));
  let step = $state('watch'), writing = $state(false), writeError = $state(''), achievement = $state('');
  let seek = $state<{ k: number; at: number }>();
  let alive = true;
  let isLocked = $derived(locked(c, l));
  let previousStage = $derived((c.phases[pn]?.stage ?? 1) - 1);
  let left = $derived(stageLessons(c, previousStage).filter(x => !c.state.done[x.id]).length);
  let steps = $derived([{ id: 'watch', title: 'Watch' }, { id: 'learn', title: 'Learn first' }, ...(l.lookups?.length ? [{ id: 'lookup', title: 'Look it up yourself' }] : []), ...(l.files?.length ? [{ id: 'read', title: 'Read the code' }] : []), { id: 'do', title: 'Do it yourself' }, { id: 'practice', title: 'Practise' }, { id: 'done', title: 'Done when' }]);
  let si = $derived(steps.findIndex(s => s.id === step));
  let ok = $derived(checked(c, l).every(Boolean) || passed(c, l));
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
    if (focus) { await tick(); document.getElementById('step-title')?.focus({ preventScroll: true }); document.getElementById('lesson-content')?.scrollIntoView({ block: 'start' }); }
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

{#if isLocked}<div class="wrap narrow"><div class="note"><h2>Stage {previousStage + 2} is locked</h2><p>{left ? `Finish Stage ${previousStage + 1} first: ${left} lesson${left === 1 ? '' : 's'} to go. Then pass its exam.` : `Pass the Stage ${previousStage + 1} exam to open this lesson. It takes about ${Math.min(10, examPool(c, previousStage).length)} minutes.`}</p><div class="row">{#if !left}<a class="btn" href={`${href(c.id)}/exam/${previousStage}`}>Take the exam <Icon name="arrow" /></a>{/if}<a class="btn secondary" href={href(c.id)}>Back to the roadmap</a></div></div></div>
{:else if l.stub || writing}<div class="wrap narrow">{#if writeError}<p class="err" role="alert">{writeError}</p><div class="row"><button class="btn secondary" id="retry-phase" onclick={writePhase}>Try again</button><a href={href(c.id)}>Back to the roadmap</a></div>{:else}<div class="loading" role="status"><h2>Writing this part of your roadmap</h2><p>The AI is writing the lessons of this phase now, using what you have done so far. This can take a minute.</p><a href={href(c.id)}>Back to the roadmap</a></div>{/if}</div>
{:else}<div class="lesson-layout">
  <div class="lesson-rail" id="lesson-rail"><div class="rail-map"><a class="back-map" href={href(c.id)}><Icon name="back" /> Full roadmap</a><Progress n={ls.filter(x => c.state.done[x.id]).length} total={ls.length} /><p>Lesson {i + 1} of {ls.length} · Phase {pn + 1}</p></div><nav class="lesson-steps" aria-label="Lesson steps">{#each steps as s, j}<button aria-current={step === s.id ? 'step' : undefined} class:visited={appState.visited[key]?.includes(s.id)} onclick={() => showStep(s.id)}><span class="step-number">{j + 1}</span>{s.title}</button>{/each}</nav></div>
  <article class="lesson-article"><div class="kicker">Phase {pn + 1} / {c.phases[pn].title}</div><h1 class="lesson-title">{l.title}</h1><p class="goal"><Formatted text={l.goal} /></p><span id="completed" class="tag" hidden={!c.state.done[l.id]}>Lesson complete</span>
    <div id="lesson-content" class="lesson-content"><div class="step-heading"><h2 id="step-title" tabindex="-1"><Icon name={steps[si].title} /> {steps[si].title}</h2><span class="small">{si + 1} / {steps.length}</span></div><div class="step-body">
      {#if step === 'watch'}<Videos {c} {l} {seek} />
      {:else if step === 'learn'}<p class="muted">Get comfortable with these ideas before you start making.</p><ol class="content-list">{#each l.concepts || [] as text, j}{@const recap = /^recap:\s*/i.test(text)}<li class:concept-known={!!l.known?.[j]}><div class="concept-copy">{#if recap}<span class="concept-recap">Recap</span>{/if}<Formatted text={recap ? text.replace(/^recap:\s*/i, '') : text} />{#if l.known?.[j]}<span class="tag">You learned this in {l.known[j]}</span>{/if}</div></li>{/each}</ol>
      {:else if step === 'lookup'}<p>These searches are <strong>your job</strong>. Open each phrase in your browser, compare what you find, and keep useful discoveries in your notes.</p>{#each l.lookups || [] as x, j}<div class="lookup"><a href={`https://duckduckgo.com/?q=${enc(x)}`}><Icon name="Look it up yourself" /> {x}</a><button class="btn secondary" aria-label={`Copy search phrase ${j + 1}`} onclick={e => copy(x, e.currentTarget)}><Icon name="copy" /> Copy</button></div>{/each}
      {:else if step === 'read'}<ProjectFiles {c} {l} />
      {:else if step === 'do'}{#if l.warn}<div class="note"><strong><Icon name="Safety" /> Before you begin</strong><p style="margin:8px 0 0"><Formatted text={l.warn} /></p></div>{/if}<p class="muted">Try it yourself first. Ask the tutor for a hint when you need one.</p><ol class="content-list">{#each l.tasks || [] as x}<li><Formatted text={x} /></li>{/each}</ol>
      {:else if step === 'practice'}<Practice {c} {l} />
      {:else if step === 'done'}<p class="muted">Check each one when you can honestly say it’s true.</p><div class="checks">{#each l.done || [] as x, j}<label><input type="checkbox" checked={!!c.state.checks[l.id + '#' + j]} onchange={e => { if (e.currentTarget.checked) c.state.checks[l.id + '#' + j] = true; else delete c.state.checks[l.id + '#' + j]; achievement = ''; save(c); }} /><span><Formatted text={x} /></span></label>{/each}</div><div class="finish"><button id="finish" class="btn" disabled={!c.state.done[l.id] && !ok} onclick={() => achievement = finishLesson()}>{c.state.done[l.id] ? 'Completed. Undo' : 'Mark lesson complete'}</button><div id="fstate" role="status" class={achievement ? 'celebrate' : 'small'}>{#if achievement}<strong>{achievement}</strong><p>Lesson complete.</p><a class="btn" href={ls[i + 1] ? href(c.id, ls[i + 1].id) : href(c.id)}>{ls[i + 1] ? 'Next lesson' : 'See the roadmap'}{#if ls[i + 1]} <Icon name="arrow" />{/if}</a>{:else}{c.state.done[l.id] ? 'Lesson complete.' : checked(c, l).every(Boolean) ? 'All checks ticked.' : ok ? 'Your quiz score shows you know this. You can complete it without the tasks.' : 'Tick every check to unlock completion. Know this already? Score 80% on the quiz to skip it.'}{/if}</div></div>{/if}
    </div><div class="step-footer">{#if si}<button class="link" onclick={() => showStep(steps[si - 1].id)}>Back</button>{:else}<span></span>{/if}{#if si < steps.length - 1}<button class="btn secondary" onclick={() => showStep(steps[si + 1].id)}>Next: {steps[si + 1].title} <Icon name="arrow" /></button>{:else}<a href={href(c.id)} class="small">See the roadmap</a>{/if}</div></div>
    <nav class="pager" aria-label="Lessons">{#if i}<a href={href(c.id, ls[i - 1].id)}>Previous: {ls[i - 1].title}</a>{:else}<span></span>{/if}{#if ls[i + 1]}<a href={href(c.id, ls[i + 1].id)}>Next: {ls[i + 1].title}</a>{:else}<a href={href(c.id)}>Back to roadmap</a>{/if}</nav>
  </article><Tutor {c} {l} seek={seekVideo} />
</div>{/if}
