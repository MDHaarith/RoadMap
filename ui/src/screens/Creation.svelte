<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { api } from '../lib/api';
  import { appState, toast, message, goCourse, aiProblem, defaultEffort } from '../lib/state.svelte';
  import EffortPicker from '../components/EffortPicker.svelte';
  import AiSetup from '../components/AiSetup.svelte';
  import { autogrow } from '../lib/autogrow';
  import type { Course } from '../lib/types';
  import Icon from '../components/Icon.svelte';
  import Waiting from '../components/Waiting.svelte';
  import Skeleton from '../components/Skeleton.svelte';
  import Elapsed from '../components/Elapsed.svelte';
  import BusyLabel from '../components/BusyLabel.svelte';
  import ModelPicker from '../components/ModelPicker.svelte';
  import { watchThinking } from '../lib/thinking';
  let { empty = false }: { empty?: boolean } = $props();
  let kind = $state('build'), folder = $state(''), prompt = $state(''), level = $state('Complete beginner: has never studied or done this'), size = $state('');
  let effort = $state(defaultEffort());
  let picking = $state(false), building = $state(false), waiting = $state(false), designing = $state(false);
  let error = $state(''), question = $state(''), options = $state<string[]>([]), own = $state('');
  let answers = $state<{ q: string; a: string }[]>([]);
  let answer: ((a: string | null) => void) | undefined;
  let promptEl: HTMLTextAreaElement;
  let alive = true;
  // The first item is sent to the AI as the learner's level, so it carries the meaning, not only the label.
  const goalLevels = [
    ['Complete beginner: has never studied or done this', 'Complete beginner', 'Start from zero. Teach only what this goal needs.'],
    ['Beginner: has tried a little and knows a few basics', 'Beginner', 'I know a few basics. Fill the gaps and build on them.'],
    ['Intermediate: knows the basics and wants to go further', 'Intermediate', 'Skip the basics. Go deeper on what this goal needs.'],
    ['Experienced: knows this well; skip what they know and focus on gaps', 'Experienced', 'Skip what I know. Focus on the gaps and the harder parts.'],
  ];
  const projectLevels = [
    ['Complete beginner', 'I’m a beginner', 'Teach me only the basics this project needs.'],
    ['I already code; skip programming basics', 'I already code', 'Skip programming basics. Teach the tools and ideas this project uses.'],
    ['Experienced; skip programming basics and focus on gaps', 'I’m experienced', 'Skip programming basics. Focus on the decisions, structure and gaps in my knowledge.'],
  ];
  let levels = $derived(kind === 'project' ? projectLevels : goalLevels);
  $effect(() => { if (!levels.some(x => x[0] === level)) level = levels[0][0]; });
  const sizes = [
    ['sprint', 'A quick start', '2 weeks', 'About two weeks. Enough to get one small thing working.', 'about 1½ minutes'],
    ['', 'One project', '1 month', 'About a month. One project from start to finish.', '1½ to 2 minutes'],
    ['deep', 'Deep', '3 months', 'About three months. Fundamentals, larger projects and professional practice.', '2 to 3 minutes'],
    ['mastery', 'Mastery', '6 months', 'About six months. From the fundamentals to how it is done in industry, with a milestone project every month.', 'about 4 minutes'],
  ];
  let noService = $derived(!!aiProblem());
  let levelHint = $derived(levels.find(x => x[0] === level)?.[2] ?? ''), sized = $derived(sizes.find(x => x[0] === size) ?? sizes[1]);
  onMount(() => { if (!empty) promptEl?.focus(); return () => { alive = false; answer?.(null); }; });
  async function pickFolder() {
    picking = true;
    try { const r = await api<{ folder: string }>('POST', 'api/pick-folder'); if (r.folder) folder = r.folder; }
    catch (error) { toast(message(error)); } finally { picking = false; }
  }
  // What the model says it is thinking about during the current wait.
  let thought = $state('');
  async function thinkingOn<T>(run: (key: string) => Promise<T>) {
    thought = '';
    const w = watchThinking(t => { if (alive) thought = t; });
    try { return await run(w.key); } finally { w.stop(); thought = ''; }
  }
  async function create() {
    if (building) return;
    if (kind === 'project' && !folder) { toast('Choose a project folder first.'); document.getElementById('pick-folder')?.focus(); return; }
    const version = appState.routeVersion, live = () => alive && version === appState.routeVersion;
    const what = { prompt, ...(kind === 'project' ? { folder } : {}) };
    building = true; answers = []; error = '';
    try {
      for (;;) {
        waiting = true; question = '';
        const r = await thinkingOn(w => api<{ done?: boolean; q?: string; options: string[] }>('POST', 'api/placement', { ...what, level, effort, thinking: w, answers: $state.snapshot(answers) }));
        if (!live()) return;
        if (r.done || !r.q) break;
        waiting = false; question = r.q; options = r.options; own = '';
        await tick(); document.getElementById('pa-q')?.focus({ preventScroll: true });
        const a = await new Promise<string | null>(resolve => { answer = resolve; });
        answer = undefined;
        if (!live()) return;
        if (a === null) break;
        answers.push({ q: r.q, a });
      }
      designing = true;
      const c = await thinkingOn(w => api<Course>('POST', 'api/courses', { ...what, level, size, effort, thinking: w, answers: $state.snapshot(answers) }));
      if (live()) goCourse(c);
    } catch (err) { if (live()) error = message(err); }
    finally { building = false; designing = false; waiting = false; }
  }
