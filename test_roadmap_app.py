"""Self-check: python3 test_roadmap_app.py. Runs the real server against a fake AI service and a fake yt-dlp."""
import json, os, stat, subprocess, tempfile, threading, time, types, urllib.error, urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

tmp = tempfile.mkdtemp()
os.environ["XDG_DATA_HOME"] = os.environ["XDG_CONFIG_HOME"] = tmp
import roadmap_app as app

COURSE = {"title": "Bake bread", "summary": "From flour to loaf.", "phases": [
    {"title": "Dough", "blurb": "Start here.", "lessons": [
        {"title": "Mix", "goal": "Mix a dough.", "concepts": ["hydration"], "tasks": ["Mix 500 g flour"], "done": ["Dough holds together"], "query": "how to mix bread dough", "lookups": ["bakers percentage chart"]},
        "not a lesson"]},
    {"title": "Empty phase", "lessons": []}]}


PRACTICE = {"quiz": [{"q": "What is hydration?", "options": ["Water to flour ratio", "Oven heat"], "answer": 0, "why": "It is a ratio."},
                     {"q": "bad answer index", "options": ["a", "b"], "answer": 7}],
            "exercise": {"lang": "python", "title": "Double", "prompt": "Write double(x).", "starter": "def double(x):\n    pass\n",
                         "tests": "import main, sys\nif main.double(2) != 4:\n    print('double(2): expected 4, got', main.double(2)); sys.exit(1)\nprint('All tests passed')"}}


class FakeAI(BaseHTTPRequestHandler):
    def log_message(self, *a): pass

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        assert self.headers["Authorization"] == "Bearer sk-test"
        system = body["messages"][0]["content"]
        assert "JSON object" in system or "1.1 Mix (completed)" in system and "[x] Dough holds together" in system  # tutor sees progress
        if "practice material" in system:
            text = json.dumps(PRACTICE)
        elif "existing, working project" in system:
            user = body["messages"][1]["content"]
            assert "--- app.py ---" in user and "print('hi')" in user and "TOKEN" not in user and ".git" not in user and "node_modules" not in user  # secrets and junk never sent
            text = json.dumps({"title": "Understand demo", "phases": [{"title": "Run it", "lessons": [{"title": "Entry", "files": ["app.py", ".env", "../../etc/passwd", "made_up.py"]}]}]})
        elif "JSON object" in system:
            text = "Here you go:\n```json\n" + json.dumps(COURSE) + "\n```"
        else:
            assert "bakers percentage chart" in system and "def double(x): return x * 2" in system and "Last quiz score: 1 of 1" in system  # tutor sees lookups, code, quiz
            text = "hint for: " + system.split("Current lesson ")[1][:8]
        data = json.dumps({"choices": [{"message": {"content": text}}]}).encode()
        self.send_response(200); self.send_header("Content-Length", str(len(data))); self.end_headers(); self.wfile.write(data)


ai = ThreadingHTTPServer(("127.0.0.1", 0), FakeAI)
threading.Thread(target=ai.serve_forever, daemon=True).start()
base = app.serve()


def call(method, path, body=None, root=None):
    req = urllib.request.Request((root or base) + path, json.dumps(body).encode() if body is not None else None, method=method)
    try:
        with urllib.request.urlopen(req) as r:
            return r.status, json.loads(r.read()) if "json" in r.headers["Content-Type"] else r.read()
    except urllib.error.HTTPError as e:
        return e.code, json.loads(e.read())


origin = base[:base.index("/", 8)]
assert call("GET", "/wrong-token/api/settings", root=origin)[0] == 404  # API hidden without the secret path
assert b"<title>Roadmap</title>" in call("GET", "")[1] and b'<html lang="en" data-theme="light">' in call("GET", "")[1]
assert call("PUT", "api/theme", {"theme": '"><script>'})[0] == 400 and call("PUT", "api/theme", {"theme": "dark"})[0] == 200
assert b'<html lang="en" data-theme="dark">' in call("GET", "")[1] and stat.S_IMODE(os.stat(app.CONF).st_mode) == 0o600
assert call("POST", "api/courses", {"prompt": "bread"})[0] == 400  # no AI service yet
assert call("PUT", "api/settings", {"base": "file:///etc/passwd", "model": "m"})[0] == 400
assert call("PUT", "api/settings", {"base": f"http://127.0.0.1:{ai.server_port}/v1", "model": "m", "key": "sk-test"})[0] == 200
assert call("PUT", "api/settings", {"base": f"http://127.0.0.1:{ai.server_port}/v1/", "model": "m"})[0] == 200  # empty key keeps the old one
assert call("GET", "api/settings")[1] == {"base": f"http://127.0.0.1:{ai.server_port}/v1/", "model": "m", "key_set": True, "theme": "dark"}  # key never returned
assert stat.S_IMODE(os.stat(app.CONF).st_mode) == 0o600

