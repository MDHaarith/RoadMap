<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { toast, message } from '../lib/state.svelte';
  import { enc, clock } from '../lib/format';
  import type { Course, Lesson, Video } from '../lib/types';
  import Icon from './Icon.svelte';
  let { c, l, seek }: { c: Course; l: Lesson; seek?: { k: number; at: number } } = $props();
  let loading = $state(true), error = $state(''), selected = $state(0), playing = $state(false), start = $state(0);
  let searching = $state(false), query = $state(''), searchError = $state(''), refreshing = $state(false);
  let v = $derived(l.videos?.[selected]);
  let alive = true;
  let applied: typeof seek;
  $effect(() => {
    if (!loading && seek && seek !== applied && l.videos?.[seek.k]) {
      applied = seek; selected = seek.k; start = seek.at; playing = true;
      document.getElementById('player')?.scrollIntoView({ block: 'center' });
    }
  });
  onMount(() => { void loadVideos(); return () => { alive = false; }; });
  async function loadVideos() {
    loading = true; error = '';
    try { if (!l.videos?.length) l.videos = await api<Video[]>('POST', 'api/courses/' + enc(c.id) + '/videos', { lesson: l.id }); }
    catch (err) { if (alive) error = message(err); } finally { loading = false; }
  }
  function showVideo(k: number) { selected = k; playing = false; start = 0; }
  function playVideo(k: number, at = 0) { selected = k; start = at; playing = true; }
  async function freshVideos() {
    refreshing = true;
    try { l.videos = await api<Video[]>('POST', 'api/courses/' + enc(c.id) + '/videos', { lesson: l.id, fresh: true }); showVideo(0); }
    catch (err) { toast(message(err)); } finally { refreshing = false; }
  }
  async function searchVideos() {
    searching = true; searchError = '';
    try {
      const had = (l.videos || []).map(v => v.id);
      l.videos = await api<Video[]>('POST', 'api/courses/' + enc(c.id) + '/videos', { lesson: l.id, query });
      showVideo(Math.max(0, l.videos.findIndex(v => !had.includes(v.id))));
    } catch (err) { searchError = message(err); } finally { searching = false; }
  }
</script>
<div id="videos">
  {#if loading}<div class="loading" role="status">Finding free lessons on YouTube…</div>
  {:else if error}<p class="err">{error}</p><button class="btn secondary" id="retry-videos" onclick={loadVideos}>Try again</button><p class="small"><a href={`https://www.youtube.com/results?search_query=${enc(l.query || '')}`}>Search YouTube for this lesson</a></p>
  {:else}
    {#if v}<div class="player" id="player">{#if playing}<iframe src={`https://www.youtube.com/embed/${enc(v.id)}?autoplay=1&start=${start | 0}`} title={v.title} allowfullscreen allow="autoplay; encrypted-media; picture-in-picture; fullscreen" referrerpolicy="strict-origin-when-cross-origin"></iframe>{:else}<img class="player-thumbnail" src={`https://i.ytimg.com/vi/${enc(v.id)}/hqdefault.jpg`} alt="" /><div class="player-placeholder"><button class="btn" id="play-video" aria-label={`Play ${v.title}`} onclick={() => playVideo(selected)}><Icon name="Watch" /> Play video</button></div>{/if}</div>
      {#if v.chapters?.length}<div class="chapters" role="group" aria-label="Jump to a part of this video">{#each v.chapters as m}<button type="button" onclick={() => playVideo(selected, m.t)}><span>{clock(m.t)}</span> {m.title}</button>{/each}</div>{/if}
      <div class="vlist">{#each l.videos || [] as x, j}<button class="vid" aria-pressed={j === selected} onclick={() => showVideo(j)}><span class="video-number">{String(j + 1).padStart(2, '0')}</span><span><b>{x.title}</b>{#if x.searched}<span class="tag">Your search</span>{/if}<small>{x.channel}{x.duration ? ' · ' + Math.floor(x.duration / 60) + ' min' : ''}</small>{#if x.why}<small class="why">{x.why}</small>{/if}</span>{#if j === selected}<Icon name="Watch" />{/if}</button>{/each}</div>
      <p class="small" style="margin-bottom:0">Video won’t play? <a href={`https://www.youtube.com/watch?v=${enc(v.id)}`}>Open it on YouTube</a> · <a href={`https://www.youtube.com/results?search_query=${enc(l.query || '')}`}>More results</a></p>
      <button class="btn secondary" type="button" id="fresh-videos" style="margin-top:12px" disabled={refreshing} onclick={freshVideos}>{refreshing ? 'Searching and choosing…' : 'Find other videos'}</button>
    {:else}<p>No videos found yet.</p><p class="small"><a href={`https://www.youtube.com/results?search_query=${enc(l.query || '')}`}>Search YouTube for this lesson</a></p>{/if}
    <form id="video-search" onsubmit={e => { e.preventDefault(); void searchVideos(); }}><label for="video-query">Video topic</label><input id="video-query" placeholder="Search for a video on any topic" maxlength="200" required bind:value={query} /><button class="btn secondary" disabled={searching}>{searching ? 'Searching…' : 'Search'}</button><div id="video-error" class="err" hidden={!searchError}>{searchError}</div></form>
  {/if}
</div>
