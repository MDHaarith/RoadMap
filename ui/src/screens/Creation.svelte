<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { api } from '../lib/api';
  import { appState, toast, message, goCourse, aiProblem, defaultEffort } from '../lib/state.svelte';
  import EffortPicker from '../components/EffortPicker.svelte';
  import BlueprintFields from '../components/BlueprintFields.svelte';
  import AiSetup from '../components/AiSetup.svelte';
  import { autogrow } from '../lib/autogrow';
  import type { Course, Blueprint } from '../lib/types';
  import Icon from '../components/Icon.svelte';
  import Waiting from '../components/Waiting.svelte';
  import Skeleton from '../components/Skeleton.svelte';
  import Elapsed from '../components/Elapsed.svelte';
  import BusyLabel from '../components/BusyLabel.svelte';
  import ModelPicker from '../components/ModelPicker.svelte';
  import Progress from '../components/Progress.svelte';
  import { watchThinking, type Draft } from '../lib/thinking';
  let { empty = false }: { empty?: boolean } = $props();
  let kind = $state('build'), folder = $state(''), prompt = $state(''), level = $state('Complete beginner: has never studied or done this'), size = $state('');
  let effort = $state(defaultEffort());
  let blueprint = $state<Blueprint>(), baseline = $state(''), editingPlan = $state(false);
  // The blueprint is plain text from the AI; show it as tidy lines, without markdown or list markers.
  const tidy = (t: string) => t.replace(/`/g, '').trim();
  function lines(t: string, sentences = false) {
    let xs = t.split('\n').map(x => tidy(x.replace(/^\s*([-*•]|\d+[.)])\s*/, ''))).filter(Boolean);
    if (sentences && xs.length === 1) xs = xs[0].split(/(?<=[.!?])\s+(?=[A-Z])/);
    return xs;
  }
  let picking = $state(false), building = $state(false), waiting = $state(false), designing = $state(false);
  let error = $state('');
  // The whole placement check arrives at once; picked[i] is the answer to qs[i], extra is an optional note.
  let qs = $state<{ q: string; options: string[] }[]>([]), picked = $state<string[]>([]), extra = $state('');
  let answered = $derived(qs.filter((_, i) => picked[i]).length);
  const answers = () => [...qs.flatMap((x, i) => picked[i] ? [{ q: x.q, a: picked[i] }] : []), ...(extra.trim() ? [{ q: 'Anything else the course should know?', a: extra.trim() }] : [])];
  // Bumped by every question request and by Edit goal, so a stale reply from the AI is dropped.
  let seq = 0, failed = $state<'' | 'ask' | 'blueprint'>(''), drafting = $state(false);
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
  onMount(() => { if (!empty) promptEl?.focus(); return () => { alive = false; }; });
  async function pickFolder() {
    picking = true;
    try { const r = await api<{ folder: string }>('POST', 'api/pick-folder'); if (r.folder) folder = r.folder; }
    catch (error) { toast(message(error)); } finally { picking = false; }
  }
  // What the model says it is thinking about during the current wait, and the roadmap it has written so far.
  let thought = $state(''), draft = $state<Draft>();
  async function thinkingOn<T>(run: (key: string) => Promise<T>) {
    thought = '';
    const w = watchThinking(t => { if (alive) thought = t; }, d => { if (alive) draft = d; });
    try { return await run(w.key); } finally { w.stop(); thought = ''; }
  }
  const what = () => ({ prompt, ...(kind === 'project' ? { folder } : {}) });
  // Each step of making a roadmap slides in from the side it comes from.
  // shortcut: CSS slide, not view transitions: startViewTransition crashes the WebKitGTK 2.54 web process; revisit when it is fixed.
  let flow = $state('step-forward');
  function go(update: () => void, dir: 'forward' | 'backward') { flow = dir === 'forward' ? 'step-forward' : 'step-back'; update(); }
  function create() {
    if (building) return;
    if (kind === 'project' && !folder) { toast('Choose a project folder first.'); document.getElementById('pick-folder')?.focus(); return; }
    go(() => { building = true; qs = []; picked = []; extra = ''; error = ''; }, 'forward');
    void ask();
  }
  // Asks for every placement question in one go; with none, the blueprint is drafted straight away.
  async function ask() {
    const mine = ++seq, version = appState.routeVersion, live = () => alive && mine === seq && version === appState.routeVersion;
    waiting = true; error = ''; failed = '';
    try {
      const r = await thinkingOn(w => api<{ questions: { q: string; options: string[] }[] }>('POST', 'api/placement', { ...what(), level, effort, thinking: w }));
      if (!live()) return;
      if (!r.questions.length) return await toBlueprint();
      qs = r.questions; picked = []; waiting = false;
      await tick(); document.getElementById('pc-title')?.focus({ preventScroll: true });
    } catch (err) { if (live()) { error = message(err); failed = 'ask'; waiting = false; } }
  }
  async function toBlueprint() {
    const mine = ++seq, live = () => alive && mine === seq;
    waiting = true; drafting = true; error = ''; failed = '';
    try {
      const plan = await thinkingOn(w => api<Blueprint>('POST', 'api/blueprint', { ...what(), level, size, effort, thinking: w, answers: answers() }));
      if (live()) go(() => { blueprint = plan; baseline = ''; editingPlan = false; building = false; }, 'forward');
    } catch (err) { if (live()) { error = message(err); failed = 'blueprint'; } }
    finally { if (live()) { waiting = false; drafting = false; } }
  }
  const levelName = () => levels.find(x => x[0] === level)?.[1] ?? level;
  // The stages (long roadmaps) or phases written so far; the last one is still being written.
  let units = $derived(draft ? (draft.stages.length ? draft.stages.map(st => ({ title: st.title, blurb: st.blurb, chips: st.phases.map(p => p.title).filter(Boolean), lessons: st.phases.reduce((n, p) => n + p.lessons.length, 0) })) : draft.phases.map(p => ({ title: p.title, blurb: '', chips: p.lessons, lessons: p.lessons.length }))) : []);
  let unitName = $derived(draft?.stages.length ? 'Stage' : 'Phase');
  let lessonCount = $derived(units.reduce((n, u) => n + u.lessons, 0));
  let log = $derived.by(() => {
    const steps: { text: string; state: 'done' | 'now' | 'next' }[] = [{ text: 'Read your goal, answers and blueprint', state: 'done' }];
    if (!draft?.title) steps.push({ text: 'Planning the stages, phases and lessons', state: 'now' });
    else {
      steps.push({ text: `Named it “${draft.title}”`, state: 'done' });
      if (!units.length) steps.push({ text: 'Writing the summary', state: 'now' });
      else {
        steps.push({ text: 'Wrote the summary', state: 'done' });
        units.forEach((u, i) => steps.push(i < units.length - 1 ? { text: `Wrote ${unitName} ${i + 1}: ${u.title}`, state: 'done' } : { text: `Writing ${unitName} ${i + 1}${u.title ? ': ' + u.title : ''}`, state: 'now' }));
      }
    }
    steps.push({ text: 'Check milestones and walk through the lessons from your starting point', state: 'next' }, { text: 'Find videos for every lesson, in the background', state: 'next' });
    return steps;
  });
  function editGoal() { seq++; go(() => { blueprint = undefined; building = false; waiting = false; drafting = false; error = ''; }, 'backward'); void tick().then(() => promptEl?.focus()); }
  async function approve() {
    if (!blueprint || building) return;
    go(() => { building = true; designing = true; error = ''; draft = undefined; }, 'forward');
    try {
      const c = await thinkingOn(w => api<Course>('POST','api/courses',{prompt,folder:kind === 'project' ? folder : '',level,size,effort,thinking:w,answers:answers(),blueprint:$state.snapshot(blueprint),baseline}));
      if (alive) goCourse(c);
    } catch (err) { if (alive) error = message(err); }
    finally { building = false; designing = false; }
  }
</script>

<div class="wrap narrow create screen-enter" class:designing class:placing={!designing && (building || !!blueprint)}>
  {#if !empty}<a class="back-map" id="cancel-new" href="#/" onclick={() => appState.screen = 'library'}><Icon name="back" /> Your library</a>{:else if !building && !blueprint}<div class="create-empty-route"><Icon name="empty-route" /></div>{/if}
  <div class="page-head create-head" hidden={building || !!blueprint}><h1>{empty ? 'What do you want to learn?' : 'New roadmap'}</h1></div>
  <AiSetup why="Add one in Settings to build a roadmap." />
  <form id="new" onsubmit={e => { e.preventDefault(); if (blueprint) void approve(); else void create(); }}>
    <div class="form-col {flow}" hidden={building || !!blueprint}>
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
    {#if blueprint && !building}<div class="pc {flow}">
      {@render goalCard()}
      <section class="pc-form bp" aria-labelledby="bp-title">
        <div class="pc-head"><p class="kicker">Your course blueprint</p><h1 id="bp-title">Check the plan before we build</h1><p class="small">The whole course works toward this. Change anything that does not fit you.</p></div>
        {#if editingPlan}<div class="bp-edit"><BlueprintFields plan={blueprint} /></div>
        {:else}
        <div class="bp-sec" style:--i={0}><p class="kicker">Final challenge</p><p class="bp-challenge">{tidy(blueprint.challenge)}</p></div>
        <div class="bp-sec" style:--i={1}><p class="kicker">You are done when</p><ul class="bp-checks">{#each lines(blueprint.criteria, true) as x, j}<li style:--j={j}><Icon name="check" />{x}</li>{/each}</ul></div>
        <div class="bp-sec" style:--i={2}><p class="kicker">Milestones</p><ol class="bp-steps">{#each lines(blueprint.milestones) as x, i}<li style:--j={i}><span class="n">{i + 1}</span><span>{x}</span></li>{/each}</ol></div>
        {/if}
        {#if blueprint.diagnostic}<details class="bp-try disclosure" style:--i={3}><summary>Optional: try a 5-minute starting task</summary><p>{tidy(blueprint.diagnostic)}</p><textarea id="baseline" aria-label="Your attempt" maxlength="2000" rows="3" bind:value={baseline} placeholder="Try it here or leave this blank."></textarea></details>{/if}
        {#if error}<p role="alert" class="err">{error}</p>{/if}
        <div class="pc-foot"><button class="btn secondary" type="button" onclick={() => editingPlan = !editingPlan}>{editingPlan ? 'Done editing' : 'Edit plan'}</button><span class="grow"></span><button class="btn secondary" type="button" onclick={() => go(() => { blueprint = undefined; editingPlan = false; error = ''; building = true; }, 'backward')}>Back</button><button class="btn">Approve and build course <Icon name="arrow" /></button></div>
      </section>
    </div>{/if}
    <div id="building" hidden={!building} class={designing ? 'designing' : 'placement'}>
      {#if designing}<div class="mk">
        <div class="mk-main">
          <p class="mk-status" role="status"><span class="spin" aria-hidden="true"></span><span>{thought || (draft?.title ? `Writing ${unitName.toLowerCase()} ${Math.max(units.length, 1)}` : 'Planning your roadmap')}</span></p>
          <p class="small mk-time"><span aria-hidden="true"><Elapsed /></span> · this usually takes {sized[4]}{lessonCount ? ` · ${lessonCount} lesson${lessonCount === 1 ? '' : 's'} so far` : ''}</p>
          <p class="kicker mk-kicker">Your roadmap</p>
          {#if draft?.title}<h1 class="mk-title arrive">{draft.title}</h1>{:else}<span class="sk sk-line" style="width:60%;height:22px;margin-top:6px"></span>{/if}
          {#if draft?.summary}<p class="mk-sum">{draft.summary}</p>{:else}<span class="sk sk-line" style="width:85%;margin-top:12px"></span><span class="sk sk-line" style="width:70%;margin-top:8px"></span>{/if}
          <ol class="spine">
            {#each units as u, i}<li class="st arrive" data-state={i < units.length - 1 ? 'done' : 'writing'}><span class="dot" aria-hidden="true">{#if i < units.length - 1}<Icon name="check" />{:else}{i + 1}{/if}</span><div><h3>{u.title || `${unitName} ${i + 1}`}</h3>{#if u.blurb}<p>{u.blurb}</p>{/if}{#if u.chips.length}<div class="chips">{#each u.chips as x}<span>{x}</span>{/each}</div>{/if}</div><span class="meta">{u.lessons} lesson{u.lessons === 1 ? '' : 's'}</span></li>{/each}
            {#if !units.length}{#each [0, 1, 2] as i}<li class="st" data-state="todo"><span class="dot" aria-hidden="true">{i + 1}</span><div><span class="sk sk-line" style="width:45%;height:13px"></span><span class="sk sk-line" style="width:75%;margin-top:8px"></span></div><span></span></li>{/each}{/if}
          </ol>
          <p class="muted small">Keep this page open. Your roadmap opens here when it is ready.</p>
        </div>
        <aside class="mk-log" aria-label="What the AI is doing"><p class="kicker">What the AI is doing</p><ol>{#each log as x}<li class={x.state}><i aria-hidden="true">{#if x.state === 'done'}<Icon name="check" />{:else if x.state === 'now'}<span class="spin"></span>{/if}</i><span>{x.text}</span></li>{/each}</ol></aside>
      </div>
      {:else}<div class="pc {flow}">
        {@render goalCard()}
        <div class="pc-form">
          <div class="pc-head"><p class="kicker">Before we build</p><h1 id="pc-title" tabindex="-1">What can you already do?</h1><p class="small">{qs.length ? `${qs.length} quick questions.` : 'A few quick questions.'} Your answers set where the course starts.</p></div>
          {#if waiting}<div class="q"><span class="n">{drafting ? '✓' : 1}</span><div class="pc-wait"><Waiting skeleton="question" inline title={drafting ? 'Drafting your course blueprint' : 'Preparing your questions'} hint="this usually takes 10 to 20 seconds" {thought} /></div></div>
          {:else if failed}<div class="q"><span class="n">!</span><div class="err pc-err" role="alert"><p>{failed === 'blueprint' ? 'The blueprint could not be drafted.' : 'The questions could not be prepared.'} {error}</p><button class="btn secondary sm" type="button" onclick={() => failed === 'blueprint' ? toBlueprint() : ask()}>Try again</button></div></div>
          {:else}{#each qs as x, i}<fieldset class="q" class:answered={!!picked[i]} style:--i={i}><legend><span class="n">{i + 1}</span><span class="qt">{x.q}</span></legend><div class="opts">{#each x.options as o, j}<label class="opt" style:--j={j}><input type="radio" name={`pq-${i}`} value={o} bind:group={picked[i]} />{o}</label>{/each}</div></fieldset>{/each}
          {#if qs.length}<label class="pc-extra" style:--i={qs.length}><span>Anything else the course should know? <span class="small">Optional</span></span><input id="pa-own" maxlength="300" placeholder="For example: what you already have or have tried" bind:value={extra} onkeydown={e => { if (e.key === 'Enter') e.preventDefault(); }} /></label>{/if}{/if}
          <div class="pc-foot"><span class="pc-meter"><Progress n={answered} total={qs.length || 4} wide={false} unit="questions answered" /><span class="small">{answered} of {qs.length || '…'} answered</span></span><button class="btn secondary" type="button" id="pa-back" disabled={waiting} onclick={editGoal}>Back</button><button class="btn" type="button" id="unplaced" disabled={waiting} onclick={() => void toBlueprint()}>{qs.length && answered === qs.length ? 'Continue to blueprint' : 'Skip to blueprint'} <Icon name="arrow" /></button></div>
        </div>
      </div>{/if}
    </div>
  </form>
</div>
{#snippet goalCard()}<aside class="pc-goal"><p class="kicker">Your goal</p><p class="pc-quote">{prompt.trim() || (kind === 'project' ? 'Understand this project so I can change it.' : '')}</p>{#if kind === 'project' && folder}<p class="small pc-folder">{folder}</p>{/if}<dl><dt>Starting</dt><dd>{levelName()}</dd><dt>Length</dt><dd>{sized[2]}</dd></dl><button class="link" type="button" id="edit-goal" onclick={editGoal}>Edit goal</button></aside>{/snippet}
<style>
  /* The whole placement check fits one screen at 1366x768. */
  .wrap.narrow.create.placing { max-width: 1100px; margin-top: var(--s5); }
  /* Placement check: the goal stays on the left; each question number lines up with its options. */
  .pc { display: grid; grid-template-columns: 280px minmax(0, 1fr); gap: 36px; margin-top: var(--s2); }
  /* The goal card runs the full height of the questions, as in the sample. */
  .pc-goal { display: grid; gap: var(--s4); align-content: start; padding: var(--s6); border-radius: var(--r-lg); background: var(--bg-subtle); }
  .pc-goal .kicker { margin: 0; }
  .pc-quote { margin: 0; font: 600 var(--t-xl)/1.45 var(--display); }
  .pc-folder { margin: 0; word-break: break-all; }
  .pc-goal dl { display: grid; grid-template-columns: auto 1fr; gap: var(--s2) var(--s5); margin: 0; font-size: var(--t-sm); }
  .pc-goal dt { color: var(--muted); }
  .pc-goal dd { margin: 0; font-weight: 550; }
  .pc-goal .link { justify-self: start; }
  .pc-head h1 { margin: 2px 0 var(--s1); font-size: 24px; }
  .pc-head .kicker { margin: 0; }
  .q { display: grid; grid-template-columns: 22px minmax(0, 1fr); column-gap: var(--s5); row-gap: var(--s3); padding: var(--s5) 0; border-bottom: 1px solid var(--line); }
  .q legend { display: contents; }
  .n { grid-column: 1; width: 22px; height: 22px; border-radius: 50%; background: var(--ink); color: var(--on-accent); font-size: var(--t-xs); font-weight: 600; display: grid; place-items: center; }
  .q.answered .n { background: var(--accent); }
  .qt { grid-column: 2; align-self: center; font-weight: 600; font-size: var(--t-lg); line-height: 1.4; }
  .opts, .pc-wait, .pc-err { grid-column: 2; }
  .opts { display: flex; flex-wrap: wrap; gap: var(--s3); }
  .opt { margin: 0; display: inline-flex; align-items: center; gap: var(--s3); min-height: 32px; padding: var(--s2) var(--s5) var(--s2) var(--s4); border: 1px solid var(--line-strong); border-radius: var(--r-btn-sm); background: var(--surface); color: var(--ink-2); font-size: var(--t-md); font-weight: 400; cursor: pointer; transition: border-color var(--motion-fast), background-color var(--motion-fast); }
  .opt input { flex: none; width: 14px; height: 14px; margin: 0; accent-color: var(--accent); }
  .opt:hover { border-color: var(--accent); }
  .opt:has(:checked) { border-color: var(--accent); background: var(--accent-soft); color: var(--accent-ink); font-weight: 550; }
  .opt:has(:focus-visible) { outline: 2px solid var(--accent); outline-offset: 2px; }
  .pc-extra { display: grid; grid-template-columns: auto minmax(0, 1fr); align-items: center; gap: var(--s5); margin: 0; padding: var(--s5) 0; border-bottom: 1px solid var(--line); font-weight: 600; font-size: var(--t-md); }
  .pc-extra .small { font-weight: 400; }
  .pc-err { display: grid; gap: var(--s3); justify-items: start; }
  .pc-err p { margin: 0; }
  .pc-foot { display: flex; flex-wrap: wrap; align-items: center; gap: var(--s5); padding-top: var(--s5); }
  .pc-meter { display: flex; align-items: center; gap: var(--s4); flex: 1; }
  .pc-meter :global(.bar) { width: 120px; }
  .pc-foot .grow { flex: 1; }
  /* Blueprint review: same frame as the questions, read first and edit on request. */
  .bp-sec { padding: var(--s5) 0; border-bottom: 1px solid var(--line); }
  .bp-sec .kicker { margin: 0 0 var(--s3); }
  .bp-challenge { margin: 0; font: 600 var(--t-xl)/1.45 var(--display); }
  .bp-checks, .bp-steps { list-style: none; margin: 0; padding: 0; display: grid; gap: var(--s3); }
  .bp-checks li { display: flex; gap: var(--s3); align-items: baseline; font-size: var(--t-md); }
  .bp-checks :global(svg) { flex: none; width: 14px; height: 14px; color: var(--success); transform: translateY(2px); }
  .bp-steps li { display: grid; grid-template-columns: 22px minmax(0, 1fr); gap: var(--s5); align-items: start; font-size: var(--t-md); line-height: 1.45; }
  .bp-steps .n { margin-top: -1px; }
  .bp-steps li:last-child .n { background: var(--accent); }
  .bp-edit { padding-top: var(--s3); }
  .bp-try { padding: var(--s5) 0; border-bottom: 1px solid var(--line); }
  .bp-try { margin: 0; }
  .bp-try > summary { cursor: pointer; }
  .bp-steps { position: relative; }
  /* The milestones hang on one line, like the course route. */
  .bp-steps::before { content: ""; position: absolute; left: 10.5px; top: 11px; bottom: 11px; width: 1px; background: var(--line-strong); transform-origin: top; }
  .bp-steps .n { position: relative; }
  @media (prefers-reduced-motion: no-preference) {
    /* Questions and blueprint parts arrive one after another, then their answers and lines. */
    :is(.q, .pc-extra, .bp-sec, .bp-try) { animation: arrive var(--motion-slow) var(--motion-ease) calc(var(--i, 0) * 70ms) backwards; }
    .opt { animation: arrive var(--motion-slow) var(--motion-ease) calc(var(--i, 0) * 70ms + 120ms + var(--j, 0) * 35ms) backwards; }
    .opt input:checked { animation: pick-dot 300ms var(--motion-mark-ease); }
    .q.answered .n { animation: pick 300ms var(--motion-mark-ease); transition: background-color var(--motion-enter); }
    .bp-challenge { animation: word-in 500ms var(--motion-ease) 60ms backwards; }
    .bp-checks li { animation: arrive var(--motion-slow) var(--motion-ease) calc(180ms + var(--j, 0) * 60ms) backwards; }
    .bp-checks :global(svg) { animation: stop-appear var(--motion-slow) var(--motion-mark-ease) calc(260ms + var(--j, 0) * 60ms) backwards; }
    .bp-steps::before { animation: route-grow 700ms var(--motion-ease) 300ms backwards; }
    .bp-steps .n { animation: stop-appear var(--motion-slow) var(--motion-mark-ease) calc(320ms + var(--j, 0) * 110ms) backwards; }
    .bp-steps li > span:last-child { animation: arrive var(--motion-slow) var(--motion-ease) calc(360ms + var(--j, 0) * 110ms) backwards; }
  }
  @keyframes pick { 40% { transform: scale(1.12); } }
  @keyframes pick-dot { 40% { transform: scale(1.4); } }
  .bp-try p { margin: var(--s3) 0; }

  /* Roadmap making: the screen fills in as the AI writes the title, summary and each stage. */
  .mk { display: grid; grid-template-columns: minmax(0, 1fr) 300px; gap: 36px; align-items: start; }
  .mk-status { display: inline-flex; align-items: center; gap: var(--s3); margin: 0; min-height: 26px; padding: 2px var(--s4) 2px var(--s3); border: 1px solid var(--line); border-radius: 999px; font-size: var(--t-sm); font-weight: 550; color: var(--ink-2); max-width: 100%; }
  .mk-status span:last-child { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .mk-time { margin: var(--s3) 0 0; }
  .mk-kicker { margin: var(--s6) 0 0; }
  .mk-title { margin: 2px 0 0; font-size: 26px; }
  .mk-sum { margin: var(--s2) 0 0; max-width: 72ch; color: var(--ink-2); font-size: var(--t-lg); }
  .sk-line { height: 10px; }
  .spine { position: relative; list-style: none; padding: 0; margin: 22px 0 var(--s6); }
  .spine::before { content: ""; position: absolute; left: 11px; top: 14px; bottom: 30px; border-left: 2px solid var(--line); }
  .st { position: relative; display: grid; grid-template-columns: 24px minmax(0, 1fr) auto; gap: 14px; padding: 4px 0 20px; }
  .st .dot { position: relative; z-index: 1; width: 24px; height: 24px; border-radius: 50%; display: grid; place-items: center; font-size: var(--t-xs); font-weight: 600; background: var(--bg); border: 2px solid var(--line-strong); color: var(--muted); }
  .st .dot :global(.icon) { width: 12px; height: 12px; }
  .st[data-state="done"] .dot { background: var(--ink); border-color: var(--ink); color: var(--on-accent); }
  .st[data-state="writing"] .dot { border-color: var(--accent); color: var(--accent-ink); }
  .st h3 { margin: 2px 0 0; font: 650 var(--t-xl)/1.3 var(--display); letter-spacing: -.01em; }
  .st p { margin: 3px 0 0; font-size: var(--t-sm); color: var(--muted); max-width: 66ch; }
  .st .meta { font-size: var(--t-sm); color: var(--muted); white-space: nowrap; padding-top: 3px; }
  .chips { display: flex; flex-wrap: wrap; gap: var(--s2); margin-top: var(--s3); }
  .chips span { font-size: var(--t-sm); padding: 2px var(--s3); border-radius: var(--r); background: var(--surface-2); color: var(--ink-2); }
  .mk-log { padding: 14px var(--s6); border: 1px solid var(--line); border-radius: var(--r-lg); position: sticky; top: 0; }
  .mk-log .kicker { margin: 0; }
  .mk-log ol { list-style: none; padding: 0; margin: var(--s4) 0 0; display: grid; gap: 9px; }
  .mk-log li { display: grid; grid-template-columns: 16px minmax(0, 1fr); gap: 9px; font-size: var(--t-sm); color: var(--ink-2); }
  .mk-log i { width: 16px; height: 16px; margin-top: 1px; border-radius: 50%; display: grid; place-items: center; color: var(--on-accent); }
  .mk-log i :global(.icon) { width: 10px; height: 10px; }
  .mk-log .done i { background: var(--success); }
  .mk-log .now { color: var(--ink); font-weight: 550; }
  .mk-log .next { color: var(--muted); }
  .mk-log .next i { border: 1.5px solid var(--line-strong); }
  .spin { display: inline-block; flex: none; width: 14px; height: 14px; border-radius: 50%; border: 2px solid var(--line); border-top-color: var(--accent); animation: spin 1.1s linear infinite; }
  .mk-log .spin { width: 16px; height: 16px; }
  .arrive { animation: arrive var(--motion-slow) var(--motion-ease) both; }
  @keyframes spin { to { rotate: 1turn; } }
  @media (prefers-reduced-motion: reduce) { .spin { animation-duration: 6s; } .arrive { animation: none; } }
  /* Forced colors paint every border one color, which hides the turning arc. */
  @media (forced-colors: active) { .spin { forced-color-adjust: none; border-color: GrayText; border-top-color: CanvasText; } }
  @media (max-width: 900px) {
    .pc, .mk { grid-template-columns: minmax(0, 1fr); } .pc-goal, .mk-log { position: static; }
    /* Narrow: the goal shrinks to a one-line strip so the questions stay on screen. */
    .pc { gap: var(--s6); }
    .pc-goal { grid-template-columns: minmax(0, 1fr) auto; align-items: center; gap: var(--s1) var(--s5); padding: var(--s4) var(--s5); }
    .pc-goal > :not(.link) { grid-column: 1; }
    .pc-goal .kicker { display: none; }
    .pc-quote { font-size: var(--t-md); }
    .pc-goal dl { display: flex; flex-wrap: wrap; gap: 0 var(--s2); }
    .pc-goal dd + dt { margin-left: var(--s4); }
    .pc-goal .link { grid-column: 2; grid-row: 1 / span 3; }
  }
</style>
