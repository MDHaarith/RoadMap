<script lang="ts">
  let { n, total, wide = true }: { n: number; total: number; wide?: boolean } = $props();
</script>
<span class="bar" class:wide role="img" aria-label={`${n} of ${total} lessons complete`}><i style:border-radius={`${999 / ((total ? n / total : 0) || 1)}px / 999px`} style:transform={`scaleX(${total ? n / total : 0})`}></i></span>
