#!/usr/bin/python3
"""Roadmap: describe what you want to learn, get a hands-on course with free YouTube lessons.

A local HTTP server (127.0.0.1, random port, secret path) serves index.html and a JSON API;
a GTK WebKit window shows it.
"""
import json, os, re, secrets, subprocess, sys, tempfile, threading, time, urllib.error, urllib.request
from concurrent.futures import ThreadPoolExecutor
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

HOME = Path.home()
DATA = Path(os.environ.get("XDG_DATA_HOME") or HOME / ".local/share") / "roadmap-app"
CONF = Path(os.environ.get("XDG_CONFIG_HOME") or HOME / ".config") / "roadmap-app" / "config.json"
UI = next((p for p in (Path(__file__).resolve().parent / "index.html", Path("/usr/share/roadmap-app/index.html")) if p.exists()), None)
PICK = lambda: ""  # main() replaces this with the GTK folder dialog
PREFIX = "/" + secrets.token_urlsafe(16) + "/"  # secret path: web pages and other users can't guess the API URL
LOCK = threading.Lock()

COURSE_PROMPT = """You are the course designer inside Roadmap, a desktop app for people who learn by doing. A learner tells you what they want to build or be able to do, and their level. You design the whole course they will follow on their own.

How the app uses your course: each lesson page shows free YouTube videos found with your search query, then the concepts to learn, then searches the learner runs themselves, then tasks the learner does themselves, then a checklist. The app writes a quiz and, for programming lessons, a small coding exercise with tests later, so do not put those in the course. A lesson only counts as complete when the learner has ticked every check. An AI tutor is available on every lesson, and it gives hints, never answers. So the course has to stand on its own: nobody will explain what you meant.

Reply with only a JSON object, with no markdown fences and no text before or after it, in this shape:
{"title": str, "summary": str, "phases": [{"title": str, "blurb": str, "lessons": [{"title": str, "goal": str, "concepts": [str], "tasks": [str], "done": [str], "query": str, "lookups": [str], "warn": str}]}]}

Designing the path
- Build towards the learner's own goal. Every lesson should leave them with something that works or that they can show (a running program, a played phrase, a baked loaf), and the last phase should finish the thing they asked for. If the goal is huge or vague, scope the course to a realistic first version and say in the summary what they will be able to do at the end.
- Start from the stated level. A complete beginner needs setup and the fundamentals the goal depends on, with a small win in the very first lesson. An experienced learner should not sit through basics: go straight to the gaps.
- Teach only what this goal needs. This is not a general course on the language or the craft. A beginner learns a fundamental in the lesson where the project first needs it, using the project as the example, and nothing the project never uses. Someone who already knows how to code gets no lessons on syntax, variables, loops or functions at all.
- The learner may have other roadmaps in this app. If the request ends with a list of concepts they already learned there, do not teach those again: leave them out, or where this course depends on one, put a single short reminder in the concept list in the form "recap: the idea (from the roadmap name)". A concept marked shaky was covered but its quiz went badly: do not skip it, give it a short practice task where this course first uses it. Spend the lessons on what is new.
- Order by dependency: no lesson may need something a later lesson teaches. Each phase is one milestone.
- Use 3 to 8 phases with 2 to 5 lessons each, sized to the goal. One lesson is one sitting of roughly one to two hours, video included.

Fields
- title (course): a short name for what they will build or be able to do. summary: one or two sentences.
- blurb: one sentence on what this phase gets them and why it comes now.
- goal: one sentence saying what the learner can do after this lesson.
- concepts: 2 to 5 specific ideas to understand before the tasks, each a short phrase with enough detail to look up ("why a function that takes an array also needs its length", not "arrays").
- tasks: 1 to 4 things the learner makes or practises. Be concrete about the result: names, numbers, inputs and expected behaviour. Describe what to produce, never how to do it. No solution code and no step-by-step answer, because working it out is the lesson.
- done: 2 to 5 checks the learner can verify by observing something: a test passes, an output matches a stated value, they can play it at a stated tempo, they can explain a named idea without notes. Avoid checks like "I understand X".
- query: the YouTube search a person would type to find a good free tutorial for this lesson's main concept. Use 3 to 8 plain words that name the specific topic and the tool or language, such as "c pointers and arrays explained" or "sourdough stretch and fold technique". No quotes, search operators, channel names or years. Write it in the language the videos should be in.
- lookups: 2 to 4 web searches the learner should run themselves, because finding and reading reference material is a skill the course is also teaching. Each is the exact words to type into a search engine, aimed at something a video does not give them: the official documentation page, a datasheet, a reference chart, a specification, a worked example to compare against. Name the tool, version or part where it matters ("python csv module DictReader documentation", "DHT22 datasheet timing diagram"). Do not repeat the YouTube query.
- warn: one sentence of safety advice when the lesson involves a real hazard (mains voltage, batteries, heat, blades, chemicals, heavy loads, physical strain, irreversible data loss, money, legal limits). Otherwise an empty string.

Write the course in the language the learner used. Every string is plain text; you may put code identifiers and commands in `backticks`."""

