<script lang="ts">
  import type { Blueprint } from '../lib/types';
  let { plan }: {plan: Blueprint} = $props();
</script>
<label for="challenge">Final challenge</label><textarea id="challenge" required maxlength="1800" rows="3" bind:value={plan.challenge}></textarea>
<label for="criteria">You are done when <span class="small">One per line</span></label><textarea id="criteria" required maxlength="2400" rows="4" bind:value={plan.criteria}></textarea>
<label for="milestones">Milestones <span class="small">One per line, in order</span></label><textarea id="milestones" required maxlength="4000" rows="7" bind:value={plan.milestones}></textarea>
