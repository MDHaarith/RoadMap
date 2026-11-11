<script lang="ts">
  import { onMount } from 'svelte';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { api } from './lib/api';
  import { appState, route, toast, message, dispose } from './lib/state.svelte';
  import { href } from './lib/format';
  import type { Settings } from './lib/types';
  import { updates, startUpdates } from './lib/updates.svelte';
  import UpdateStatus from './components/UpdateStatus.svelte';
  import TopBar from './components/TopBar.svelte';
  import Ideas from './components/Ideas.svelte';
  import Library from './screens/Library.svelte';
  import Creation from './screens/Creation.svelte';
  import Overview from './screens/Overview.svelte';
  import Lesson from './screens/Lesson.svelte';
  import Exam from './screens/Exam.svelte';
  import Review from './screens/Review.svelte';
  import SettingsScreen from './screens/Settings.svelte';
  import Search from './screens/Search.svelte';
  import Mistakes from './screens/Mistakes.svelte';
  function toastMotion(node: Element) {
    const styles = getComputedStyle(node);
    return {
      duration: matchMedia('(prefers-reduced-motion: reduce)').matches ? 0 : parseFloat(styles.getPropertyValue('--motion-enter')),
      css: (t: number) => `opacity: ${t}; transform: translate(-50%, ${(1 - t) * 5}px)`
    };
  }
  onMount(() => {
    const stopUpdates = startUpdates();
    let alive = true;
    void api<Settings>('GET', 'api/settings').then(c => { if (alive) appState.config = c; }).catch(e => toast(message(e)));
    void route();
    return () => { stopUpdates(); alive = false; void dispose(); };
  });
  function externalLink(e: MouseEvent) {
    const a = (e.target as Element).closest<HTMLAnchorElement>('a[href]');
    if (!a || !/^https?:/i.test(a.href)) return;
    e.preventDefault(); void openUrl(a.href).catch(error => toast(message(error)));
  }
  $effect(() => { if (appState.screen === 'lesson' && appState.course && !appState.current) location.hash = href(appState.course.id); });
</script>
<svelte:window onhashchange={() => void route()} />
<svelte:document onclick={externalLink} onauxclick={e => { if (e.button === 1) externalLink(e); }} />
<TopBar />
{#if appState.toast}<p id="toast" class="err" role="alert" transition:toastMotion>{appState.toast}</p>{/if}
<main>
  {#if !updates.dismissed && (updates.state === 'ready' || updates.state === 'download')}<aside class="update-notice"><UpdateStatus /><button class="btn ghost sm" aria-label="Dismiss update notice" onclick={() => updates.dismissed = true}>Dismiss</button></aside>{/if}
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
    {:else if appState.screen === 'error'}<div class="wrap narrow"><p class="err">{appState.error}</p><a href="#/">Back to your roadmaps</a></div>
    {:else}<div class="wrap narrow loading" role="status">Opening your roadmap…</div>{/if}
  {/key}
</main>
<Ideas />
