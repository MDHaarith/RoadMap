<script lang="ts">
  // How hard ChatGPT thinks for one job; starts at the Settings default.
  import Select from './Select.svelte';
  import { EFFORTS } from '../lib/state.svelte';
  let { value = $bindable(), id }: { value: string; id?: string } = $props();
</script>
<Select {id} ariaLabel="Thinking effort" heading="How hard ChatGPT thinks" {value} onchange={v => value = v} options={EFFORTS.map(([v, label, hint]) => ({ value: v, label, hint }))} />
