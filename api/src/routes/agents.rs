//! 给 AI 助手读的那几个文件（REWRITE §3.2）。
//!
//! 为什么值得有：DESIGN §3.2 把 `playtest mcp` 定成「广场最大的供给管道」，而一个助手
//! 要先**知道**这个工具存在、能做什么、不能做什么，才会去用它。MCP 只有装过的人能看到；
//! 搜到这个域名的助手看到的是这几个文件。
//!
//! 它们由控制面自己出，不是 `deploy/` 里的静态文件——路径常量取自
//! [`playtest_common::api::routes`]，端点改了这里跟着改，测试盯着两边对齐
//! （见本文件末尾的 `every_route_is_documented`）。一份写在别处的 API 文档一定会漂。
//!
//! 语气上只说做得到的事，并且明写做不到的（REWRITE §1.1「不虚报」）：助手拿错工具
//! 比拿不到工具更浪费用户的时间。

use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use playtest_common::api::routes as paths;
use playtest_common::{contract, limits, DEVELOPER_API_URL};
use serde_json::{json, Value};

/// 这几个文件都不带用户数据，缓存久一点没关系；助手可能一天拉好几次。
const CACHE: &str = "public, max-age=3600";

fn text(body: String, mime: &'static str) -> Response {
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, HeaderValue::from_static(mime)),
            (header::CACHE_CONTROL, HeaderValue::from_static(CACHE)),
            // 助手常从浏览器侧的沙箱里取这几个文件。
            (
                header::ACCESS_CONTROL_ALLOW_ORIGIN,
                HeaderValue::from_static("*"),
            ),
        ],
        body,
    )
        .into_response()
}

/// `GET /llms.txt`：一屏说清「这是什么、什么时候用、怎么用」。
pub async fn llms_txt() -> Response {
    text(short(), "text/plain; charset=utf-8")
}

/// `GET /llms-full.txt`：加上完整端点表与失败形态。
pub async fn llms_full_txt() -> Response {
    text(
        format!("{}\n{}", short(), long()),
        "text/plain; charset=utf-8",
    )
}

/// `GET /skill.md`：可以直接装进 Claude / Cursor 的 skill 文件。
pub async fn skill_md() -> Response {
    text(skill(), "text/markdown; charset=utf-8")
}

/// `GET /openapi.json`。
pub async fn openapi_json() -> Json<Value> {
    Json(openapi())
}

/// `GET /.well-known/agent.json`：发现清单。
pub async fn agent_json() -> Json<Value> {
    Json(json!({
        "name": "playtest",
        "description": "Put what you're making in front of real people with one command — zero-friction previews, versioned feedback, and voluntary followers for your next iteration.",
        "documentation": format!("{DEVELOPER_API_URL}/llms.txt"),
        "openapi": format!("{DEVELOPER_API_URL}/openapi.json"),
        "skill": format!("{DEVELOPER_API_URL}/skill.md"),
        "mcp": {
            "command": "playtest",
            "args": ["mcp"],
            "install": "curl -fsSL https://playtest.run/install.sh | bash",
            "install_windows": "https://github.com/roviix/playtest.run/releases/latest"
        }
    }))
}

fn short() -> String {
    format!(
        "\
# playtest

> Build in public. Show the work, not the hype.

One command puts what you're making (web build, interactive prototype, article, or video) in front of real people. Zero-friction playtesting, versioned feedback, and followers for what's next.

## What it is

A work-centric build-in-public platform, not a generic hosting product or vanity social feed.
You publish a built web directory (or tunnel a running dev server) and get `https://<slug>.playtest.run`
or `https://playtest.run/p/<slug>`. Audience opens it and tries it — no account, no install.
The creator gets a clean roster of who visited, on what, where they stopped, and what they said.

## When to use it

- The user has a **playable or interactive web build** (game, Three.js scene, p5 sketch, WebXR
  demo, prototype) and wants people to try it.
- The user has a **dev server running** and wants someone on another device or network to open it.
- The user is **building in public** and wants versioned feedback rather than social media likes.
- The user wants a **QR code or an invite card image** to drop into a group chat.

## When NOT to use it

- Plain static hosting for a report, dashboard, docs site or landing page. That is a hosting
  product's job, not this one. Links here are for *iterative testing and build-in-public*, the root domain is the Plaza,
  where projects look for testers, and anonymous links expire in {anon_hours} hours.
- Anything that needs server-side code. We never run user code. If the user has a backend,
  keep it on their machine and use `--backend` (hybrid mode).
- Long-term production hosting or a custom domain. We do not do custom domains.
- A vanity social network for posting text updates, daily streaks, or revenue screenshots. We focus strictly on the tangible work itself.

## The fastest path

Prefer the MCP server if it is installed — it hands back the link, the QR text and the invite
card as an image block in one call:

    playtest mcp

Tools: `playtest_upload` (a built directory), `playtest_share` (a local port), `playtest_list`,
`playtest_site`, `playtest_card`.

Otherwise use the CLI (single binary, no runtime; install with
`curl -fsSL https://playtest.run/install.sh | bash` on macOS / Linux):

    playtest ./dist            # upload a built directory -> link + QR (no image file written)
    playtest 5173              # tunnel a running dev server
    playtest ./dist --backend 3000   # static from us, everything else tunneled to their backend
    playtest ./dist --json     # one JSON object on stdout, narration on stderr

Or call the HTTP API directly: see {api}/llms-full.txt and {api}/openapi.json.

## Three things to pass on to the user

1. **The url.** That is the thing they share.
2. **`qr_text`** — paste it verbatim in a code block when they will open it on a phone.
3. **The invite card**, when needed: use `playtest card --out invite.png` or the image block from MCP. CLI uploads do not download it by default. In a chat app a bare link has no
   title and no face; the card is a picture with the developer's name on it, and pictures travel.
   Hand the image over, do not just mention it.

## Hard promises (safe to repeat to the user)

- Players never log in and never install anything to play.
- We never rewrite a single byte of their build.
- We never run their code.
- Free tier has hard ceilings and cannot generate a bill.
",
        anon_hours = playtest_common::ANON_LINK_TTL_HOURS,
        api = DEVELOPER_API_URL,
    )
}