real_run = subprocess.run
app.subprocess = types.SimpleNamespace(TimeoutExpired=subprocess.TimeoutExpired, DEVNULL=subprocess.DEVNULL, run=lambda a, **k: real_run(a, **k) if a[0] != "yt-dlp" else types.SimpleNamespace(returncode=0, stderr="", stdout=json.dumps({"entries": [
    {"id": "aaaaaaaaaaa", "title": "Full lesson", "channel": "Baker", "duration": 900.0},
    {"id": "bbbbbbbbbbb", "title": "A Short", "duration": 40},
    {"id": "<script>", "title": "bad id", "duration": 900}]})))
code, c = call("POST", "api/courses", {"prompt": "bake bread", "level": "Complete beginner"})
assert code == 200 and c["title"] == "Bake bread", c
assert [l["id"] for p in c["phases"] for l in p["lessons"]] == ["1.1"] and len(c["phases"]) == 1  # junk lesson and empty phase dropped
cid = c["id"]

assert c["phases"][0]["lessons"][0]["lookups"] == ["bakers percentage chart"]
for _ in range(50):  # videos are searched in the background right after the course is made
    if call("GET", f"api/courses/{cid}")[1]["phases"][0]["lessons"][0].get("videos"): break
    time.sleep(0.1)
else:
    raise AssertionError("videos were not prefetched")
code, vids = call("POST", f"api/courses/{cid}/videos", {"lesson": "1.1"})
assert code == 200 and [v["id"] for v in vids] == ["aaaaaaaaaaa"], vids

assert call("POST", "api/run", {"course": cid, "lesson": "1.1", "code": ""})[0] == 404  # no exercise yet
code, pr = call("POST", f"api/courses/{cid}/practice", {"lesson": "1.1"})
assert code == 200 and len(pr["quiz"]) == 1 and pr["exercise"]["lang"] == "python", pr  # bad question dropped
if os.path.exists("/usr/bin/bwrap"):
    assert call("POST", "api/run", {"course": cid, "lesson": "1.1", "code": "def double(x): return x + 1", "test": True})[1] == {"ok": False, "output": "double(2): expected 4, got 3\n"}
    assert call("POST", "api/run", {"course": cid, "lesson": "1.1", "code": "def double(x): return x * 2", "test": True})[1] == {"ok": True, "output": "All tests passed\n"}
    leak = call("POST", "api/run", {"course": cid, "lesson": "1.1", "code": f"import os, socket\nprint(os.path.exists({str(app.CONF)!r}), os.listdir('/home'))\ntry:\n    socket.create_connection(('127.0.0.1', {ai.server_port}), 2); print('net')\nexcept OSError: print('no net')"})[1]
    assert leak == {"ok": True, "output": "False []\nno net\n"}, leak  # sandbox: no home, no config, no network

assert call("GET", "api/review")[1]["count"] == 0  # quiz not taken yet
proj = os.path.join(tmp, "demo"); os.makedirs(proj + "/node_modules/x"); os.makedirs(proj + "/.git")
for name, text in (("app.py", "print('hi')"), (".env", "TOKEN=1"), ("node_modules/x/i.js", "x"), (".git/config", "x"), ("README.md", "Demo")):
    open(os.path.join(proj, name), "w").write(text)
code, pc = call("POST", "api/courses", {"folder": proj, "level": "I code already"})
assert code == 200 and pc["folder"] == proj and pc["phases"][0]["lessons"][0]["files"] == ["app.py"], pc  # only real, safe files kept
assert call("POST", "api/file", {"course": pc["id"], "path": "app.py"})[1] == {"text": "print('hi')"}
assert all(call("POST", "api/file", {"course": pc["id"], "path": x})[0] == 404 for x in (".env", "../../etc/passwd", "/etc/passwd", ".git/config", "nope.py"))
assert call("POST", "api/courses", {"folder": os.path.expanduser("~")})[0] == 400
time.sleep(0.3); assert call("DELETE", f"api/courses/{pc['id']}")[0] == 200

