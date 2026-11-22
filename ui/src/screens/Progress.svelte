<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { appState, message } from '../lib/state.svelte';
  import { enc, href } from '../lib/format';
  import { nextLesson } from '../lib/progress';
  import type { Chart as ChartData, Course, Stats } from '../lib/types';
  import Icon from '../components/Icon.svelte';
  import Chart from '../components/Chart.svelte';
  import ProgressBar from '../components/Progress.svelte';
  import Ring from '../components/Ring.svelte';
  import Select from '../components/Select.svelte';
  import Waiting from '../components/Waiting.svelte';
  import LoadError from '../components/LoadError.svelte';
  let stats = $state<Stats>(), error = $state(''), goalError = $state(''), goal = $state(3);
  let alive = true;
  function load() { error = ''; api<Stats>('GET', 'api/progress').then(s => { if (alive) { stats = s; goal = s.activity.goal; if (!mapId) pickMap(s); } }).catch(e => { if (alive) error = message(e); }); }
  onMount(() => { load(); return () => { alive = false; }; });
  // Day numbers count days since 1970 on the learner's calendar, so the UTC date of that many days is the right date.
  const date = (day: number) => new Date(day * 864e5).toLocaleDateString(undefined, { day: 'numeric', month: 'short', timeZone: 'UTC' });
  let daily = $derived<ChartData | undefined>(stats && { title: 'Steps each day', kind: 'bar', x: '', y: 'Steps', labels: stats.activity.days.map(d => date(d.day)), series: [{ name: '', values: stats.activity.days.map(d => d.n) }], note: `Finished lessons, finished quizzes and review answers over the last five weeks. Your goal is ${stats.activity.goal} a day.` });
  // Weeks with no answers have no accuracy, so they are left out rather than drawn as zero.
  let answered = $derived(stats ? stats.weeks.filter(w => w.asked > 0) : []);
  let accuracy = $derived<ChartData | undefined>(answered.length > 1 ? { title: 'Right answers each week', kind: 'line', x: '', y: '% right', labels: answered.map(w => w.week ? `${w.week} wk ago` : 'This week'), series: [{ name: '', values: answered.map(w => Math.round(w.right / w.asked * 100)) }], note: 'Quizzes and daily review answers, as a share of all answers that week.' } : undefined);
  async function saveGoal() {
    goalError = '';
    try { await api('PUT', 'api/goal', { goal }); if (stats) stats.activity.goal = goal; if (appState.activity) appState.activity.goal = goal; load(); }
    catch (e) { goalError = message(e); }
  }
  // The lesson map shows one roadmap: the one open last, else the first active one.
  let mapId = $state(''), map = $state<Course>(), mapError = $state('');
  function pickMap(s: Stats) {
    const r = s.roadmaps.find(x => x.id === appState.course?.id) ?? s.roadmaps.find(x => !x.paused && x.done < x.total) ?? s.roadmaps[0];
    if (r) showMap(r.id);
  }
  function showMap(id: string) {
    mapId = id; mapError = ''; map = undefined;
    api<Course>('GET', 'api/courses/' + enc(id)).then(c => { if (alive && mapId === id) map = c; }).catch(e => { if (alive) mapError = message(e); });
  }
  // Phases grouped under their stage; a course without stages is one group.
  let blocks = $derived.by(() => {
    if (!map) return [];
    const c = map, out: { title: string; phases: { i: number; title: string; lessons: Course['phases'][number]['lessons'] }[] }[] = [];
    let stage = -1;
    c.phases.forEach((p, i) => {
      const s = c.stages?.length ? (p.stage ?? stage) : 0;
      if (s !== stage || !out.length) { out.push({ title: c.stages?.length ? `Stage ${s + 1} · ${c.stages[s]?.title ?? ''}` : 'Lessons', phases: [] }); stage = s; }
      out[out.length - 1].phases.push({ i, title: p.title, lessons: p.lessons });
    });
    return out;
  });
  let mapNext = $derived(map ? nextLesson(map) : undefined);