fn long() -> String {
    let mut out = String::from(
        "\
## HTTP API

Base URL: ",
    );
    out.push_str(DEVELOPER_API_URL);
    out.push_str(
        "
Auth: `Authorization: Bearer <token>`. Get an anonymous token from `POST /v1/anon/sessions`
(the link it makes expires; see the ceilings below). Long-lived tokens come from playtest login (email or GitHub),
which is a device-code flow a human has to finish in a browser.

### Publishing is three steps

",
    );
    out.push_str(&format!(
        "1. `POST {sites}` -> a project with a slug.\n\
         2. `POST {uploads}` with the file list (path + size + sha256) -> which hashes we are\n   \
            missing. Upload only those with `PUT {blob}`; content-addressed, so re-running after\n   \
            a failure only sends what is still missing.\n\
         3. `POST {commit}` -> the version goes live at `https://<slug>.playtest.run`.\n",
        sites = paths::SITES,
        uploads = paths::SITE_UPLOADS,
        blob = paths::BLOB,
        commit = paths::SITE_UPLOAD_COMMIT,
    ));
    out.push_str("\n### Every endpoint\n\n");
    for e in endpoints() {
        let auth = match e.auth {
            contract::Auth::Public => "",
            contract::Auth::Bearer => " (bearer token)",
            contract::Auth::Ingest => " (called by the page, not by you)",
            contract::Auth::Admin => " (operator only)",
        };
        out.push_str(&format!(
            "- `{} {}` — {}{auth}\n",
            e.method, e.path, e.summary
        ));
    }
    out.push_str(&format!(
        "
### Ceilings

- One file: {file} MB. One version: {version} MB, {files} files max.
- Anonymous links live {hours} hours and carry a small traffic allowance.
- Hitting a ceiling stops serving. It never produces a bill and never silently charges anyone.

### Failure shapes

Errors are `{{\"code\": \"...\", \"message\": \"...\"}}` with a human-readable message in the
user's language. Codes: {codes}. The full list with meanings is `ErrorCode` in /openapi.json.
The CLI turns them into exit codes: 0 success, 1 unexpected, 2 usage, 3 needs login, 4 network,
5 server, 6 bad input, 7 quota.

`invalid` on upload is usually a real problem with the build that we detected before anyone
opened the link — a missing `index.html` at the top level, a Godot threaded export that needs
`--isolated`, a missing `.pck` or `.data`. Read `message` and `hint` and fix the build; do not
retry blindly.
",
        file = limits::MAX_FILE_BYTES / limits::MIB,
        version = limits::MAX_VERSION_BYTES / limits::MIB,
        files = limits::MAX_FILES_PER_VERSION,
        hours = playtest_common::ANON_LINK_TTL_HOURS,
        codes = error_codes(),
    ));
    out
}

/// `ErrorCode` 的每个值，从 schema 里读，和实际能回的一致。
fn error_codes() -> String {
    let defs = contract::definitions();
    let mut out = Vec::new();
    if let Some(one_of) = defs
        .get("ErrorCode")
        .and_then(|s| s.get("oneOf"))
        .and_then(Value::as_array)
    {
        for variant in one_of {
            if let Some(c) = variant.get("const").and_then(Value::as_str) {
                out.push(format!("`{c}`"));
            }
            if let Some(list) = variant.get("enum").and_then(Value::as_array) {
                out.extend(
                    list.iter()
                        .filter_map(Value::as_str)
                        .map(|c| format!("`{c}`")),
                );
            }
        }
    }
    out.join(", ")
}

