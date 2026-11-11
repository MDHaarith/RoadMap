<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { appState, toast, message } from '../lib/state.svelte';
  import { enc, clock } from '../lib/format';
  import type { Course, Lesson, Video } from '../lib/types';
  import Icon from './Icon.svelte';
  import Waiting from './Waiting.svelte';
  let { c, l, seek }: { c: Course; l: Lesson; seek?: { k: number; at: number } } = $props();
  let loading = $state(true), error = $state(''), selected = $state(0), playing = $state(false), start = $state(0);
  let searching = $state(false), query = $state(''), searchError = $state(''), refreshing = $state(false);
  let v = $derived(l.videos?.[selected]);
  $effect(() => { appState.openVideo = v && /^[\w-]{11}$/.test(v.id) ? v.id : undefined; return () => { appState.openVideo = undefined; }; });
  let alive = true;
  // The app serves the YouTube player from 127.0.0.1 (src/player.rs): YouTube refuses embeds from tauri:// pages.
  const PLAYER: string = (window as { __ROADMAP_PLAYER__?: string }).__ROADMAP_PLAYER__ || '';
  let frame = $state<HTMLIFrameElement>(), playError = $state(PLAYER ? '' : 'The video player could not start.');
  const reasons: Record<number, string> = {
    0: 'YouTube could not be reached. Check your internet connection.',
    2: 'This video link is not valid.',
    5: 'This video cannot play in the app’s player.',
    100: 'This video was removed or made private.',
    101: 'YouTube does not allow this video to play inside other apps; its owner may have turned that off.',
    150: 'YouTube does not allow this video to play inside other apps; its owner may have turned that off.'
  };
  function playerMessage(e: MessageEvent) {
    const code = e.data?.roadmapPlayer;
    if (e.origin === PLAYER && e.source === frame?.contentWindow && typeof code === 'number') playError = reasons[code] || 'YouTube could not play this video here.';
  }
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
  function showVideo(k: number) { selected = k; playing = false; start = 0; if (PLAYER) playError = ''; }
  function playVideo(k: number, at = 0) { if (PLAYER && k !== selected) playError = ''; selected = k; start = at; playing = true; }
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
<svelte:window onmessage={playerMessage} />
<div id="videos">
  {#if loading}<Waiting title="Finding free lessons on YouTube" hint="this usually takes a few seconds" />
  {:else if error}<p class="err">{error}</p><button class="btn secondary" id="retry-videos" onclick={loadVideos}>Try again</button><p class="small"><a href={`https://www.youtube.com/results?search_query=${enc(l.query || l.title)}`}>Search YouTube for this lesson</a></p>
  {:else}
    {#if v}<div class="player" id="player">{#if playing && playError}<div class="player-placeholder player-error" role="alert"><p>{playError}</p><a href={`https://www.youtube.com/watch?v=${enc(v.id)}`}>Open it on YouTube</a></div>{:else if playing}<iframe bind:this={frame} src={`${PLAYER}/player.html?v=${enc(v.id)}&start=${start | 0}`} title={v.title} allowfullscreen allow="autoplay; encrypted-media; picture-in-picture; fullscreen"></iframe>{:else}<img class="player-thumbnail" src={`https://i.ytimg.com/vi/${enc(v.id)}/hqdefault.jpg`} alt="" /><div class="player-placeholder"><button class="btn" id="play-video" aria-label={`Play ${v.title}`} onclick={() => playVideo(selected)}><Icon name="Watch" /> Play video</button></div>{/if}</div>
      <p class="video-title"><b>{v.title}</b><span class="small">{v.channel}{v.duration ? ' · ' + Math.floor(v.duration / 60) + ' min' : ''}</span></p>
      {#if v.chapters?.length}<div class="chapters" role="group" aria-label="Jump to a part of this video">{#each v.chapters as m}<button type="button" onclick={() => playVideo(selected, m.t)}><span>{clock(m.t)}</span> {m.title}</button>{/each}</div>{/if}
      <div class="vlist">{#each l.videos || [] as x, j}<button class="vid" aria-pressed={j === selected} onclick={() => showVideo(j)}><span class="video-number">{String(j + 1).padStart(2, '0')}</span><span><b>{x.title}</b>{#if x.searched}<span class="tag">Your search</span>{/if}<small>{x.channel}{x.duration ? ' · ' + Math.floor(x.duration / 60) + ' min' : ''}</small>{#if x.why}<small class="why">{x.why}</small>{/if}</span>{#if j === selected}<Icon name="Watch" />{/if}</button>{/each}</div>
      <div class="video-tools"><span>Video won’t play? <a href={`https://www.youtube.com/watch?v=${enc(v.id)}`}>Open it on YouTube</a> · <a href={`https://www.youtube.com/results?search_query=${enc(l.query || l.title)}`}>More results</a></span><button class="btn secondary sm" type="button" id="fresh-videos" disabled={refreshing} onclick={freshVideos}>{refreshing ? 'Searching and choosing…' : 'Find other videos'}</button></div>
    {:else}<p>No videos found yet.</p><p class="small"><a href={`https://www.youtube.com/results?search_query=${enc(l.query || l.title)}`}>Search YouTube for this lesson</a></p>{/if}
    <form id="video-search" onsubmit={e => { e.preventDefault(); void searchVideos(); }}><label for="video-query">Video topic</label><input id="video-query" placeholder="Search for a video on any topic" maxlength="200" required bind:value={query} /><button class="btn secondary" disabled={searching}>{searching ? 'Searching…' : 'Search'}</button><div id="video-error" class="err" hidden={!searchError}>{searchError}</div></form>
  {/if}
</div>