</script>
<div class="wrap screen-enter progress-page" class:narrow={!stats}><a class="back-map" href="#/"><Icon name="back" /> Your library</a><div class="page-head"><h1>Your progress</h1></div>
  {#if error}<LoadError what="Your progress could not load." {error} retry={load} />{:else if !stats}<Waiting skeleton="rows" inline title="Adding up your progress" />{:else}{@const a = stats.activity}
  <div class="stats">
    <div class="stat"><div class="grow"><p class="kicker">Daily goal</p><strong>{a.done} / {a.goal}</strong><span class="small">steps today</span></div><Ring value={a.done} max={a.goal} label="Daily goal" /></div>
    <div class="stat"><div class="grow"><p class="kicker">Lessons</p><strong>{stats.lessons_done}</strong><span class="small">lessons complete</span></div></div>
    <div class="stat"><div class="grow"><p class="kicker">Streak</p><strong>{a.streak} day{a.streak === 1 ? '' : 's'}</strong><span class="small">{a.best > a.streak ? `best ${a.best} days` : 'your best so far'}</span></div></div>
    <a class="stat" href="#/mistakes"><div class="grow"><p class="kicker">Mistakes notebook</p><strong>{stats.mistakes_open}</strong><span class="small">mistakes to fix</span></div><Icon name="arrow" /></a>
  </div>
  <form class="goal-form" onsubmit={e => { e.preventDefault(); void saveGoal(); }}><label for="goal">Daily goal</label><input id="goal" type="number" min="1" max="50" required bind:value={goal} /><span class="small">steps a day. A finished lesson, a finished quiz and each review answer count as one step.</span><button class="btn secondary sm" disabled={goal === a.goal}>Save</button>{#if goalError}<p class="err" role="alert">{goalError}</p>{/if}</form>
  <div class="pg">
    <div>
      <section><p class="kicker">Roadmaps</p>{#each stats.roadmaps as r}<a class="rm" href={href(r.id)}><span class="rm-row"><b>{r.title}</b><span class="small">{r.paused ? 'Paused · ' : ''}{r.done} of {r.total} lessons</span></span><ProgressBar n={r.done} total={r.total} /></a>{/each}</section>
      <section class="pg-weak"><p class="kicker">Lessons to practise again</p>{#if stats.weak.length}<ul class="mist">{#each stats.weak as w}<li><a href={href(w.course, w.lesson)}>{w.title}<small> · {w.roadmap}</small></a><span class="tag">Quiz {Math.round(w.score / w.total * 100)}%</span></li>{/each}</ul>{:else}<p class="small">No quiz under 70% right now.</p>{/if}</section>
    </div>
    <section class="map" aria-labelledby="map-title">
      <div class="map-head"><p class="kicker" id="map-title">Lesson map</p>{#if stats.roadmaps.length > 1}<Select id="map-roadmap" ariaLabel="Roadmap on the map" value={mapId} options={stats.roadmaps.map(r => ({ value: r.id, label: r.title }))} onchange={showMap} />{/if}</div>
      {#if mapError}<p class="err">{mapError}</p>{:else if !map}<span class="sk" style="display:block;height:10px;width:60%;margin-top:8px"></span>{:else}{@const c = map}
        {#if stats.roadmaps.length <= 1}<p class="map-title">{c.title}</p>{/if}
        {#each blocks as b}{@const all = b.phases.flatMap(p => p.lessons)}{@const done = all.filter(l => c.state.done[l.id]).length}<div class="stage-block">
          <div class="stage-row"><b>{b.title}</b>{#if done === all.length}<span class="tag ok"><Icon name="check" /> Complete</span>{:else}<span class="small">{done} / {all.length}</span>{/if}</div>
          {#each b.phases as p}<h3>{p.title}</h3><div class="tiles">{#each p.lessons as l, li}<a class="tile" class:done={c.state.done[l.id]} class:now={l === mapNext && !c.state.done[l.id]} href={href(c.id, l.id)} title={l.title} aria-label={`${p.i + 1}.${li + 1} ${l.title}${c.state.done[l.id] ? ', complete' : ''}`}>{p.i + 1}.{li + 1}</a>{/each}</div>{/each}
        </div>{/each}
      {/if}
    </section>
  </div>
  <div class="progress-charts">{#if daily}<Chart chart={daily} />{/if}{#if accuracy}<Chart chart={accuracy} />{/if}</div>
  {/if}
</div>
<style>
  .stats { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: var(--s5); margin: var(--s3) 0 var(--s6); }
  .stat { display: flex; align-items: center; gap: var(--s5); padding: 14px var(--s6); border: 1px solid var(--line); border-radius: var(--r-lg); color: var(--ink); }
  a.stat:hover { border-color: var(--line-strong); text-decoration: none; }
  a.stat :global(.icon) { color: var(--muted); }
  .stat .kicker { margin: 0; }
  .stat strong { display: block; margin-top: 3px; font: 650 26px/1.1 var(--display); font-variant-numeric: tabular-nums; }
  .goal-form { display: flex; flex-wrap: wrap; align-items: center; gap: var(--s3); margin-bottom: var(--s6); }
  .goal-form input { width: 5em; }
  .goal-form .small { flex: 1 1 260px; }
  .pg { display: grid; grid-template-columns: minmax(0, 1fr) 340px; gap: 32px; margin-bottom: var(--s7); }
  .rm { display: grid; gap: var(--s3); padding: var(--s5) 0 14px; border-top: 1px solid var(--line); color: var(--ink); }
  .rm:hover { text-decoration: none; }
  .rm:hover b { color: var(--accent-ink); }
  .rm-row { display: flex; justify-content: space-between; align-items: baseline; gap: var(--s5); }
  .rm b { font: 650 var(--t-lg)/1.3 var(--display); }
  .rm-row .small { white-space: nowrap; }
  .rm :global(.bar) { width: 100%; }
  .pg-weak { margin-top: var(--s6); }
  .pg-weak small { color: var(--muted); }
  .map-head { display: flex; align-items: center; justify-content: space-between; gap: var(--s3); }
  .map-head .kicker { margin: 0; white-space: nowrap; }
  .map-title { margin: var(--s2) 0 0; font: 650 var(--t-lg)/1.3 var(--display); }
  .stage-block { padding-bottom: var(--s5); border-bottom: 1px solid var(--line); }
  .stage-row { display: flex; justify-content: space-between; align-items: baseline; gap: var(--s3); margin-top: var(--s4); font-size: var(--t-sm); }
  .map h3 { margin: var(--s4) 0 var(--s2); font-size: var(--t-sm); font-weight: 600; }
  .tiles { display: flex; flex-wrap: wrap; gap: 5px; }
  .tile { width: 40px; height: 24px; display: grid; place-items: center; border: 1px solid var(--line-strong); border-radius: 6px; background: var(--surface); font-size: 10.5px; font-weight: 600; color: var(--muted); transition: transform var(--motion-fast); }
  .tile:hover { text-decoration: none; border-color: var(--accent); }
  .tile.done { background: var(--accent); border-color: var(--accent); color: var(--on-accent); }
  .tile.now { border-color: var(--accent); background: var(--accent-soft); color: var(--accent-ink); }
  /* Two charts side by side; each keeps its caption above the graph and its note below. */
  .progress-charts { display: grid; grid-template-columns: repeat(auto-fit, minmax(420px, 1fr)); gap: var(--s6); }
  .progress-charts :global(.chart) { grid-template-columns: minmax(0, 1fr); }
  .progress-charts :global(.chart > *) { grid-row: auto; grid-column: 1; }
  @media (max-width: 1000px) { .pg { grid-template-columns: minmax(0, 1fr); } .stats { grid-template-columns: repeat(2, minmax(0, 1fr)); } }
</style>