</script>

<div class="wrap narrow create screen-enter" class:designing>
  {#if !empty}<a class="back-map" id="cancel-new" href="#/" onclick={() => appState.screen = 'library'}><Icon name="back" /> Your library</a>{:else if !building}<div class="create-empty-route"><Icon name="empty-route" /></div>{/if}
  <div class="page-head create-head" hidden={designing}><h1>{building ? 'Setting up your roadmap' : empty ? 'What do you want to learn?' : 'New roadmap'}</h1></div>
  <AiSetup why="Add one in Settings to build a roadmap." />
  <form id="new" onsubmit={e => { e.preventDefault(); void create(); }}>
    <div class="form-col" hidden={building}>
      <fieldset class="segmented kind-choice"><legend class="sr">What kind of roadmap?</legend><label><input type="radio" name="kind" value="build" bind:group={kind} />Learn or build something new</label><label><input type="radio" name="kind" value="project" bind:group={kind} />Understand a project I have</label></fieldset>
      <div id="project-fields" hidden={kind !== 'project'}>
        <div class="folder-row"><button class="btn secondary" type="button" id="pick-folder" disabled={picking} onclick={pickFolder}><BusyLabel busy={picking} idle={folder ? 'Choose another folder' : 'Choose the project folder'} label="Choosing folder…" /></button><p class="folder-path" id="folder-path" role="status">{folder || 'No folder selected.'}</p></div>
        <p class="privacy"><strong>Before you share this project:</strong> Roadmap reads text files in this folder and sends part of them to the AI service set in <a href="#/settings">Settings</a>. Hidden files, dependency folders and files that look like secrets or keys are skipped. A local model keeps the code on this computer.</p>
      </div>
      <label for="prompt" id="prompt-label">{kind === 'project' ? 'What do you want to be able to change? (optional)' : 'I want to…'}</label>
      <textarea id="prompt" use:autogrow={{ max: 180, value: prompt }} rows={kind === 'project' ? 2 : 3} required={kind !== 'project'} bind:this={promptEl} bind:value={prompt} placeholder={kind === 'project' ? 'For example: change how sign-in works, or add a new export format.' : 'Build a weather station with an ESP32. I know basic Arduino.'}></textarea>
      <div class="examples" id="goal-examples" aria-label="Example goals" hidden={kind === 'project'}><span class="small">Try:</span>{#each ['Build an ESP32 weather station', 'Bake my first sourdough', 'Learn Python with a todo CLI'] as x}<button type="button" onclick={() => { prompt = x; promptEl.focus(); }}>{x}</button>{/each}</div>
      <fieldset class="field levels" aria-describedby="level-hint"><legend>Where are you starting?</legend><div class="segmented full">{#each levels as [v, t]}<label><input type="radio" name="level" value={v} bind:group={level} />{t}</label>{/each}</div><p class="hint" id="level-hint">{levelHint}</p></fieldset>
      <fieldset class="field" id="size-field" aria-describedby="size-hint"><legend>How far do you want to go?</legend><div class="segmented full">{#each sizes as [v, t, m]}<label><input type="radio" name="size" value={v} bind:group={size} /><span>{t}<small>{m}</small></span></label>{/each}</div><p class="hint" id="size-hint">{sized[3]} At about five hours a week.</p></fieldset>
      <div class="create-pickers"><span class="create-model"><label class="small" for="create-model">AI model</label><ModelPicker id="create-model" /></span>{#if appState.config.base === 'codex'}<span class="create-model"><label class="small" for="create-effort">Thinking</label><EffortPicker id="create-effort" bind:value={effort} /></span>{/if}</div>
      <div class="create-go"><button class="btn big" id="build-roadmap" disabled={noService}>{error ? 'Try again' : 'Build my roadmap'} <Icon name="arrow" /></button>{#if error}<p class="create-error" role="alert"><strong>The roadmap could not be built.</strong> {error} If it keeps failing, check the AI service in <a href="#/settings">Settings</a>.</p>{:else}<span class="small">Next: a few quick questions about what you already know.</span>{/if}</div>
    </div>
    <div id="building" hidden={!building} class={designing ? 'designing' : 'placement'}>
      {#if designing}<div class="overview creating" aria-busy="true"><div class="overview-side" role="status"><h1>Writing your roadmap</h1>{#if prompt.trim()}<p class="create-goal"><span>Your goal</span> {prompt}</p>{/if}<p class="creating-time"><span aria-hidden="true"><Elapsed /></span> · this usually takes {sized[4]}</p>{#if thought}{#key thought}<p class="thought">{thought}</p>{/key}{/if}<ol class="build-steps"><li>Plan the stages, phases and lessons for your goal.</li><li>Then find videos for every lesson, in the background.</li></ol><p class="muted small">Keep this page open. Your roadmap opens here when it is ready.</p></div><div class="overview-main"><Skeleton kind="lessons" /></div></div>
      {:else}{#if prompt.trim()}<p class="create-goal"><span>Your goal</span> {prompt}</p>{/if}<h2>What can you already do?</h2><p class="muted">{waiting ? 'A few questions, one at a time. Each one depends on your last answer.' : 'Pick the honest answer or type your own. “No” is useful: it puts that topic into your roadmap.'}</p>
        {#if answers.length}<ol class="told stagger">{#each answers as x}<li><span>{x.q}</span><b>{x.a}</b></li>{/each}</ol>{/if}
        {#if waiting}<Waiting skeleton="question" title={answers.length ? 'Reading your answer' : 'Preparing the first question'} hint="this usually takes 10 to 20 seconds" {thought} />
        {:else}<div class="asking"><h3 class="question" id="pa-q" tabindex="-1">{question}</h3><div class="options stagger">{#each options as o, i}<button class="option" type="button" onclick={() => answer?.(o)}><span class="letter">{String.fromCharCode(65 + i)}</span><span>{o}</span></button>{/each}</div><div class="own"><input id="pa-own" aria-label="Your own answer" placeholder="Or answer in your own words" maxlength="300" bind:value={own} onkeydown={e => { if (e.key === 'Enter') { e.preventDefault(); if (own.trim()) answer?.(own.trim()); } }} /><button class="btn secondary" type="button" id="pa-send" onclick={() => { if (own.trim()) answer?.(own.trim()); }}>Send</button></div></div><button class="link" type="button" id="unplaced" onclick={() => answer?.(null)}>Skip the questions and build now</button>{/if}
      {/if}
    </div>
  </form>
</div>
