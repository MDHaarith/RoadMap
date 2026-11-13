<script lang="ts">
  import type { Course, Lesson, PlanPreview } from '../lib/types';
  import { api } from '../lib/api';
  import { message, flushSave } from '../lib/state.svelte';
  import { enc } from '../lib/format';
  import PlanOptions from './PlanOptions.svelte';
  import Waiting from './Waiting.svelte';
  let { c, l, compact = false }: { c: Course; l: Lesson; compact?: boolean } = $props();
  let open = $state(false), menu = $state<HTMLDetailsElement>(), preview = $state<PlanPreview>(), busy = $state(false), error = $state('');
  async function edit(action: string) {
    open = false; busy = true; error = ''; preview = undefined;
    try { await flushSave(); preview = await api<PlanPreview>('POST', 'api/courses/' + enc(c.id) + '/edit', { lesson: l.id, action }); }
    catch (e) { error = message(e); } finally { busy = false; }
  }
</script>
<svelte:window onclick={e => { if (open && !menu?.contains(e.target as Node)) open = false; }} onkeydown={e => { if (e.key === 'Escape' && open) { open = false; menu?.querySelector('summary')?.focus(); } }} />
{#if !c.state.done[l.id]}<details class="menu" bind:this={menu} bind:open><summary class="btn secondary sm" class:compact aria-label={`Change lesson: ${l.title}`} title="Change lesson">{compact ? '…' : 'Change lesson'}</summary><div class="menu-list">{#each [['swap','Swap for another lesson'],['harder','Make it harder'],['easier','Make it easier'],['deeper','Go deeper'],['remove','Remove']] as [action,label]}<button disabled={busy} onclick={() => edit(action)}>{label}</button>{/each}</div></details>{/if}
{#if busy || preview || error}<section class="edit-preview">{#if busy}<Waiting title="Preparing lesson options" hint="this can take about a minute" />{/if}{#if error}<p class="err" role="alert">{error}</p>{/if}{#if preview}<PlanOptions {preview} cancel={() => preview = undefined} />{/if}</section>{/if}
<style>.edit-preview{flex-basis:100%;min-width:0;width:100%}.compact{min-width:28px;padding-inline:8px}</style>
