<script lang="ts">
  import { icon } from '../lib/format';
  let { name }: { name: string } = $props();
</script>
{@html icon(name)}
