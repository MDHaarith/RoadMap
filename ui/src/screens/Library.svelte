<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { appState, loadLibrary, toast, message } from '../lib/state.svelte';
  import { href } from '../lib/format';
  import Icon from '../components/Icon.svelte';
  import LibraryCard from '../components/LibraryCard.svelte';
  import Creation from './Creation.svelte';
  let importing = $state(false);
  let alive = true;
  onMount(() => () => { alive = false; });
  let active = $derived(appState.library.filter(c => !c.paused)), paused = $derived(appState.library.filter(c => c.paused));
  let continuing = $derived(active.filter(c => c.done < c.total && c.next));
  async function importRoadmap() {
    importing = true;
    try { const r = await api<{ id?: string }>('POST', 'api/import'); if (r.id && alive) await loadLibrary(); }
    catch (error) { toast(message(error)); } finally { importing = false; }
  }
</script>
{#if !appState.library.length}<Creation empty />
{:else}<div class="wrap narrow">
  <div class="intro"><div><div class="kicker">Your next step is ready</div><h1>Today</h1></div><div class="row"><a class="btn secondary" href="#/mistakes">Mistakes</a><button id="import-roadmap" class="btn secondary" title="Open a roadmap file saved with Export" disabled={importing} onclick={importRoadmap}>Import</button><button id="add-roadmap" class="btn" onclick={() => appState.screen = 'creation'}><Icon name="plus" /> Add roadmap</button></div></div>
  {#if continuing.length}<section class="today-board" aria-labelledby="continue-heading"><div class="today-heading"><h2 id="continue-heading">Continue</h2></div>{#each continuing as c, i}<div class:primary={i === 0} class="continue-row"><span class="step-number">{i + 1}</span><div><span class="small">{c.title}</span><h3>{c.next!.title}</h3><p>{c.done} of {c.total} lessons complete{i === 0 ? ' · Your most recent roadmap' : ''}</p></div><a class={i ? 'link' : 'btn'} href={href(c.id, c.next!.id)}>{i ? 'Resume' : 'Continue learning'} <Icon name="arrow" /></a></div>{/each}</section>{:else}<p class="nothing-due">No lessons waiting. Revisit a finished roadmap or start something new.</p>{/if}
  {#if appState.review.count}<section class="review-entry"><div><h2>{appState.review.count} question{appState.review.count === 1 ? '' : 's'} to review today</h2><p>Bring back what you’ve learned, a question at a time.</p></div><a class="btn secondary" href="#/review">Start review <Icon name="arrow" /></a></section>{:else}<p class="nothing-due">Nothing due for review today.</p>{/if}
  <div class="library-title"><h2>Your library</h2><span>{active.length} roadmap{active.length === 1 ? '' : 's'}</span></div>
  {#if active.length}<div class="cards">{#each active as c (c.id)}<LibraryCard course={c} />{/each}</div>{:else}<p class="muted">Your roadmaps are paused. Resume one below when you’re ready.</p>{/if}
  {#if paused.length}<div class="library-title"><h2>Paused</h2><span>Ready whenever you are</span></div><div class="cards paused-cards">{#each paused as c (c.id)}<LibraryCard course={c} />{/each}</div>{/if}
  <p class="library-caption"><Icon name="Do it yourself" /> Made for learning by doing. Start small. Keep going.</p>
</div>{/if}
