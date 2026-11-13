<script lang="ts">
  import { untrack } from 'svelte';
  import { api } from '../lib/api';
  import { appState, toast, message, flushSave } from '../lib/state.svelte';
  import { enc } from '../lib/format';
  import type { Course, VideoPrefs } from '../lib/types';
  let { initial, course }: { initial?: VideoPrefs; course?: Course } = $props();
  let prefs = $state<VideoPrefs>({ style: '', length: '', lang: '', recent: '', ...untrack(() => initial) });
  let busy = $state(false);
  async function save() {
    busy = true;
    try {
      if (course) {
        await flushSave();
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
    <label>Style<select bind:value={prefs.style}><option value="">{course ? 'Use Settings' : 'Thorough and clear'}</option><option value="popular">Most watched</option><option value="depth">In depth / most thorough</option><option value="gems">Hidden gems</option></select></label>
    <label>Length<select bind:value={prefs.length}><option value="">{course ? 'Use Settings' : '10 minutes or longer'}</option><option value="short">Under 20 minutes</option><option value="medium">10–60 minutes</option><option value="long">Over 30 minutes / full courses</option></select></label>
    <label>Language<input bind:value={prefs.lang} maxlength="40" pattern={"[\\p{L} ]*"} placeholder={course ? 'Use Settings' : 'Lesson’s language'} /></label>
    <label>Recent uploads<select bind:value={prefs.recent}><option value="">{course ? 'Use Settings' : 'Default'}</option><option value="yes">Prefer last 3 years</option><option value="no">No preference for recent uploads</option></select></label>
  </div><button class="btn secondary sm" disabled={busy}>{busy ? 'Saving…' : 'Save video preferences'}</button>
</form>
<style>.prefs-grid{display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:8px;margin:8px 0}.prefs-grid input,.prefs-grid select{width:100%;min-width:0}</style>
