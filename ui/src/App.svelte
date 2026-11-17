<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { api } from './lib/api';
  import { appState, route, toast, message, dispose, flushSave } from './lib/state.svelte';
  import { href } from './lib/format';
  import type { Settings } from './lib/types';
  import { updates, startUpdates } from './lib/updates.svelte';
  import { watchScrollRegions } from './lib/scroll';
  import Icon from './components/Icon.svelte';
  import Waiting from './components/Waiting.svelte';
  import LoadError from './components/LoadError.svelte';
  import UpdateStatus from './components/UpdateStatus.svelte';
  import TopBar from './components/TopBar.svelte';
  import Ideas from './components/Ideas.svelte';
  import Confirm from './components/Confirm.svelte';
  import Library from './screens/Library.svelte';
  import Creation from './screens/Creation.svelte';
  import Overview from './screens/Overview.svelte';
  import Lesson from './screens/Lesson.svelte';
  import Exam from './screens/Exam.svelte';
  import Review from './screens/Review.svelte';
  import SettingsScreen from './screens/Settings.svelte';
  import Search from './screens/Search.svelte';
  import Mistakes from './screens/Mistakes.svelte';
  import Import from './screens/Import.svelte';
  function toastMotion(node: Element) {
    const styles = getComputedStyle(node);
    return {
      duration: matchMedia('(prefers-reduced-motion: reduce)').matches ? 0 : parseFloat(styles.getPropertyValue('--motion-enter')),
      css: (t: number) => `opacity: ${t}; transform: translate(-50%, ${(t - 1) * 5}px)`
    };
  }
  onMount(() => {
    const stopUpdates = startUpdates(), stopScroll = watchScrollRegions();
    let alive = true;
    void api<Settings>('GET', 'api/settings').then(c => { if (alive) appState.config = c; }).catch(e => { if (!location.hash.startsWith('#/settings')) toast(message(e)); }); // Settings shows this error itself.
    void route();
    return () => { stopUpdates(); stopScroll(); alive = false; void dispose(); };
  });
  function externalLink(e: MouseEvent) {
    const a = (e.target as Element).closest<HTMLAnchorElement>('a[href]');
    // In `tauri dev` the app itself is served over http, so its own links must stay in the window.
    if (!a || !/^https?:/i.test(a.href) || a.origin === location.origin) return;
    e.preventDefault(); void openUrl(a.href).catch(error => toast(message(error)));
  }
  $effect(() => { if (appState.screen === 'lesson' && appState.course && !appState.current) location.hash = href(appState.course.id); });
  // After a route change the clicked link is gone and focus falls to the page; start keyboard and screen-reader users at the new page title.
  $effect(() => {
    if (appState.screen === 'loading') return;
    appState.routeVersion;
    void tick().then(() => {
      if (document.activeElement && document.activeElement !== document.body) return;
      const h = document.querySelector<HTMLElement>('main h1');
      if (h) { h.tabIndex = -1; h.focus({ preventScroll: true }); }
    });
  });
</script>
<svelte:window onhashchange={() => void route()} />
<svelte:document onclick={externalLink} onauxclick={e => { if (e.button === 1) externalLink(e); }} />
<TopBar />
{#if appState.toast}<p id="toast" class="err" role="alert" transition:toastMotion>{appState.toast}</p>{/if}
<main>
  {#if !updates.dismissed && (updates.state === 'ready' || updates.state === 'download')}<aside class="update-notice"><UpdateStatus /><button class="btn ghost sm" aria-label="Dismiss update notice" onclick={() => updates.dismissed = true}>Dismiss</button></aside>{/if}
  {#if appState.saveError}<aside class="update-notice save-notice" role="alert"><span>Your progress is not saved yet. {appState.saveError} It stays here and is sent again with your next change.</span><button class="btn secondary sm" onclick={() => void flushSave()}>Try again</button></aside>{/if}
  {#key appState.routeVersion}
    {#if appState.screen === 'library'}<Library />
    {:else if appState.screen === 'creation'}<Creation />
    {:else if appState.screen === 'overview' && appState.course}<Overview c={appState.course} />
    {:else if appState.screen === 'lesson' && appState.course && appState.current}<Lesson c={appState.course} l={appState.current} />
    {:else if appState.screen === 'exam' && appState.course}<Exam c={appState.course} stage={appState.examStage} />
    {:else if appState.screen === 'review'}<Review />
    {:else if appState.screen === 'settings'}<SettingsScreen />
    {:else if appState.screen === 'search'}<Search />
    {:else if appState.screen === 'mistakes'}<Mistakes />
    {:else if appState.screen === 'import'}<Import />
    {:else if appState.screen === 'error'}{@const roadmap = location.hash.startsWith('#/c/')}<div class="wrap narrow screen-enter">{#if roadmap}<a class="back-map" href="#/"><Icon name="back" /> Your library</a>{/if}<div class="page-head"><h1>{roadmap ? 'Could not open this roadmap' : 'Could not open your library'}</h1></div><LoadError what={roadmap ? 'The roadmap could not load.' : 'Your roadmaps could not load.'} error={appState.error} retry={() => void route()} /></div>
    {:else}<div class="wrap narrow screen-enter"><a class="back-map" href="#/"><Icon name="back" /> Your library</a><div class="page-head"><h1>Your roadmap</h1></div><Waiting inline title="Opening your roadmap" /></div>{/if}
  {/key}
</main>
<Ideas />
<Confirm />
