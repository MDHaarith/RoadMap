<script lang="ts">
  import Skeleton from './Skeleton.svelte';
  import { onMount, tick } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { api } from '../lib/api';
  import { appState, openIdeas, dispose, toast, message } from '../lib/state.svelte';
  import { lessons } from '../lib/progress';
  import { enc, href } from '../lib/format';
  import type { LibraryCourse, ReviewData } from '../lib/types';
  let c = $derived(appState.course), ls = $derived(c ? lessons(c) : []), n = $derived(c ? ls.filter(l => c.state.done[l.id]).length : 0);
  let query = $state(''), menuOpen = $state(false), menuLoading = $state(false), menuError = $state(''), others = $state<LibraryCourse[]>([]);
  let maximized = $state(false), switcher: HTMLDivElement, menu: HTMLDivElement;
  let win: ReturnType<typeof getCurrentWindow> | undefined;
  onMount(() => {
    let cleanup: (() => void) | undefined, alive = true;
    win = getCurrentWindow();
    void win.isMaximized().then(value => { maximized = value; }).catch(() => {});
    void win.onResized(async () => { maximized = await win!.isMaximized(); }).then(fn => { if (alive) cleanup = fn; else fn(); }).catch(() => {});
    return () => { alive = false; cleanup?.(); };
  });
  $effect(() => {
    appState.routeVersion;
    let alive = true;
    void api<ReviewData>('GET', 'api/review').then(review => { if (alive) appState.review.count = review.count; }).catch(() => {});
    return () => { alive = false; };
  });
  $effect(() => { appState.routeVersion; menuOpen = false; if (appState.screen === 'search') { try { query = decodeURIComponent(location.hash.split('/')[2] || ''); } catch { query = ''; } } });
  async function windowAction(action: 'minimize' | 'maximize' | 'close') {
    try {
      if (action === 'close') { await dispose(); await win?.close(); }
      else if (action === 'maximize') { await win?.toggleMaximize(); maximized = await win!.isMaximized(); }
      else await win?.minimize();
    } catch (error) { toast(message(error)); }
  }
  async function toggleSwitcher() {
    if (menuOpen) { menuOpen = false; return; }
    menuOpen = true; menuLoading = true; menuError = ''; const current = c;
    try {
      const list = await api<LibraryCourse[]>('GET', 'api/courses');
      if (c !== current || !menuOpen) return;
      others = list.filter(x => !x.paused && x.id !== c?.id && x.next && x.done < x.total);
      menuLoading = false;
      await tick(); menu?.querySelector('a')?.focus();
    } catch (error) { menuError = message(error); } finally { menuLoading = false; }
  }
  function closeSwitcher(focus = false) { menuOpen = false; if (focus) document.getElementById('switch-toggle')?.focus(); }
  function keyboard(e: KeyboardEvent) {
    if (e.key === 'Escape' && appState.ideasOpen) return;
    if (!menuOpen) return;
    const items = [...menu.querySelectorAll<HTMLAnchorElement>('[role=menuitem]')];
    if (e.key === 'Escape') { e.preventDefault(); closeSwitcher(true); }
    else if (e.key === 'Tab') closeSwitcher(true);
    else if (['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(e.key)) {
      e.preventDefault(); const i = items.indexOf(document.activeElement as HTMLAnchorElement);
      items[e.key === 'Home' ? 0 : e.key === 'End' ? items.length - 1 : (i + (e.key === 'ArrowUp' ? -1 : 1) + items.length) % items.length]?.focus();
    }
  }
</script>
<svelte:window onkeydown={keyboard} onclick={e => { if (menuOpen && !switcher?.contains(e.target as Node)) closeSwitcher(); }} />
<!-- Tauri’s drag region handles double-click maximization on each platform. -->
<header class="top" data-tauri-drag-region>
  <a class="brand" href="#/" aria-label="Roadmap library"><svg viewBox="0 0 34 40" fill="none" stroke="currentColor" stroke-width="4" aria-hidden="true"><path d="M8 33V24c0-8 18-2 18-11V7" /><circle cx="8" cy="33" r="4" fill="currentColor" /><path d="M22 7h8V1h-8z" fill="currentColor" /></svg>Roadmap</a>
  {#if c}<span class="crumb-sep" aria-hidden="true">/</span>{/if}
  <a id="ctitle" hidden={!c} href={c ? href(c.id) : '#/'}>{c?.title}</a>
  <div class="switcher" id="switcher" hidden={!c} bind:this={switcher}><button class="switch-toggle" id="switch-toggle" aria-haspopup="menu" aria-expanded={menuOpen} aria-controls="switch-menu" onclick={toggleSwitcher} onkeydown={e => { if (e.key === 'ArrowDown' && !menuOpen) { e.preventDefault(); void toggleSwitcher(); } }}><span>Switch</span><svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="m6 9 6 6 6-6" /></svg></button><div class="switch-menu popover-enter stagger" id="switch-menu" role="menu" aria-label="Other active roadmaps" hidden={!menuOpen} bind:this={menu}>{#if menuLoading}<Skeleton kind="rows" />{:else if menuError}<p>{menuError}</p>{:else if !others.length}<p>No other active roadmaps.</p>{:else}{#each others as x}<a role="menuitem" tabindex="-1" href={href(x.id, x.next!.id)}><b>{x.title}</b><span>Next: {x.next!.title}</span></a>{/each}{/if}</div></div>
  <span class="grow" data-tauri-drag-region></span><span class="meter" id="meter" hidden={!c} title="Lessons complete in this roadmap"><span class="bar"><i id="bar" style:width="100%" style:border-radius={`${999 / ((ls.length ? n / ls.length : 0) || 1)}px / 999px`} style:transform={`scaleX(${ls.length ? n / ls.length : 0})`}></i></span><span id="count">{n} / {ls.length}</span></span>
  <form id="search" class="top-search" role="search" onsubmit={e => { e.preventDefault(); if (query.trim()) location.hash = '#/search/' + enc(query.trim()); }}><label class="sr" for="q">Search lessons, notes, tutor chats and ideas</label><input id="q" type="search" placeholder="Search" maxlength="100" bind:value={query} /></form>
  <a class="top-link" href="#/review" title="Questions due for review today"><svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M3 12a9 9 0 1 0 3-6.7L3 8M3 3v5h5" /></svg><span>Review</span><span class="count">{appState.review.count}</span></a>
  <button class="top-link" id="ideas-open" type="button" aria-expanded={appState.ideasOpen} aria-controls="ideas" onclick={() => openIdeas()}><svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M5 3h14v12l-6 6H5zM13 21v-6h6" /></svg><span>Ideas</span></button>
  <a class="top-link" href="#/settings" aria-label="Settings" title="Settings"><svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="m9 3-1 4-4 1v8l4 1 1 4h6l1-4 4-1V8l-4-1-1-4z" /><circle cx="12" cy="12" r="3" /></svg></a>
  <div class="window-controls">{#each ['minimize', 'maximize', 'close'] as action, i}{@const label = action === 'maximize' ? maximized ? 'Restore' : 'Maximise' : action === 'minimize' ? 'Minimise' : 'Close'}<button type="button" aria-label={label} title={label} onclick={() => windowAction(action as 'minimize' | 'maximize' | 'close')}><svg viewBox="0 0 16 16" aria-hidden="true"><path d={i === 0 ? 'M3 8h10' : i === 1 ? maximized ? 'M5 3h8v8M3 5h8v8H3z' : 'M3 3h10v10H3z' : 'M3 3l10 10M13 3L3 13'} /></svg></button>{/each}</div>
</header>
