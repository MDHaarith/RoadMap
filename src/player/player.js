// Plays one YouTube video and tells the app (the parent frame) the YouTube error code if it cannot.
// Codes: 2 bad id, 5 HTML5 player error, 100 removed or private, 101/150 embedding disabled; 0 means YouTube never loaded.
(function () {
  var q = new URLSearchParams(location.search);
  var id = q.get('v') || '';
  var start = Math.max(0, parseInt(q.get('start') || '0', 10) || 0);
  var ready = false;
  function report(code) { parent.postMessage({ roadmapPlayer: code }, '*'); }
  if (!/^[A-Za-z0-9_-]{11}$/.test(id)) { report(2); return; }
  window.onYouTubeIframeAPIReady = function () {
    ready = true;
    new YT.Player('player', {
      videoId: id,
      width: '100%',
      height: '100%',
      playerVars: { autoplay: 1, start: start, playsinline: 1, rel: 0 },
      events: { onError: function (e) { report(e.data); } }
    });
  };
  setTimeout(function () { if (!ready) report(0); }, 15000);
})();
