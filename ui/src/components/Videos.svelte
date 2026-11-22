<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { appState, toast, message, ask } from '../lib/state.svelte';
  import { enc, clock } from '../lib/format';
  import type { Course, Lesson, Video } from '../lib/types';
  import Icon from './Icon.svelte';
  import Waiting from './Waiting.svelte';
  import BusyLabel from './BusyLabel.svelte';
  import LoadError from './LoadError.svelte';
  import Formatted from './Formatted.svelte';
  let { c, l, seek }: { c: Course; l: Lesson; seek?: { k: number; at: number } } = $props();
  let loading = $state(true), error = $state(''), selected = $state(0), playing = $state(false), start = $state(0);
  let searching = $state(false), query = $state(''), searchError = $state(''), refreshing = $state(false);
  let v = $derived(l.videos?.[selected]);
  // Concepts no saved video teaches: only known when the videos were checked for coverage.
  let uncovered = $derived(l.videos?.some(x => x.covers?.length) ? (l.concepts || []).filter(k => !l.videos!.some(x => x.covers?.includes(k))) : []);
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
  let lessonParts = $state(''), guided = $state(false), partDone = $state(false);
  function fullVideo(context = false) {
    guided = false; partDone = false;
    frame?.contentWindow?.postMessage({ roadmapCommand: context ? 'context' : 'full' }, PLAYER);
  }
  let running = (false);
  // "Ask about this moment" and "I'm missing a step": the tutor already has this video's timed transcript, so the time is enough.
  let timeWaiter: ((t: number) => void) | undefined;
  let asking = $state<'' | 'moment' | 'missing'>(''), askText = $state('');
  function playerTime() {
    if (!playing || playError || !frame?.contentWindow) return Promise.resolve(start);
    return new Promise<number>(done => {
      timeWaiter = done; setTimeout(() => { if (timeWaiter === done) { timeWaiter = undefined; done(start); } }, 800);
      frame!.contentWindow!.postMessage({ roadmapCommand: 'time' }, PLAYER);
    });
  }
  async function askMoment() {
    if (!v || appState.busyKey) return;
    const at = clock(Math.floor(await playerTime())), q = askText.trim();
    command('pause');
    const what = asking === 'missing'
      ? `I'm missing a step at ${at} in video ${selected + 1} (${v.name || v.title}).${q ? ' What I don\'t get: ' + q : ''} Find what I missed in the transcript before that point and explain just that step. If an earlier part of a lesson video covers it, point me to it as "video N at m:ss".`
      : `About video ${selected + 1} (${v.name || v.title}) at ${at}: ${q || 'Explain what is happening at this moment and why it matters for this lesson.'}`;
    if (await ask(what)) { asking = ''; askText = ''; }
  }
  function playerMessage(e: MessageEvent) {
    if (e.origin !== PLAYER || e.source !== frame?.contentWindow) return;
    if (e.data?.roadmapPartDone) { partDone = true; guided = false; }
    if (e.data?.roadmapFull) guided = false;
    if (typeof e.data?.roadmapTime === 'number') { timeWaiter?.(e.data.roadmapTime); timeWaiter = undefined; }
    const code = e.data?.roadmapPlayer, on = e.data?.roadmapPlaying;
    if (typeof code === 'number') playError = reasons[code] || 'YouTube could not play this video here.';
    if (typeof on === 'boolean') { running = on; if (navigator.mediaSession) navigator.mediaSession.playbackState = on ? 'playing' : 'paused'; }
  }
  // Media keys and the desktop's media controls (MPRIS on Linux) reach the player through these.
  function command(cmd: 'play' | 'pause' | 'back' | 'forward') {
    if (playing && !playError && frame?.contentWindow) frame.contentWindow.postMessage({ roadmapCommand: cmd }, PLAYER);
    else if (cmd === 'play' && v) playVideo(selected, v.start || 0);
  }
  let lastTrack = 0;
  function track(step: number) {
    const k = selected + step;
    // The desktop may deliver one key press both as a key event and as a media-session action.
    if (Date.now() - lastTrack < 400 || !l.videos?.[k]) return;
    lastTrack = Date.now(); playVideo(k, l.videos[k].start || 0);
  }
  function mediaKey(e: KeyboardEvent) {
    if (!v) return;
    const act: Record<string, () => void> = {
      MediaPlayPause: () => command(playing && running ? 'pause' : 'play'), MediaPlay: () => command('play'), MediaPause: () => command('pause'),
      MediaRewind: () => command('back'), MediaFastForward: () => command('forward'),
      MediaTrackPrevious: () => track(-1), MediaTrackNext: () => track(1)
    };
    if (act[e.key]) { e.preventDefault(); act[e.key](); }
  }
  $effect(() => {
    const ms = navigator.mediaSession;
    if (!ms || !v || !playing) return;
    if (typeof MediaMetadata !== 'undefined') ms.metadata = new MediaMetadata({ title: v.name || v.title, artist: v.channel, album: l.title, artwork: [{ src: 'https://i.ytimg.com/vi/' + enc(v.id) + '/hqdefault.jpg', sizes: '480x360', type: 'image/jpeg' }] });
    const acts: [MediaSessionAction, MediaSessionActionHandler][] = [
      ['play', () => command('play')], ['pause', () => command('pause')], ['seekbackward', () => command('back')], ['seekforward', () => command('forward')],
      ['previoustrack', () => track(-1)], ['nexttrack', () => track(1)]
    ];
    for (const [a, f] of acts) try { ms.setActionHandler(a, f); } catch { /* action not supported here */ }
    return () => { for (const [a] of acts) try { ms.setActionHandler(a, null); } catch { /* not supported */ } ms.metadata = null; ms.playbackState = 'none'; };
  });
  let applied: typeof seek;
  $effect(() => {
    if (!loading && seek && seek !== applied && l.videos?.[seek.k]) {
      applied = seek; jump(seek.k, seek.at);
      document.getElementById('player')?.scrollIntoView({ block: 'center' });
    }
  });
  onMount(() => { void loadVideos(); return () => { alive = false; }; });
  async function loadVideos() {
    loading = true; error = '';
    try { if (!l.videos?.length) l.videos = await api<Video[]>('POST', 'api/courses/' + enc(c.id) + '/videos', { lesson: l.id }); void nameVideos(); }
    catch (err) { if (alive) error = message(err); } finally { loading = false; }
  }
  // Videos picked before names existed get one in the course's terms; until then the YouTube title shows.
  async function nameVideos() {
    if (!l.videos?.some(v => !v.name)) return;
    try { const named = await api<Video[]>('POST', 'api/courses/' + enc(c.id) + '/video-names', { lesson: l.id }); if (alive) for (const v of l.videos || []) v.name ||= named.find(n => n.id === v.id)?.name; } catch { /* the YouTube titles stay */ }
  }
  let strip = $state<HTMLDivElement>(), canLeft = $state(false), canRight = $state(false);
  function measure() { if (strip) { canLeft = strip.scrollLeft > 1; canRight = strip.scrollLeft + strip.clientWidth < strip.scrollWidth - 1; } }
  function watchStrip(node: HTMLDivElement) {
    strip = node; measure();
    const ro = new ResizeObserver(measure); ro.observe(node);
    const mo = new MutationObserver(() => { node.scrollLeft = 0; measure(); }); mo.observe(node, { childList: true, subtree: true });
    return { destroy() { ro.disconnect(); mo.disconnect(); strip = undefined; canLeft = canRight = false; } };
  }
  function showVideo(k: number) { guided = false; partDone = false; lessonParts = ''; selected = k; playing = false; start = 0; if (PLAYER) playError = ''; }
  // The player draws captions itself; they are fetched once per video and handed over when its frame loads.
  const captionCache: Record<string, Promise<unknown[]>> = {};
  async function sendCaptions(id: string) {
    const cues = await (captionCache[id] ||= api<unknown[]>('GET', 'api/captions/' + enc(id)).catch(() => { delete captionCache[id]; return []; }));
    if (alive && v?.id === id) frame?.contentWindow?.postMessage({ roadmapCaptions: cues }, PLAYER);
  }
  // Same video already playing: move it in place (player.js listens), so the timestamp works without a reload.
  function jump(k: number, at: number) {
    if (playing && !playError && k === selected && frame?.contentWindow) frame.contentWindow.postMessage({ roadmapSeek: at }, PLAYER);
    else playVideo(k, at, false);
  }
  function playVideo(k: number, at = 0, lessonMode = true) {
    const parts = lessonMode ? l.videos?.[k]?.segments || [] : [];
    lessonParts = parts.map(s => s.start + '-' + s.end).join(',');
    guided = parts.length > 0; partDone = false;
    if (guided) at = parts[0].start;
     if (PLAYER && k !== selected) playError = ''; selected = k; start = at; playing = true; }
  async function freshVideos(plan = false) {
    refreshing = true;
    try { l.videos = await api<Video[]>('POST', 'api/courses/' + enc(c.id) + '/videos', { lesson: l.id, fresh: !plan, plan }); showVideo(0); }
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
<svelte:window onmessage={playerMessage} onkeydown={mediaKey} />
<div id="videos">
  {#if loading}<Waiting skeleton="videos" title="Finding videos for this lesson" hint="this usually takes a few seconds" />
  {:else if error}<LoadError id="retry-videos" what="Videos for this lesson could not load." {error} retry={loadVideos} /><p class="small"><a href={`https://www.youtube.com/results?search_query=${enc(l.query || l.title)}`}>Search YouTube for this lesson</a></p>
  {:else}
    <div class="video-layout"><div class="video-main">
    {#if v}<div class="player-space"><div class="player" id="player">{#if playing && playError}<div class="player-placeholder player-error" role="alert"><p>{playError}</p><a href={`https://www.youtube.com/watch?v=${enc(v.id)}`}>Open it on YouTube</a></div>{:else if playing}<iframe bind:this={frame} onload={() => void sendCaptions(v.id)} src={`${PLAYER}/player.html?v=${enc(v.id)}&start=${start | 0}&parts=${enc(lessonParts)}${c.saved_videos?.includes(v.id) ? '&local=1' : ''}`} title={v.title} allowfullscreen allow="autoplay; encrypted-media; picture-in-picture; fullscreen"></iframe>{:else}<img class="player-thumbnail" src={`https://i.ytimg.com/vi/${enc(v.id)}/hqdefault.jpg`} alt="" /><div class="player-placeholder"><button class="btn" id="play-video" aria-label={`Play ${v.title}`} onclick={() => playVideo(selected, v.start || 0)}><Icon name="Watch" /> {v.segments?.length ? 'Play lesson parts' : 'Play video'}{v.start ? ` from ${clock(v.start)}` : ''}</button></div>{/if}</div></div>
      <p class="video-title"><b>{v.name || v.title}</b><span class="small">{#if v.name}On YouTube: {v.title} · {/if}{v.channel}{v.duration ? ' · ' + Math.floor(v.duration / 60) + ' min' : ''} · <a href={`https://www.youtube.com/watch?v=${enc(v.id)}`} title="Use this if the video won’t play here">Open on YouTube</a></span></p>
      {#if v.segments?.length}<p class="small">{v.segments.map(s => clock(s.start) + '–' + clock(s.end) + ' ' + s.title).join(' · ')}</p>{/if}
      {#if v.action}<p><strong>Try it</strong> <Formatted text={v.action} /></p>{/if}
      {#if guided || partDone || (playing && !playError)}<div class="row video-help">{#if partDone}<span role="status">Lesson parts complete.</span>{/if}{#if guided || partDone}<button type="button" class="btn secondary sm" onclick={() => fullVideo()}>{partDone ? 'Keep watching' : 'Watch full video'}</button><button type="button" class="btn ghost sm" onclick={() => fullVideo(true)}>Rewind 1 min</button>{/if}{#if playing && !playError}<button type="button" class="btn secondary sm" aria-expanded={asking === 'missing'} onclick={() => asking = asking === 'missing' ? '' : 'missing'}>I’m missing a step</button><button type="button" class="btn secondary sm" aria-expanded={asking === 'moment'} onclick={() => asking = asking === 'moment' ? '' : 'moment'}>Ask about this moment</button>{/if}</div>{/if}
      {#if asking && playing}<form class="row video-ask" onsubmit={e => { e.preventDefault(); void askMoment(); }}><label class="sr" for="video-ask">{asking === 'missing' ? 'What are you missing?' : 'Your question about this moment'}</label><input id="video-ask" maxlength="600" bind:value={askText} placeholder={asking === 'missing' ? 'What lost you? (optional)' : 'What do you want to know? (optional)'} /><button class="btn sm" disabled={!!appState.busyKey}>Ask the tutor</button></form>{/if}
      <!-- One row that scrolls sideways: any number of parts costs the player the same single line. -->
      {#if v.chapters?.length}<div class="chapter-strip" class:fade-left={canLeft} class:fade-right={canRight}>{#if canLeft || canRight}<button type="button" class="strip-arrow" aria-label="Earlier parts" disabled={!canLeft} onclick={() => strip?.scrollBy({ left: -strip.clientWidth * 0.8, behavior: 'smooth' })}><Icon name="back" /></button>{/if}<div class="chapters" role="group" aria-label="Jump to a part of this video" use:watchStrip onscroll={measure}>{#each v.chapters as m}<button type="button" onclick={() => jump(selected, m.t)}><span>{clock(m.t)}</span>{m.title}</button>{/each}</div>{#if canLeft || canRight}<button type="button" class="strip-arrow" aria-label="Later parts" disabled={!canRight} onclick={() => strip?.scrollBy({ left: strip.clientWidth * 0.8, behavior: 'smooth' })}><Icon name="arrow" /></button>{/if}</div>{/if}
    {:else}<p>{l.novideos ? 'No YouTube video fits this lesson well. Try your own search below.' : 'No videos found. Try a topic search.'}</p>{/if}
    </div><div class="video-choices">
    {#if refreshing || searching}<Waiting inline skeleton="rows" title="Finding videos" />
    {:else if v}
      <div class="vlist">{#each l.videos || [] as x, j}<button class="vid" aria-pressed={j === selected} onclick={() => showVideo(j)}><span class="video-number" aria-hidden="true">{j === selected ? '▶' : String(j + 1).padStart(2, '0')}</span><span><b title={x.name ? x.title : undefined}>{x.name || x.title}</b>{#if x.reference}<span class="tag">Extra reference</span>{/if}{#if x.searched}<span class="tag">Your search</span>{/if}<small>{x.channel}{x.duration ? ' · ' + Math.floor(x.duration / 60) + ' min' : ''}</small>{#if x.why}<small class="why"><Formatted text={x.why} /></small>{/if}{#if x.covers?.length}<small class="covers">Covers: <Formatted text={x.covers.join(" · ")} /></small>{/if}</span>{#if j === selected}<span class="playing-label">{playing && !playError ? 'Playing' : 'Selected'}</span>{/if}</button>{/each}</div>
      {#if uncovered.length}<p class="small uncovered" title={uncovered.join('\n\n')}>{uncovered.length === 1 ? 'One key idea of this lesson is' : `${uncovered.length} key ideas of this lesson are`} not in these videos. Search below for {uncovered.length > 1 ? 'them' : 'it'}.</p>{/if}
    {/if}
    <div class="video-tools">{#if v}<button class="btn secondary sm" type="button" disabled={refreshing || searching} onclick={() => freshVideos(true)}>Plan these videos for my lesson</button>{/if}<h3>Want a different video?</h3><button class="btn secondary sm" type="button" id="fresh-videos" disabled={refreshing} onclick={() => freshVideos()}><BusyLabel busy={refreshing} idle="Find more for this lesson" label="Finding videos…" /></button><a href={`https://www.youtube.com/results?search_query=${enc(l.query || l.title)}`}>Browse this lesson on YouTube</a></div>
    <form id="video-search" onsubmit={e => { e.preventDefault(); void searchVideos(); }}><label for="video-query">Search any topic</label><input id="video-query" placeholder="Enter a topic" maxlength="200" required bind:value={query} /><button class="btn secondary" disabled={searching}><BusyLabel busy={searching} idle="Find videos" label="Searching…" /></button><div id="video-error" class="err" hidden={!searchError}>{searchError}</div></form>
    </div></div>
  {/if}
</div>

<style>.covers{display:block;color:var(--muted)}.uncovered{margin:6px 0;color:var(--muted)}
  /* Summaries stay short in the list; the chosen video shows a little more. */
  .why,.covers{display:-webkit-box;-webkit-box-orient:vertical;-webkit-line-clamp:2;line-clamp:2;overflow:hidden}
  .covers{-webkit-line-clamp:1;line-clamp:1}
  .vid[aria-pressed=true] .why{-webkit-line-clamp:4;line-clamp:4}</style>
