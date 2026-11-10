<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { api } from '../lib/api';
  import { appState, save, toast, message, adaptNow, expandPhase, flushSave } from '../lib/state.svelte';
  import { lessons, nextLesson, status, locked, stageOpen, stageLessons } from '../lib/progress';
  import { enc, href } from '../lib/format';
  import type { Course } from '../lib/types';
  import Icon from '../components/Icon.svelte';
  import Progress from '../components/Progress.svelte';
  import ExamRow from '../components/ExamRow.svelte';
  let { c }: { c: Course } = $props();
  let ls = $derived(lessons(c)), n = $derived(ls.filter(l => c.state.done[l.id]).length), next = $derived(nextLesson(c));
  let complete = $derived(ls.length > 0 && n === ls.length), pending = $derived(ls.filter(l => !l.videos && !l.stub).length);
  let found = $state<Record<string, boolean> | false | undefined>(undefined);
  let reviseOpen = $state(false), ask = $state('');
  let bulk = $state<{ stop: boolean; text: string; error: string } | null>(null);
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
    if (!confirm(`Write ${phases.length} phases now? This uses many AI tokens.`)) return;
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
    if (await adaptNow(true, ask.trim())) { reviseOpen = false; ask = ''; }
  }
  async function exportCourse(kind: string) {
    try {
      await flushSave();
      const r = await api<{ path: string }>('POST', 'api/courses/' + enc(c.id) + (kind === 'summary' ? '/summary' : '/export'), kind === 'template' ? { template: true } : undefined);
      toast('Saved to ' + r.path);
    } catch (error) { toast(message(error)); }
  }
  async function work() {
    try { c.work = (await api<{ work?: string }>('PUT', 'api/courses/' + enc(c.id) + '/work', c.work ? { clear: true } : {})).work; }
    catch (error) { toast(message(error)); }
  }
  function toggleStage(s: number) { (c.state.stageOpen ||= {})[s] = !stageOpen(c, s); save(c); }
  async function jump(i: number) {
    const stage = c.phases[i].stage;
    if (c.stages && stage !== undefined && !stageOpen(c, stage)) { (c.state.stageOpen ||= {})[stage] = true; save(c); await tick(); }
    document.getElementById('phase-' + (i + 1))?.scrollIntoView({ block: 'start' });
  }
</script>

