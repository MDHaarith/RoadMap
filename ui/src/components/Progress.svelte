<script lang="ts">
  let { n, total, wide = true }: { n: number; total: number; wide?: boolean } = $props();
</script>
<span class="bar" class:wide role="img" aria-label={`${n} of ${total} lessons complete`}><i style:width={`${total ? n / total * 100 : 0}%`}></i></span>
