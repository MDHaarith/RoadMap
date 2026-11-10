<script lang="ts">
  import type { Chart } from '../lib/types';
  import Formatted from './Formatted.svelte';
  let { chart }: { chart: Chart } = $props();
  const W = 640, H = 300, L = 64, R = 16, T = 16, B = 48;
  const COLORS = ['var(--fill)', 'var(--ink)', 'var(--success)', 'var(--danger)'];
  let all = $derived(chart.series.flatMap(s => s.values));
  let lo = $derived(Math.min(0, ...all)), hi = $derived(Math.max(0, ...all) || 1);
  let ticks = $derived.by(() => {
    const raw = (hi - lo) / 4, mag = 10 ** Math.floor(Math.log10(raw)), step = [1, 2, 5, 10].map(m => m * mag).find(s => s >= raw)!;
    const out = []; for (let v = Math.ceil(lo / step) * step; v <= hi + step / 2; v += step) out.push(+v.toFixed(10));
    return out;
  });
  let top = $derived(Math.max(hi, ticks[ticks.length - 1])), bottom = $derived(Math.min(lo, ticks[0]));
  const y = (v: number) => T + (H - T - B) * (1 - (v - bottom) / (top - bottom));
  let n = $derived(chart.labels.length), band = $derived((W - L - R) / n);
  const x = (i: number) => L + band * (i + 0.5);
  const num = (v: number) => Math.abs(v) >= 1e6 ? +(v / 1e6).toFixed(1) + 'M' : Math.abs(v) >= 1e4 ? +(v / 1e3).toFixed(1) + 'k' : String(+v.toFixed(2));
  let every = $derived(Math.ceil(n / 8));
</script>

<figure class="chart">
  {#if chart.title}<figcaption><strong>{chart.title}</strong></figcaption>{/if}
  <svg viewBox={`0 0 ${W} ${H}`} role="img" aria-label={`${chart.title}. ${chart.series.map(s => `${s.name}: ${s.values.map((v, i) => `${chart.labels[i]} ${v}`).join(', ')}`).join('. ')}`}>
    {#each ticks as t}<line x1={L} x2={W - R} y1={y(t)} y2={y(t)} class={t === 0 ? 'axis' : 'grid'} /><text x={L - 8} y={y(t) + 4} text-anchor="end">{num(t)}</text>{/each}
    {#each chart.labels as label, i}{#if i % every === 0}<text x={x(i)} y={H - B + 18} text-anchor="middle">{label}</text>{/if}{/each}
    {#if chart.x}<text x={(L + W - R) / 2} y={H - 6} text-anchor="middle" class="axis-name">{chart.x}</text>{/if}
    {#if chart.y}<text x={14} y={(T + H - B) / 2} text-anchor="middle" class="axis-name" transform={`rotate(-90 14 ${(T + H - B) / 2})`}>{chart.y}</text>{/if}
    {#each chart.series as s, k}
      {#if chart.kind === 'bar'}
        {@const w = band * 0.8 / chart.series.length}
        {#each s.values as v, i}<rect x={x(i) - band * 0.4 + w * k} y={Math.min(y(v), y(0))} width={Math.max(w - 2, 1)} height={Math.abs(y(v) - y(0))} fill={COLORS[k]}><title>{s.name} · {chart.labels[i]}: {v}</title></rect>{/each}
      {:else}
        <polyline points={s.values.map((v, i) => `${x(i)},${y(v)}`).join(' ')} fill="none" stroke={COLORS[k]} stroke-width="2.5" />
        {#each s.values as v, i}<circle cx={x(i)} cy={y(v)} r="3.5" fill={COLORS[k]}><title>{s.name} · {chart.labels[i]}: {v}</title></circle>{/each}
      {/if}
    {/each}
  </svg>
  {#if chart.series.length > 1 || chart.series[0].name}<div class="legend">{#each chart.series as s, k}<span><i style:background={COLORS[k]}></i>{s.name}</span>{/each}</div>{/if}
  {#if chart.note}<p class="muted"><Formatted text={chart.note} /></p>{/if}
</figure>

<style>
  .chart { margin: 0 0 24px; padding: 16px; border: 1px solid var(--line); border-radius: var(--radius); background: var(--surface); }
  figcaption { margin-bottom: 8px; }
  svg { width: 100%; height: auto; display: block; }
  text { font-size: 12px; fill: var(--muted); }
  .axis-name { fill: var(--ink); }
  .grid { stroke: var(--line); }
  .axis { stroke: var(--line-strong); }
  .legend { display: flex; flex-wrap: wrap; gap: 16px; font-size: 14px; margin-top: 8px; }
  .legend i { display: inline-block; width: 12px; height: 12px; border-radius: 3px; margin-right: 6px; vertical-align: -1px; }
  p { margin: 8px 0 0; }
</style>
