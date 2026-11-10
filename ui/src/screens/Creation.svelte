<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { api } from '../lib/api';
  import { appState, toast, message, goCourse } from '../lib/state.svelte';
  import type { Course } from '../lib/types';
  import Icon from '../components/Icon.svelte';
  let { empty = false }: { empty?: boolean } = $props();
  let kind = $state('build'), folder = $state(''), prompt = $state(''), level = $state('Complete beginner'), size = $state('');
  let picking = $state(false), building = $state(false), waiting = $state(false), designing = $state(false);
  let question = $state(''), options = $state<string[]>([]), own = $state('');
  let answers = $state<{ q: string; a: string }[]>([]);
  let answer: ((a: string | null) => void) | undefined;
  let promptEl: HTMLTextAreaElement;
  let alive = true;
  const levels = [
    ['Complete beginner', 'I’m a beginner', 'Teach me only the basics this goal or project needs.'],
    ['I already code; skip programming basics', 'I already code', 'Skip programming basics. Teach the tools and ideas I need for this goal or project.'],
    ['Experienced; skip programming basics and focus on gaps', 'I’m experienced', 'Skip programming basics. Focus on the decisions, structure and gaps in my knowledge.'],
  ];
  const sizes = [
    ['sprint', 'A quick start', '2 weeks', 'About two weeks. Enough to get one small thing working.'],
    ['', 'One project', '1 month', 'About a month. One project from start to finish.'],
    ['deep', 'Deep', '3 months', 'About three months. Fundamentals, larger projects and professional practice.'],
    ['mastery', 'Mastery', '6 months', 'About six months. From the fundamentals to how it is done in industry, with a milestone project every month.'],
  ];
  onMount(() => { if (!empty) promptEl?.focus(); return () => { alive = false; answer?.(null); }; });
  async function pickFolder() {
    picking = true;
    try { const r = await api<{ folder: string }>('POST', 'api/pick-folder'); if (r.folder) folder = r.folder; }
    catch (error) { toast(message(error)); } finally { picking = false; }
  }
  async function create() {
    if (building) return;
    if (kind === 'project' && !folder) { toast('Choose a project folder first.'); document.getElementById('pick-folder')?.focus(); return; }
    const version = appState.routeVersion, live = () => alive && version === appState.routeVersion;
    const what = { prompt, ...(kind === 'project' ? { folder } : {}) };
    building = true; answers = [];
    try {
      for (;;) {
        waiting = true; question = '';
        const r = await api<{ done?: boolean; q?: string; options: string[] }>('POST', 'api/placement', { ...what, answers: $state.snapshot(answers) });
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
      const c = await api<Course>('POST', 'api/courses', { ...what, level, size, answers: $state.snapshot(answers) });
      if (live()) goCourse(c);
    } catch (error) { if (live()) toast(message(error)); }
    finally { building = false; designing = false; waiting = false; }
  }
</script>

<div class="wrap narrow">
  <div class="intro"><div><div class="kicker"><Icon name="flag" /> Your next adventure</div><h1>{empty ? 'What do you want to learn?' : 'Add roadmap'}</h1><p>Build something of your own, or find your way around a project you already have.</p></div>{#if !empty}<button class="btn secondary" id="cancel-new" type="button" onclick={() => appState.screen = 'library'}>Back to library</button>{/if}</div>
  {#if !appState.config.base || !appState.config.model}<div class="note setup"><div><strong>No AI service connected</strong><p>Add one in Settings to build a roadmap.</p></div><a class="btn secondary" href="#/settings">Open Settings</a></div>{/if}
  <div class="creation">
    <div class="creation-story"><h2>A goal.<br /> A route.<br /> Your own work.</h2><p>Free videos, things to try, and a tutor for the moments you get stuck.</p></div>
    <form id="new" onsubmit={e => { e.preventDefault(); void create(); }}>
      <div class="form-col" hidden={building}>
        <fieldset class="kind-choice"><legend>What kind of roadmap?</legend><label><input type="radio" name="kind" value="build" bind:group={kind} /><span>Build something new</span></label><label><input type="radio" name="kind" value="project" bind:group={kind} /><span>Understand an existing project</span></label></fieldset>
        <div id="project-fields" hidden={kind !== 'project'}>
          <button class="btn secondary" type="button" id="pick-folder" disabled={picking} onclick={pickFolder}>{picking ? 'Choosing folder…' : folder ? 'Choose another folder' : 'Choose folder'}</button>
          <p class="folder-path" id="folder-path" role="status">{folder || 'No folder selected.'}</p>
          <div class="note privacy"><strong>Before you share this project</strong><p>Roadmap reads text files in this folder and sends part of them to the AI service configured in <a href="#/settings">Settings</a>. Hidden files, dependency folders and files that look like secrets or keys are skipped.</p><p>A local model keeps the code on this computer.</p></div>
        </div>
        <label for="prompt" id="prompt-label">{kind === 'project' ? 'What do you want to be able to change? (optional)' : 'I want to…'}</label>
        <textarea id="prompt" rows="3" required={kind !== 'project'} bind:this={promptEl} bind:value={prompt} placeholder={kind === 'project' ? 'For example: change how sign-in works, or add a new export format.' : 'Build a weather station with an ESP32. I know basic Arduino.'}></textarea>
        <div class="examples" id="goal-examples" aria-label="Example goals" hidden={kind === 'project'}>{#each ['Build an ESP32 weather station', 'Bake my first sourdough', 'Learn Python with a todo CLI'] as x}<button type="button" onclick={() => { prompt = x; promptEl.focus(); }}>{x}</button>{/each}</div>
      </div>
      <div class="form-col" hidden={building}>
        <fieldset class="levels"><legend>Where are you starting?</legend>{#each levels as [v, t, d]}<label><input type="radio" name="level" value={v} bind:group={level} /><span><b>{t}</b><small>{d}</small></span></label>{/each}</fieldset>
        <fieldset class="levels" id="size-field"><legend>How far do you want to go? (at about five hours a week)</legend>{#each sizes as [v, t, m, d]}<label><input type="radio" name="size" value={v} bind:group={size} /><span><b>{t}<i>{m}</i></b><small>{d}</small></span></label>{/each}</fieldset>
        <button class="btn" id="build-roadmap">Build my roadmap <Icon name="arrow" /></button>
      </div>
      <div id="building" hidden={!building} class={designing ? 'loading' : 'placement'}>
        {#if designing}<h2>Designing your roadmap</h2><p>The AI is deciding how to get from your starting point to your goal. This can take a minute or more.</p><div class="build-stages"><div><span class="step-number">1</span><strong>Design the phases and lessons</strong></div><div class="muted"><span class="step-number">2</span><span>Search videos for every lesson in the background</span></div></div><p class="small" style="margin-top:24px">Your roadmap opens as soon as the design is ready. Videos may arrive a little later.</p>
        {:else}<h2>What can you already do?</h2><p>{waiting ? 'The AI asks a few questions, one at a time. Each one depends on your last answer.' : 'Pick the honest answer or type your own. “No” is useful: it puts that topic into your roadmap.'}</p>
          {#each answers as x}<div class="told"><span>{x.q}</span><b>{x.a}</b></div>{/each}
          {#if waiting}<p class="muted" role="status">{answers.length ? 'Reading your answer…' : 'Preparing the first question…'}</p>
          {:else}<div class="asking"><h3 id="pa-q" tabindex="-1">{question}</h3><div class="picks">{#each options as o}<button class="btn secondary" type="button" onclick={() => answer?.(o)}>{o}</button>{/each}</div><div class="own"><input id="pa-own" aria-label="Your own answer" placeholder="Or answer in your own words" maxlength="300" bind:value={own} onkeydown={e => { if (e.key === 'Enter') { e.preventDefault(); if (own.trim()) answer?.(own.trim()); } }} /><button class="btn secondary" type="button" id="pa-send" onclick={() => { if (own.trim()) answer?.(own.trim()); }}>Send</button></div></div><button class="plain" type="button" id="unplaced" onclick={() => answer?.(null)}>Skip the questions and build now</button>{/if}
        {/if}
      </div>
    </form>
  </div>
</div>
