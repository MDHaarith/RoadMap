<script lang="ts">
  import { untrack } from 'svelte';
  import { api } from '../lib/api';
  import { appState, toast, message, saveFirst } from '../lib/state.svelte';
  import { enc } from '../lib/format';
  import type { Course, VideoPrefs } from '../lib/types';
  import Select from './Select.svelte';
  import BusyLabel from './BusyLabel.svelte';
  const uid = $props.id();
  let { initial, course }: { initial?: VideoPrefs; course?: Course } = $props();
  let prefs = $state<VideoPrefs>({ style: '', length: '', lang: '', recent: '', ...untrack(() => initial) });
  let busy = $state(false);
  async function save() {
    busy = true;
    try {
      if (course) {
        await saveFirst();
        await api('PUT', 'api/courses/' + enc(course.id) + '/prefs', { ...prefs, refresh: true });
        course.prefs = { ...prefs };
        Object.assign(course, await api<Course>('GET', 'api/courses/' + enc(course.id)));
      } else { await api('PUT', 'api/video-prefs', prefs); appState.config.videos = { ...prefs }; }
      toast(course ? 'Saved. Videos for unfinished lessons are being picked again.' : 'Video preferences saved.');
    } catch (error) { toast(message(error)); } finally { busy = false; }
  }
</script>
<form onsubmit={e => { e.preventDefault(); void save(); }}>
  <div class="prefs-grid">
    <div><label for={`${uid}-style`}>Style</label><Select id={`${uid}-style`} value={prefs.style} onchange={v => prefs.style = v} options={[{value: '', label: course ? 'Use Settings' : 'Thorough and clear'}, {value: 'popular', label: 'Most watched'}, {value: 'depth', label: 'In depth / most thorough'}, {value: 'gems', label: 'Hidden gems'}]} /></div>
    <div><label for={`${uid}-length`}>Length</label><Select id={`${uid}-length`} value={prefs.length} onchange={v => prefs.length = v} options={[{value: '', label: course ? 'Use Settings' : 'Any length'}, {value: 'short', label: 'Under 20 minutes'}, {value: 'medium', label: '10–60 minutes'}, {value: 'long', label: 'Over 30 minutes / full courses'}]} /></div>
    <div><label for={`${uid}-lang`}>Language</label><input id={`${uid}-lang`} bind:value={prefs.lang} maxlength="40" pattern={"[\\p{L} ]*"} placeholder={course ? 'Use Settings' : 'Lesson’s language'} /></div>
    <div><label for={`${uid}-recent`}>Recent uploads</label><Select id={`${uid}-recent`} value={prefs.recent} onchange={v => prefs.recent = v} options={[{value: '', label: course ? 'Use Settings' : 'Default'}, {value: 'yes', label: 'Prefer last 3 years'}, {value: 'no', label: 'No preference for recent uploads'}]} /></div>
  </div><button class="btn secondary sm" disabled={busy}><BusyLabel {busy} idle="Save video preferences" label="Saving…" /></button>
</form>
<style>.prefs-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:8px;margin:8px 0}.prefs-grid input{width:100%;min-width:0}.prefs-grid label{margin-top:0}</style>
