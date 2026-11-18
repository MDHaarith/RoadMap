<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { appState, message } from '../lib/state.svelte';
  import { href } from '../lib/format';
  import type { Chart as ChartData, Stats } from '../lib/types';
  import Icon from '../components/Icon.svelte';
  import Chart from '../components/Chart.svelte';
  import ProgressBar from '../components/Progress.svelte';
  import Waiting from '../components/Waiting.svelte';
  import LoadError from '../components/LoadError.svelte';
  let stats = $state<Stats>(), error = $state(''), goalError = $state(''), goal = $state(3);
  let alive = true;
  function load() { error = ''; api<Stats>('GET', 'api/progress').then(s => { if (alive) { stats = s; goal = s.activity.goal; } }).catch(e => { if (alive) error = message(e); }); }
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
</script>
<div class="wrap screen-enter progress-page" class:narrow={!stats}><a class="back-map" href="#/"><Icon name="back" /> Your library</a><div class="page-head"><h1>Your progress</h1></div>
  {#if error}<LoadError what="Your progress could not load." {error} retry={load} />{:else if !stats}<Waiting skeleton="rows" inline title="Adding up your progress" />{:else}{@const a = stats.activity}
  <div class="progress-figures ruled">
    <div><strong>{a.streak}</strong><span>day streak{a.best > a.streak ? ` · best ${a.best}` : ''}</span></div>
    <div><strong>{a.done} / {a.goal}</strong><span>steps today</span></div>
    <div><strong>{stats.lessons_done}</strong><span>lessons complete</span></div>
    <div><a href="#/mistakes"><strong>{stats.mistakes_open}</strong><span>mistakes to fix</span></a></div>
  </div>
  <form class="goal-form" onsubmit={e => { e.preventDefault(); void saveGoal(); }}><label for="goal">Daily goal</label><input id="goal" type="number" min="1" max="50" required bind:value={goal} /><span class="small">steps a day. A finished lesson, a finished quiz and each review answer count as one step.</span><button class="btn secondary sm" disabled={goal === a.goal}>Save</button>{#if goalError}<p class="err" role="alert">{goalError}</p>{/if}</form>
  <div class="progress-charts">{#if daily}<Chart chart={daily} />{/if}{#if accuracy}<Chart chart={accuracy} />{/if}</div>
  <div class="progress-lists">
    <section><div class="section-title"><h2>Roadmaps</h2></div><div class="ruled">{#each stats.roadmaps as r}<a class="resume-row" href={href(r.id)}><span class="grow"><b>{r.title}</b>{#if r.paused}<small>Paused</small>{/if}</span><ProgressBar n={r.done} total={r.total} /><span class="count">{r.done} / {r.total}</span></a>{/each}</div></section>
    <section><div class="section-title"><h2>Lessons to practise again</h2></div>{#if stats.weak.length}<div class="ruled">{#each stats.weak as w}<a class="resume-row" href={href(w.course, w.lesson)}><span class="grow"><small>{w.roadmap}</small><b>{w.title}</b></span><span class="count">Quiz {w.score} / {w.total}</span><Icon name="arrow" /></a>{/each}</div>{:else}<p class="muted">No quiz under 70% right now.</p>{/if}</section>
  </div>{/if}
</div>
<style>
  .progress-figures { display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: var(--s4); margin: var(--s3) 0 var(--s5); }
  .progress-figures div, .progress-figures a { display: flex; flex-direction: column; gap: 2px; color: var(--ink); }
  .progress-figures strong { font-size: 1.6rem; font-weight: 600; font-variant-numeric: tabular-nums; }
  .progress-figures span { font-size: var(--t-sm); color: var(--muted); }
  .goal-form { display: flex; flex-wrap: wrap; align-items: center; gap: var(--s3); margin-bottom: var(--s5); }
  .goal-form input { width: 5em; }
  .goal-form .small { flex: 1 1 260px; }
  /* Two charts side by side; each keeps its caption above the graph and its note below. */
  .progress-charts { display: grid; grid-template-columns: repeat(auto-fit, minmax(420px, 1fr)); gap: var(--s6); }
  .progress-charts :global(.chart) { grid-template-columns: minmax(0, 1fr); }
  .progress-charts :global(.chart > *) { grid-row: auto; grid-column: 1; }
  .progress-lists { display: grid; grid-template-columns: repeat(auto-fit, minmax(320px, 1fr)); gap: var(--s6); }
  @media (max-width: 700px) { .progress-figures { grid-template-columns: repeat(2, minmax(0, 1fr)); } }
</style>
