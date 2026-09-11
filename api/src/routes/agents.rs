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
use playtest_common::ingest::routes as ingest_paths;
use playtest_common::results::routes as result_paths;
use playtest_common::{limits, DEVELOPER_API_URL};
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
        "description": "Put a playable web build in front of real people and see what happened.",
        "documentation": format!("{DEVELOPER_API_URL}/llms.txt"),
        "openapi": format!("{DEVELOPER_API_URL}/openapi.json"),
        "skill": format!("{DEVELOPER_API_URL}/skill.md"),
        "mcp": {
            "command": "playtest",
            "args": ["mcp"],
            "install": "https://github.com/roviix/playtest.run/releases"
        }
    }))
}

fn short() -> String {
    format!(
        "\
# playtest

One command puts a playable web build in front of real people, and tells you what happened.
一条命令，把你手上这个能玩的版本放到别人面前，然后知道他们玩成了什么样。

## What it is

A playtest link, not a hosting product. You publish a built web directory (or tunnel a running
dev server) and get `https://<slug>.playtest.run`. Players open it and play — no account, no
install. The developer gets a roster of who played, on what, where they stopped, and what they said.

## When to use it

- The user has a **playable or interactive web build** (game, Three.js scene, p5 sketch, WebXR
  demo, prototype) and wants people to try it.
- The user has a **dev server running** and wants someone on another device or network to open it.
- The user wants a **QR code or an invite card image** to drop into a group chat.

## When NOT to use it

- Plain static hosting for a report, dashboard, docs site or landing page. That is a hosting
  product's job, not this one. Links here are for *playtesting*, the root domain is a plaza of
  works looking for testers, and anonymous links expire in {anon_hours} hours.
- Anything that needs server-side code. We never run user code. If the user has a backend,
  keep it on their machine and use `--backend` (hybrid mode).
- Long-term production hosting or a custom domain. We do not do custom domains.

## The fastest path

Prefer the MCP server if it is installed — it hands back the link, the QR text and the invite
card as an image block in one call:

    playtest mcp

Tools: `playtest_upload` (a built directory), `playtest_share` (a local port), `playtest_list`,
`playtest_site`, `playtest_card`.

Otherwise use the CLI (single binary, no runtime):

    playtest ./dist            # upload a built directory -> link + QR + invite card
    playtest 5173              # tunnel a running dev server
    playtest ./dist --backend 3000   # static from us, everything else tunneled to their backend
    playtest ./dist --json     # one JSON object on stdout, narration on stderr

Or call the HTTP API directly: see {api}/llms-full.txt and {api}/openapi.json.

## Three things to pass on to the user

1. **The url.** That is the thing they share.
2. **`qr_text`** — paste it verbatim in a code block when they will open it on a phone.
3. **The invite card** (`card_path` / an image block from MCP). In WeChat a bare link has no
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
(the link it makes expires; see the ceilings below). Long-lived tokens come from GitHub login,
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
    for (method, path, what) in endpoints() {
        out.push_str(&format!("- `{method} {path}` — {what}\n"));
    }
    out.push_str(&format!(
        "
### Ceilings

- One file: {file} MB. One version: {version} MB, {files} files max.
- Anonymous links live {hours} hours and carry a small traffic allowance.
- Hitting a ceiling stops serving. It never produces a bill and never silently charges anyone.

### Failure shapes

Errors are `{{\"code\": \"...\", \"message\": \"...\"}}` with a human-readable message in the
user's language. Codes: `usage`, `needs_login`, `network`, `server_error`, `bad_input`,
`quota_exceeded`. The CLI maps them to exit codes 2/3/4/5/6/7; 0 is success, 1 is unexpected.

`bad_input` on upload is usually a real problem with the build that we detected before anyone
opened the link — a missing `index.html` at the top level, a Godot threaded export that needs
`--isolated`, a missing `.pck` or `.data`. Read `message` and `hint` and fix the build; do not
retry blindly.
",
        file = limits::MAX_FILE_BYTES / limits::MIB,
        version = limits::MAX_VERSION_BYTES / limits::MIB,
        files = limits::MAX_FILES_PER_VERSION,
        hours = playtest_common::ANON_LINK_TTL_HOURS,
    ));
    out
}

/// 端点表。这一份同时喂 `llms-full.txt` 和 `openapi.json`，所以只有一处。
fn endpoints() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("GET", paths::HEALTH, "is this control plane up"),
        (
            "POST",
            paths::ANON_SESSIONS,
            "an anonymous token; the links it makes expire",
        ),
        ("GET", paths::ME, "who this token belongs to"),
        ("GET", paths::SITES, "the caller's projects"),
        ("POST", paths::SITES, "create a project"),
        ("GET", paths::SITE, "one project as it is right now"),
        (
            "PATCH",
            paths::SITE,
            "change plaza listing, seats, community link, public feedback",
        ),
        ("DELETE", paths::SITE, "delete it; the link stops working"),
        (
            "POST",
            paths::SITE_UPLOADS,
            "declare the files, learn which hashes are missing",
        ),
        ("PUT", paths::BLOB, "upload one file by content hash"),
        (
            "POST",
            paths::SITE_UPLOAD_COMMIT,
            "make the uploaded files the live version",
        ),
        ("GET", paths::SITE_VERSIONS, "every version, newest first"),
        (
            "POST",
            paths::SITE_VERSION_ACTIVATE,
            "roll back: point players at an older version",
        ),
        (
            "POST",
            paths::SITE_TUNNEL,
            "a signed token for the tunnel (the CLI uses this)",
        ),
        (
            "GET",
            result_paths::SITE_RESULTS,
            "per-version roster: opened, started, stayed, where they came from",
        ),
        (
            "GET",
            result_paths::SITE_VERSION_SESSIONS,
            "one row per player for a version",
        ),
        ("GET", result_paths::SITE_FEEDBACK, "what players wrote"),
        (
            "PATCH",
            result_paths::SITE_FEEDBACK_ITEM,
            "mark a note seen/done, or hide it",
        ),
        (
            "POST",
            ingest_paths::EVENTS,
            "the in-page SDK reports errors and milestones here",
        ),
        (
            "POST",
            ingest_paths::FEEDBACK,
            "the in-page SDK submits a player's note here",
        ),
    ]
}