/// 端点表在 `common::contract`，`llms-full.txt`、`openapi.json` 与控制台的 TS 都读那一份。
fn endpoints() -> Vec<contract::Endpoint> {
    contract::endpoints()
}

fn openapi() -> Value {
    let mut paths_obj = serde_json::Map::new();
    for endpoint in endpoints() {
        let entry = paths_obj
            .entry(endpoint.path.to_string())
            .or_insert_with(|| json!({}));
        entry[endpoint.method.to_lowercase()] = contract::openapi_operation(&endpoint);
    }
    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "playtest",
            "version": env!("CARGO_PKG_VERSION"),
            "summary": "Put a playable web build in front of real people.",
            "description":
                "Publish a built web directory and get a link players can open with no account \
                 and no install. Static files only — we never run user code. See /llms-full.txt \
                 for the three-step publish flow.",
        },
        "servers": [{ "url": DEVELOPER_API_URL }],
        "components": {
            "securitySchemes": {
                "bearer": { "type": "http", "scheme": "bearer", "description": "a developer token from `playtest login` or POST /v1/anon/sessions" },
                "admin": { "type": "http", "scheme": "bearer", "description": "the operator token; not for developers" }
            },
            "schemas": contract::openapi_schemas(),
        },
        "security": [{ "bearer": [] }],
        "paths": Value::Object(paths_obj),
    })
}

