<script lang="ts">
  // A failed load: what failed, why, and one retry in the same place.
  let { what, error, retry, id }: { what: string; error: string; retry: () => void; id?: string } = $props();
</script>
<div class="err load-error" role="alert"><strong>{what}</strong><p>{error}</p><button type="button" class="btn secondary sm" {id} onclick={retry}>Try again</button></div>
