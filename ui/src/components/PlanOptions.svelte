<script lang="ts">
  import type { PlanPreview } from '../lib/types';
  import { applyPlan, message } from '../lib/state.svelte';
  import Waiting from './Waiting.svelte';
  let { preview, ask = '', cancel, applied = cancel }: { preview: PlanPreview; ask?: string; cancel: () => void; applied?: () => void } = $props();
  let busy = $state(false), error = $state('');
  async function use(plan: object) {
    busy = true; error = '';
    try { await applyPlan(plan, ask); applied(); } catch (e) { error = message(e); } finally { busy = false; }
  }
</script>
<section aria-label="Preview roadmap changes" class="note">
  {#if preview.note}<p>{preview.note}</p>{/if}
  {#if error}<p class="err" role="alert">{error}</p>{/if}
  {#if busy}<Waiting inline title="Applying your changes…" />{/if}
  <div class="options">{#each preview.options as option}<article class="card"><h3>{option.label}</h3><p>{option.note}</p><ul>
    {#each option.added as item}<li class="added">+ Added: {item.title}{#if item.goal}<small class="goal">{item.goal}</small>{/if}{#if item.phase}<ul>{#each item.lessons || [] as title}<li>{title}</li>{/each}</ul>{/if}</li>{/each}
    {#each option.dropped as title}<li class="removed">− Removed: {title}</li>{/each}
    {#each option.changed as item}<li>~ Rewritten: {item.from} → {item.title}{#if item.goal}<small class="goal">{item.goal}</small>{/if}</li>{/each}
  </ul><button class="btn secondary sm" disabled={busy} onclick={() => use(option.plan)}>Use this</button></article>{/each}</div>
  <button class="link" disabled={busy} onclick={cancel}>{ask ? 'Try different wording / Cancel' : 'Cancel'}</button>
</section>
<style>.options{display:grid;grid-template-columns:repeat(auto-fit,minmax(200px,1fr));gap:8px;margin:8px 0}.card{padding:12px;min-width:0;overflow-wrap:anywhere}h3{margin:0}ul{padding-left:16px;font-size:12px}.added{color:var(--success)}.removed{color:var(--muted)}.goal{display:block;color:var(--muted)}</style>