assert call("POST", "api/install", {"lang": "python"})[1] == {"ok": True, "output": "Python is already installed."}  # never reaches pkexec
assert call("POST", "api/install", {"lang": "gcc; rm -rf ~"})[0] == 404
app.LANGS["lua"] = app.LANGS["lua"][:2] + ("no-such-program",) + app.LANGS["lua"][3:]
assert app.run_code("lua", "print(1)")["missing"] == "lua"

state = {"checks": {"1.1#0": True}, "done": {"1.1": True}, "notes": {"1.1": "sticky"}, "last": "1.1", "quiz": {"1.1": {"score": 1, "total": 1}}, "code": {"1.1": "def double(x): return x * 2"}}
assert call("PUT", f"api/courses/{cid}/state", state)[0] == 200
assert call("PUT", f"api/courses/{cid}/state", {"checks": []})[0] == 400
assert app.learned() == ["Concepts already learned in their other roadmaps: hydration (from Bake bread)"] and app.learned(cid) == []  # other roadmaps are told what is known
assert call("GET", "api/review")[1]["count"] == 0  # quiz taken today: first review is tomorrow
real_today = app.today; app.today = lambda: real_today() + 1
due = call("GET", "api/review")[1]
assert due["count"] == 1 and due["questions"][0]["q"] == "What is hydration?" and due["questions"][0]["roadmap"] == "Bake bread", due
assert call("POST", "api/review", {"course": cid, "lesson": "1.1", "i": 0, "right": True})[1] == {"step": 0, "due": app.today() + 1}
assert call("GET", "api/review")[1]["count"] == 0  # answered: back the day after
app.today = real_today
twin = {"id": "0123456789ab", "title": "Pizza", "summary": "", "prompt": "pizza", "phases": [{"title": "P", "blurb": "", "lessons": [
    {"id": "1.1", "title": "Dough", "goal": "", "concepts": ["Dough hydration", "hydration", "Yeast"], "tasks": [], "done": [], "query": "q"}]}],
    "state": {"checks": {}, "done": {}, "notes": {}}}
twin["phases"][0]["lessons"][0]["concepts"][0] = "hydration of dough"; COURSE_C = call("GET", f"api/courses/{cid}")[1]["phases"][0]["lessons"][0]["concepts"]
app.save(twin); assert COURSE_C == ["hydration"] and call("GET", "api/courses/0123456789ab")[1]["phases"][0]["lessons"][0]["known"] == {}  # one shared word is not enough
twin["phases"][0]["lessons"][0]["concepts"][0] = "Hydration: water to flour"; app.save(twin)
real_known = app.known; app.known = lambda skip=None: [("hydration: the ratio of water to flour", "Bake bread", False)]
assert call("GET", "api/courses/0123456789ab")[1]["phases"][0]["lessons"][0]["known"] == {"0": "Bake bread"}  # same idea in another roadmap is tagged
app.known = real_known; app.path_of("0123456789ab").unlink()
assert call("PUT", f"api/courses/{cid}/state", state)[0] == 200  # a page saving an old copy keeps the schedule
state["review"] = {"1.1#0": {"step": 0, "due": app.today() + 2}}
assert call("PUT", f"api/courses/{cid}/paused", {"paused": True})[0] == 200 and call("GET", "api/courses")[1][0]["paused"] is True
assert call("PUT", f"api/courses/{cid}/paused", {"paused": False})[0] == 200
saved = call("GET", f"api/courses/{cid}")[1]
assert saved["state"] == state and saved["phases"][0]["lessons"][0]["videos"] == vids and saved["phases"][0]["lessons"][0]["practice"] == pr and app.learned() == ["Concepts already learned in their other roadmaps: hydration (from Bake bread)"]  # videos cached, progress kept
assert call("GET", "api/courses")[1] == [{"id": cid, "title": "Bake bread", "summary": "From flour to loaf.", "total": 1, "done": 1, "paused": False, "project": False, "next": None}]
assert call("GET", f"api/courses/{cid}")[1]["phases"][0]["lessons"][0]["known"] == {}

assert call("POST", "api/chat", {"course": cid, "lesson": "1.1", "messages": [{"role": "user", "content": "hint?"}]})[1] == {"text": "hint for: 1.1: Mix"}
assert call("POST", "api/chat", {"course": "../../etc/passwd", "lesson": "1.1", "messages": []})[0] == 404
assert call("POST", "api/chat", {"course": cid, "lesson": "9.9", "messages": []})[0] == 404
assert call("GET", "api/courses/..%2Fconfig")[0] == 404

assert call("DELETE", f"api/courses/{cid}")[0] == 200
assert call("GET", f"api/courses/{cid}")[0] == 404 and call("GET", "api/courses")[1] == []
print("ok")