PROJECT_PROMPT = """You are the course designer inside Roadmap, a desktop app for people who learn by doing. This learner does not want to build something new. They have an existing, working project on their computer and want to understand it well enough to change it with confidence. You are given their goal, their level, the project's file list and the contents of some of its files. You design the whole course they will follow on their own.

How the app uses your course: each lesson page shows free YouTube videos found with your search query, then the concepts to learn, then searches the learner runs themselves, then the project files to read, then tasks the learner does themselves in the project, then a checklist. A lesson only counts as complete when the learner has ticked every check. An AI tutor that can see the lesson's files is available, and it gives hints, never answers. The app writes a quiz later, so do not put one in the course.

Reply with only a JSON object, with no markdown fences and no text before or after it, in this shape:
{"title": str, "summary": str, "phases": [{"title": str, "blurb": str, "lessons": [{"title": str, "goal": str, "concepts": [str], "files": [str], "tasks": [str], "done": [str], "query": str, "lookups": [str], "warn": str}]}]}

Designing the path
- Follow the code, not a textbook. A good order is: get it running and see what it does; the map of folders and what each is for; one feature traced from the outside in (the entry point, through each layer, to where data ends up); then each main part in the order a change would touch them; then a real change the learner makes themselves. Build towards the learner's stated goal if they named a part or a change they care about.
- Only describe what you can see. You were given the full file list but the contents of only some files. Base lessons on files whose contents you saw. You may point a lesson at a file you only saw by name, but then the task is for the learner to find out what it does, and you must not state what it contains. Never invent a file, function or behaviour.
- The level decides how much language teaching there is. If the learner already codes, teach no programming basics at all: every lesson is about this project's structure, decisions and data flow, and any framework or library they may not know. If the learner is a beginner or cannot code, teach only the language features, tools and ideas this project actually uses, each in the lesson where the code first uses it, with this project's own lines as the example. Never add a general language course, and never teach a feature the project does not use.
- The learner may have other roadmaps in this app. If the request ends with a list of concepts they already learned there, do not teach those again: leave them out, or where this course depends on one, put a single short reminder in the concept list in the form "recap: the idea (from the roadmap name)". A concept marked shaky was covered but its quiz went badly: do not skip it, give it a short practice task where this course first uses it. Spend the lessons on what is new.
- Use 3 to 7 phases with 2 to 5 lessons each, sized to the project. One lesson is one sitting of roughly one hour.

Fields
- title (course): "Understand" plus the project's name or what it is. summary: one or two sentences on what the project does and what the learner will be able to change at the end.
- blurb: one sentence on what this phase gets them and why it comes now.
- goal: one sentence saying what the learner can do or explain after this lesson.
- concepts: 2 to 5 specific ideas needed to read this lesson's code, each a short phrase with enough detail to look up.
- files: 1 to 4 paths from the file list, exactly as written there, that the learner reads in this lesson. Prefer few files and name the function or section to focus on in the tasks.
- tasks: 1 to 4 things the learner does in the project: find where something happens, predict what a run will print and then check, draw the path of one request, explain a function in their own words in the notes, add a temporary log line, make a small change and see its effect. Say what to find or produce, never the answer.
- done: 2 to 5 checks the learner can verify by observing something: the program printed a stated thing, they can name the file and function where X happens without looking, their change shows on screen, the project's tests still pass. Avoid checks like "I understand X".
- query: the YouTube search for the main general concept, library or framework behind this lesson, 3 to 8 plain words. Videos about this exact project will not exist, so search for the technique.
- lookups: 2 to 4 web searches the learner should run themselves, each the exact words to type, aimed at the official documentation for a library, function or tool this lesson's files use. Name the library and the function.
- warn: one sentence when a task could lose data or cost money or touch a live system (deleting files, running migrations, real credentials, production servers), with the safe way to do it, such as working on a copy or a Git branch. Otherwise an empty string.

Write the course in the language the learner used. Every string is plain text; you may put code identifiers, paths and commands in `backticks`."""

SKIP_DIRS = {"node_modules", "target", "build", "dist", "out", "venv", "env", "__pycache__", "vendor", "Pods", "DerivedData", "coverage"}
SECRET = re.compile(r"\.env|secret|credential|password|id_rsa|id_ed25519|\.(pem|key|p12|pfx|keystore|jks)$", re.I)  # never read or send these

TUTOR_PROMPT = """You are the tutor inside Roadmap, a desktop app where people teach themselves by building a real project, one lesson at a time. The learner chose to learn by doing, so the work has to stay theirs: a solution handed over is a lesson they did not get. Your job is to keep them moving and thinking, like a good mentor sitting next to them.

What you can and cannot see: the course outline, the lesson they are on and which of its checks they have ticked are all below. If the course is about an existing project, the lesson's files from that project are below as they are on disk right now, so refer to their real names and lines; you cannot see the rest of the project unless they paste it. If the lesson has a coding exercise, the exercise and the code they have typed into the app for it are below too, along with their last quiz score. You cannot see their screen, their other files, the output of their runs or the lesson videos. If you need their code, an error message or a description of what happened, ask for it. Do not guess what a video said.

Hints
- Never give the solution to a lesson task, in code or in words, even when asked directly. If they ask for the answer, say in one sentence that you won't because working it out is the point, then give the next smallest hint.
- Give the smallest hint that gets them unstuck. First point at the concept or the place to look, usually as a question. If they are still stuck, be more specific about what is wrong. If they are still stuck after that, show the idea on a different, smaller example and let them carry it over.
- An illustration is at most a line or two and uses different names and values from the task.
- Things that are not the point of the lesson get a direct answer: installing a tool, what an error message means, a definition, syntax they have misremembered, a plain fact. Do not make them struggle over those.

Teach them to look things up. When the answer lives in documentation, a datasheet, a manual or a reference, do not recite it at length: give the exact words to search for, say what kind of page to open and what to look for on it, and offer to go through what they find. A learner who can find things no longer needs a tutor, and that is the goal. A one-line fact they just need in order to move on still gets a direct answer.

When they ask for a quiz, ask one question at a time about this lesson's concepts and wait for the answer. Mix recall with "what would happen if" questions. After each answer say what was right, what was wrong and why, then ask the next. After about five questions, name the concept they should revisit, if any.

When they ask you to explain a concept and have not said which, list this lesson's concepts and ask which one. Explain in plain words with one concrete example or analogy at their level, then ask one question to check it landed.

When they ask for a review and have not shared their work, ask them to paste it, or to describe exactly what they made and what happened. Then review it against the "Done when" checks: say which are met, which are not, and why each gap matters. Point to where the problem is and what kind of problem it is; do not rewrite their work. Be honest: say something is good only when it is, and do not call a check met without evidence.

If what they describe could hurt them, damage equipment or lose data, say so plainly and first. Safety outranks the no-answers rule.

If a list of concepts they learned in their other roadmaps is below, build on it: do not re-explain those from zero, connect the new idea to the one they know and name the roadmap it came from.

Keep to this course. Answer a side question briefly, then bring them back to the lesson. If something belongs to a later lesson, say which one covers it.

Style: you are in a narrow chat panel. Reply in the learner's language, in short plain paragraphs, usually under 150 words; a review may be longer. No headings or tables. Use `backticks` for code identifiers and a fenced block for a snippet of several lines. If you are not sure of something, say so."""


