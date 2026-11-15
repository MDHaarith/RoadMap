<script lang="ts">
  import Skeleton from './Skeleton.svelte';
  let { kind = 'text', rows = 3 }: { kind?: string; rows?: number } = $props();
</script>
<div class={`skeleton skeleton-${kind}`} aria-busy="true" role="status">
  <span class="sr">Loading</span>
  <div class="skeleton-shape" aria-hidden="true">
    {#if kind === 'home'}
      <div class="home-workspace"><div class="today-column"><h1>Today</h1><div class="hero">{@render lines(4)}<i class="sk control"></i></div></div><div class="library-column"><h2>Your library</h2>{@render list(3)}</div></div>
    {:else if kind === 'overview'}
      <div class="overview"><div class="overview-side">{@render lines(4)}{@render list(3)}</div><div class="overview-main">{#each Array(3) as _}<i class="sk stage-line"></i><div class="skeleton-lessons">{#each Array(4) as _}<div class="skeleton-lesson-row"><i class="sk stop"></i><i class="sk text-line"></i></div>{/each}</div>{/each}</div></div>
    {:else if kind === 'lesson'}
      <div class="skeleton-lesson-layout"><div>{@render lines(3)}<div class="sk rail"></div><Skeleton kind="videos" /></div><div class="skeleton-sidebar">{@render lines(3)}</div></div>
    {:else if kind === 'videos'}
      <div class="video-layout"><div class="video-main"><div class="player-space"><i class="sk player-block"></i></div>{@render lines(2)}</div><div class="video-choices">{@render list(3)}{@render lines(2)}<i class="sk control"></i></div></div>
    {:else if kind === 'question'}
      <div class="quiz-meta"><i class="sk short"></i></div><i class="sk question-line"></i><div class="options">{#each Array(4) as _}<div class="sk option-block"></div>{/each}</div>
    {:else if kind === 'plans' || kind === 'options'}
      <div class="skeleton-plan-options">{#each Array(kind === 'plans' ? 3 : 1) as _}<div class="card">{@render lines(2)}<div class="skeleton-plan-copy">{@render lines(5)}</div><i class="sk control"></i></div>{/each}</div>
    {:else if kind === 'plan'}
      {@render list(3)}
    {:else if kind === 'settings'}
      {#each Array(4) as _}<div class="settings-layout"><i class="sk short"></i><div>{@render lines(3)}<i class="sk control"></i></div></div>{/each}
    {:else if kind === 'chart'}
      <div class="sk chart-block"></div>{@render lines(2)}
    {:else if kind === 'code' || kind === 'notes'}
      {#if kind === 'code'}<i class="sk file-heading"></i>{/if}<div class="skeleton-document">{@render lines(kind === 'code' ? 12 : 8)}</div>
    {:else if kind === 'rows' || kind === 'models'}
      {@render list(rows)}
    {:else}
      {@render lines(rows)}
    {/if}
  </div>
</div>
{#snippet lines(n: number)}<div class="skeleton-lines">{#each Array(n) as _, i}<i class="sk text-line" style:width={i === n - 1 ? '62%' : i % 2 ? '88%' : '100%'}></i>{/each}</div>{/snippet}
{#snippet list(n: number)}{#each Array(n) as _}<div class="skeleton-row"><i class="sk short"></i>{@render lines(2)}</div>{/each}{/snippet}
