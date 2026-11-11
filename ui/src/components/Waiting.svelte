<script lang="ts">
  import type { Snippet } from 'svelte';
  import Elapsed from './Elapsed.svelte';
  // An honest wait: what is happening, how long it usually takes, and a live elapsed clock.
  let { title, hint = '', inline = false, children }: { title: string; hint?: string; inline?: boolean; children?: Snippet } = $props();
</script>
<div class="waiting" class:inline role="status"><span class="spinner" aria-hidden="true"></span>{#if inline}<span>{title} <Elapsed /></span>{:else}<div><strong>{title}</strong><p><Elapsed />{hint ? ' · ' + hint : ''}</p>{@render children?.()}</div>{/if}</div>
