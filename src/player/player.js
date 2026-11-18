// Plays one YouTube video under the app's own controls and tells the app (the parent frame) the YouTube error code if it cannot.
// Codes: 2 bad id, 5 HTML5 player error, 100 removed or private, 101/150 embedding disabled; 0 means YouTube never loaded.
// The app moves the video with postMessage({ roadmapSeek: seconds }), so timestamps jump without reloading the player,
// sends media-key presses as { roadmapCommand: 'toggle' | 'play' | 'pause' | 'back' | 'forward' }, and hears { roadmapPlaying } back.
(function () {
  var q = new URLSearchParams(location.search);
  var id = q.get('v') || '';
  var start = Math.max(0, parseInt(q.get('start') || '0', 10) || 0);
  // local=1: the app saved this video for offline use, and this server plays its own copy instead of YouTube.
  var local = q.get('local') === '1';
  var ready = false, player = null, pending = null, dragging = false, idleTimer = 0;
  var $ = function (s) { return document.getElementById(s); };
  var wrap = $('wrap'), seek = $('seek'), time = $('time');
  function report(code) { parent.postMessage({ roadmapPlayer: code }, '*'); }
  if (!/^[A-Za-z0-9_-]{11}$/.test(id)) { report(2); return; }

  function clock(s) {
    s = Math.max(0, Math.floor(s || 0));
    var h = Math.floor(s / 3600), m = Math.floor(s / 60) % 60, x = String(s % 60).padStart(2, '0');
    return h ? h + ':' + String(m).padStart(2, '0') + ':' + x : m + ':' + x;
  }
  function playing() { return player && player.getPlayerState && player.getPlayerState() === 1; }
  function toggle() { if (!player) return; playing() ? player.pauseVideo() : player.playVideo(); }
  function jump(t) {
    if (!player || !player.seekTo) { pending = t; return; }
    var d = player.getDuration() || Infinity;
    player.seekTo(Math.max(0, Math.min(t, d - 0.5)), true); player.playVideo();
    wrap.classList.remove('ended'); tick();
  }
  function tick() {
    if (!player || !player.getCurrentTime) return;
    var d = player.getDuration() || 0, t = player.getCurrentTime() || 0;
    seek.max = String(Math.floor(d));
    if (!dragging) seek.value = String(Math.floor(t));
    time.textContent = clock(dragging ? +seek.value : t) + ' / ' + clock(d);
    showCaption();
  }
  function wake() {
    wrap.classList.remove('idle'); clearTimeout(idleTimer);
    idleTimer = setTimeout(function () { if (playing() && !dragging) wrap.classList.add('idle'); }, 2500);
  }
  function fullscreen() { document.fullscreenElement ? document.exitFullscreen() : wrap.requestFullscreen && wrap.requestFullscreen(); }
  function setMuteIcon() {
    $('mute').title = player && player.isMuted() ? 'Unmute (M)' : 'Mute (M)';
    $('mute-icon').setAttribute('d', player && player.isMuted()
      ? 'M16.5 12 19 9.5 17.6 8 15 10.6 12.4 8 11 9.5l2.5 2.5-2.5 2.5 1.4 1.4 2.6-2.5 2.5 2.5 1.4-1.4zM3 9v6h4l5 5V4L7 9H3z'
      : 'M3 9v6h4l5 5V4L7 9H3zm13.5 3A4.5 4.5 0 0 0 14 8v8a4.5 4.5 0 0 0 2.5-4z');
  }

  // YouTube draws its title bar and suggestions at the edges of its frame. The frame is made taller than the box so those
  // edges fall outside it, and twice as large then scaled to half, because YouTube picks quality by player size.
  function layout() {
    if (local) return;
    var f = $('player'), W = wrap.clientWidth, H = wrap.clientHeight;
    var vw = Math.min(W, H * 16 / 9), vh = vw * 9 / 16, pad = vh * 0.2;
    f.style.left = (W - vw) / 2 + 'px'; f.style.top = (H - vh) / 2 - pad + 'px';
    f.style.width = 2 * vw + 'px'; f.style.height = 2 * (vh + 2 * pad) + 'px';
    f.style.transform = 'scale(.5)'; f.style.transformOrigin = '0 0';
  }
  window.addEventListener('resize', layout);
  document.addEventListener('fullscreenchange', layout);

  // Captions come from the app (postMessage({ roadmapCaptions: [[startMs, endMs, text], ...] })) and are drawn here,
  // because YouTube's own caption module is undocumented and does not switch on reliably from the API.
  var cc = $('cc'), caption = $('caption'), cues = null, ccOn = false;
  function captions(on) {
    if (!cues || !cues.length) return;
    ccOn = on; cc.setAttribute('aria-pressed', String(on)); showCaption();
  }
  function showCaption() {
    var t = player && player.getCurrentTime ? player.getCurrentTime() * 1000 : 0;
    var now = ccOn && cues ? cues.filter(function (c) { return c[0] <= t && t < c[1]; }).slice(-2).map(function (c) { return c[2]; }).join('\n') : '';
    caption.hidden = !now;
    if (caption.firstChild.textContent !== now) caption.firstChild.textContent = now;
  }
  cc.addEventListener('click', function () { captions(!ccOn); });

  $('shield').addEventListener('click', toggle);
  $('shield').addEventListener('dblclick', fullscreen);
  $('play').addEventListener('click', toggle);
  $('replay').addEventListener('click', function () { jump(0); });
  $('full').addEventListener('click', fullscreen);
  $('mute').addEventListener('click', function () { if (!player) return; player.isMuted() ? player.unMute() : player.mute(); setTimeout(setMuteIcon, 50); });
  $('rate').addEventListener('change', function () { if (player) player.setPlaybackRate(+this.value); });
  seek.addEventListener('input', function () { dragging = true; tick(); });
  seek.addEventListener('change', function () { dragging = false; jump(+seek.value); });
  wrap.addEventListener('mousemove', wake);
  document.addEventListener('keydown', function (e) {
    if (!player || e.target.tagName === 'SELECT' || e.target.tagName === 'INPUT' || e.altKey || e.ctrlKey || e.metaKey) return;
    var k = e.key.toLowerCase(), t = player.getCurrentTime();
    if (k === ' ' && e.target.tagName === 'BUTTON') return;
    if (k === ' ' || k === 'k' || k === 'mediaplaypause') toggle();
    else if (k === 'mediarewind') jump(t - 10);
    else if (k === 'mediafastforward') jump(t + 10);
    else if (k === 'arrowleft' || k === 'j') jump(t - (k === 'j' ? 10 : 5));
    else if (k === 'arrowright' || k === 'l') jump(t + (k === 'l' ? 10 : 5));
    else if (k === 'f') fullscreen();
    else if (k === 'm') $('mute').click();
    else if (k === 'c') captions(!ccOn);
    else return;
    e.preventDefault(); wake();
  });
  window.addEventListener('message', function (e) {
    if (e.source === parent && Array.isArray(e.data && e.data.roadmapCaptions)) {
      cues = e.data.roadmapCaptions.filter(function (c) { return Array.isArray(c) && typeof c[0] === 'number' && typeof c[1] === 'number' && typeof c[2] === 'string'; });
      cc.disabled = !cues.length; cc.title = cues.length ? 'Captions (C)' : 'This video has no captions';
      return;
    }
    var cmd = e.source === parent && e.data && e.data.roadmapCommand;
    if (cmd && player) {
      if (cmd === 'toggle') toggle();
      else if (cmd === 'play') player.playVideo();
      else if (cmd === 'pause') player.pauseVideo();
      else if (cmd === 'back' || cmd === 'forward') jump(player.getCurrentTime() + (cmd === 'back' ? -10 : 10));
      return;
    }
    var t = e.data && e.data.roadmapSeek;
    if (e.source === parent && typeof t === 'number' && isFinite(t)) jump(t);
  });

  // YouTube states: 1 playing, 2 paused, 0 ended.
  function stateChanged(s) {
    $('play-icon').setAttribute('d', s === 1 ? 'M6 5h4v14H6zm8 0h4v14h-4z' : 'M8 5v14l11-7z');
    $('play').setAttribute('aria-label', s === 1 ? 'Pause' : 'Play');
    $('play').title = s === 1 ? 'Pause (K or Space)' : 'Play (K or Space)';
    if (s === 1 || s === 2 || s === 0) parent.postMessage({ roadmapPlaying: s === 1 }, '*');
    wrap.classList.toggle('ended', s === 0);
    wake(); tick();
  }
  // The saved copy in a <video>, behind the same few calls the YouTube player answers, so the controls above work unchanged.
  if (local) {
    ready = true;
    var v = document.createElement('video');
    v.id = 'player'; v.autoplay = true; v.playsInline = true;
    v.style.cssText = 'position:absolute;inset:0;width:100%;height:100%;background:#000';
    v.src = '/video/' + id + '.mp4';
    $('player').replaceWith(v);
    var state = function () { return v.ended ? 0 : v.paused ? 2 : 1; };
    player = {
      getPlayerState: state, playVideo: function () { v.play().catch(function () {}); }, pauseVideo: function () { v.pause(); },
      seekTo: function (t) { v.currentTime = t; }, getDuration: function () { return v.duration || 0; }, getCurrentTime: function () { return v.currentTime; },
      isMuted: function () { return v.muted; }, mute: function () { v.muted = true; }, unMute: function () { v.muted = false; },
      setPlaybackRate: function (r) { v.playbackRate = r; }
    };
    ['play', 'pause', 'ended'].forEach(function (e) { v.addEventListener(e, function () { stateChanged(state()); }); });
    v.addEventListener('error', function () { report(5); });
    v.addEventListener('loadedmetadata', function () { if (start) v.currentTime = start; if (pending !== null) { jump(pending); pending = null; } });
    setMuteIcon(); tick(); setInterval(tick, 250); wake();
  }
  window.onYouTubeIframeAPIReady = function () {
    if (local) return;
    ready = true;
    layout();
    new YT.Player('player', {
      videoId: id,
      width: '100%',
      height: '100%',
      playerVars: { autoplay: 1, start: start, playsinline: 1, rel: 0, controls: 0, disablekb: 1, fs: 0, iv_load_policy: 3, cc_load_policy: 0, vq: 'hd1080' },
      events: {
        onReady: function (e) {
          player = e.target; layout(); setMuteIcon(); if (player.unloadModule) player.unloadModule('captions'); tick(); setInterval(tick, 250); wake();
          if (pending !== null) { jump(pending); pending = null; }
        },
        onStateChange: function (e) { stateChanged(e.data); },
        onError: function (e) { report(e.data); }
      }
    });
  };
  setTimeout(function () { if (!ready) report(0); }, 15000);
})();