fn openapi() -> Value {
    let mut paths_obj = serde_json::Map::new();
    for (method, path, what) in endpoints() {
        let entry = paths_obj
            .entry(path.to_string())
            .or_insert_with(|| json!({}));
        entry[method.to_lowercase()] = json!({
            "summary": what,
            "responses": { "200": { "description": what } },
        });
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
                "bearer": { "type": "http", "scheme": "bearer" }
            }
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

Single binary, no runtime. Releases: https://github.com/roviix/playtest.run/releases

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
- `-m, --note` what changed in this version — shown on the gate page, sent to followers, and
  used as the \"what I want you to look at\" line on the plaza
- `--summary` one line about what the project is
- `--cover cover.png` the first thing players see; without it we typeset a plain title card
- `--public` put it on the plaza; `--seats 10` also marks it as looking for that many testers
- `--card <path>` where to write the invite card (`-` to skip it)
- `--isolated=on|off|auto` cross-origin isolation; Godot 4 threaded exports need it (auto detects)

## After publishing, hand the user three things

1. The **url**.
2. The **QR code** — paste `qr_text` verbatim inside a code block. It only scans in a monospace
   block; do not reflow or summarise it.
3. The **invite card** image at `card_path`. This matters more than it sounds: in WeChat a bare
   link shows no title and no face, and a picture with the developer's name on it is what people
   actually forward. Attach the image; do not just say it exists.

## Reading failures

Exit codes: 0 ok, 2 usage, 3 needs login, 4 network, 5 server, 6 bad input, 7 quota.

Exit 6 usually means we inspected the build and know it will not open — a missing top-level
`index.html`, a threaded export without isolation, a missing `.pck`/`.data`. `message` and `hint`
say what to change. Fix the build; retrying the same bytes will fail the same way.

## What to tell the user honestly

- Without logging in, the link expires in {hours} hours. `playtest login` (GitHub) keeps it, and
  adopts links they already made anonymously.
- We never run their code and never rewrite their bytes.
- Players never need an account. First play is zero-friction by design.
- This is not general static hosting; the root domain is a plaza for works looking for testers.

## Other commands

```
playtest ls                 # projects on this machine (with follower counts)
playtest versions <slug>    # every version, and which one players see
playtest rollback <slug> 3  # point players back at v3; no bytes re-uploaded
playtest card <slug>        # fetch the current invite card again
playtest unlist <slug>      # take it off the plaza; the link keeps working
playtest open <slug>
playtest rm <slug> -y
```

## HTTP API

If you cannot run the binary: {api}/llms-full.txt and {api}/openapi.json.
Publishing is `POST /v1/sites` -> `POST /v1/sites/{{slug}}/uploads` -> `PUT /v1/blobs/{{hash}}`
-> `POST /v1/sites/{{slug}}/uploads/{{id}}/commit`.
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
        for (method, path, _) in endpoints() {
            assert!(listed.contains_key(path), "OpenAPI 里少了 {method} {path}");
            assert!(
                listed[path].get(method.to_lowercase()).is_some(),
                "OpenAPI 的 {path} 上少了 {method}"
            );
            assert!(text.contains(path), "llms-full.txt 里少了 {path}");
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
