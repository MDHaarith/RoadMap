<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { message, openIdeas } from '../lib/state.svelte';
  import { href, marks, SEARCH_GROUPS } from '../lib/format';
  import type { SearchHit } from '../lib/types';
  import Icon from '../components/Icon.svelte';
  import Waiting from '../components/Waiting.svelte';
  import LoadError from '../components/LoadError.svelte';
  let q = $state(''), results = $state<SearchHit[]>([]), loading = $state(true), error = $state('');
  let groups = $derived(SEARCH_GROUPS.map(([kind, name]) => ({ name, items: results.filter(r => r.kind === kind) })).filter(g => g.items.length));
  let alive = true;
  async function search() {
    loading = true; error = '';
    try { q = decodeURIComponent(location.hash.split('/')[2] || ''); const r = q ? (await api<{ results: SearchHit[] }>('POST', 'api/search', { q })).results : []; if (alive) results = r; }
    catch (err) { if (alive) error = message(err); } finally { if (alive) loading = false; }
  }
  onMount(() => { void search(); return () => { alive = false; }; });
</script>
<div class="wrap screen-enter" class:narrow={!!error || (!loading && !results.length)}><a class="back-map" href="#/"><Icon name="back" /> Your library</a><div class="page-head"><h1>Search</h1>{#if !loading && !error}<span class="session-count">{results.length}{results.length >= 40 ? '+' : ''} result{results.length === 1 ? '' : 's'}</span>{/if}</div>
  {#if loading}<Waiting skeleton="rows" inline title="Searching" />{:else if error}<LoadError what="The search could not run." {error} retry={search} />{:else if !results.length}<div class="empty-state"><Icon name="empty-route" /><h2>{q ? `Nothing found for “${q}”.` : 'Type something in the search box.'}</h2><p>Every word must appear in the same lesson, note, chat message or idea.</p></div>
  {:else}<p class="small result-count">Matches for “{q}”</p>{#each groups as g}<div class="section-title"><h2>{g.name}</h2><span>{g.items.length}</span></div><div class="result-list stagger">{#each g.items as x}{#if x.kind === 'idea'}<button class="result" onclick={() => openIdeas(true)}><p>{#each marks(x.snippet, q) as m}{#if m.m}<mark>{m.t}</mark>{:else}{m.t}{/if}{/each}</p></button>{:else}<a class="result" href={href(x.course, x.lesson)}><span class="result-head"><b>{x.title}</b> <span class="small">{x.roadmap}</span></span><p>{#each marks(x.snippet, q) as m}{#if m.m}<mark>{m.t}</mark>{:else}{m.t}{/if}{/each}</p></a>{/if}{/each}</div>{/each}{/if}
</div>