PRACTICE_PROMPT = """You write the practice material for one lesson of a course in Roadmap, a desktop app for people who learn by doing. The lesson and the course it belongs to are given below. The learner sees your quiz as multiple-choice questions in the app, and your exercise in a small code editor with a Run button and a Run tests button.

Reply with only a JSON object, with no markdown fences and no text before or after it, in this shape:
{"quiz": [{"q": str, "options": [str, str, str, str], "answer": int, "why": str}], "exercise": null or {"lang": one of "python", "javascript", "bash", "ruby", "php", "lua", "c", "cpp", "rust", "go", "java", "title": str, "prompt": str, "starter": str, "tests": str}}

Quiz
- Write 5 questions on this lesson's concepts, easiest first. Test understanding, not memory of wording: ask what would happen, which option is wrong and why, which choice fits a situation.
- Each question has 4 options and exactly one correct answer. Make the wrong options the mistakes a learner at this level really makes, each about the same length as the right one. Never use "all of the above" or "none of the above".
- answer is the index of the correct option, 0 to 3. Spread the correct answers across the positions.
- why: one or two sentences saying why the right option is right and what is wrong with the most tempting wrong one.
- Do not ask anything whose answer is the solution to one of the lesson's tasks.

Exercise
- Write one only when the lesson teaches programming and the idea can be practised in a single file in one of the languages listed above. Use the language the lesson teaches. If that language is not in the list, or the lesson is not about programming, or it needs hardware, a network, third-party packages, a GUI or keyboard input, the exercise is null. Do not translate a lesson into a different language. For a microcontroller lesson in C or C++, an exercise is fine only when the idea is plain language logic that runs on a PC.
- It is a warm-up of 10 to 20 minutes on this lesson's main concept. It must be smaller than the lesson's tasks and must not be one of them.
- The app saves the learner's code in one file and your tests in a second file in the same folder, then runs the tests file. There is no network, a short time limit and only the standard library. The file names and how the tests reach the learner's code are fixed per language:
  python: main.py and test.py. Tests do import main.
  javascript: main.js and test.js. Tests do const main = require("./main.js"). The starter must end with a module.exports line that exports every function the tests use.
  bash: main.sh and test.sh. Tests run bash main.sh with arguments, or source it.
  ruby: main.rb and test.rb. Tests do require_relative "main".
  php: main.php and test.php. Tests do require "main.php".
  lua: main.lua and test.lua. Tests do local main = require("main"), so the starter must return a table of its functions.
  c: main.c and test.c. The starter has the functions plus a small main that calls them, so Run shows something. The tests file must begin with these three lines so the learner's main does not clash with yours: #define main learner_main, then #include "main.c", then #undef main. Then write your own main.
  cpp: main.cpp and test.cpp, with the same three lines as C using "main.cpp".
  rust: main.rs and test.rs. The starter has pub functions and a fn main. The tests file begins with #[path = "main.rs"] mod learner; and has its own fn main that calls learner::name and uses std::process::exit(1) on failure. Do not use #[test].
  go: main.go and main_test.go, both package main. The starter has a func main. Tests are ordinary func TestXxx(t *testing.T) functions; use t.Errorf so every case runs.
  java: Main.java with public class Main holding public static methods and a main method, and Test.java with public class Test whose main calls Main.name and ends with System.exit(1) on failure.
- prompt: what to write, in plain text. Name every function with its parameters and return value, and give two or three input and output examples.
- starter: the file the learner starts from: the function signatures with a placeholder body that compiles and a short comment where their code goes. No part of the solution.
- tests: the complete tests file, run as is. Check 4 to 8 cases, including an edge case. Run every case even after one fails. For each failure print one line with the input, the expected value and the actual value. If any case failed, exit with status 1. If all passed, print "All tests passed". A learner must be able to tell what is wrong from the output alone.

Write in the language the course is written in. Keep code identifiers in English."""

# lang: (name, Fedora packages, program that must exist in /usr/bin, learner's file, tests file, run command, test command, seconds allowed)
# Only python3 and bash come with the app. The rest are installed when a lesson first needs them (see install_lang).
LANGS = {
    "python": ("Python", "python3", "python3", "main.py", "test.py", "python3 main.py", "python3 test.py", 10),
    "javascript": ("JavaScript", "nodejs", "node", "main.js", "test.js", "node main.js", "node test.js", 10),
    "bash": ("Bash", "bash", "bash", "main.sh", "test.sh", "bash main.sh", "bash test.sh", 10),
    "ruby": ("Ruby", "ruby", "ruby", "main.rb", "test.rb", "ruby main.rb", "ruby test.rb", 10),
    "php": ("PHP", "php-cli", "php", "main.php", "test.php", "php main.php", "php test.php", 10),
    "lua": ("Lua", "lua", "lua", "main.lua", "test.lua", "lua main.lua", "lua test.lua", 10),
    "c": ("C", "gcc", "gcc", "main.c", "test.c", "gcc -Wall -o prog main.c -lm && ./prog", "gcc -o prog test.c -lm && ./prog", 30),
    "cpp": ("C++", "gcc-c++", "g++", "main.cpp", "test.cpp", "g++ -Wall -o prog main.cpp && ./prog", "g++ -o prog test.cpp && ./prog", 30),
    "rust": ("Rust", "rust", "rustc", "main.rs", "test.rs", "rustc -o prog main.rs && ./prog", "rustc -o prog test.rs && ./prog", 60),
    "go": ("Go", "golang", "go", "main.go", "main_test.go", "go run main.go", "go test main.go main_test.go", 60),
    "java": ("Java", "java-latest-openjdk-devel", "javac", "Main.java", "Test.java", "javac Main.java && java Main", "javac Main.java Test.java && java Test", 60),
}