fn skill() -> String {
    format!(
        "\
---
name: playtest
description: >-
  Publish a playable web build (game, demo, interactive sketch) to a link real people can open
  with no account and no install, get a QR code and a forwardable invite card, and see who played.
  Use when the user has a built web game or interactive demo and wants people to try it, or has a
  dev server running and wants someone else to open it. Do not use for plain static hosting of
  reports, dashboards or docs sites.
---

# playtest

## Install

Single binary, no runtime. macOS / Linux:

```
curl -fsSL https://playtest.run/install.sh | bash
```

Installs to `~/.local/bin/playtest` after a SHA-256 check. Windows: download the zip from
https://github.com/roviix/playtest.run/releases/latest and put `playtest.exe` on PATH.
Verify with `playtest --version`.

Prefer the MCP server if you can run it — one call gives you the link, the QR text and the
invite card image:

```
playtest mcp
```

## Decide which path first

| The user has | Do this |
| --- | --- |
| A built directory (`dist/`, `build/web/`, a Godot/Unity web export) | `playtest ./dist` |
| A dev server already running on a port | `playtest 5173` |
| A built frontend **and** a backend they run locally | `playtest ./dist --backend 3000` |
| Only source, no build | Run their build first. Do not upload source. |

Always pass `--json` when you are going to parse the answer. stdout is exactly one JSON object;
all narration goes to stderr.

## Publishing

```
playtest ./dist --json
```

Answer fields worth reading: `url`, `qr_text`, `card_path`, `expires_at`, `console_url`,
`findings`, `elapsed_ms`.

Useful flags:

- `-n, --name` the title players see before they start
- `-m, --note` what changed in this version — shown on the invitation page, sent to followers, and
  used as the \"what I want you to look at\" line on the Plaza
- `--summary` the persistent project description; omitted values keep the existing description
- `--cover cover.png` the first thing players see; without it we typeset a plain title card
- `--public` put it on the Plaza; `--seats 10` also marks it as looking for that many testers
- `--card <path>` explicitly download an invite card (default: no download; `-` also skips it)
- `--isolated=on|off|auto` cross-origin isolation; Godot 4 threaded exports need it (auto detects)

## After publishing, hand the user three things

1. The **url**.
2. The **QR code** — paste `qr_text` verbatim inside a code block. It only scans in a monospace
   block; do not reflow or summarise it.
3. If an image is needed, run `playtest card --out invite.png` or use the MCP image block. CLI uploads do not write a card by default; `card_path` appears only after an explicit successful download. This matters more than it sounds: in most chat apps a bare
   link shows no title and no face, and a picture with the developer's name on it is what people
   actually forward. Attach the image; do not just say it exists.

## Reading failures

Exit codes: 0 ok, 2 usage, 3 needs login, 4 network, 5 server, 6 bad input, 7 quota.

Exit 6 usually means we inspected the build and know it will not open — a missing top-level
`index.html`, a threaded export without isolation, a missing `.pck`/`.data`. `message` and `hint`
say what to change. Fix the build; retrying the same bytes will fail the same way.

## What to tell the user honestly

- Without logging in, the link expires in {hours} hours. `playtest login` (email or GitHub) keeps it, and
  adopts links they already made anonymously.
- We never run their code and never rewrite their bytes.
- Players never need an account. First play is zero-friction by design.
- This is not general static hosting; the root domain is the Plaza, for projects looking for testers.

## Other commands

```
playtest ls                 # projects on this machine (with follower counts)
playtest versions <slug>    # every version, and which one players see
playtest rollback <slug> 3  # point players back at v3; no bytes re-uploaded
playtest card <slug>        # fetch the current invite card again
playtest files <slug>       # what is actually live: every path, size and sha256
playtest unlist <slug>      # take it off the Plaza; the link keeps working
playtest open <slug>
playtest rm <slug> -y
```

## HTTP API

If you cannot run the binary: {api}/llms-full.txt and {api}/openapi.json.
Publishing is `POST /v1/projects` -> `POST /v1/projects/{{slug}}/uploads` -> `PUT /v1/blobs/{{hash}}`
-> `POST /v1/projects/{{slug}}/uploads/{{id}}/commit`.
",
        hours = playtest_common::ANON_LINK_TTL_HOURS,
        api = DEVELOPER_API_URL,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 这几个文件是给助手读的，读错了它就会拿错工具。三条底线：
    /// 说清什么时候**不要**用、给出三步发布、不出现内部比喻。
    #[test]
    fn the_short_page_says_when_not_to_use_us() {
        let s = short();
        assert!(s.contains("When NOT to use it"));
        assert!(s.contains("never run user code") || s.contains("never run their code"));
        assert!(s.contains("playtest_upload"), "先推 MCP");
        assert!(s.contains("--backend"), "有后端的人要知道这条路");
        // 匿名链接会过期是助手必须转述的事实。
        assert!(s.contains(&playtest_common::ANON_LINK_TTL_HOURS.to_string()));
    }

    #[test]
    fn the_long_page_spells_out_the_three_steps_and_the_ceilings() {
        let s = long();
        for path in [
            paths::SITES,
            paths::SITE_UPLOADS,
            paths::BLOB,
            paths::SITE_UPLOAD_COMMIT,
        ] {
            assert!(s.contains(path), "三步里少了 {path}");
        }
        assert!(s.contains("quota_exceeded"));
        assert!(s.contains("never produces a bill"));
    }

    /// 端点表和 OpenAPI 是同一份数据，不可能只改一边。
    #[test]
    fn the_spec_and_the_text_list_the_same_endpoints() {
        let spec = openapi();
        let listed = spec["paths"].as_object().unwrap();
        let text = long();
        for e in endpoints() {
            let (method, path) = (e.method, e.path);
            assert!(listed.contains_key(path), "OpenAPI 里少了 {method} {path}");
            assert!(
                listed[path].get(method.to_lowercase()).is_some(),
                "OpenAPI 的 {path} 上少了 {method}"
            );
            assert!(text.contains(path), "llms-full.txt 里少了 {path}");
        }
        // 每个 $ref 都指得到 components/schemas 里的一项。
        let schemas = spec["components"]["schemas"].as_object().unwrap();
        let text = serde_json::to_string(&spec).unwrap();
        for piece in text.split("#/components/schemas/").skip(1) {
            let name: String = piece.chars().take_while(|c| c.is_alphanumeric()).collect();
            assert!(schemas.contains_key(&name), "OpenAPI 引用了不存在的 {name}");
        }
        assert_eq!(spec["openapi"], "3.1.0");
        assert_eq!(spec["servers"][0]["url"], DEVELOPER_API_URL);
    }

    /// 发布那三步必须在表里——助手照着 `llms-full.txt` 就能发出去，不用装二进制。
    #[test]
    fn the_publish_flow_is_reachable_without_the_binary() {
        let listed = openapi();
        let listed = listed["paths"].as_object().unwrap();
        assert!(listed[paths::SITES].get("post").is_some());
        assert!(listed[paths::SITE_UPLOADS].get("post").is_some());
        assert!(listed[paths::BLOB].get("put").is_some());
        assert!(listed[paths::SITE_UPLOAD_COMMIT].get("post").is_some());
    }

    #[test]
    fn the_skill_tells_the_assistant_to_hand_over_the_picture() {
        let s = skill();
        assert!(s.contains("qr_text"));
        assert!(s.contains("card_path"));
        assert!(s.contains("Attach the image"), "光提一句不算给");
        assert!(s.contains("Do not use for plain static hosting"));
        // 退出码是助手判断「要不要重试」的依据。
        assert!(s.contains("Exit codes"));
        assert!(s.contains("retrying the same bytes will fail the same way"));
    }

    /// 不出现内部比喻和内部词（AGENTS 第 8 条）。助手会把这些话原样转述给用户。
    #[test]
    fn no_internal_words_leak_into_what_the_assistant_reads() {
        for page in [short(), long(), skill()] {
            for word in ["yamux", "resvg", "blob 目录", "清单文件", "点名册式"] {
                assert!(!page.contains(word), "内部词漏出去了：{word}");
            }
        }
    }
}
