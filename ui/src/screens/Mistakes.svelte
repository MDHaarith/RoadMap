<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { appState, message } from '../lib/state.svelte';
  import { href } from '../lib/format';
  import type { Mistake } from '../lib/types';
  import Icon from '../components/Icon.svelte';
  import Formatted from '../components/Formatted.svelte';
  import Waiting from '../components/Waiting.svelte';
  import LoadError from '../components/LoadError.svelte';
  import Select from '../components/Select.svelte';
  let items = $state<Mistake[]>([]), loading = $state(true), error = $state('');
  // One notebook per roadmap: #/mistakes/<roadmap id>, else the roadmap open last, else the first with mistakes.
  let chosen = $state(decodeURIComponent(location.hash.split('/')[2] || ''));
  let roadmaps = $derived([...new Map(items.map(m => [m.course, m.roadmap])).entries()].map(([value, title]) => {
    const n = items.filter(m => m.course === value && !m.fixed).length;
    return { value, label: `${title} · ${n} open` };
  }));
  let course = $derived(roadmaps.find(r => r.value === chosen)?.value ?? roadmaps.find(r => r.value === appState.course?.id)?.value ?? roadmaps[0]?.value ?? '');
  let mine = $derived(items.filter(m => m.course === course));
  let open = $derived(mine.filter(m => !m.fixed)), fixed = $derived(mine.filter(m => m.fixed));
  const times = (n: number) => n + ' time' + (n === 1 ? '' : 's');
  let alive = true;
  function load() { loading = true; error = ''; api<{ items: Mistake[] }>('GET', 'api/mistakes').then(r => { if (alive) items = r.items; }).catch(e => { if (alive) error = message(e); }).finally(() => { if (alive) loading = false; }); }
  // replaceState: switching roadmaps keeps the page instead of reloading it through the router.
  function pick(v: string) { chosen = v; history.replaceState(history.state,'', '#/mistakes/' + encodeURIComponent(v)); }
  onMount(() => { load(); return () => { alive = false; }; });
</script>
{#snippet row(m: Mistake)}
  <section class="mistake" class:fixed={m.kind !== 'code' && !!m.fixed}><p class="small">{m.lesson_title}{m.kind !== 'code' ? ' · missed ' + times(m.n) : ''}</p>
    {#if m.kind === 'code'}<h3>Coding exercise: {m.title || 'not passed yet'}</h3><div class="mistake-foot"><p class="picked">The tests have not passed yet, after {times(m.n)}.</p><a class="mistake-link" href={href(m.course, m.lesson)}>Open the lesson <Icon name="arrow" /></a></div>
    {:else}<h3><Formatted text={m.q} /></h3>{#if m.options[m.picked] !== undefined}<p class="picked">You answered: {m.options[m.picked]}</p>{/if}<div class="mistake-foot"><details class="disclosure"><summary>Show the answer</summary><div class="answer"><p><strong>{m.options[m.answer]}</strong></p><p><Formatted text={m.why} /></p></div></details><a class="mistake-link" href={href(m.course, m.lesson)}>Open the lesson <Icon name="arrow" /></a></div>{/if}
  </section>
{/snippet}
<div class="wrap screen-enter" class:narrow={!!error || (!loading && !items.length)}><a class="back-map" href="#/"><Icon name="back" /> Back</a><div class="page-head"><h1>Mistakes notebook</h1>{#if !loading && !error && items.length}<div class="row">{#if roadmaps.length > 1}<Select id="mistakes-roadmap" ariaLabel="Roadmap" value={course} options={roadmaps} onchange={pick} />{:else}<span class="session-count">{open.length} open</span>{/if}</div>{/if}</div>
  {#if loading}<Waiting skeleton="rows" inline title="Opening mistakes" />{:else if error}<LoadError what="Your mistakes notebook could not load." {error} retry={load} />{:else if !items.length}<div class="empty-state"><Icon name="empty-route" /><h2>Nothing here yet.</h2><p>Missed quiz, exam and review questions stay here, along with unfinished coding exercises.</p></div>
  {:else}{#if roadmaps.length === 1}<p class="small">{items[0].roadmap}</p>{/if}{#if open.length}<p class="muted lead">Each one comes back in your daily review. Answer it right there and it moves to Fixed.</p>{/if}{#key course}<div class="mistakes-list stagger">{#each open as m}{@render row(m)}{/each}</div>{#if fixed.length}<div class="section-title"><h2>Fixed</h2><span>{fixed.length}</span></div><div class="mistakes-list stagger">{#each fixed as m}{@render row(m)}{/each}</div>{/if}{/key}{/if}
</div>