def config():
    try:
        return json.loads(CONF.read_text())
    except (OSError, ValueError):
        return {}


def save_config(body):
    c = config()
    base = str(body.get("base", "")).strip()
    if not re.match(r"https?://", base):  # urllib would also open file:// URLs
        raise ValueError("Base URL must start with http:// or https://")
    c.update(base=base, model=str(body.get("model", "")).strip())
    if body.get("key"):
        c["key"] = str(body["key"]).strip()
    write_config(c)


THEMES = ("light", "dark", "system")


def theme():
    t = config().get("theme")
    return t if t in THEMES else "light"


def write_config(c):
    CONF.parent.mkdir(parents=True, exist_ok=True)
    with os.fdopen(os.open(CONF, os.O_WRONLY | os.O_CREAT | os.O_TRUNC, 0o600), "w") as f:  # holds the API key
        json.dump(c, f)


def llm(messages):
    c = config()
    if not (c.get("base") and c.get("model")):
        raise ValueError("Set up the AI service in Settings first.")
    headers = {"Content-Type": "application/json", "User-Agent": "roadmap-app"}
    if c.get("key"):
        headers["Authorization"] = "Bearer " + c["key"]
    req = urllib.request.Request(c["base"].rstrip("/") + "/chat/completions",
                                 json.dumps({"model": c["model"], "messages": messages}).encode(), headers)
    try:
        with urllib.request.urlopen(req, timeout=300) as r:
            return json.load(r)["choices"][0]["message"]["content"]
    except urllib.error.HTTPError as e:
        raise ValueError(f"AI service error {e.code}: {e.read(400).decode(errors='replace')}")
    except urllib.error.URLError as e:
        raise ValueError(f"Can't reach the AI service: {e.reason}")
    except (KeyError, IndexError, TypeError, ValueError):
        raise ValueError("The AI service sent an answer this app can't read.")


def project_file(folder, rel):
    """Text of one project file, or None. Refuses anything outside the folder, hidden, secret-looking, big or binary."""
    root = Path(folder).resolve()
    f = (root / str(rel)).resolve()
    parts = f.relative_to(root).parts if f.is_relative_to(root) else ()
    if not parts or any(x.startswith(".") or x in SKIP_DIRS for x in parts) or SECRET.search(f.name) or not f.is_file() or f.stat().st_size > 200_000:
        return None
    try:
        return f.read_text()
    except (OSError, UnicodeDecodeError):
        return None


def read_project(folder):
    """The file list of a project and as much of its text as fits in one request."""
    root = Path(folder).expanduser().resolve()
    if not root.is_dir() or root in (Path("/"), HOME.resolve()):
        raise ValueError("Choose the folder of one project, not your whole home folder.")
    files = []
    for d, dirs, names in os.walk(root):
        dirs[:] = sorted(x for x in dirs if not x.startswith(".") and x not in SKIP_DIRS)
        files += [str(Path(d, n).relative_to(root)) for n in sorted(names)]
        if len(files) > 3000:
            break
    files = [f for f in files[:3000] if project_file(root, f) is not None]
    if not files:
        raise ValueError("No readable text files in that folder.")
    # shortcut: README, then shallow files first, 6000 characters per file, 60000 in total. Large projects are only sampled;
    # move to asking the model which files it wants in a second request when courses on big projects come out shallow.
    files.sort(key=lambda f: (not f.lower().startswith("readme"), f.count("/"), f))
    text, room = [], 60000
    for f in files:
        body = project_file(root, f)[:6000]
        if len(body) > room:
            break
        room -= len(body)
        text.append(f"--- {f} ---\n{body}")
    return str(root), files, "File list:\n" + "\n".join(files[:600]) + "\n\nContents of some files:\n" + "\n".join(text)


def lesson_files(c, l, room=12000):
    out = []
    for f in l.get("files") or []:
        body = project_file(c["folder"], f) if c.get("folder") else None
        if body is not None and room > 0:
            out.append(f"--- {f} ---\n{body[:room]}")
            room -= len(body)
    return out


def courses():
    files = [f for f in DATA.glob("*.json") if re.fullmatch(r"[0-9a-f]{12}", f.stem)]
    return [json.loads(f.read_text()) for f in sorted(files, key=lambda f: f.stat().st_mtime, reverse=True)]


def learned(skip=None, n=120):
    """What the learner already covered in their other roadmaps: the concepts of every completed lesson. Computed, not stored."""
    out = [f"{x} (from {title}{', shaky' if shaky else ''})" for x, title, shaky in known(skip)]
    return ["Concepts already learned in their other roadmaps: " + "; ".join(out[:n])] if out else []


def known(skip=None):
    """(concept, roadmap title, shaky) for every completed lesson outside one roadmap. Shaky: under 60% on that lesson's quiz."""
    out = []
    for c in courses():
        for p in c["phases"] if c["id"] != skip else []:
            for l in p["lessons"]:
                q = (c["state"].get("quiz") or {}).get(l["id"]) or {}
                if c["state"]["done"].get(l["id"]):
                    out += [(x, c["title"], bool(q.get("total")) and q.get("score", 0) / q["total"] < 0.6) for x in l["concepts"] if not x.startswith("recap:")]
    return out


