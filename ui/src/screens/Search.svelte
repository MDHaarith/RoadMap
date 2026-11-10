<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { message, openIdeas } from '../lib/state.svelte';
  import { href } from '../lib/format';
  import Icon from '../components/Icon.svelte';
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
<div class="wrap narrow"><div class="intro"><div><div class="kicker"><Icon name="Look it up yourself" /> Lessons, notes, tutor chats and ideas</div><h1>Search</h1></div><a href="#/">Back to Today</a></div>
  {#if loading}<div class="loading" role="status">Searching…</div>{:else if error}<p class="err" role="alert">{error}</p>{:else if !results.length}<div class="note"><h2>{q ? `Nothing found for “${q}”.` : 'Type something in the search box.'}</h2><p>Every word has to appear in the same lesson, note, chat message or idea.</p></div>
  {:else}<p class="muted">{results.length}{results.length >= 40 ? '+' : ''} result{results.length === 1 ? '' : 's'} for “{q}”</p>{#each results as x}{#if x.kind === 'idea'}<button class="result" onclick={() => openIdeas(true)}><span class="tag">Idea</span><p>{x.snippet}</p></button>{:else}<a class="result" href={href(x.course, x.lesson)}><span class="tag">{kinds[x.kind]}</span> <b>{x.title}</b> <span class="small">{x.roadmap}</span><p>{x.snippet}</p></a>{/if}{/each}{/if}
</div>
