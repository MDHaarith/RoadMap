<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { toast, message } from '../lib/state.svelte';
  import { slide } from '../lib/motion';
  import type { Usage, Tally } from '../lib/types';
  import Icon from '../components/Icon.svelte';
  import BusyLabel from '../components/BusyLabel.svelte';
  import Waiting from '../components/Waiting.svelte';
  import LoadError from '../components/LoadError.svelte';
  import { confirmAsk } from '../components/Confirm.svelte';
  type ByModel = Record<string, Tally>;
  type Row = { key: string; name: string; sub?: string; href?: string; service?: string; t: ByModel; create?: ByModel; tutor?: ByModel; use?: ByModel };
  let u = $state<Usage>(), prices = $state<Record<string, number[]>>({}), error = $state(''), busy = $state(false), view = $state<'roadmap' | 'model' | 'day'>('roadmap');
  let alive = true;
  function load() { error = ''; api<{ usage: Usage; prices: Record<string, number[]> }>('GET', 'api/usage').then(r => { if (alive) { u = r.usage ?? {}; prices = r.prices ?? {}; } }).catch(e => { if (alive) error = message(e); }); }
  onMount(() => { load(); return () => { alive = false; }; });

  const big = (x = 0) => Intl.NumberFormat('en', { notation: 'compact', maximumSignificantDigits: 3 }).format(x);
  const n = (x = 0) => x.toLocaleString('en');
  const pct = (x: number) => `${(x * 100).toFixed(x < 0.1 && x > 0 ? 1 : 0)}%`;
  const money = (c: number) => `$${c.toFixed(c > 0 && c < 0.01 ? 4 : 2)}`;
  // Day numbers count days since 1970 on the learner's calendar, so the UTC date of that many days is the right date.
  const date = (d: number, long = false) => new Date(d * 864e5).toLocaleDateString('en', { weekday: long ? undefined : 'short', day: 'numeric', month: 'short', year: long ? 'numeric' : undefined, timeZone: 'UTC' });
  const KEYS = ['in', 'cached', 'out', 'calls'] as const;
  const add = (into: ByModel, from: ByModel = {}, sign = 1) => { for (const [m, x] of Object.entries(from)) { const t = into[m] ??= {}; for (const k of KEYS) t[k] = Math.max(0, (t[k] ?? 0) + sign * (x[k] ?? 0)); } return into; };
  const sum = (t: ByModel = {}) => Object.values(t).reduce<Record<typeof KEYS[number], number>>((a, x) => ({ in: a.in + (x.in ?? 0), cached: a.cached + (x.cached ?? 0), out: a.out + (x.out ?? 0), calls: a.calls + (x.calls ?? 0) }), { in: 0, cached: 0, out: 0, calls: 0 });
  const tokens = (t: ByModel = {}) => { const s = sum(t); return s.in + s.out; };
  const hits = (t: ByModel = {}) => { const s = sum(t); return s.in ? s.cached / s.in : 0; };

  // A ChatGPT sign-in and Claude Code are paid by the learner's plan, so their models cost nothing here.
  const PLANS: Record<string, string> = { 'ChatGPT (sign in)': 'ChatGPT', 'Claude Code on this computer': 'Claude' };
  const serviceOf = (m: string) => u?.models?.[m]?.service ?? '';
  const planOf = (m: string) => PLANS[serviceOf(m)] ?? '';
  // Dollars for usage split by model; null when a model in it has no price yet.
  function cost(t: ByModel = {}): number | null {
    let total = 0;
    for (const [m, x] of Object.entries(t)) {
      if (planOf(m)) continue;
      const p = prices[m];
      if (!p) { if ((x.in ?? 0) + (x.out ?? 0)) return null; continue; }
      const cached = Math.min(x.cached ?? 0, x.in ?? 0);
      total += (((x.in ?? 0) - cached) * p[0] + cached * p[1] + (x.out ?? 0) * p[2]) / 1e6;
    }
    return total;
  }
  const inPlan = (t: ByModel = {}) => Object.keys(t).length > 0 && Object.keys(t).every(m => planOf(m));
  // The headline of an amount: dollars when known, else the tokens.
  const amount = (t: ByModel = {}) => { const c = cost(t); return inPlan(t) ? 'In plan' : c === null ? big(tokens(t)) : money(c); };
  const detail = (t: ByModel = {}) => cost(t) === null && !inPlan(t) ? 'tokens, no price' : `${big(tokens(t))} tokens`;

  // Each service keeps its own color wherever it appears; slots follow the chart palette's fixed order.
  const SERVICES = ['ChatGPT (sign in)', 'Claude (Anthropic)', 'DeepSeek', 'Google Gemini', 'Qwen (Alibaba)', 'Kimi (Moonshot)', 'GLM (Z.ai)', 'OpenRouter'];
  // Claude Code shares the Claude color: both are Claude models, and the name always shows beside it.
  const color = (s = '') => { const i = SERVICES.indexOf(s === 'Claude Code on this computer' ? 'Claude (Anthropic)' : s); return i < 0 ? 'var(--series-other)' : `var(--series-${i + 1})`; };

  let models = $derived(Object.fromEntries(Object.entries(u?.models ?? {}).map(([m, t]) => [m, { in: t.in, cached: t.cached, out: t.out, calls: t.calls }])) as ByModel);
  let total = $derived(sum(models));
  let all = $derived(total.in + total.out);
  const phase = (k: 'create' | 'tutor') => Object.values(u?.roadmaps ?? {}).reduce<ByModel>((a, r) => add(a, r[k]), {});
  // Where the tokens went: making roadmaps, the tutor, and everything else (practice, videos, exams, notes).
  let activities = $derived.by(() => {
    const create = phase('create'), tutor = phase('tutor');
    const rest = add(add(add({}, models), create, -1), tutor, -1);
    const made = Object.entries(u?.roadmaps ?? {}).filter(([id, r]) => !id.startsWith('new:') && tokens(r.create)).length;
    const asked = Object.values(u?.roadmaps ?? {}).filter(r => tokens(r.tutor)).length;
    return [
      { key: 'create', name: 'Making roadmaps', note: made ? `${made} roadmap${made === 1 ? '' : 's'}${cost(create) ? ` · ${money(cost(create)! / made)} each` : ''}` : 'Placement, blueprint, writing, videos', t: create, c: 'var(--accent)' },
      { key: 'tutor', name: 'Tutor', note: asked ? `${n(sum(tutor).calls)} requests in ${asked} roadmap${asked === 1 ? '' : 's'}` : 'No questions yet', t: tutor, c: '#6a9fe0' },
      { key: 'rest', name: 'Learning tools', note: 'Practice, exams, video picks, notes', t: rest, c: 'var(--accent-line)' },
    ];
  });
  let services = $derived.by(() => {
    const by = new Map<string, ByModel>();
    for (const [m, t] of Object.entries(u?.models ?? {})) { const s = t.service || 'Other services'; by.set(s, add(by.get(s) ?? {}, { [m]: t })); }
    return [...by].map(([name, t]) => ({ name, t, n: tokens(t) })).sort((a, b) => b.n - a.n);
  });
  let saved = $derived(Object.entries(models).reduce((a, [m, x]) => a + (prices[m] && !planOf(m) ? (x.cached ?? 0) * (prices[m][0] - prices[m][1]) / 1e6 : 0), 0));

  let rows = $derived.by((): Row[] => {
    if (!u) return [];
    if (view === 'model') return Object.entries(u.models ?? {}).map(([m, t]) => ({ key: m, name: m, sub: t.service, service: t.service, t: { [m]: t } }));
    if (view === 'day') return Object.entries(u.days ?? {}).map(([d, t]) => ({ key: d, name: date(+d), t })).sort((a, b) => +b.key - +a.key);
    const out: Row[] = [], unfinished: ByModel = {};
    for (const [id, r] of Object.entries(u.roadmaps ?? {})) {
      if (id.startsWith('new:')) add(unfinished, r.create);
      else out.push({ key: id, name: r.title || 'Deleted roadmap', href: r.title ? `#/c/${id}` : undefined, t: add(add(add({}, r.create), r.tutor), r.use), create: r.create, tutor: r.tutor, use: r.use });
    }
    if (Object.keys(unfinished).length) out.push({ key: 'new', name: 'Roadmaps not finished', sub: 'Started, then left before saving', t: unfinished, create: unfinished });
    return out.sort((a, b) => tokens(b.t) - tokens(a.t));
  });
  let top = $derived(Math.max(1, ...rows.map(r => tokens(r.t))));
  let unpriced = $derived(Object.keys(models).filter(m => !prices[m] && !planOf(m)));

  // Price fields as typed; a model's price is saved once all three are filled, and forgotten when all three are empty.
  const typed: Record<string, string[]> = {};
  async function setPrice(m: string, k: number, v: string) {
    const f = typed[m] ??= (prices[m] ?? []).map(String).concat(['', '', '']).slice(0, 3);
    f[k] = v.trim();
    const price = f.every(x => x === '') ? null : f.every(x => x !== '' && Number(x) >= 0) ? f.map(Number) : undefined;
    if (price === undefined) return;
    try {
      await api('PUT', 'api/prices', { model: m, price });
      const next = { ...prices };
      if (price) next[m] = price; else delete next[m];
      prices = next;
    } catch (e) { toast(message(e)); }
  }
  async function reset() {
    if (!(await confirmAsk({ message: 'Clear every count on this page and start again? Prices stay.', action: 'Clear counts' }))) return;
    busy = true;
    try { await api('PUT', 'api/usage'); u = { since: Math.floor(Date.now() / 864e5) }; }
    catch (e) { toast(message(e)); } finally { busy = false; }
  }
