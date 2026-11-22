<script lang="ts">
  // A progress ring drawn on a native <progress>, so assistive tech reads it as progress.
  let { value, max, label, size = 54 }: { value: number; max: number; label: string; size?: number } = $props();
  let pct = $derived(max > 0 ? Math.min(100, value / max * 100) : 0);
</script>
<span class="ring-wrap"><progress class="ring" {value} {max} aria-label={label} aria-valuetext={`${value} of ${max}`} style:--value={pct} style:--size={`${size}px`}></progress><span aria-hidden="true">{Math.min(value, max)}/{max}</span></span>
<style>
  @property --value { syntax: '<number>'; inherits: true; initial-value: 0; }
  .ring-wrap { position: relative; display: grid; place-items: center; flex: none; }
  .ring-wrap span { position: absolute; font-size: var(--t-xs); font-weight: 600; }
  progress.ring { --thickness: 6px; appearance: none; -webkit-appearance: none; width: var(--size); height: var(--size); border-radius: 50%; border: var(--thickness) solid transparent; background: conic-gradient(var(--accent) calc(var(--value) * 1%), var(--surface-2) 0) border-box; background-clip: border-area; }
  @media (prefers-reduced-motion: no-preference) { progress.ring { transition: --value .5s var(--motion-ease); } }
  /* Engines without border-area clipping cut the hole with a mask instead. */
  @supports not (background-clip: border-area) { progress.ring { border: 0; mask-image: radial-gradient(farthest-side, transparent calc(100% - var(--thickness)), #000 calc(100% - var(--thickness) + .5px)); } }
  progress.ring::-webkit-progress-bar, progress.ring::-webkit-progress-value { display: none; background: none; }
  progress.ring::-moz-progress-bar { display: none; background: none; }
  @media (forced-colors: active) { progress.ring { forced-color-adjust: none; background: conic-gradient(CanvasText calc(var(--value) * 1%), Canvas 0); } }
</style>
