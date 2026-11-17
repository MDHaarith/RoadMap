<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { api } from '../lib/api';
  import { appState, save, toast, message, adaptNow, expandPhase, flushSave, saveFirst } from '../lib/state.svelte';
  import { lessons, nextLesson, status, locked, stageOpen, stageLessons } from '../lib/progress';
  import { enc, href } from '../lib/format';
  import type { Course, PlanPreview } from '../lib/types';
  import PlanOptions from '../components/PlanOptions.svelte';
  import VideoPrefs from '../components/VideoPrefs.svelte';
  import LessonActions from '../components/LessonActions.svelte';
  import Icon from '../components/Icon.svelte';
  import Progress from '../components/Progress.svelte';
  import ExamRow from '../components/ExamRow.svelte';
  import Waiting from '../components/Waiting.svelte';
  import BusyLabel from '../components/BusyLabel.svelte';
  import LoadError from '../components/LoadError.svelte';
  import { confirmAsk } from '../components/Confirm.svelte';
  let { c }: { c: Course } = $props();
  let ls = $derived(lessons(c)), n = $derived(ls.filter(l => c.state.done[l.id]).length), next = $derived(nextLesson(c));
  let complete = $derived(ls.length > 0 && n === ls.length), pending = $derived(ls.filter(l => !l.videos && !l.stub).length);
  let found = $state<Record<string, boolean> | false | undefined>(undefined);
  let reviseOpen = $state(false), ask = $state(''), menuOpen = $state(false), menuEl = $state<HTMLDetailsElement>();
  let bulk = $state<{ stop: boolean; text: string; error: string } | null>(null);
  let preview = $state<PlanPreview>(), revising = $state(false), prefsOpen = $state(false), originalAsk = $state(''), reviseError = $state('');
  let alive = true;
  onMount(() => {
    let timer: ReturnType<typeof setTimeout>;
    async function watchVideoSearch(attempt = 0) {
      if (!alive || attempt >= 12 || !lessons(c).some(l => !l.videos && !l.stub)) return;
      timer = setTimeout(async () => {
        try {
          const fresh = await api<Course>('GET', 'api/courses/' + enc(c.id));
          if (!alive) return;
          const byId = new Map(lessons(fresh).map(l => [l.id, l]));
          lessons(c).forEach(l => { if (byId.get(l.id)?.videos) l.videos = byId.get(l.id)!.videos; });
        } catch { /* Video arrival is optional; the lesson can search again. */ }
        void watchVideoSearch(attempt + 1);
      }, 3500);
    }
    void watchVideoSearch();
    if (c.needs?.some(x => x.command)) api<{ found: Record<string, boolean> }>('POST', 'api/courses/' + enc(c.id) + '/setup').then(r => { if (alive) found = r.found; }).catch(() => { found = false; });
    return () => { alive = false; clearTimeout(timer); if (bulk) bulk.stop = true; };
  });
  async function writeEveryLesson() {
    const phases = c.phases.map((p, i) => ({ i, id: p.lessons.find(l => l.stub)?.id })).filter(p => p.id);
    if (!(await confirmAsk({ message: `Write ${phases.length} phases now? This uses many AI tokens.`, action: 'Write phases' }))) return;
    bulk = { stop: false, text: '', error: '' }; const job = bulk;
    try {
      for (const [i, p] of phases.entries()) {
        if (!alive || job.stop) break;
        job.text = `Writing phase ${i + 1} of ${phases.length}`;
        try { await expandPhase(c, p.id!); } catch (error) { job.error = `Phase ${p.i + 1}: ${message(error)}`; break; }
      }
    } finally { job.text = ''; }
  }
  async function revise() {
    if (!ask.trim()) return;
    revising = true; preview = undefined; reviseError = ''; originalAsk = ask.trim();
    try { await flushSave(); preview = await api<PlanPreview>('POST', 'api/courses/' + enc(c.id) + '/revise', { ask: originalAsk }); }
    catch (error) { reviseError = message(error); } finally { revising = false; }
  }
  async function exportCourse(kind: string) {
    try {
      await saveFirst();
      const r = await api<{ path: string }>('POST', 'api/courses/' + enc(c.id) + (kind === 'summary' ? '/summary' : '/export'), kind === 'template' ? { template: true } : undefined);
      toast('Saved to ' + r.path);
    } catch (error) { toast(message(error)); }
  }
  async function work() {
    try { c.work = (await api<{ work?: string }>('PUT', 'api/courses/' + enc(c.id) + '/work', c.work ? { clear: true } : {})).work; }
    catch (error) { toast(message(error)); }
  }
  let showIndex = $derived((c.stages?.length ?? c.phases.length) > 1);
  let nextPhase = $derived(c.phases.findIndex(p => p.lessons.includes(next)));
  let nextStage = $derived(c.phases[nextPhase]?.stage);
  const doneIn = (ls: Course['phases'][number]['lessons']) => ls.filter(l => c.state.done[l.id]).length;
  function rowState(l: Course['phases'][number]['lessons'][number]) {
    if (c.state.done[l.id]) return 'done';
    if (next === l && !complete) return 'next';
    if (locked(c, l)) return 'locked';
    if (status(c, l) === 'part') return 'part';
    return l.stub ? 'outline' : '';
  }
  const META: Record<string, string> = { done: 'Complete', next: 'Up next', locked: 'Locked', part: 'In progress', outline: 'Not written yet', '': '' };
  async function act(fn: () => unknown) { menuOpen = false; await fn(); }
  let deleting = $state(false);
  async function deleteRoadmap() {
    if (!(await confirmAsk({ message: `Delete "${c.title}" and all its progress? This can't be undone.`, action: 'Delete', danger: true }))) return;
    deleting = true;
    try { await api('DELETE', 'api/courses/' + enc(c.id)); appState.course = null; location.hash = '#/'; }
    catch (e) { toast(message(e)); } finally { deleting = false; }
  }
  // Escape closes the innermost open thing (menu, plan preview, Change my roadmap, video preferences) and puts focus back where it was opened.
  function menuKeys(e: KeyboardEvent) {
    if (e.key !== 'Escape' || e.defaultPrevented) return;
    const more = () => menuEl?.querySelector('summary')?.focus();
    if (menuOpen) { menuOpen = false; more(); }
    else if (preview && reviseOpen) { preview = undefined; document.getElementById('ask')?.focus(); }
    else if (reviseOpen) { reviseOpen = false; reviseError = ''; more(); }
    else if (prefsOpen) { prefsOpen = false; more(); }
    else return;
    e.preventDefault();
  }
  function toggleStage(s: number) { (c.state.stageOpen ||= {})[s] = !stageOpen(c, s); save(c); }
  async function jump(i: number) {
    const stage = c.phases[i].stage;
    if (c.stages && stage !== undefined && !stageOpen(c, stage)) { (c.state.stageOpen ||= {})[stage] = true; save(c); await tick(); }
    // The first phase of a stage starts under its stage heading, so show the heading too.
    const el = document.getElementById('phase-' + (i + 1));
    (c.stages && stage !== undefined && c.phases[i - 1]?.stage !== stage ? el?.previousElementSibling : el)?.scrollIntoView({ block: 'start' });
  }
