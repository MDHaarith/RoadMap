<script lang="ts">
  // A tutor reply. While it streams, finished paragraphs render formatted and the paragraph still arriving
  // shows word by word, released at an even pace so network bursts don't jump, each word fading in.
  import Formatted from './Formatted.svelte';
  import { untrack } from 'svelte';
  let { text, live }: { text: string; live: boolean } = $props();
  let cut = $derived(live ? text.lastIndexOf('\n\n') + 1 : text.length);
  // Markdown marks show only once their paragraph is finished and formatted.
  let words = $derived(text.slice(cut).replace(/[*_`#]/g, '').split(/(\s+)/).filter(Boolean));
  let shown = $state(0), at = 0;
  $effect(() => {
    if (cut !== at) { at = cut; shown = 0; }
    if (untrack(() => shown) >= words.length) return;
    const t = setInterval(() => {
      const left = words.length - shown;
      if (left <= 0) clearInterval(t);
      else shown += Math.ceil(left / 10);
    }, 35);
    return () => clearInterval(t);
  });
</script>
{#if text.slice(0, cut).trim()}<Formatted text={text.slice(0, cut)} tutor />{/if}
{#if live && words.length}<p class="tutor-p stream-tail">{#each words.slice(0, shown) as w, i (i)}<span class="word">{w}</span>{/each}</p>{/if}
