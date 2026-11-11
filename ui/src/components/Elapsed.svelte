<script lang="ts">
  import { onMount } from 'svelte';
  import { clock } from '../lib/format';
  // Live m:ss since this element appeared. Long AI waits show it so the app never looks frozen.
  let seconds = $state(0);
  onMount(() => { const timer = setInterval(() => seconds++, 1000); return () => clearInterval(timer); });
</script>
<span class="elapsed">{clock(seconds)}</span>