</script>

<svelte:window onkeydown={menuKeys} onclick={e => { if (menuOpen && !menuEl?.contains(e.target as Node)) menuOpen = false; }} />
<div class="overview">

  <aside class="overview-side">
  <div class="overview-head"><a class="back-map" href="#/"><Icon name="back" /> Your library</a>
    <div class="head-row page-head"><h1>{c.title}</h1>
      <details class="menu" bind:this={menuEl} bind:open={menuOpen}><summary class="btn secondary sm" aria-label="More roadmap actions"><Icon name="more" /> More</summary><div class="menu-list popover-enter">
        <button aria-expanded={prefsOpen} aria-controls="video-prefs" onclick={() => act(async () => { prefsOpen = !prefsOpen; await tick(); document.querySelector<HTMLElement>(prefsOpen ? '#video-prefs h2' : '.overview-head summary')?.focus(); })}>Video preferences for this roadmap</button>
        <button id="adapt" disabled={appState.adapting} onclick={() => act(() => adaptNow(true))}>{appState.adapting ? 'Reading your results…' : 'Adjust to my progress'}<small>Add practice or drop lessons, based on your quiz and exercise results</small></button>
        <button id="revise-open" aria-expanded={reviseOpen} aria-controls="revise" onclick={() => act(async () => { reviseOpen = !reviseOpen; await tick(); if (reviseOpen) document.getElementById('ask')?.focus(); })}>Change my roadmap<small>Take the lessons you have not started somewhere else</small></button>
        {#if ls.some(l => l.stub) && !bulk?.text}<button id="write-all" onclick={writeEveryLesson}>Write every lesson now<small>Writes the remaining phases now. Uses many AI tokens.</small></button>{/if}
        {#if !c.folder}<button id="work" onclick={() => act(work)}>{c.work ? 'Disconnect folder: ' + c.work.split(/[\\/]/).pop() : 'Connect my project folder'}<small>The tutor can then read the text files in that folder. Parts of them are sent to your AI service.</small></button>{/if}
        <hr />
        <button id="export" onclick={() => act(() => exportCourse('export'))}>Export<small>Save this roadmap with your progress and notes to a file</small></button>
        <button id="template" onclick={() => act(() => exportCourse('template'))}>Share as template<small>Save the roadmap alone, without your progress or notes</small></button>
        <button id="summary" onclick={() => act(() => exportCourse('summary'))}>Save summary<small>A Markdown summary of what you made, your notes and quiz scores</small></button>
        <hr />
        <button id="roadmap-mistakes" onclick={() => act(() => { location.hash = '#/mistakes/' + enc(c.id); })}>Mistakes notebook<small>The questions you missed in this roadmap</small></button>
        <button id="delete-roadmap" class="danger" disabled={deleting} onclick={() => act(deleteRoadmap)}>Delete roadmap<small>Removes it with all its progress and notes</small></button>
      </div></details>
    </div>
    <p class="goal">{c.summary}</p>
    <div class="progress-line"><Progress {n} total={ls.length} wide={false} /><span>{n} of {ls.length} lessons complete · {ls.length ? Math.round(n / ls.length * 100) : 0}%</span></div>
    {#if complete}<div class="celebrate"><strong>Roadmap complete.</strong><p>Every lesson is complete.</p><a class="btn" class:secondary={reviseOpen} href={href(c.id, ls[0].id)}>Open the first lesson <Icon name="arrow" /></a></div>
    {:else if next}<section class="next-card" aria-labelledby="next-title"><div><div class="kicker">{n ? 'Up next' : 'Start here'} · Phase {nextPhase + 1}</div><h2 id="next-title">{next.title}</h2><p>Lesson {ls.indexOf(next) + 1} of {ls.length}{next.stub ? ' · The AI writes this phase when you open it' : ''}</p></div><a class="btn big" class:secondary={reviseOpen} href={href(c.id, next.id)}>{n ? 'Continue learning' : 'Start the first lesson'} <Icon name="arrow" /></a></section>{/if}
  </div>
  {#if showIndex}<div><p class="side-title">{c.stages ? 'Stages' : 'Phases'}</p><nav class="phase-index" aria-label={`Jump to ${c.stages ? 'stage' : 'phase'}`}>{#each c.phases as p, i}{#if !c.stages || (p.stage !== undefined && c.phases[i - 1]?.stage !== p.stage)}{@const group = c.stages && p.stage !== undefined ? stageLessons(c, p.stage) : p.lessons}{@const isCurrent = !complete && (c.stages && p.stage !== undefined ? p.stage === nextStage : i === nextPhase)}<a href={`#phase-${i + 1}`} class:current={isCurrent} class:done={doneIn(group) === group.length} aria-current={isCurrent ? 'location' : undefined} onclick={e => { e.preventDefault(); void jump(i); }}><span class="dot" aria-hidden="true"></span><span>{c.stages && p.stage !== undefined ? c.stages[p.stage].title : `${i + 1} · ${p.title}`}</span><span class="count">{doneIn(group)}/{group.length}</span></a>{/if}{/each}</nav></div>{/if}
  </aside>
  <div class="overview-main">
  {#if appState.adapting}<Waiting skeleton="plan" inline title="Reading your results and updating the roadmap. This can take a minute." />{/if}
  {#if bulk?.text || bulk?.error}<div class="row bulk-row">{#if bulk?.text}{#key bulk.text}<Waiting skeleton="plan" inline title={`${bulk.text}. Each phase takes about a minute.`} />{/key}<button class="btn secondary sm" id="stop-writing" disabled={bulk.stop} onclick={() => { if (bulk) bulk.stop = true; }}><BusyLabel busy={bulk.stop} idle="Stop" label="Stopping after this phase…" /></button>{/if}{#if bulk?.error}<p class="err">{bulk.error}</p>{/if}</div>{/if}
  {#if prefsOpen}<section class="note" id="video-prefs" aria-labelledby="video-prefs-title"><h2 id="video-prefs-title" tabindex="-1">Video preferences for this roadmap</h2><VideoPrefs initial={c.prefs} course={c} /><button type="button" class="link revise-cancel" onclick={() => { prefsOpen = false; menuEl?.querySelector('summary')?.focus(); }}>Close</button></section>{/if}
  <form id="revise" class="note" hidden={!reviseOpen} onsubmit={e => { e.preventDefault(); void revise(); }}><strong>Change my roadmap</strong><p>Say what should change in the lessons you have not started. Your progress stays.</p><div class="own"><input id="ask" required maxlength="600" aria-label="What should change in the lessons you have not started?" placeholder="For example: show me how this is used in industry, with real applications" bind:value={ask} /><button class="btn" disabled={revising}><BusyLabel busy={revising} idle="Preview changes" label="Preparing options…" /> <Icon name="arrow" /></button></div>{#if revising}<Waiting skeleton="plans" title="Preparing roadmap options" hint="this can take about a minute" />{/if}{#if reviseError}<LoadError what="The options could not be prepared." error={reviseError} retry={() => void revise()} />{/if}{#if preview}<PlanOptions {preview} ask={originalAsk} cancel={() => preview = undefined} applied={() => { preview = undefined; reviseOpen = false; ask = ''; }} />{/if}<button type="button" class="link revise-cancel" hidden={!!preview} onclick={() => { reviseOpen = false; reviseError = ''; menuEl?.querySelector('summary')?.focus(); }}>Cancel</button></form>
  <div id="video-status">{#if pending}<p class="video-status"><Icon name="Watch" /> Videos may still be arriving for {pending} of {ls.length} lessons. Open any lesson to search again if needed.</p>{/if}</div>
  {#if c.changes?.length}{@const x = c.changes[c.changes.length - 1]}<div class="note change"><strong>Roadmap updated</strong><p>{x.note}</p>{#if x.added.length}<p class="small">Added: {x.added.join(', ')}</p>{/if}{#if x.dropped.length}<p class="small">Removed: {x.dropped.join(', ')}</p>{/if}{#if x.changed?.length}<p class="small">Rewritten: {x.changed.join(', ')}</p>{/if}</div>{/if}
  {#if c.needs?.length}<details class="needs" open={!n}><summary>Before you start: {c.needs.length} thing{c.needs.length === 1 ? '' : 's'} to have ready</summary><ul id="needs">{#each c.needs as x, i}<li><label><input type="checkbox" checked={!!c.state.have?.[i]} onchange={e => { if (e.currentTarget.checked) (c.state.have ||= {})[i] = true; else delete c.state.have?.[i]; save(c); }} /><span><b>{x.name}</b>{#if x.why}<small>{x.why}</small>{/if}</span></label>{#if x.kind === 'tool' && x.command && found !== false}{#if !found}<span class="sk tag-line" role="status"><span class="sr">Checking</span></span>{:else}<span class="tag" class:ok={found[x.command]}>{found[x.command] ? 'Installed' : 'Not found on this computer'}</span>{/if}{/if}</li>{/each}</ul></details>{/if}
  <section class="route-board" aria-labelledby="lessons-heading"><div class="route-heading"><h2 id="lessons-heading">Lessons</h2><span class="small">{c.phases.length} phases</span></div>
    {#each c.phases as p, pi}
      {#if c.stages && p.stage !== undefined && c.phases[pi - 1]?.stage !== p.stage}{@const st = c.stages[p.stage]}{@const sl = stageLessons(c, p.stage)}<button type="button" class="stage-head" aria-expanded={stageOpen(c, p.stage)} onclick={() => toggleStage(p.stage!)}><span><span class="stage-label">Stage {p.stage + 1}</span><strong>{st.title}</strong>{#if st.blurb}<span class="blurb">{st.blurb}</span>{/if}</span><span class="stage-count">{doneIn(sl)} / {sl.length} lessons <Icon name="chevron" /></span></button>{/if}
      <section class="route-phase" hidden={!!c.stages && p.stage !== undefined && !stageOpen(c, p.stage)} id={`phase-${pi + 1}`}><div class="phase-flag"><h3><span>Phase {pi + 1}</span>{p.title}</h3><b class="small">{doneIn(p.lessons)} / {p.lessons.length}{p.lessons.every(l => c.state.done[l.id]) ? ' · Complete' : ''}</b></div>{#if p.blurb}<p class="phase-blurb">{p.blurb}</p>{/if}<div class="lesson-list route stagger">{#each p.lessons as l, li}{@const st = rowState(l)}<div class={`lesson-action-row route-item ${st}`}><a class={`lesson-row ${st}`} href={href(c.id, l.id)} aria-current={st === 'next' ? 'step' : undefined}><span class="state route-stop" aria-hidden="true">{#if st === 'done'}<Icon name="check" />{:else if st === 'locked'}<Icon name="lock" />{/if}</span><span class="num">{pi + 1}.{li + 1}</span><span class="stop-title">{l.title}{#if l.added} <span class="tag accent">Added for you</span>{/if}</span><span class="stop-meta">{META[st]}</span></a><LessonActions {c} {l} compact /></div>{/each}</div></section>
      {#if c.stages && p.stage !== undefined && c.phases[pi + 1]?.stage !== p.stage && stageOpen(c, p.stage)}<ExamRow {c} stage={p.stage} />{/if}
    {/each}
  </section>
  </div>
</div>