</script>

{#snippet cell(t: ByModel | undefined)}<td>{#if tokens(t)}<span class="us-amt">{amount(t)}</span><small>{detail(t)}</small>{:else}<span class="us-none">—</span>{/if}</td>{/snippet}
{#snippet meter(x: number, c = 'var(--accent)')}<span class="us-meter" aria-hidden="true"><span style:width={`${Math.max(2, x * 100)}%`} style:background={c}></span></span>{/snippet}

<div class="wrap screen-enter usage-page" class:narrow={!u}><a class="back-map" href="#/settings"><Icon name="back" /> Settings</a>
  <div class="page-head"><div><h1>Usage</h1>{#if u && all}<p class="us-period">{n(total.calls)} AI requests{u.since ? ` since ${date(u.since, true)}` : ''}</p>{/if}</div>{#if u && all}<button type="button" class="btn secondary sm" disabled={busy} onclick={() => void reset()}><BusyLabel {busy} idle="Reset counter" label="Clearing…" /></button>{/if}</div>
  {#if error}<LoadError what="Usage could not load." {error} retry={load} />
  {:else if !u}<Waiting inline title="Adding up your usage" />
  {:else if !all}<p class="empty-state">No AI requests counted yet{u.since ? ` since ${date(u.since, true)}` : ''}. Build a roadmap or ask the tutor, and the tokens show up here.</p>
  {:else}
  <section class="us-spend" aria-label="Where it went">
    <div class="us-card us-total"><h2>Total</h2><p class="us-big">{amount(models)}</p><p class="us-small">{big(all)} tokens · {big(total.out)} written by the AI</p>
      <div class="us-bar" role="img" aria-label={activities.map(a => `${a.name} ${pct(tokens(a.t) / all)}`).join(', ')}>{#each activities as a (a.key)}{#if tokens(a.t)}<span style:flex-grow={tokens(a.t)} style:background={a.c}></span>{/if}{/each}</div></div>
    {#each activities as a (a.key)}<div class="us-card"><h2><i style:background={a.c} aria-hidden="true"></i>{a.name}<span>{pct(tokens(a.t) / all)}</span></h2><p class="us-mid">{tokens(a.t) ? amount(a.t) : '—'}</p><p class="us-small">{tokens(a.t) ? `${detail(a.t)} · ` : ''}{a.note}</p></div>{/each}
  </section>

  <section class="us-pair">
    <div class="us-card"><h2>Cache</h2>
      <div class="us-cache"><p class="us-mid">{pct(hits(models))}</p><p class="us-small">of the {big(total.in)} tokens sent were read from the service's cache{saved ? `, saving ${money(saved)}` : ''}.</p></div>
      <ul class="us-list">{#each services as s (s.name)}<li><i style:background={color(s.name)} aria-hidden="true"></i><span>{s.name}</span>{@render meter(hits(s.t), color(s.name))}<b>{pct(hits(s.t))}</b></li>{/each}</ul>
    </div>
    <div class="us-card"><h2>Services</h2>
      <ul class="us-list">{#each services as s (s.name)}<li><i style:background={color(s.name)} aria-hidden="true"></i><span>{s.name}<small>{n(sum(s.t).calls)} requests · {big(s.n)} tokens</small></span>{@render meter(s.n / all, color(s.name))}<b>{amount(s.t)}</b></li>{/each}</ul>
    </div>
  </section>

  <section class="us-card us-break" aria-labelledby="us-break-h">
    <div class="us-break-head"><h2 id="us-break-h">Breakdown</h2>
      <div class="segmented" role="radiogroup" aria-label="Break down by">{#each [['roadmap', 'Roadmap'], ['model', 'Model'], ['day', 'Day']] as [v, label] (v)}<label><input type="radio" name="us-view" value={v} checked={view === v} onchange={() => view = v as typeof view} />{label}</label>{/each}</div></div>
    <div class="us-scroll"><table class="us-table">
      <thead><tr><th scope="col">{view === 'model' ? 'Model' : view === 'roadmap' ? 'Roadmap' : 'Day'}</th>{#if view === 'roadmap'}<th scope="col">Making it</th><th scope="col">Tutor</th><th scope="col">Learning tools</th>{:else}<th scope="col">Requests</th>{/if}<th scope="col">Cache</th><th scope="col">Total</th></tr></thead>
      <tbody>{#each rows as r (r.key)}<tr>
        <th scope="row"><span class="us-name">{#if r.service}<i style:background={color(r.service)} aria-hidden="true"></i>{/if}<span>{#if r.href}<a href={r.href}>{r.name}</a>{:else}{r.name}{/if}{#if r.sub}<small>{r.sub}</small>{/if}</span></span></th>
        {#if view === 'roadmap'}{@render cell(r.create)}{@render cell(r.tutor)}{@render cell(r.use)}{:else}<td>{n(sum(r.t).calls)}</td>{/if}
        <td>{sum(r.t).in ? pct(hits(r.t)) : '—'}</td>
        <td class="us-sum"><span class="us-amt">{amount(r.t)}</span>{@render meter(tokens(r.t) / top)}<small>{detail(r.t)}</small></td>
      </tr>{/each}</tbody>
    </table></div>
    <p class="us-small us-note">Tokens are sent plus received; a service that does not report them gets an estimate.{unpriced.length ? ' Amounts show tokens until every model in them has a price.' : ''}{Object.values(u.models ?? {}).some(t => PLANS[t.service ?? '']) ? ' ChatGPT sign-in and Claude Code usage is paid by your plan.' : ''}</p>
  </section>

  <details class="us-card us-prices" use:slide open={unpriced.length > 0 && unpriced.length === Object.keys(models).filter(m => !planOf(m)).length}><summary><h2>Prices</h2><span class="us-small">{unpriced.length ? `${unpriced.length} model${unpriced.length === 1 ? '' : 's'} without a price` : 'US dollars per million tokens'}</span></summary>
    <div class="us-scroll"><table class="us-table us-price-table">
      <thead><tr><th scope="col">Model</th><th scope="col">Input</th><th scope="col">Cached input</th><th scope="col">Output</th></tr></thead>
      <tbody>{#each Object.keys(models) as m (m)}<tr>
        <th scope="row"><span class="us-name"><i style:background={color(serviceOf(m))} aria-hidden="true"></i><span>{m}<small>{serviceOf(m)}</small></span></span></th>
        {#if planOf(m)}<td colspan="3" class="us-plan">Included in your {planOf(m)} plan</td>
        {:else}{#each ['Input', 'Cached input', 'Output'] as label, k (label)}<td><label class="us-money"><span aria-hidden="true">$</span><input type="number" min="0" step="any" inputmode="decimal" aria-label={`${m} ${label.toLowerCase()} price per million tokens`} value={prices[m]?.[k] ?? ''} onchange={e => void setPrice(m, k, e.currentTarget.value)} /></label></td>{/each}{/if}
      </tr>{/each}</tbody>
    </table></div>
    <p class="us-small us-note">Copy them from each service's pricing page. A model's price saves once all three are filled.</p>
  </details>
  {/if}
</div>