SMALL = set("a an the of to in on for and or is are how why what with that this it its as by from not when which you your".split())
words = lambda x: set(re.findall(r"\w+", x.lower())) - SMALL


def mark_known(c):
    """Tag each concept of a roadmap that the learner already completed in another one, so the page can say so and they can skim it.
    shortcut: matches by shared words (at least 2, and half of all words), so differently worded twins are missed; use the model to match if that matters."""
    seen = [(words(x), title) for x, title, shaky in known(c["id"]) if not shaky]
    for p in c["phases"]:
        for l in p["lessons"]:
            l["known"] = {}
            for i, x in enumerate(l["concepts"]):
                w = words(x)
                hit = next((t for v, t in seen if len(w & v) >= 2 and len(w & v) * 2 >= len(w | v)), None)
                if hit:
                    l["known"][str(i)] = hit
    return c


STEPS = [1, 3, 7, 16, 35]  # days until a quiz question comes back after each right answer in a row
today = lambda: int(time.time() // 86400)


def review_due(limit=10):
    """Quiz questions due today across all active roadmaps, oldest first. A question joins once its lesson's quiz was taken."""
    due = []
    for c in courses():
        st = c["state"]
        if c.get("paused"):
            continue
        for p in c["phases"]:
            for l in p["lessons"]:
                if l["id"] in (st.get("quiz") or {}):
                    for i, q in enumerate((l.get("practice") or {}).get("quiz") or []):
                        when = ((st.get("review") or {}).get(f"{l['id']}#{i}") or {}).get("due", 0)
                        if when <= today():
                            due.append((when, {"course": c["id"], "roadmap": c["title"], "lesson": l["id"], "lesson_title": l["title"], "i": i, **q}))
    due.sort(key=lambda x: x[0])
    return {"count": len(due), "questions": [q for _, q in due[:limit]]}


def strs(v, n=8):
    return [str(x)[:600] for x in v][:n] if isinstance(v, list) else []


def build_course(prompt, level, folder=None):
    """Ask the model for a course and keep only the fields the app knows (model output is untrusted)."""
    known = set()
    if folder:
        folder, files, digest = read_project(folder)
        known = set(files)
        text = llm([{"role": "system", "content": PROJECT_PROMPT},
                    {"role": "user", "content": "\n".join([f"What I want from this project: {prompt or 'Understand how it works so I can change it.'}", f"My level: {level}", *learned(), "", digest])}])
    else:
        text = llm([{"role": "system", "content": COURSE_PROMPT},
                    {"role": "user", "content": "\n".join([f"What I want to learn or build: {prompt}", f"My level: {level}", *learned()])}])
    d = parse(text, "a course")
    phases = []
    for p in d.get("phases") or []:
        if not isinstance(p, dict):
            continue
        n = len(phases) + 1
        lessons = [{"id": f"{n}.{j}", "title": str(l.get("title") or "Lesson")[:200], "goal": str(l.get("goal") or "")[:600],
                    "concepts": strs(l.get("concepts")), "tasks": strs(l.get("tasks")), "done": strs(l.get("done")),
                    "query": str(l.get("query") or l.get("title") or "")[:200], "lookups": strs(l.get("lookups"), 4), "files": [f for f in strs(l.get("files"), 6) if f in known], "warn": str(l.get("warn") or "")[:600]}
                   for j, l in enumerate([l for l in p.get("lessons") or [] if isinstance(l, dict)], 1)]
        if lessons:
            phases.append({"title": str(p.get("title") or f"Phase {n}")[:200], "blurb": str(p.get("blurb") or "")[:600], "lessons": lessons})
    if not phases:
        raise ValueError("The AI returned a course with no lessons. Try again, or use a stronger model.")
    return {"id": secrets.token_hex(6), "prompt": prompt, "level": level, "folder": folder, "title": str(d.get("title") or prompt)[:200],
            "summary": str(d.get("summary") or "")[:600], "phases": phases,
            "state": {"checks": {}, "done": {}, "notes": {}, "last": "1.1"}}


def parse(text, what):
    try:
        d = json.loads(text[text.index("{"):text.rindex("}") + 1])  # tolerates code fences around the JSON
    except ValueError:
        d = None
    if not isinstance(d, dict):
        raise ValueError(f"The AI didn't return {what}. Try again, or use a stronger model.")
    return d


def build_practice(c, l):
    """Ask the model for a quiz and, for programming lessons, a coding exercise. Model output is untrusted."""
    d = parse(llm([{"role": "system", "content": PRACTICE_PROMPT}, {"role": "user", "content": "\n".join([
        f"Course: {c['title']}", f"The learner's goal: {c['prompt']}", f"Level: {c.get('level') or 'not stated'}",
        f"Lesson {l['id']}: {l['title']}", "Goal: " + l["goal"], "Concepts: " + "; ".join(l["concepts"]),
        "Tasks (do not reuse): " + "; ".join(l["tasks"])] + (
        ["This lesson is about reading the learner's existing project. Ask the quiz about what this code does and why. The exercise is null.", *lesson_files(c, l)]
        if l.get("files") else []))}]), "a quiz")
    quiz = []
    for q in d.get("quiz") or []:
        opts = strs(q.get("options"), 6) if isinstance(q, dict) else []
        if len(opts) > 1 and isinstance(q.get("answer"), int) and 0 <= q["answer"] < len(opts):
            quiz.append({"q": str(q.get("q") or "")[:1000], "options": opts, "answer": q["answer"], "why": str(q.get("why") or "")[:1000]})
    if not quiz:
        raise ValueError("The AI returned a quiz with no usable questions. Try again, or use a stronger model.")
    e = d.get("exercise")
    ok = isinstance(e, dict) and e.get("lang") in LANGS and e.get("tests")
    return {"quiz": quiz[:10], "exercise": {"lang": e["lang"], **{k: str(e.get(k) or "")[:n] for k, n in
            (("title", 200), ("prompt", 4000), ("starter", 8000), ("tests", 20000))}} if ok else None}


def run_code(lang, code, tests=None):
    """Run the learner's code, or the exercise tests against it, in a sandbox. Both the tests and the code are untrusted."""
    name, pkg, prog, main, test, run, check, secs = LANGS[lang]
    if not os.path.exists("/usr/bin/" + prog):  # the sandbox only sees /usr/bin, so a copy in the home folder does not count
        return {"ok": False, "missing": lang, "output": f"{name} is not installed on this computer yet. Install it to run this exercise (package: {pkg})."}
    with tempfile.TemporaryDirectory() as d:
        Path(d, main).write_text(code)
        if tests is not None:
            Path(d, test).write_text(tests)
        # bwrap: no network, the whole system read-only, home and other private folders hidden, only the run folder writable.
        # shortcut: no memory or disk cap beyond the 10 s limit, add a cgroup limit (systemd-run --user --scope) if runs ever hurt the desktop
        box = ["bwrap", "--unshare-all", "--die-with-parent", "--new-session", "--ro-bind", "/", "/", "--dev", "/dev", "--proc", "/proc",
               "--tmpfs", "/tmp", "--tmpfs", "/run", "--tmpfs", "/home", "--tmpfs", "/root", "--tmpfs", "/var", "--tmpfs", "/mnt", "--tmpfs", "/media",
               "--bind", d, "/tmp/work", "--chdir", "/tmp/work", "--clearenv", "--setenv", "PATH", "/usr/bin:/bin", "--setenv", "HOME", "/tmp/work",
               "--setenv", "PYTHONDONTWRITEBYTECODE", "1"]
        try:
            p = subprocess.run(box + ["sh", "-c", check if tests is not None else run], capture_output=True, text=True,
                               errors="replace", timeout=secs, stdin=subprocess.DEVNULL)
        except FileNotFoundError:
            raise ValueError("bubblewrap is not installed. Run: sudo dnf install bubblewrap")
        except subprocess.TimeoutExpired as e:
            out = e.stdout.decode(errors="replace") if isinstance(e.stdout, bytes) else e.stdout or ""
            return {"ok": False, "output": out[-20000:] + f"\nStopped: ran longer than {secs} seconds. Look for a loop that never ends."}
        return {"ok": p.returncode == 0, "output": (p.stdout + p.stderr)[-20000:]}


def install_lang(lang):
    """Install a language the learner asked for. pkexec shows the system password dialog; the package name comes from LANGS, never from the model or the page."""
    name, pkg, prog = LANGS[lang][:3]
    if os.path.exists("/usr/bin/" + prog):
        return {"ok": True, "output": f"{name} is already installed."}
    try:
        p = subprocess.run(["pkexec", "dnf", "install", "-y", pkg], capture_output=True, text=True, errors="replace", timeout=1800, stdin=subprocess.DEVNULL)
    except FileNotFoundError:
        raise ValueError(f"Can't install from inside the app on this system. Run: sudo dnf install {pkg}")
    except subprocess.TimeoutExpired:
        raise ValueError(f"The install took too long and was stopped. Run: sudo dnf install {pkg}")
    if p.returncode in (126, 127):  # pkexec: dialog dismissed or not authorised
        raise ValueError(f"Nothing was installed, because the password dialog was cancelled. You can also run: sudo dnf install {pkg}")
    ok = os.path.exists("/usr/bin/" + prog)
    return {"ok": ok, "output": f"{name} is installed." if ok else f"Install failed. Try: sudo dnf install {pkg}\n" + (p.stdout + p.stderr)[-2000:]}


def prefetch(cid):
    """Find the videos for every lesson right after a course is made, so lessons open ready. Failures are fine: the lesson page searches again."""
    def one(l):
        try:
            return l["id"], find_videos(l["query"])
        except ValueError:
            return l["id"], []
    try:
        with ThreadPoolExecutor(4) as ex:
            found = dict(ex.map(one, [l for p in load(cid)["phases"] for l in p["lessons"]]))
        with LOCK:
            c = load(cid)
            for p in c["phases"]:
                for l in p["lessons"]:
                    if found.get(l["id"]) and not l.get("videos"):
                        l["videos"] = found[l["id"]]
            save(c)
    except FileNotFoundError:  # course deleted while searching
        pass


def find_videos(query):
    try:
        out = subprocess.run(["yt-dlp", "--flat-playlist", "-J", "--no-warnings", "ytsearch8:" + query],
                             capture_output=True, text=True, timeout=60)
    except FileNotFoundError:
        raise ValueError("yt-dlp is not installed. Run: sudo dnf install yt-dlp")
    except subprocess.TimeoutExpired:
        raise ValueError("YouTube search timed out. Check your connection and try again.")
    if out.returncode:
        raise ValueError("YouTube search failed: " + out.stderr.strip()[-300:])
    vids = [{"id": e["id"], "title": str(e.get("title") or ""), "channel": str(e.get("channel") or e.get("uploader") or ""),
             "duration": int(e.get("duration") or 0)}
            for e in json.loads(out.stdout).get("entries") or [] if re.fullmatch(r"[\w-]{11}", str(e.get("id")))]
    return [v for v in vids if not v["duration"] or v["duration"] >= 120][:4]  # under 2 min is a Short, not a lesson


def path_of(cid):
    return DATA / f"{cid}.json"


def load(cid):
    return json.loads(path_of(cid).read_text())


def save(c):
    DATA.mkdir(parents=True, exist_ok=True)
    tmp = path_of(c["id"]).with_suffix(".tmp")
    tmp.write_text(json.dumps(c))
    os.replace(tmp, path_of(c["id"]))  # atomic: a crash never leaves half a course file


def lesson_of(c, lid):
    return next(l for p in c["phases"] for l in p["lessons"] if l["id"] == lid)


def tutor_system(c, l):
    """The tutor's instructions plus what it needs to know about this learner and this lesson."""
    st = c["state"]
    outline = [f"{x['id']} {x['title']}" + (" (completed)" if st["done"].get(x["id"]) else "") for p in c["phases"] for x in p["lessons"]]
    checks = [("[x] " if st["checks"].get(f"{l['id']}#{k}") else "[ ] ") + d for k, d in enumerate(l["done"])]
    ex = (l.get("practice") or {}).get("exercise")
    code = str((st.get("code") or {}).get(l["id"]) or "")[:6000]
    quiz = (st.get("quiz") or {}).get(l["id"])
    extra = ([f"Coding exercise ({ex['lang']}): {ex['prompt']}", "Their code for it right now:\n" + (code or "(nothing typed yet)")] if ex else []) \
        + ([f"Last quiz score: {quiz.get('score')} of {quiz.get('total')}"] if isinstance(quiz, dict) else [])
    return "\n".join([TUTOR_PROMPT, "", f"Course: {c['title']}", f"The learner's goal in their own words: {c['prompt']}",
                      f"Level: {c.get('level') or 'not stated'}", "Lessons: " + "; ".join(outline), "",
                      f"Current lesson {l['id']}: {l['title']}", "Goal: " + l["goal"], "Concepts: " + "; ".join(l["concepts"]),
                      "Tasks: " + "; ".join(l["tasks"]), "Done when, with the learner's ticks: " + "; ".join(checks),
                      "Searches the course told them to run themselves: " + ("; ".join(l.get("lookups") or []) or "none"),
                      "Safety note: " + (l.get("warn") or "none")] + extra + learned(c["id"], 40) + lesson_files(c, l))


def route(method, path, body):
    if (method, path) == ("GET", ""):
        # The theme goes into the page itself so it never flashes the wrong one. Browser storage can't hold it: the port changes every start.
        return UI.read_bytes().replace(b'data-theme="light"', b'data-theme="%s"' % theme().encode(), 1)
    if (method, path) == ("GET", "api/settings"):
        c = config()
        return {"base": c.get("base", ""), "model": c.get("model", ""), "key_set": bool(c.get("key")), "theme": theme()}  # never send the key back
    if (method, path) == ("PUT", "api/theme"):
        if body.get("theme") not in THEMES:
            raise ValueError("Unknown theme.")
        return write_config({**config(), "theme": body["theme"]}) or {}
    if (method, path) == ("PUT", "api/settings"):
        return save_config(body) or {}
    if (method, path) == ("GET", "api/courses"):
        out = []
        for c in courses():  # most recently used first
            ls = [l for p in c["phases"] for l in p["lessons"]]
            nxt = next((l for l in ls if not c["state"]["done"].get(l["id"])), None)
            out.append({"id": c["id"], "title": c["title"], "summary": c["summary"], "total": len(ls), "done": sum(map(bool, c["state"]["done"].values())),
                        "paused": bool(c.get("paused")), "project": bool(c.get("folder")), "next": nxt and {"id": nxt["id"], "title": nxt["title"]}})
        return out
    if (method, path) == ("GET", "api/review"):
        return review_due()
    if (method, path) == ("POST", "api/review"):
        key = f"{body.get('lesson')}#{int(body.get('i', -1))}"
        with LOCK:
            c = load_id(body.get("course"))
            r = c["state"].setdefault("review", {})
            step = min(r.get(key, {}).get("step", -1) + 1, len(STEPS) - 1) if body.get("right") else 0  # a miss starts the question over
            r[key] = {"step": step, "due": today() + STEPS[step]}
            save(c)
        return r[key]
    if (method, path) == ("POST", "api/courses"):
        prompt = str(body.get("prompt", "")).strip()[:2000]
        folder = str(body.get("folder") or "").strip()
        if not prompt and not folder:
            raise ValueError("Say what you want to learn first.")
        c = build_course(prompt, str(body.get("level", ""))[:100], folder)
        save(c)
        threading.Thread(target=prefetch, args=(c["id"],), daemon=True).start()
        return c
    if (method, path) == ("POST", "api/run"):
        l = lesson_of(load_id(body.get("course")), body.get("lesson"))
        ex = (l.get("practice") or {}).get("exercise")
        if not ex:
            raise KeyError("exercise")
        return run_code(ex["lang"], str(body.get("code", ""))[:100000], ex["tests"] if body.get("test") else None)
    if (method, path) == ("POST", "api/pick-folder"):
        return {"folder": PICK()}
    if (method, path) == ("POST", "api/file"):
        text = project_file(load_id(body.get("course")).get("folder") or "/nonexistent", body.get("path"))
        if text is None:
            raise KeyError("file")
        return {"text": text}
    if (method, path) == ("POST", "api/install"):
        if body.get("lang") not in LANGS:
            raise KeyError("lang")
        return install_lang(body["lang"])
    if (method, path) == ("POST", "api/chat"):
        c = load_id(body.get("course"))
        l = lesson_of(c, body.get("lesson"))
        msgs = [{"role": m["role"], "content": str(m["content"])[:20000]} for m in body.get("messages", [])[-30:]
                if m.get("role") in ("user", "assistant")]
        return {"text": llm([{"role": "system", "content": tutor_system(c, l)}] + msgs)}
    m = re.fullmatch(r"api/courses/([0-9a-f]{12})(/state|/videos|/practice|/paused)?", path)
    if not m:
        raise KeyError(path)
    cid, sub = m.groups()
    if method == "GET" and not sub:
        return mark_known(load(cid))
    if method == "DELETE" and not sub:
        path_of(cid).unlink()
        return {}
    if method == "PUT" and sub == "/state":
        if not all(isinstance(body.get(k), dict) for k in ("checks", "done", "notes")):
            raise ValueError("Bad progress data.")
        with LOCK:
            c = load(cid)
            old = c["state"]
            c["state"] = {**body, "review": old.get("review") or {}}  # the review schedule is the server's: a page with an old copy must not undo it
            for lid in set(body.get("quiz") or {}) - set(old.get("quiz") or {}):  # quiz just taken: its questions first come back tomorrow, not today
                for i in range(len((lesson_of(c, lid).get("practice") or {}).get("quiz") or [])):
                    c["state"]["review"].setdefault(f"{lid}#{i}", {"step": -1, "due": today() + 1})
            save(c)
        return {}
    if method == "PUT" and sub == "/paused":
        with LOCK:
            c = load(cid)
            c["paused"] = bool(body.get("paused"))
            save(c)
        return {}
    if method == "POST" and sub == "/videos":
        l = lesson_of(load(cid), body.get("lesson"))
        vids = l.get("videos") or find_videos(l["query"])
        if vids and not l.get("videos"):
            with LOCK:  # re-read: progress may have been saved while the search ran
                c = load(cid)
                lesson_of(c, l["id"])["videos"] = vids
                save(c)
        return vids
    if method == "POST" and sub == "/practice":
        c = load(cid)
        l = lesson_of(c, body.get("lesson"))
        if l.get("practice") and not body.get("fresh"):
            return l["practice"]
        pr = build_practice(c, l)
        with LOCK:
            c = load(cid)
            lesson_of(c, l["id"])["practice"] = pr
            r = c["state"].get("review") or {}
            c["state"]["review"] = {k: v for k, v in r.items() if not k.startswith(l["id"] + "#")}  # new questions, so the old schedule no longer fits
            save(c)
        return pr
    raise KeyError(path)


def load_id(cid):
    if not re.fullmatch(r"[0-9a-f]{12}", str(cid)):
        raise KeyError(cid)
    return load(cid)


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *a):
        pass

    def reply(self, code, out):
        data = out if isinstance(out, bytes) else json.dumps(out).encode()
        self.send_response(code)
        self.send_header("Content-Type", "text/html; charset=utf-8" if isinstance(out, bytes) else "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(data)

    def handle_any(self):
        if not self.path.startswith(PREFIX):
            return self.reply(404, {"error": "Not found"})
        try:
            n = int(self.headers.get("Content-Length") or 0)
            if n > 1 << 20:
                return self.reply(413, {"error": "Too large"})
            body = json.loads(self.rfile.read(n)) if n else {}
            if not isinstance(body, dict):
                raise ValueError("Bad request.")
            self.reply(200, route(self.command, self.path[len(PREFIX):].split("?")[0], body))
        except (KeyError, StopIteration, FileNotFoundError):
            self.reply(404, {"error": "Not found"})
        except ValueError as e:
            self.reply(400, {"error": str(e)})
        except Exception as e:
            self.reply(500, {"error": f"{type(e).__name__}: {e}"})

    do_GET = do_POST = do_PUT = do_DELETE = handle_any


def serve():
    httpd = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    threading.Thread(target=httpd.serve_forever, daemon=True).start()
    return f"http://127.0.0.1:{httpd.server_port}{PREFIX}"


def main():
    import gi
    # shortcut: GTK3 WebKit (webkit2gtk4.1), move to GTK4 webkitgtk6.0 when Fedora drops the 4.1 package
    gi.require_version("Gtk", "3.0")
    gi.require_version("WebKit2", "4.1")
    from gi.repository import Gio, GLib, Gtk, WebKit2

    url = serve()
    GLib.set_prgname("roadmap-app")  # matches roadmap-app.desktop so the window gets its icon

    def policy(view, decision, kind):
        # Links that leave the app (YouTube, docs) open in the user's browser, not inside the window.
        T = WebKit2.PolicyDecisionType
        if kind not in (T.NAVIGATION_ACTION, T.NEW_WINDOW_ACTION):
            return False
        action = decision.get_navigation_action()
        uri = action.get_request().get_uri()
        clicked = kind == T.NEW_WINDOW_ACTION or action.get_navigation_type() == WebKit2.NavigationType.LINK_CLICKED
        if clicked and re.match(r"https?://", uri) and not uri.startswith(url):
            Gio.AppInfo.launch_default_for_uri(uri, None)
            decision.ignore()
            return True
        return False

    def pick():
        # Called from a server thread: the dialog has to run on the GTK thread, so hand it over and wait.
        got, done = [""], threading.Event()

        def ask():
            dlg = Gtk.FileChooserNative.new("Choose the project folder", win, Gtk.FileChooserAction.SELECT_FOLDER, None, None)
            if dlg.run() == Gtk.ResponseType.ACCEPT:
                got[0] = dlg.get_filename() or ""
            dlg.destroy()
            done.set()
        GLib.idle_add(ask)
        done.wait()
        return got[0]

    global PICK
    PICK = pick
    view = WebKit2.WebView()
    view.connect("decide-policy", policy)
    view.load_uri(url)
    win = Gtk.Window(title="Roadmap")
    win.set_default_size(1280, 840)
    win.set_icon_name("roadmap-app")
    win.add(view)
    win.connect("destroy", Gtk.main_quit)
    win.show_all()
    Gtk.main()


if __name__ == "__main__":
    sys.exit(main())
