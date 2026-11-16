<script lang="ts">
  // Both labels share one grid cell, so the button keeps the width of the longer one.
  let { busy, idle, label }: { busy: boolean; idle: string; label: string } = $props();
</script>
<span class="busy-label"><span class:off={busy} aria-hidden={busy}>{idle}</span><span class:off={!busy} aria-hidden={!busy}>{label}</span></span>
