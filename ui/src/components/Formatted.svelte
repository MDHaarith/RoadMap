<script lang="ts">
  import { fmt, fmtTutor } from '../lib/format';
  let { text, tutor = false }: { text: unknown; tutor?: boolean } = $props();
</script>
{@html tutor ? fmtTutor(text) : fmt(text)}
