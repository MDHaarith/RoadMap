<script lang="ts">
  import type { Snippet } from 'svelte';
  import Elapsed from './Elapsed.svelte';
  import Skeleton from './Skeleton.svelte';
  import { appState } from '../lib/state.svelte';
  let { title, hint = '', inline = false, skeleton = 'text', children }: { title: string; hint?: string; inline?: boolean; skeleton?: string; children?: Snippet } = $props();
  // The route shell owns first loads; its destination determines the placeholder layout.
  let initial = $derived(appState.screen === 'loading');
  let shape = $derived(initial ? (/^#\/c\/[^/]+\/exam\//.test(location.hash) ? 'question' : /^#\/c\/[^/]+\/[^/]+/.test(location.hash) ? 'lesson' : location.hash.startsWith('#/c/') ? 'overview' : 'home') : skeleton);
</script>
<div class="waiting" class:inline class:initial aria-busy="true">
  {#if !initial}<div class="waiting-copy" role="status"><strong>{title}</strong><p><span aria-hidden="true"><Elapsed /></span>{hint ? ' · ' + hint : ''}</p>{@render children?.()}</div>{/if}
  <Skeleton kind={shape} />
</div>
