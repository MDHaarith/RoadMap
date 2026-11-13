<script lang="ts">
  import { tick } from 'svelte';
  import { appState, saveIdeas } from '../lib/state.svelte';
  import Icon from './Icon.svelte';
  import Waiting from './Waiting.svelte';
  let panel: HTMLElement;
  $effect(() => { if (appState.ideasOpen && appState.ideas) void tick().then(() => panel?.querySelector<HTMLElement>('textarea, #idea-add')?.focus()); });
  function close() { appState.ideasOpen = false; }
  async function add() { appState.ideas?.unshift(''); saveIdeas(); await tick(); panel?.querySelector('textarea')?.focus(); }
  function remove(i: number) { if (appState.ideas![i].trim() && !confirm('Delete this idea?')) return; appState.ideas!.splice(i, 1); saveIdeas(); }
  function keyboard(e: KeyboardEvent) { if (e.key === 'Escape' && appState.ideasOpen) { close(); document.getElementById('ideas-open')?.focus(); } }
</script>
<svelte:window onkeydown={keyboard} />
<aside id="ideas" class="ideas" aria-label="Idea notes" hidden={!appState.ideasOpen} bind:this={panel}>
  {#if !appState.ideas}<Waiting inline title="Loading your ideas" />{:else}<div class="ideas-head"><div class="grow"><h2>Ideas</h2><p class="small">Saved as you type.</p></div><button class="btn secondary sm" id="idea-add" type="button" onclick={add}><Icon name="plus" /> New note</button><button class="btn ghost sm" id="ideas-close" type="button" onclick={close}>Close</button></div><div class="stickies">{#if appState.ideas.length}{#each appState.ideas as x, i}<div class="sticky"><textarea rows="4" maxlength="4000" aria-label={`Idea ${i + 1}`} placeholder="Write an idea before it gets away…" value={x} oninput={e => { appState.ideas![i] = e.currentTarget.value; saveIdeas(); }}></textarea><button class="link danger" type="button" aria-label={`Delete idea ${i + 1}`} onclick={() => remove(i)}>Delete</button></div>{/each}{:else}<p class="muted">No ideas yet. Something to build, a question to chase, a roadmap to start later: put it here.</p>{/if}</div>{/if}
</aside>
