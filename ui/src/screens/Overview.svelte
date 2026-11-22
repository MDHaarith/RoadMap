<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { api } from '../lib/api';
  import { appState, save, toast, message, adaptNow, expandPhase, flushSave, saveFirst } from '../lib/state.svelte';
  import { lessons, nextLesson, status, locked, stageLessons, whereAmI } from '../lib/progress';
  import { enc, href } from '../lib/format';
  import type { Course, Blueprint, PlanPreview, OfflineStatus, Activity, Mistake } from '../lib/types';
  import Ring from '../components/Ring.svelte';
  import BlueprintFields from '../components/BlueprintFields.svelte';
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
  let where = $derived(whereAmI(c));
  let complete = $derived(ls.length > 0 && n === ls.length), pending = $derived(ls.filter(l => !l.videos && !l.stub).length);
  let found = $state<Record<string, boolean> | false | undefined>(undefined);
  let reviseOpen = $state(false), ask = $state(''), menuOpen = $state(false), menuEl = $state<HTMLDetailsElement>();
  let bulk = $state<{ stop: boolean; text: string; error: string } | null>(null);
  let preview = $state<PlanPreview>(), revising = $state(false), prefsOpen = $state(false), originalAsk = $state(''), reviseError = $state('');
  let draft = $state<Blueprint>(), planning = $state(false), planError = $state('');
  async function retrofit(approve = false) {
    planning = true; planError = '';
    const path = 'api/courses/' + enc(c.id) + '/blueprint';
    try {
      if (approve && draft) {
        await saveFirst();
        const result = await api<Course>('POST', path, {blueprint: $state.snapshot(draft)});
        c.blueprint = result.blueprint;
        c.phases = result.phases;
        draft = undefined;
      } else { draft = await api<Blueprint>('POST', path); }
    } catch(e) { planError = message(e); } finally { planning = false; }
  }
  let alive = true;
  // "Before you start" tools: the one being installed, and why the last install failed.
  let installing = $state(''), installError = $state('');
  async function install(cmd: string) {
    installing = cmd; installError = '';
    try {
      const r = await api<{ ok: boolean; output: string }>('POST', 'api/courses/' + enc(c.id) + '/setup', { install: cmd });
      if (r.ok && found) found[cmd] = true; else installError = r.output;
    } catch (error) { installError = message(error); } finally { installing = ''; }
  }
  let offline = $state<OfflineStatus>();
  const loadOffline = () => api<OfflineStatus>('GET', 'api/courses/' + enc(c.id) + '/offline').then(r => { if (alive) offline = r; }, () => {});
  async function toggleOffline() {
    const on = !c.offline;
    if (!on && !(await confirmAsk({ message: 'Delete the videos saved for offline use? Lessons, notes and practice stay.', action: 'Delete videos' }))) return;
    try { offline = await api<OfflineStatus>('POST', 'api/courses/' + enc(c.id) + '/offline', { on }); c.offline = on; if (!on) c.saved_videos = []; }
    catch (error) { toast(message(error)); }
  }
  const mb = (b: number) => b >= 1e9 ? (b / 1e9).toFixed(1) + ' GB' : Math.round(b / 1e6) + ' MB';
  onMount(() => {
    void loadOffline();
    const offlineTimer = setInterval(() => { if (c.offline && document.visibilityState === 'visible') void loadOffline(); }, 3000);
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
    if (!activity) api<Activity>('GET', 'api/activity').then(a => { if (alive) activity = a; }).catch(() => {});
    api<{ items: Mistake[] }>('GET', 'api/mistakes').then(r => { if (alive) mistakes = r.items.filter(m => m.course === c.id && !m.fixed); }).catch(() => { if (alive) mistakes = []; });
    if (c.needs?.some(x => x.command)) api<{ found: Record<string, boolean> }>('POST', 'api/courses/' + enc(c.id) + '/setup').then(r => { if (alive) found = r.found; }).catch(() => { found = false; });
    return () => { alive = false; clearTimeout(timer); clearInterval(offlineTimer); if (bulk) bulk.stop = true; };
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
  let nextPhase = $derived(c.phases.findIndex(p => p.lessons.includes(next)));
  // Phases without a stage of their own belong to the stage before them.
  let stageOf = $derived(c.phases.reduce<number[]>((a, p, i) => { a.push(p.stage ?? a[i - 1] ?? 0); return a; }, []));
  let nextStage = $derived(stageOf[nextPhase] ?? 0);
  // The stage on screen: the learner's pick, else the stage of the next lesson.
  let picked = $state<number>();
  let cur = $derived(c.stages?.length ? Math.min(picked ?? nextStage, c.stages.length - 1) : 0);
  let tabbed = $derived((c.stages?.length ?? 0) > 1);
  let tabsEl = $state<HTMLElement>();
  // Stage tabs: arrow keys, Home and End move and select at once (selection follows focus).
  function stageKeys(e: KeyboardEvent) {
    const n = c.stages!.length, k = ({ ArrowRight: cur + 1, ArrowLeft: cur - 1 + n, Home: 0, End: n - 1 } as Record<string, number>)[e.key];
    if (k === undefined) return;
    e.preventDefault(); picked = k % n;
    void tick().then(() => tabsEl?.querySelector<HTMLElement>('[aria-selected="true"]')?.focus());
  }
  // Keep the selected tab in view when the row scrolls; `container` limits the scroll to the row.
  $effect(() => { void cur; tabsEl?.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: 'nearest', inline: 'nearest', container: 'nearest' } as ScrollIntoViewOptions); });
  let shown = $derived(c.phases.map((p, i) => ({ p, i })).filter(x => !c.stages?.length || stageOf[x.i] === cur));
  let stageLs = $derived(shown.flatMap(x => x.p.lessons));
  let stageDone = $derived(stageLs.filter(l => c.state.done[l.id]).length);
  let here = $derived(stageLs.includes(next) && !c.state.done[next.id] ? next : stageLs.find(l => !c.state.done[l.id]));
  let herePhase = $derived(here ? c.phases.findIndex(p => p.lessons.includes(here!)) : -1);
  let milestone = $derived(stageLs.find(l => l.checkpoint && !c.state.portfolio?.[l.id]?.attempts.some(a => a.mode === 'independent')) ?? stageLs.find(l => l.checkpoint));
  let activity = $state<Activity | null>(appState.activity), mistakes = $state<Mistake[]>();
  let stageMistakes = $derived((mistakes ?? []).filter(m => stageLs.some(l => l.id === m.lesson)));
  const stageDoneAll = (i: number) => { const sl = stageLessons(c, i); return sl.length > 0 && sl.every(l => c.state.done[l.id]); };
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
</script>

