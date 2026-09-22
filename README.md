# playtest

> One command puts the build you already have in front of real people.

You have a `dist/`. Someone should play it. Today that means uploading to a host, sending a link, and then never finding out where they got stuck.

```bash
playtest ./dist
```

You get a link and a QR code in a few seconds. Whoever you send it to opens it on their phone and plays — no account, no install, no app. You see who showed up, how long they stayed, and what they said about *this* version.

[Website](https://playtest.run) · [Docs](https://playtest.run/console/#/docs/start) · [Plaza](https://playtest.run) · [Releases](https://github.com/roviix/playtest.run/releases)

---

## Install

macOS and Linux — downloads a single binary, verifies its SHA-256, installs to `~/.local/bin`:

```bash
curl -fsSL https://playtest.run/install.sh | bash
playtest --version
```

Windows, or if you'd rather do it by hand: grab the archive for your platform from [Releases](https://github.com/roviix/playtest.run/releases/latest) (macOS arm64/x86_64, Linux x86_64/arm64 musl, Windows x86_64) and put `playtest` on your `PATH`. The binaries are not code-signed yet; on macOS you'll have to allow it once under *System Settings → Privacy & Security*.

## Publish

From the directory with `index.html` in it:

```console
$ playtest ./dist
Reading ./dist: 2 files, 1.1 MB
Looks like a Phaser build
All 2 files are already on the server. Nothing to upload.
Published "phaser-jump" v1

https://playtest.run/p/misty-frog-44

Anonymous link, expires 2026-09-16 22:22. Run playtest login to keep it.
See how people played in the Developer Console: https://playtest.run/console/#/s/misty-frog-44
```

The first run needs no account. The link lives 24 hours; `playtest login` makes it permanent and moves anything you already published into your account.

Run it again in the same directory and it becomes v2 at the same address, so the people you already invited don't need a new link.

## Share a dev server instead

```bash
playtest 5173                  # tunnel your local Vite / Next dev server
playtest ./dist --backend 3000 # upload the static build, tunnel /api and WebSockets to your machine
```

The tunnel runs on your uplink, so `playtest` tells you what that costs before anyone waits on it: *"This page downloads about 32.0 MB. The tunnel runs on your uplink, so at a typical 30 Mbps home connection one player waits around 9 seconds."*

## What it checks before uploading

This is the part other hosts don't do. `playtest` reads your export and tells you why it won't run, at the moment you publish rather than after someone fails to load it:

- **Threaded builds.** If the wasm asks for shared memory, cross-origin isolation (`COOP`/`COEP`) is turned on for you and it says so. Godot 4 threaded exports need this; most static hosts can't set those headers at all.
- **Missing engine payloads.** A Godot export with no `.pck`, or a Unity export with no `.data`, means players sit on the loading bar forever. It names the file.
- **Files the page asks for but nobody uploaded.** Parsed out of `index.html`.
- **Unity Decompression Fallback.** If both `.br`/`.gz` and `.unityweb` are present, it reads `dataUrl` to tell you which set actually gets served, and that the other one is uploaded for nothing.
- **Source directories.** `node_modules/` or a `.git/` in what you're publishing usually means you meant `dist/`.

## Commands

| Command | What it does |
|---|---|
| `playtest <dir>` | Publish an exported static directory |
| `playtest <port>` | Tunnel a running local dev server |
| `playtest <dir> --backend <port>` | Static files from the edge, everything else tunneled to your machine |
| `playtest ls` | Projects published from this machine |
| `playtest open` | Print the link and open it |
| `playtest card` | Download the invite card PNG to send to a chat |
| `playtest versions` | Every version, and which one players see now |
| `playtest rollback <version>` | Put players back on an earlier version; nothing is re-uploaded |
| `playtest files` | Exact paths, sizes and content hashes that are live |
| `playtest rm` | Delete a project; its link stops working |
| `playtest unlist` | Take it off the Plaza; the link keeps working |
| `playtest mcp` | Run as an MCP server so Cursor / Claude Code can publish for you |

Publishing flags: `-m` (what changed in this version), `--name`, `--summary`, `--cover`, `--public` (list on the Plaza), `--seats N` (mark it seeking testers), `--community <url>`, `--isolated auto|on|off`, `--spa`, `--card <path>`, `--no-qr`, `--json`.

`playtest --help` is the full list, and every command exits with a documented code (`3` needs login, `4` network, `7` out of quota) so scripts don't have to match on text.

## For AI assistants

`playtest mcp` speaks the Model Context Protocol: five tools — publish a directory, share a local port, list projects, check how one project is doing, fetch an invite card. `playtest mcp --setup` prints the config block for Cursor or Claude Code. The publish tool hands the invite card back as an image, so the assistant can paste it straight into the conversation.

The site also serves [`/skill.md`](https://playtest.run/skill.md), [`/llms.txt`](https://playtest.run/llms.txt), [`/openapi.json`](https://playtest.run/openapi.json) and `/.well-known/agent.json`.

## What it doesn't do

- It doesn't run your code on our servers. Everything you upload is served as static files.
- It doesn't rewrite or inject anything into your files.
- It doesn't invent numbers. Opens are opens; an empty roster says nothing rather than showing a zero.
- It isn't a store. There's no cart, no ratings, no algorithmic feed.
- Anonymous links expire after 24 hours, and free accounts have hard caps that stop rather than bill you.

## How it fits together

- **`playtest.run`** — the Plaza, collections, following, the Developer Console, docs and the agent endpoints. No creator code ever runs on this origin.
- **`<slug>.playtest.run`** — where your project actually runs, on its own origin. Platform cookies are `__Host-` prefixed and never reach it.

The repository is one Rust workspace plus two TypeScript packages:

- **`cli/`** — the `playtest` binary: upload, tunnel, export inspection, MCP server.
- **`common/`** — manifests, contracts, the tunnel protocol, shared validation.
- **`edge/`** — serves `*.playtest.run`: invitation pages, assets, the Plaza, rate limiting, structured data.
- **`api/`** — control plane: accounts, versions, feedback, quotas, email.
- **`console/`** — the developer console (Preact).
- **`sdk/`** — the optional in-page feedback SDK.
- **`deploy/`** — single-machine compose, Caddy config, install script.

## Build it yourself

```bash
cargo build --workspace
cargo test --workspace
cd console && pnpm install && pnpm build
```

See [`deploy/README.md`](deploy/README.md) for running your own instance. Design decisions live in [`docs/DESIGN.md`](docs/DESIGN.md) (Chinese), and every "this works" claim is backed by a dated record in [`docs/spikes/`](docs/spikes).

## Security

Report vulnerabilities through GitHub's private advisory form — see [`SECURITY.md`](SECURITY.md). `playtest` opens a tunnel from your machine and hosts other people's work, so we treat security reports as product defects, not PR problems.

## Thanks

- 本项目积极参与并认可 [LINUX DO 社区](https://linux.do)

## License

- CLI and SDK: [Apache-2.0](LICENSE)
- Edge and API services: AGPL-3.0
