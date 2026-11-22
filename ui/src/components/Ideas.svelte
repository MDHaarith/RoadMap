<script lang="ts">
  import { tick } from 'svelte';
  import { slide } from 'svelte/transition';
  import { appState, saveIdeas } from '../lib/state.svelte';
  import Icon from './Icon.svelte';
  import { autogrow } from '../lib/autogrow';
  import Waiting from './Waiting.svelte';
  import { confirmAsk } from './Confirm.svelte';
  let panel = $state<HTMLElement>();
  const ms = () => matchMedia('(prefers-reduced-motion: reduce)').matches ? 0 : 180;
  // A manual popover: it sits in the top layer under the Ideas button, and a confirm dialog opened from it does not close it.
  $effect(() => {
    if (!panel) return;
    const open = panel.matches(':popover-open');
    if (appState.ideasOpen && !open) panel.showPopover(); else if (!appState.ideasOpen && open) panel.hidePopover();
  });
  $effect(() => { if (appState.ideasOpen && appState.ideas) void tick().then(() => panel?.querySelector<HTMLElement>('#idea-new-text')?.focus()); });
  function close() { appState.ideasOpen = false; document.getElementById('ideas-open')?.focus(); }
  // Ideas are plain strings, newest first. The first one is the one being written; the rest are listed under it.
  let ids: number[] = [], nextId = 0;
  const keys = (n: number) => { while (ids.length < n) ids.push(nextId++); ids.length = n; return ids; };
  async function add() {
    if (appState.ideas![0]?.trim() || !appState.ideas!.length) { appState.ideas!.unshift(''); ids.unshift(nextId++); saveIdeas(); }
    await tick(); panel?.querySelector<HTMLElement>('#idea-new-text')?.focus();
  }
  async function remove(i: number) { if (appState.ideas![i].trim() && !(await confirmAsk({ message: 'Delete this idea?', action: 'Delete', danger: true }))) return; ids.splice(i, 1); appState.ideas!.splice(i, 1); saveIdeas(); }
  function edit(i: number, text: string) { appState.ideas![i] = text; saveIdeas(); }
  // Escape closes the panel before anything under it; an open dialog keeps its own Escape.
  function keyboard(e: KeyboardEvent) { if (e.key === 'Escape' && appState.ideasOpen && !e.defaultPrevented && !document.querySelector('dialog[open]')) { e.preventDefault(); close(); } }
  // Light dismiss by hand, since a manual popover has none: a click outside the panel, its button and any dialog closes it.
  function outside(e: MouseEvent) {
    const t = e.target as Element;
    if (appState.ideasOpen && panel && !panel.contains(t) && !t.closest?.('#ideas-open, dialog')) appState.ideasOpen = false;
  }
</script>
<svelte:window onkeydowncapture={keyboard} onclick={outside} />
<div id="ideas" class="ideas" popover="manual" aria-label="Ideas" bind:this={panel}>
  {#if appState.ideasOpen}{#if !appState.ideas}<Waiting skeleton="notes" inline title="Loading your ideas" />{:else}{@const k = keys(appState.ideas.length)}
    <div class="ideas-head"><h2>Ideas</h2><button class="btn secondary sm" id="idea-add" type="button" onclick={add}><Icon name="plus" /> New idea</button><button class="btn ghost sm" id="ideas-close" type="button" onclick={close}>Close</button></div>
    <textarea id="idea-new-text" class="idea-new" use:autogrow={{ max: 200, value: appState.ideas[0] ?? '' }} rows="3" maxlength="4000" aria-label="Newest idea" placeholder="Write an idea before it gets away…" value={appState.ideas[0] ?? ''} oninput={e => { if (!appState.ideas!.length) { appState.ideas!.push(''); keys(1); } edit(0, e.currentTarget.value); }}></textarea>
    <p class="small ideas-saved" aria-live="polite">{appState.ideas[0]?.trim() ? 'Saved as you type.' : 'Something to build, a question to chase, a roadmap to start later.'}</p>
    {#if appState.ideas.length > 1}<ul class="idea-list">{#each appState.ideas.slice(1) as x, j (k[j + 1])}<li transition:slide={{ duration: ms() }}><textarea use:autogrow={{ max: 160, value: x }} rows="1" maxlength="4000" aria-label={`Idea ${j + 2}`} value={x} oninput={e => edit(j + 1, e.currentTarget.value)}></textarea><button class="link danger" type="button" aria-label={`Delete idea ${j + 2}`} onclick={() => remove(j + 1)}>Delete</button></li>{/each}</ul>{/if}
  {/if}{/if}
</div>
<style>
  .ideas { width: min(340px, calc(100vw - 24px)); max-height: calc(100dvh - var(--top-h) - 24px); overflow: auto; margin: 0; inset: calc(var(--top-h) + var(--s2)) var(--s3) auto auto; padding: 0 0 var(--s2); color: var(--ink); background: var(--surface); border: 1px solid var(--line); border-radius: var(--r-btn); box-shadow: 0 16px 40px rgba(20, 20, 30, .14); }
  /* Under the Ideas button where anchor positioning exists; the fixed corner above is the fallback. */
  @supports (position-area: block-end) {
    .ideas { position-anchor: --ideas-button; inset: auto; margin: var(--s2) 0 0; position-area: block-end span-inline-start; position-try-fallbacks: flip-inline; }
  }
  .ideas:popover-open { animation: unfold var(--motion-enter) var(--motion-ease); }
  @media (prefers-reduced-motion: reduce) { .ideas:popover-open { animation: none; } }
  .ideas-head { display: flex; align-items: center; gap: var(--s2); padding: var(--s4) var(--s4) var(--s3) 14px; margin: 0; }
  .ideas-head h2 { flex: 1; margin: 0; font-size: var(--t-md); font-weight: 600; }
  .idea-new { display: block; width: calc(100% - 24px); margin: 0 12px; min-height: 84px; padding: 9px var(--s4); font-size: var(--t-md); line-height: 1.5; resize: none; border-radius: var(--r-lg); }
  .ideas-saved { margin: 0; padding: var(--s2) 14px var(--s3); font-size: var(--t-xs); }
  .idea-list { list-style: none; margin: 0; padding: 0; border-top: 1px solid var(--line); }
  .idea-list li { display: grid; grid-template-columns: minmax(0, 1fr) auto; align-items: start; gap: var(--s3); padding: var(--s1) 14px var(--s1) var(--s2); border-bottom: 1px solid var(--line); }
  .idea-list li:last-child { border-bottom: 0; }
  .idea-list textarea { font-size: var(--t-sm); border-color: transparent; background: none; resize: none; padding: var(--s2); }
  .idea-list textarea:hover { border-color: var(--line-strong); }
  .idea-list .link { margin-top: 7px; font-size: var(--t-xs); opacity: 0; transition: opacity var(--motion-fast); }
  .idea-list li:hover .link, .idea-list li:focus-within .link { opacity: 1; }
  .ideas :global(.waiting) { padding: var(--s4); }
</style>