<svelte:window onkeydown={menuKeys} onclick={e => { if (menuOpen && !menuEl?.contains(e.target as Node)) menuOpen = false; }} />
<div class="overview stage-view">
  <div class="overview-main sv-main">
    <div class="course-head">
      <a class="back-map" href="#/"><Icon name="back" /> Your library</a>
      <h1>{c.title}</h1>
      <details class="menu" bind:this={menuEl} bind:open={menuOpen}><summary class="btn secondary sm" aria-label="More roadmap actions"><Icon name="more" /> More</summary><div class="menu-list popover-enter">
        <button aria-expanded={prefsOpen} aria-controls="video-prefs" onclick={() => act(async () => { prefsOpen = !prefsOpen; await tick(); document.querySelector<HTMLElement>(prefsOpen ? '#video-prefs h2' : '.course-head summary')?.focus(); })}>Video preferences for this roadmap</button>
        <button id="adapt" disabled={appState.adapting} onclick={() => act(() => adaptNow(true))}>{appState.adapting ? 'Reading your results…' : 'Adjust to my progress'}<small>Add practice or drop lessons, based on your quiz and exercise results</small></button>
        <button id="revise-open" aria-expanded={reviseOpen} aria-controls="revise" onclick={() => act(async () => { reviseOpen = !reviseOpen; await tick(); if (reviseOpen) document.getElementById('ask')?.focus(); })}>Change my roadmap<small>Take the lessons you have not started somewhere else</small></button>
        {#if ls.some(l => l.stub) && !bulk?.text}<button id="write-all" onclick={writeEveryLesson}>Write every lesson now<small>Writes the remaining phases now. Uses many AI tokens.</small></button>{/if}
        <button id="offline" onclick={() => act(toggleOffline)}>{c.offline ? 'Remove offline videos' : 'Save videos for offline'}<small>{c.offline ? 'Deletes the saved copies. Videos then play from YouTube again.' : 'Downloads each lesson’s main video (up to 480p), so lessons work without internet.'}</small></button>
        {#if !c.folder}<button id="work" onclick={() => act(work)}>{c.work ? 'Disconnect folder: ' + c.work.split(/[\\/]/).pop() : 'Connect my project folder'}<small>The tutor can then read the text files in that folder. Parts of them are sent to your AI service.</small></button>{/if}
        <hr />
        <button id="export" onclick={() => act(() => exportCourse('export'))}>Export<small>Save this roadmap with your progress and notes to a file</small></button>
        <button id="template" onclick={() => act(() => exportCourse('template'))}>Share as template<small>Save the roadmap alone, without your progress or notes</small></button>
        <button id="summary" onclick={() => act(() => exportCourse('summary'))}>Save summary<small>A Markdown summary of what you made, your notes and quiz scores</small></button>
        <hr />
        <button id="roadmap-mistakes" onclick={() => act(() => { location.hash = '#/mistakes/' + enc(c.id); })}>Mistakes notebook<small>The questions you missed in this roadmap</small></button>
        <button id="delete-roadmap" class="danger" disabled={deleting} onclick={() => act(deleteRoadmap)}>Delete roadmap<small>Removes it with all its progress and notes</small></button>
      </div></details>
      <p class="goal">{c.summary}</p>
    </div>
    <div class="progress-line"><Progress {n} total={ls.length} wide={false} /><span>{n} of {ls.length} lessons complete · {ls.length ? Math.round(n / ls.length * 100) : 0}%</span></div>
    {#if tabbed}<div class="stage-nav" role="tablist" aria-label="Stages" tabindex="-1" bind:this={tabsEl} onkeydown={stageKeys}>{#each c.stages! as st, i}<button type="button" role="tab" id={`stage-tab-${i}`} aria-selected={i === cur} tabindex={i === cur ? 0 : -1} title={`Stage ${i + 1}: ${st.title} · ${stageDoneAll(i) ? 'Complete' : i === nextStage ? 'In progress' : 'Not started'}`} onclick={() => picked = i}><i class:on={i === nextStage && !complete} class:done={stageDoneAll(i)} aria-hidden="true"></i><span>Stage {i + 1}{c.stages!.length <= 4 || i === cur ? ` · ${st.title}` : ''}</span></button>{/each}</div>{/if}
  {#if appState.adapting}<Waiting skeleton="plan" inline title="Preparing a recommendation from your results. This can take a minute." />{/if}
  {#if bulk?.text || bulk?.error}<div class="row bulk-row">{#if bulk?.text}{#key bulk.text}<Waiting skeleton="plan" inline title={`${bulk.text}. Each phase takes about a minute.`} />{/key}<button class="btn secondary sm" id="stop-writing" disabled={bulk.stop} onclick={() => { if (bulk) bulk.stop = true; }}><BusyLabel busy={bulk.stop} idle="Stop" label="Stopping after this phase…" /></button>{/if}{#if bulk?.error}<p class="err">{bulk.error}</p>{/if}</div>{/if}
  {#if appState.suggestion?.cid === c.id}<PlanOptions preview={appState.suggestion.preview} cancel={() => appState.suggestion = null} />{/if}
  {#if prefsOpen}<section class="note" id="video-prefs" aria-labelledby="video-prefs-title"><h2 id="video-prefs-title" tabindex="-1">Video preferences for this roadmap</h2><VideoPrefs initial={c.prefs} course={c} /><button type="button" class="link revise-cancel" onclick={() => { prefsOpen = false; menuEl?.querySelector('summary')?.focus(); }}>Close</button></section>{/if}
  <form id="revise" class="note" hidden={!reviseOpen} onsubmit={e => { e.preventDefault(); void revise(); }}><strong>Change my roadmap</strong><p>Say what should change in the lessons you have not started. Your progress stays.</p><div class="own"><input id="ask" required maxlength="600" aria-label="What should change in the lessons you have not started?" placeholder="For example: show me how this is used in industry, with real applications" bind:value={ask} /><button class="btn" disabled={revising}><BusyLabel busy={revising} idle="Preview changes" label="Preparing options…" /> <Icon name="arrow" /></button></div>{#if revising}<Waiting skeleton="plans" title="Preparing roadmap options" hint="this can take about a minute" />{/if}{#if reviseError}<LoadError what="The options could not be prepared." error={reviseError} retry={() => void revise()} />{/if}{#if preview}<PlanOptions {preview} ask={originalAsk} cancel={() => preview = undefined} applied={() => { preview = undefined; reviseOpen = false; ask = ''; }} />{/if}<button type="button" class="link revise-cancel" hidden={!!preview} onclick={() => { reviseOpen = false; reviseError = ''; menuEl?.querySelector('summary')?.focus(); }}>Cancel</button></form>

  <div role={tabbed ? 'tabpanel' : undefined} aria-labelledby={tabbed ? `stage-tab-${cur}` : undefined}>
    {#if c.stages?.length}{@const st = c.stages[cur]}<div class="sg-head"><p class="kicker">Stage {cur + 1} of {c.stages.length} · {stageDone} of {stageLs.length} lessons</p><h2>{st.title}</h2>{#if st.blurb}<p>{st.blurb}</p>{/if}</div>{/if}
    {#if complete}<div class="celebrate"><strong>Roadmap complete.</strong><p>Every lesson is complete.</p><a class="btn" class:secondary={reviseOpen} href={href(c.id, ls[0].id)}>Open the first lesson <Icon name="arrow" /></a></div>
    {:else if here}{@const blocked = locked(c, here)}<section class="next-card" aria-labelledby="next-title"><div class="grow"><div class="kicker">{here === next ? (n ? 'Where you are' : 'Start here') : 'First lesson in this stage'} · Lesson {herePhase + 1}.{c.phases[herePhase].lessons.indexOf(here) + 1}</div><h2 id="next-title">{here.title}</h2><p>{blocked ? `Opens after Stage ${cur} and its exam.` : here.stub ? 'The AI writes this phase when you open it.' : here.goal}</p></div>{#if blocked}<span class="btn secondary" aria-disabled="true"><Icon name="lock" /> Locked</span>{:else}<a class="btn" class:secondary={reviseOpen} href={href(c.id, here.id)}>{here === next && n ? 'Continue' : 'Open lesson'} <Icon name="arrow" /></a>{/if}</section>
    {:else}<div class="celebrate"><strong>Stage complete.</strong><p>Every lesson in this stage is complete.{c.stages?.length ? ' Take the stage exam to prove it.' : ''}</p></div>{/if}
  {#if c.needs?.length}<details class="needs" open={!n}><summary>Before you start: {c.needs.length} thing{c.needs.length === 1 ? '' : 's'} to have ready</summary><ul id="needs">{#each c.needs as x, i}<li><label><input type="checkbox" checked={!!c.state.have?.[i]} onchange={e => { if (e.currentTarget.checked) (c.state.have ||= {})[i] = true; else delete c.state.have?.[i]; save(c); }} /><span><b>{x.name}</b>{#if x.why}<small>{x.why}</small>{/if}</span></label>{#if x.kind === 'tool' && x.command && found !== false}{#if !found}<span class="sk tag-line" role="status"><span class="sr">Checking</span></span>{:else}<span class="tag" class:ok={found[x.command]}>{found[x.command] ? 'Installed' : 'Not found on this computer'}</span>{#if !found[x.command] && appState.config.os !== 'windows'}<button type="button" class="btn secondary sm install" disabled={!!installing} onclick={() => void install(x.command!)}><BusyLabel busy={installing === x.command} idle="Install" label="Installing…" /></button>{/if}{/if}{/if}</li>{/each}</ul>{#if installError}<p class="err" role="alert">{installError}</p>{/if}</details>{/if}

    <section class="route-board" aria-labelledby="lessons-heading"><h2 class="sr" id="lessons-heading">Lessons</h2>
      {#each shown as { p, i: pi } (pi)}<details class="route-phase" id={`phase-${pi + 1}`} open={!p.lessons.every(l => c.state.done[l.id]) || shown.length === 1}><summary class="phase-flag"><Icon name="chevron" /><h3><span>Phase {pi + 1}</span>{p.title}</h3><b class="small">{doneIn(p.lessons)} / {p.lessons.length}{p.lessons.every(l => c.state.done[l.id]) ? ' · Complete' : ''}</b></summary>{#if p.blurb}<p class="phase-blurb">{p.blurb}</p>{/if}<div class="lesson-list route stagger">{#each p.lessons as l, li}{@const st = rowState(l)}<div class={`lesson-action-row route-item ${st}`}><a class={`lesson-row ${st}`} href={href(c.id, l.id)} aria-current={st === 'next' ? 'step' : undefined}><span class="state route-stop" aria-hidden="true">{#if st === 'done'}<Icon name="check" />{:else if st === 'locked'}<Icon name="lock" />{/if}</span><span class="num">{pi + 1}.{li + 1}</span><span class="stop-title">{l.title}{#if l.added} <span class="tag accent">Added for you</span>{/if}</span><span class="stop-meta">{META[st]}</span></a><LessonActions {c} {l} compact /></div>{/each}</div></details>{/each}
    </section>
  </div>
  {#if c.blueprint}<details class="note"><summary>Course blueprint and final challenge</summary><h3>{c.blueprint.challenge}</h3><p style="white-space:pre-line">{c.blueprint.criteria}</p><h3>Milestones</h3><p style="white-space:pre-line">{c.blueprint.milestones}</p></details>
  {:else}<details class="note"><summary>Design milestones for this course</summary>
    {#if draft}<form class="form-col" onsubmit={e => { e.preventDefault(); void retrofit(true); }}>
      <BlueprintFields plan={draft} /><div class="row"><button class="btn" disabled={planning}>Approve milestones</button><button type="button" class="link" disabled={planning} onclick={() => draft = undefined}>Cancel</button></div></form>
    {:else}<button class="btn secondary" disabled={planning} onclick={() => retrofit()}>Draft a blueprint from my lessons</button>{/if}
    {#if planning}<p role="status">Reviewing your course…</p>{/if}{#if planError}<p role="alert" class="err">{planError}</p>{/if}
  </details>{/if}
  {#if where && !complete && n > 0}<section class="note where" aria-labelledby="where-title"><h2 id="where-title">Where you are</h2>
    <p><strong>Learning now</strong> <a href={href(c.id, where.next.id)}>{where.next.title}</a>: {where.next.goal}</p>
    <p><strong>Recommended next</strong> <a href={href(c.id, where.why.l.id)}>{where.why.text}</a></p>
    {#if where.milestone}<p><strong>Next milestone</strong> <a href={href(c.id, where.milestone.id) + '/studio'}>{where.milestone.checkpoint?.task}</a></p>{/if}
    {#if where.shown.length}<p><strong>You have shown</strong></p><ul>{#each where.shown.slice(-5) as s}<li><a href={href(c.id, s.l.id)}>{s.l.title}</a>: {s.text}</li>{/each}</ul>{/if}
    {#if where.shaky.length}<p><strong>Still shaky</strong></p><ul>{#each where.shaky as s}<li><a href={href(c.id, s.l.id)}>{s.l.title}</a>: {s.text}</li>{/each}</ul>{/if}
  </section>{/if}
  {#if ls.some(l => l.checkpoint || c.state.portfolio?.[l.id])}<details class="note"><summary>Milestones and evidence</summary>
    {#each ls.filter(l => l.checkpoint || c.state.portfolio?.[l.id]) as l}
      {@const attempts = c.state.portfolio?.[l.id]?.attempts || []}{@const last = attempts[attempts.length-1]}
      <article><h3><a href={href(c.id,l.id)+'/studio'}>{l.title}</a></h3><p>{l.checkpoint?.task || l.goal}</p>
      {#if last}<p><strong>{last.mode === 'independent' ? 'Independent attempt' : 'Guided practice'}</strong> {last.evidence}</p><p>{last.improvement} {last.next}</p>{:else}<p class="small">No assessed attempt yet.</p>{/if}</article>
    {/each}</details>{/if}
  {#if c.offline && offline}<p class="offline-status" role="status"><Icon name="Watch" /> {offline.running ? 'Saving videos for offline' : 'Videos saved for offline'}: {offline.saved} of {offline.total} · {mb(offline.bytes)}{#if offline.failed && !offline.running}{' · '}{offline.failed} could not be saved. {offline.error}{/if}</p>{/if}
  <div id="video-status">{#if pending}<p class="video-status"><Icon name="Watch" /> Videos may still be arriving for {pending} of {ls.length} lessons. Open any lesson to search again if needed.</p>{/if}</div>
  {#if c.changes?.length}<details class="note change"><summary>Roadmap change history</summary>{#each c.changes.slice().reverse() as x}<p>{x.note}</p>{#if x.added.length}<p class="small">Added: {x.added.join(', ')}</p>{/if}{#if x.dropped.length}<p class="small">Removed: {x.dropped.join(', ')}</p>{/if}{#if x.changed?.length}<p class="small">Rewritten: {x.changed.join(', ')}</p>{/if}{/each}</details>{/if}
  </div>
  <aside class="sv-side" aria-label="This stage at a glance">
    {#if activity}<section class="side-card goal-card"><div class="grow"><p class="kicker">Daily goal</p><strong>{activity.done} / {activity.goal}</strong><span class="small">steps today{activity.streak ? ` · ${activity.streak}-day streak` : ''}</span></div><Ring value={activity.done} max={activity.goal} label="Daily goal" /></section>{/if}
    <section class="side-card"><p class="kicker">{c.stages?.length ? 'Stage milestone' : 'Milestone'}</p>{#if milestone}<a class="ms-title" href={href(c.id, milestone.id) + '/studio'}>{milestone.checkpoint?.task}</a><p class="small">In {milestone.title}</p>{:else}<p class="small">{c.blueprint ? 'No milestone in this stage.' : 'No milestones yet. Design them below the lessons.'}</p>{/if}<div class="ms-row"><Progress n={stageDone} total={stageLs.length} /><span class="small">{stageDone} of {stageLs.length}</span></div></section>
    {#if c.stages?.length}<section class="side-card exam-card"><p class="kicker">Stage exam</p><ExamRow {c} stage={cur} /></section>{/if}
    <section class="side-card"><div class="side-card-head"><p class="kicker">Mistakes notebook</p><a class="small" href={'#/mistakes/' + enc(c.id)}>Open</a></div>{#if mistakes === undefined}<span class="sk" style="height:10px;width:70%"></span>{:else if stageMistakes.length}<ul class="mist">{#each stageMistakes.slice(0, 4) as m}<li><a href={href(m.course, m.lesson)}>{m.kind === 'code' ? 'Coding exercise: ' + (m.title || m.lesson_title) : m.q}</a><span class="tag">{m.kind === 'code' ? 'Tests' : 'Missed ' + m.n + '×'}</span></li>{/each}</ul>{#if stageMistakes.length > 4}<p class="small">and {stageMistakes.length - 4} more</p>{/if}{:else}<p class="small">No open mistakes in this {c.stages?.length ? 'stage' : 'roadmap'}.</p>{/if}</section>
  </aside>
</div>
