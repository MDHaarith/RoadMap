<script lang="ts">
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { updates, installUpdate } from '../lib/updates.svelte';
</script>
<span role="status">
  {#if updates.state === 'latest'}You have the latest version
  {:else if updates.state === 'downloading'}Downloading {updates.available}…
  {:else if updates.state === 'ready' || updates.state === 'download'}Roadmap {updates.available} is ready.
  {:else if updates.state === 'error'}Could not check for updates. Check your internet connection.
  {:else if updates.state === 'install-error'}Could not install the update. Please check again and retry.{/if}
</span>
{#if updates.state === 'ready'}<button class="btn secondary sm" onclick={() => void installUpdate()}>Restart and update</button>
{:else if updates.state === 'download'}<button class="btn secondary sm" onclick={() => void openUrl('https://github.com/MDHaarith/RoadMap/releases/latest').catch(() => { updates.state = 'error'; })}>Download</button>{/if}
