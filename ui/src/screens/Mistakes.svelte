<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { message } from '../lib/state.svelte';
  import { href } from '../lib/format';
  import type { Mistake } from '../lib/types';
  import Icon from '../components/Icon.svelte';
  import Formatted from '../components/Formatted.svelte';
  let items = $state<Mistake[]>([]), loading = $state(true), error = $state('');
  let open = $derived(items.filter(m => !m.fixed)), fixed = $derived(items.filter(m => m.fixed));
  const times = (n: number) => n + ' time' + (n === 1 ? '' : 's');
  onMount(() => { let alive = true; api<{ items: Mistake[] }>('GET', 'api/mistakes').then(r => { if (alive) items = r.items; }).catch(e => { if (alive) error = message(e); }).finally(() => { if (alive) loading = false; }); return () => { alive = false; }; });
</script>
{#snippet row(m: Mistake)}
  <section class="mistake" class:fixed={m.kind !== 'code' && !!m.fixed}><p class="small">{m.roadmap} · {m.lesson_title}{m.kind !== 'code' ? ' · missed ' + times(m.n) : ''}</p>
    {#if m.kind === 'code'}<h3>Coding exercise: {m.title || 'not passed yet'}</h3><p>The tests have not passed yet, after {times(m.n)}.</p><a href={href(m.course, m.lesson)}>Open the lesson</a>
    {:else}<h3><Formatted text={m.q} /></h3>{#if m.options[m.picked] !== undefined}<p class="picked">You answered: {m.options[m.picked]}</p>{/if}<details><summary>Show the answer</summary><p><strong>{m.options[m.answer]}</strong></p><p><Formatted text={m.why} /></p></details><p style="margin-bottom:0"><a href={href(m.course, m.lesson)}>Open the lesson</a></p>{/if}
  </section>
{/snippet}
<div class="wrap narrow"><div class="intro"><div><div class="kicker"><Icon name="Practise" /> What tripped you up</div><h1>Mistakes notebook</h1></div><a href="#/">Back to Today</a></div>
  {#if loading}<div class="loading" role="status">Opening mistakes…</div>{:else if error}<p class="err" role="alert">{error}</p>{:else if !items.length}<div class="note"><h2>Nothing here yet.</h2><p>Questions you get wrong in a quiz, a stage exam or a review are kept here, with exercises whose tests have not passed.</p></div>
  {:else}{#if open.length}<p class="muted">{open.length} still open. Each one comes back in your daily review. Answer it right there and it moves to Fixed.</p>{/if}{#each open as m}{@render row(m)}{/each}{#if fixed.length}<div class="library-title"><h2>Fixed</h2><span>{fixed.length}</span></div>{#each fixed as m}{@render row(m)}{/each}{/if}{/if}
</div>
