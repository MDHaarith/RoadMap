<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { message, openIdeas } from '../lib/state.svelte';
  import { href } from '../lib/format';
  import Icon from '../components/Icon.svelte';
  import Waiting from '../components/Waiting.svelte';
  interface Hit { kind: string; snippet: string; course: string; lesson: string; title: string; roadmap: string }
  let q = $state(''), results = $state<Hit[]>([]), loading = $state(true), error = $state('');
  const kinds: Record<string, string> = { lesson: 'Lesson', note: 'My notes', chat: 'Tutor chat' };
  onMount(() => {
    let alive = true;
    async function search() {
      try { q = decodeURIComponent(location.hash.split('/')[2] || ''); const r = q ? (await api<{ results: Hit[] }>('POST', 'api/search', { q })).results : []; if (alive) results = r; }
      catch (err) { if (alive) error = message(err); } finally { if (alive) loading = false; }
    }
    void search(); return () => { alive = false; };
  });
</script>
<div class="wrap screen-enter" class:narrow={!!error || (!loading && !results.length)}><a class="back-map" href="#/"><Icon name="back" /> Your library</a><div class="page-head"><h1>Search</h1>{#if !loading && !error}<span class="session-count">{results.length}{results.length >= 40 ? '+' : ''} result{results.length === 1 ? '' : 's'}</span>{/if}</div>
  {#if loading}<Waiting skeleton="rows" inline title="Searching" />{:else if error}<p class="err" role="alert">{error}</p>{:else if !results.length}<div class="empty-state"><Icon name="empty-route" /><h2>{q ? `Nothing found for “${q}”.` : 'Type something in the search box.'}</h2><p>Every word must appear in the same lesson, note, chat message or idea.</p></div>
  {:else}<p class="small result-count">Matches for “{q}”</p><div class="result-list stagger">{#each results as x}{#if x.kind === 'idea'}<button class="result" onclick={() => openIdeas(true)}><span class="result-head"><span class="tag">Idea</span></span><p>{x.snippet}</p></button>{:else}<a class="result" href={href(x.course, x.lesson)}><span class="result-head"><span class="tag" class:accent={x.kind === 'lesson'}>{kinds[x.kind]}</span> <b>{x.title}</b> <span class="small">{x.roadmap}</span></span><p>{x.snippet}</p></a>{/if}{/each}</div>{/if}
</div>
