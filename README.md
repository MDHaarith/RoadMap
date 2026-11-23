# Roadmap

A Tauri 2 learning app with a Svelte 5 / TypeScript frontend in `ui/` and a Rust backend in `src/`. Linux keeps the existing XDG data and settings paths without conversion; Windows and macOS use their native data/config directories under `roadmap-app`.

Build on Fedora with Rust/Cargo 1.92 or newer, Node.js 24, npm, GCC/G++, pkgconf-pkg-config, gtk3-devel, webkit2gtk4.1-devel and openssl-devel. First run `npm --prefix ui ci`. Run with `make run`; run the tests with `make test`. `make build` embeds `ui/dist` in the release app and also builds `roadmap-mcp`; `make bundle` creates native installers (deb/rpm/AppImage on Linux, MSI/NSIS on Windows, app/DMG on macOS). Build installers on their target OS. Plain `cargo build` builds both binaries; its debug app expects the Vite dev server.
The tests use local fake AI and yt-dlp services and real bubblewrap sandboxes,
with Bash exercise fixtures, and need no Python interpreter or external AI key.
They require bubblewrap and permission to create unprivileged user namespaces.

Build the Fedora 44 RPM with:

```sh
podman run --rm --security-opt label=disable -v "$PWD":/src -w /src \
  registry.fedoraproject.org/fedora:44 sh -c \
  'dnf -y -q install rpm-build make cargo rust gcc-c++ pkgconf-pkg-config gtk3-devel webkit2gtk4.1-devel openssl-devel nodejs npm >/dev/null 2>&1 && make rpm'
```

The architecture-specific RPM is written under `dist/`. Cargo downloads the
versions in `Cargo.lock`, so the container needs network access; crates are not
vendored. Runtime packages are GTK3, WebKitGTK 4.1, bubblewrap, Bash, and
the system CA certificates.
On Linux, other exercise interpreters/compilers are installed on demand through the same Polkit / DNF dialog as before. Sandbox execution and runtime installation return an explicit unsupported error on Windows and macOS. The app downloads its own yt-dlp (and, for saving videos offline, Deno) from their GitHub releases on first use, on every system. Python exercises still use an optional Python
interpreter.

The root crate is also the Tauri app; `tauri.conf.json` points at `ui/dist`. The `api(method, path, body)` command runs the existing routes on a blocking worker. `chat_stream(body, onPiece)` sends tutor pieces over a Tauri channel and returns the final text. Both reject invalid bodies and preserve the 1 MiB limit. Errors serialize as `{ "status": 400, "message": "..." }`. File/folder selection uses native dialogs; window controls and HTTP(S) links use scoped Tauri capabilities.

The AI service is ChatGPT unless Settings names another: the app signs in to the learner's ChatGPT account itself (Settings, Sign in with ChatGPT, which opens the browser) and asks ChatGPT's Codex server directly, so there is no key to paste and no Codex program to install. This uses the same sign-in and private server as the Codex CLI, not a public API, so OpenAI may change or block it. In Settings this is the base `codex`; the model `default` is the first model ChatGPT lists. Tokens are kept in `chatgpt.json` next to `config.json`. Any OpenAI-compatible service can be set instead. `cargo test real_ai -- --ignored --nocapture` runs the main flows against the real service (see the comment above that test).

A new roadmap has a size: about two weeks, a month, three months or six months.
The two long sizes are planned as an outline of stages, phases and lesson titles
(`OUTLINE_PROMPT`). The lessons of a phase are written when the learner reaches it
(`POST api/courses/<id>/expand`), so later phases use their results. "Change my
roadmap" (`POST api/courses/<id>/revise`) lets the learner ask for a new direction
part-way through; new phases arrive as outlines and are written the same way.

## Connect an AI agent (MCP)

`roadmap-mcp` lets an agent list, read, create and fill learning roadmaps, read progress, append ideas and search. It never calls AI or the network and cannot change progress, delete roadmaps, access settings or API keys, run exercises, or read project folders. Search includes learner notes and tutor chats.

Claude Code: `claude mcp add roadmap -- roadmap-mcp`

Codex, in `~/.codex/config.toml`:

```toml
[mcp_servers.roadmap]
command = "roadmap-mcp"
```

From source, run `cargo build --release --locked --bin roadmap-mcp` and use the absolute path to `target/release/roadmap-mcp`. A roadmap created while the app is open appears after the library page is reopened.

The model picker lists models from your AI provider, or the ones your ChatGPT sign-in may use, and lets you change the main model independently. Notes can be tidied by the AI without saving over the original. Video captions are fetched in the background and cached, including unavailable captions; the tutor receives cached captions for the selected video and can read other available transcripts with its tool.

## Windows

Windows users need WebView2 (included with current Windows 10/11), and a ChatGPT account. Exercises are unavailable on Windows.

From Linux, install cargo-xwin and the Rust x86_64-pc-windows-msvc target, then run `make windows`. This compiles both executables and the Tauri frontend without bundling. Linux cannot run the Windows tests or build the Windows installer here; CI runs tests and builds the Inno Setup installer (`windows/roadmap.iss`) on Windows.

## Releasing

Set the same version in `Cargo.toml`, `tauri.conf.json`, `roadmap-app.spec`, `ui/package.json`
and the root package entries in `ui/package-lock.json`. The UI version matches
for release validation; the running version comes from Tauri. Run `cargo check`
to update the root package version in `Cargo.lock`, then `make release-check`.
Commit the changes and merge them into `master` through a pull request. When CI
passes on `master`, the release workflow builds signed installers and publishes
`vX.Y.Z` to `MDHaarith/RoadMap-releases`. A version that is already released is
skipped, so bump the version for each release. A manual Actions run on `master`
does the same.

Configure these repository Actions secrets once:

- `TAURI_SIGNING_PRIVATE_KEY`: the signing private key content (never commit it).
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: empty, since this key has no password.
- `RELEASES_TOKEN`: a token with contents write access to the public
  `MDHaarith/RoadMap-releases` repository.

The release workflow builds a Windows Inno Setup installer, Linux AppImage/deb/rpm installers, and their update signatures, then submits an MSIX build to the Microsoft Store once the `MSSTORE_*` and `MSIX_*` repo settings exist.
A final job waits for both builds, assembles a manifest with all four package
entries, uploads everything to a draft public release, then publishes it.
This prevents clients from seeing an incomplete manifest. Check the Actions
logs, the public release assets (including each `.sig` and `latest.json`), and
that the latest-download endpoint resolves. Two published versions are needed
to test an actual upgrade. Ordinary bundle builds do not require signing keys;
only `--config tauri.release.conf.json` enables updater artifacts.

The app checks after startup and every six hours, downloads signed updates,
and waits for “Restart and update”. RPM/deb may request administrator approval.
Development/unrecognized packages offer the public release download page.
