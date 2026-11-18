<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { appState, loadLibrary } from '../lib/state.svelte';
  import { href } from '../lib/format';
  import Icon from '../components/Icon.svelte';
  import LibraryCard from '../components/LibraryCard.svelte';
  import Progress from '../components/Progress.svelte';
  import Creation from './Creation.svelte';
  let active = $derived(appState.library.filter(c => !c.paused)), paused = $derived(appState.library.filter(c => c.paused));
  let continuing = $derived(active.filter(c => c.done < c.total && c.next));
  let goal = $derived(appState.activity);
  // A roadmap written by an agent through the MCP server, or by another copy of the app, shows up without reopening the page.
  onMount(() => {
    let last = '', busy = false;
    const check = async () => {
      if (busy || document.hidden) return; busy = true;
      try { const { stamp } = await api<{ stamp: string }>('GET', 'api/stamp'); if (last && stamp !== last) await loadLibrary(); last = stamp; }
      catch { /* The next check tries again. */ } finally { busy = false; }
    };
    void check(); const timer = setInterval(check, 4000);
    addEventListener('focus', check);
    return () => { clearInterval(timer); removeEventListener('focus', check); };
  });
</script>
{#if !appState.library.length}<Creation empty />
{:else}{@const first = continuing[0]}<div class="wrap home-workspace screen-enter"><div class="today-column">
  <div class="home-head"><h1>Today</h1></div>
  {#if first}<section class="hero" aria-labelledby="continue-heading"><div class="kicker">Up next in {first.title}</div><h2 id="continue-heading">{first.next!.title}</h2><div class="hero-meta"><Progress n={first.done} total={first.total} /><span>{first.done} of {first.total} lessons complete</span></div><a class="btn big" href={href(first.id, first.next!.id)}>{first.done ? 'Continue learning' : 'Start the first lesson'} <Icon name="arrow" /></a></section>
  {:else}<section class="hero hero-empty"><Icon name="empty-route" /><div class="kicker">All caught up</div><h2>No lessons waiting.</h2><p class="muted">Revisit a finished roadmap or start something new.</p><button class="btn big" onclick={() => appState.screen = 'creation'}><Icon name="plus" /> New roadmap</button></section>{/if}
  <div class="side-actions ruled stagger">{#if goal}<a class="side-action" id="daily-goal" href="#/progress"><span class="grow"><b>{goal.done >= goal.goal ? 'Daily goal met' : `${goal.done} of ${goal.goal} steps today`}</b><span>{goal.streak ? `${goal.streak}-day streak` : 'Meet your daily goal to start a streak'} · See your progress</span></span><Progress n={Math.min(goal.done, goal.goal)} total={goal.goal} wide={false} unit="steps toward today’s goal" /><Icon name="arrow" /></a>{/if}{#if appState.review.count}<a class="side-action" href="#/review"><span class="grow"><b>{appState.review.count} question{appState.review.count === 1 ? '' : 's'} to review today</b></span><Icon name="arrow" /></a>{:else}<div class="side-action"><span class="grow"><b>Review</b><span>Nothing due for review today.</span></span></div>{/if}<a class="side-action" href="#/mistakes"><span class="grow"><b>Mistakes notebook</b></span><Icon name="arrow" /></a></div>
  {#if continuing.length > 1}<div class="section-title"><h2>Also in progress</h2></div><nav class="resume-list ruled stagger" aria-label="Other roadmaps in progress">{#each continuing.slice(1) as c}<a class="resume-row" href={href(c.id, c.next!.id)}><span class="grow"><small>{c.title}</small><b>{c.next!.title}</b></span><Progress n={c.done} total={c.total} /><span class="count">{c.done} / {c.total}</span><Icon name="arrow" /></a>{/each}</nav>{/if}
  </div><div class="library-column"><div class="section-title"><h2>Your library</h2><span class="grow">{active.length} roadmap{active.length === 1 ? '' : 's'}</span><div class="row"><a id="import-roadmap" class="btn ghost sm" title="Open a roadmap file saved with Export" href="#/import">Import</a><button id="add-roadmap" class="btn secondary sm" onclick={() => appState.screen = 'creation'}><Icon name="plus" /> New roadmap</button></div></div>
  {#if active.length}<div class="cards ruled stagger">{#each active as c (c.id)}<LibraryCard course={c} />{/each}</div>{:else}<p class="muted">Your roadmaps are paused. Resume one below when you’re ready.</p>{/if}
  {#if appState.config.damaged > 0}{@const n = appState.config.damaged}<p class="small damaged-note">{n} roadmap file{n === 1 ? ' is' : 's are'} damaged and not shown. {n === 1 ? 'The file was' : 'The files were'} left untouched in the app’s data folder.</p>{/if}
  {#if paused.length}<div class="section-title"><h2>Paused</h2><span>Ready whenever you are</span></div><div class="cards paused-cards ruled stagger">{#each paused as c (c.id)}<LibraryCard course={c} />{/each}</div>{/if}
</div></div>{/if}
