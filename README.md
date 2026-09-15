# playtest

> **Build in public. Show the work, not the hype.**
>
> One command to put what you're making — web apps, tools, games, articles, or videos — in front of real people. Zero-friction playtesting, versioned feedback, and followers for what's next.

[Website](https://playtest.run) · [Docs](https://playtest.run/console/#/docs/start) · [Plaza](https://playtest.run) · [Releases](https://github.com/roviix/playtest.run/releases)

---

## What is playtest?

**playtest** is a lightweight, zero-friction delivery and feedback platform designed for makers who **build in public**. 

Unlike generic static hosting or heavy app stores, playtest bridges the critical gap between *"I built something"* and *"real people gave me actionable feedback"*:

- **⚡ Instant Single-Command Publishing**: Run `playtest ./dist` or `playtest 5173` to get a live HTTPS URL (`https://<slug>.playtest.run`) and QR code in seconds.
- **🛡️ Zero-Friction for Players & Testers**: Anyone can open and test your work immediately. No mandatory sign-up, no downloads, and no app installs required.
- **🔄 Version-Bound Feedback**: Testers leave one-sentence impressions and bug reports directly pinned to the exact version they tested (`v1`, `v2`, etc.).
- **👥 Build an Audience Across Releases**: Testers can voluntarily follow your project or collection via email or browser notifications to get notified when your next version drops.
- **🎨 Multi-Format Support**: Built for modern indie creators — supports HTML5 games (Phaser, Godot, Unity, Cocos), Web applications (Vite, Next, React), interactive tools, articles, and video demos.
- **✨ Public Plaza & Collections**: Option to feature your work on the global Plaza (`playtest.run`) with `playtest ./dist --public` or recruit specific testers with `--seats 10`.

---

## 3-Minute Quickstart

### 1. Installation

One line on macOS or Linux (downloads the single binary, verifies its SHA-256, installs to `~/.local/bin`):

```bash
curl -fsSL https://playtest.run/install.sh | bash
playtest --version
```

Windows, or if you prefer to install by hand: download the archive for your platform from [GitHub Releases](https://github.com/roviix/playtest.run/releases/latest) (macOS arm64/x86_64, Linux x86_64/arm64 musl, Windows x86_64) and put `playtest` on your `PATH`. Binaries are not yet code-signed; on macOS allow it once under *System Settings → Privacy & Security*.

### 2. Publish Your First Project

Navigate to your build output directory (containing `index.html`):

```bash
cd ./dist

# Publish an anonymous link valid for 24 hours (or claim with an account)
playtest .

# Or publish to the public Plaza seeking 10 playtesters
playtest . --public --seats 10
```

Within 3 seconds, your terminal outputs:
```text
✓ Published to: https://brisk-otter-41.playtest.run
  Plaza invitation: https://playtest.run/p/brisk-otter-41
  QR code: [rendered in terminal]
```

### 3. Share a Local Dev Server / Tunnel

Need to share an active development server or multiplayer backend without building?

```bash
# Expose your local Vite / Next dev server on port 5173
playtest 5173

# Expose static frontend while proxying unmatched API requests to port 3000
playtest ./dist --backend 3000
```

---

## CLI Command Reference

| Command | Description |
|---|---|
| `playtest <dir>` | Upload and publish an exported static directory (`./dist`, `./build`). |
| `playtest <port>` | Establish a real-time secure tunnel to a running local dev server. |
| `playtest <dir> --backend <port>` | Hybrid mode: serve static assets with edge caching while proxying `/api` & WebSockets to local backend. |
| `playtest open` | Open the current project's live URL in your default browser. |
| `playtest card` | Generate an invitation pass image (`invite.png`) suitable for messaging apps. |
| `playtest versions` | Inspect deployed release history, timestamps, and active version. |
| `playtest rollback <version>` | Instantly roll back to any previous version with zero downtime. |
| `playtest files` | Verify exact files, byte sizes, and content SHA-256 hashes currently live. |
| `playtest rm` | Safely take down or remove a project. |
| `playtest mcp` | Launch a Model Context Protocol (MCP) server for AI assistants (Cursor, Claude, Antigravity). |

### Publishing Flags
- `--public`: List your project on the global `playtest.run` Plaza.
- `--seats <N>`: Display a recruitment tag requesting *N* playtesters.
- `--summary "text"`: Add a one-line description visible on your card and meta preview.
- `--isolated`: Enforce Cross-Origin Isolation headers (`COOP`/`COEP`) for multi-threaded Godot or WebAssembly builds.
- `--json`: Format CLI outputs as raw JSON for CI/CD or automation scripts.

---

## Architecture & Principles

### 1. One Product, One Root Domain
- **`playtest.run`**: The main platform root. Hosts the Plaza, Collections, Following updates, Developer Console, Documentation, and AI discovery endpoints. No untrusted creator scripts ever run on the root domain.
- **`<slug>.playtest.run`**: Isolated origin where user projects run. Platform cookies are host-only (`Host-` prefixed) and never leak to project subdomains.

### 2. No Hype, Just Facts
The user interface only displays concrete verified facts:
- No artificial follower counts, vanity metrics, or fake reviews.
- Roster shows actual dwell time and completion medians (e.g., *"3 sessions, median duration 4m 12s"*).
- Every feedback note is tied to the author's specific version commit.

### 3. AI Agent Ready
playtest is designed from day one to be machine-readable:
- `/llms.txt`: Standard LLM navigation pointers.
- `/skill.md`: Direct agent instruction manual.
- `/openapi.json`: Machine-executable API schema.
- Built-in `playtest mcp` server.

---

## Repository Structure

playtest is organized as a unified Rust Cargo workspace (`cargo test --workspace`) with TypeScript frontend components:

- **`cli/`**: The `playtest` command-line tool (upload, tunnel, inspections, MCP server).
- **`common/`**: Shared protocol contracts, manifest validation, schemas, and wording models.
- **`edge/`**: High-performance edge server handling wildcards (`*.playtest.run`), project gates, asset serving, streaming rate limits, and JSON-LD structured data.
- **`api/`**: Developer control plane handling auth, versioning, feedback pipelines, email dispatch, and console APIs.
- **`console/`**: Preact-based mobile-responsive developer dashboard (`playtest.run/console/`).
- **`deploy/`**: Single-machine production container manifests, Caddy TLS configurations, and deployment scripts.

---

## Self-Hosting & Development

To build and run locally:

```bash
# Build the backend and CLI
cargo build --workspace

# Run complete workspace test suite
cargo test --workspace

# Build the developer console
cd console && npm install && npm run build
```

See [`deploy/README.md`](deploy/README.md) for production provisioning, Caddy automatic TLS setup, and single-machine deployment instructions.

---

## License

- CLI & SDK: [Apache-2.0](LICENSE)
- Edge & API services: AGPL-3.0