<div class="wrap">
  <div class="overview-head"><div><a class="back-map" href="#/"><Icon name="back" /> Your roadmaps</a><h1>{c.title}</h1><p class="goal">{c.summary}</p>{#if complete}<div class="celebrate"><strong>Roadmap complete.</strong><p>Every lesson is complete.</p><a class="btn" href={href(c.id, ls[0].id)}>Open the first lesson <Icon name="arrow" /></a></div>{:else if next}<p class="continue-title">Next lesson: {next.title}</p><a class="btn" href={href(c.id, next.id)}>{n ? 'Continue learning' : 'Start the first lesson'} <Icon name="arrow" /></a>{/if}</div><div class="progress-board"><span>Your progress</span><strong>{ls.length ? Math.round(n / ls.length * 100) : 0}%</strong><span>{n} of {ls.length} lessons complete</span><Progress {n} total={ls.length} wide={false} /></div></div>
  <div id="video-status">{#if pending}<p class="video-status"><Icon name="Watch" /> Videos may still be arriving for {pending} of {ls.length} lessons. Open any lesson to search again if needed.</p>{/if}</div>
  {#if c.changes?.length}{@const x = c.changes[c.changes.length - 1]}<div class="note change"><strong>Roadmap updated</strong><p>{x.note}</p>{#if x.added.length}<p class="small">Added: {x.added.join(', ')}</p>{/if}{#if x.dropped.length}<p class="small">Removed: {x.dropped.join(', ')}</p>{/if}</div>{/if}
  {#if c.needs?.length}<details class="needs" open={!n}><summary>Before you start: {c.needs.length} thing{c.needs.length === 1 ? '' : 's'} to have ready</summary><ul id="needs">{#each c.needs as x, i}<li><label><input type="checkbox" checked={!!c.state.have?.[i]} onchange={e => { if (e.currentTarget.checked) (c.state.have ||= {})[i] = true; else delete c.state.have?.[i]; save(c); }} /><span><b>{x.name}</b>{#if x.why}<small>{x.why}</small>{/if}</span></label>{#if x.kind === 'tool' && x.command && found !== false}<span class="tag" class:ok={found?.[x.command]}>{!found ? 'Checking…' : found[x.command] ? 'Installed' : 'Not found on this computer'}</span>{/if}</li>{/each}</ul></details>{/if}
  <div class="row tools">
    <button class="btn secondary" id="adapt" disabled={appState.adapting} title="The AI reads your quiz and exercise results and adds practice or removes lessons you no longer need" onclick={() => adaptNow(true)}>{appState.adapting ? 'Reading your results…' : 'Adjust to my progress'}</button>
    {#if !c.folder}<button class="btn secondary" id="work" title="The tutor can then read the text files in that folder. Parts of them are sent to your AI service." onclick={work}>{c.work ? 'Disconnect folder: ' + c.work.split(/[\\/]/).pop() : 'Connect my project folder'}</button>{/if}
    <button class="btn secondary" id="revise-open" aria-expanded={reviseOpen} aria-controls="revise" title="Ask the AI to take the lessons you have not started somewhere else: more depth, industry practice, another application" onclick={async () => { reviseOpen = !reviseOpen; await tick(); if (reviseOpen) document.getElementById('ask')?.focus(); }}>Change my roadmap</button>
    {#if ls.some(l => l.stub) && !bulk?.text}<button class="btn secondary" id="write-all" onclick={writeEveryLesson}>Write every lesson now</button>{/if}
    {#if bulk?.text}<span role="status">{bulk.text}</span><button class="btn secondary" id="stop-writing" disabled={bulk.stop} onclick={() => { if (bulk) bulk.stop = true; }}>Stop</button>{/if}
    {#if bulk?.error}<p class="err">{bulk.error}</p>{/if}
    <button class="btn secondary" id="export" title="Save this roadmap with your progress and notes to a file" onclick={() => exportCourse('export')}>Export</button><button class="btn secondary" id="template" title="Save the roadmap alone, without your progress or notes, to give to someone else" onclick={() => exportCourse('template')}>Share as template</button><button class="btn secondary" id="summary" title="Save a Markdown summary of what you made, your notes and your quiz scores" onclick={() => exportCourse('summary')}>Save summary</button>
  </div>
  <form id="revise" class="own" hidden={!reviseOpen} onsubmit={e => { e.preventDefault(); void revise(); }}><input id="ask" required maxlength="600" aria-label="What should change in the lessons you have not started?" placeholder="For example: show me how this is used in industry, with real applications" bind:value={ask} /><button class="btn" disabled={appState.adapting}>{appState.adapting ? 'Changing your roadmap…' : 'Change it'} <Icon name="arrow" /></button></form>
  {#if c.phases.length > 4}<nav class="phase-index" aria-label={`Jump to ${c.stages ? 'stage' : 'phase'}`}>{#each c.phases as p, i}{#if !c.stages || (p.stage !== undefined && c.phases[i - 1]?.stage !== p.stage)}<a href={`#phase-${i + 1}`} onclick={e => { e.preventDefault(); void jump(i); }}>{c.stages && p.stage !== undefined ? c.stages[p.stage].title : `${i + 1} · ${p.title}`}</a>{/if}{/each}</nav>{/if}
  <section class="route-board"><div class="route-heading"><h2>Lessons</h2><span class="small">{c.phases.length} phases</span></div>
    {#each c.phases as p, pi}
      {#if c.stages && p.stage !== undefined && c.phases[pi - 1]?.stage !== p.stage}{@const st = c.stages[p.stage]}<button type="button" class="stage-head" aria-expanded={stageOpen(c, p.stage)} onclick={() => toggleStage(p.stage!)}><span><span class="small">Stage {p.stage + 1}</span><strong>{st.title}</strong>{#if st.blurb}<span class="blurb">{st.blurb}</span>{/if}</span><span class="small">{stageLessons(c, p.stage).filter(l => c.state.done[l.id]).length} / {stageLessons(c, p.stage).length} lessons <span aria-hidden="true">{stageOpen(c, p.stage) ? '▾' : '▸'}</span></span></button>{/if}
      <section class="route-phase" hidden={!!c.stages && p.stage !== undefined && !stageOpen(c, p.stage)} id={`phase-${pi + 1}`}><div class="phase-flag"><span class="flag"><Icon name="flag" /> Phase {pi + 1}</span><h3>{p.title}</h3><p>{p.blurb}</p><b class="small">{p.lessons.filter(l => c.state.done[l.id]).length} / {p.lessons.length} lessons{p.lessons.every(l => c.state.done[l.id]) ? ' · Complete' : ''}</b></div><div class="lesson-list">{#each p.lessons as l, li}<a class={`lesson-row ${status(c, l)}${next === l && !complete ? ' next' : ''}`} href={href(c.id, l.id)}><span class="num">{#if c.state.done[l.id]}<Icon name="Done when" />{:else}{pi + 1}.{li + 1}{/if}</span><span class="stop-title">{l.title}{#if l.added} <span class="tag">Added for you</span>{/if}</span><span class="stop-meta">{c.state.done[l.id] ? 'Complete' : status(c, l) === 'part' ? 'In progress' : locked(c, l) ? 'Locked' : next === l && !complete ? 'Next' : l.stub ? 'Outline' : ''}</span></a>{/each}</div></section>
      {#if c.stages && p.stage !== undefined && c.phases[pi + 1]?.stage !== p.stage && stageOpen(c, p.stage)}<ExamRow {c} stage={p.stage} />{/if}
    {/each}
  </section>
</div>
