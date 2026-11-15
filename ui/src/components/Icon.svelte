<script lang="ts">
  import { icon } from '../lib/format';
  let { name }: { name: string } = $props();
</script>
{#if name === 'empty-route'}
  <svg class="empty-route" viewBox="0 0 88 48" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><path d="M8 36h22c16 0 8-24 26-24h24" /><circle cx="8" cy="36" r="4" fill="var(--bg)" /><circle cx="44" cy="24" r="4" fill="var(--bg)" /><circle cx="80" cy="12" r="4" fill="var(--accent)" stroke="var(--accent)" /></svg>
{:else if name === 'check'}
  <svg class="icon check-mark" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path pathLength="1" d="m5 12 5 5 9-10" /></svg>
{:else if name === 'stop'}
  <svg class="icon" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><rect x="6" y="6" width="12" height="12" rx="1" /></svg>
{:else if name === 'arrow'}
  <svg class="icon button-arrow" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 12h16m-6-6 6 6-6 6" /></svg>
{:else}{@html icon(name)}{/if}
