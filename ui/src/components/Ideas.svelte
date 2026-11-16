<script lang="ts">
  import { tick } from 'svelte';
  import { appState, saveIdeas } from '../lib/state.svelte';
  import Icon from './Icon.svelte';
  import { autogrow } from '../lib/autogrow';
  import Waiting from './Waiting.svelte';
  import { confirmAsk } from './Confirm.svelte';
  let panel: HTMLElement;
  $effect(() => { if (appState.ideasOpen && appState.ideas) void tick().then(() => panel?.querySelector<HTMLElement>('textarea, #idea-add')?.focus()); });
  function close() { appState.ideasOpen = false; document.getElementById('ideas-open')?.focus(); }
  async function add() { appState.ideas?.unshift(''); saveIdeas(); await tick(); panel?.querySelector('textarea')?.focus(); }
  async function remove(i: number) { if (appState.ideas![i].trim() && !(await confirmAsk({ message: 'Delete this idea?', action: 'Delete', danger: true }))) return; appState.ideas!.splice(i, 1); saveIdeas(); }
  // Captured first: the panel floats over the page, so Escape closes it before anything under it. An open dialog keeps its own Escape.
  function keyboard(e: KeyboardEvent) { if (e.key === 'Escape' && appState.ideasOpen && !e.defaultPrevented && !document.querySelector('dialog[open]')) { e.preventDefault(); close(); } }
</script>
<svelte:window onkeydowncapture={keyboard} />
<aside id="ideas" class="ideas" aria-label="Idea notes" hidden={!appState.ideasOpen} bind:this={panel}>
  {#if !appState.ideas}<Waiting skeleton="notes" inline title="Loading your ideas" />{:else}<div class="ideas-head"><div class="grow"><h2>Ideas</h2><p class="small">Saved as you type.</p></div><button class="btn secondary sm" id="idea-add" type="button" onclick={add}><Icon name="plus" /> New note</button><button class="btn ghost sm" id="ideas-close" type="button" onclick={close}>Close</button></div><div class="stickies ruled stagger">{#if appState.ideas.length}{#each appState.ideas as x, i}<div class="sticky"><textarea use:autogrow={{ max: 240, value: x }} rows="4" maxlength="4000" aria-label={`Idea ${i + 1}`} placeholder="Write an idea before it gets away…" value={x} oninput={e => { appState.ideas![i] = e.currentTarget.value; saveIdeas(); }}></textarea><button class="link danger" type="button" aria-label={`Delete idea ${i + 1}`} onclick={() => remove(i)}>Delete</button></div>{/each}{:else}<p class="muted">No ideas yet. Something to build, a question to chase, a roadmap to start later: put it here.</p>{/if}</div>{/if}
</aside>
