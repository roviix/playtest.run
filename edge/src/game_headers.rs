//! 「对游戏有感」的那几个响应头，抽成一个不碰 IO 的纯函数（DESIGN §4.2、§4.3）。
//!
//! 上传路径和隧道路径必须给出同一套头：Godot 4 的开发者敲 `playtest 5173` 和
//! `playtest ./dist` 不该一个能玩一个白屏。dev server 不发这些头（Vite 不发 COOP/COEP，
//! `python3 -m http.server` 一个都不发），真机试过的十个竞品隧道也没有一个会补。
//! 所以这里只做「补头」，[`apply`] 不知道响应体长什么样，也永远不改它一个字节（DESIGN §3.7）。
//!
//! 隧道路径这样用：拿到上游响应之后
//! `game_headers::apply(resp.headers_mut(), 请求路径, manifest.isolated, Source::Upstream,
//!     Opts { resource: 不是顶层文档, ..Opts::default() })`。
//! `Source::Upstream` 只补上游没给或给错到会让游戏跑不起来的那几个，不越权：
//! `Content-Encoding` 一律不动（体是上游的，我们没压过它），`Accept-Ranges` 也不替它承诺。

use axum::http::{HeaderMap, HeaderValue};

use crate::paths::content_type_for;

/// 这条响应的字节是谁产生的。决定我们对已有的头有多大发言权。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// 上传路径：文件是边缘按清单从对象存储取出来的，路径说了算，直接覆盖。
    Ours,
    /// 隧道路径：字节来自开发者自己的进程。只补它没给的，以及 `.wasm` 这种
    /// 给错了就直接没有流式编译的。别的保持原样——猜错一个头比少补一个头难查。
    Upstream,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Opts {
    /// 这条响应是子资源而不是顶层文档：跨源隔离时多给一条 CORP。
    pub resource: bool,
    /// 出去的字节确实是这个编码压过的。`None` 表示原样，别写这个头。
    pub encoding: Option<&'static str>,
    /// 这条路径的响应会随 `Accept-Encoding` 变（目录里同时有压缩和未压缩两份）。
    pub vary_encoding: bool,
    /// 这条响应支持按字节续传。
    pub ranges: bool,
}

/// 把该补的头补上。`path` 是请求路径（`/Build/game.wasm`，可以带前导斜杠）。
pub fn apply(headers: &mut HeaderMap, path: &str, isolated: bool, source: Source, opts: Opts) {
    content_type(headers, path, source, opts.encoding);
    encoding(headers, path, source, opts.encoding);
    if opts.vary_encoding {
        put(headers, "vary", "Accept-Encoding");
    }
    if opts.ranges {
        put(headers, "accept-ranges", "bytes");
    }
    isolation(headers, isolated, opts.resource);
}

/// `isolated` 为真就给整个作品加跨源隔离头：Godot 4 的线程导出要靠它才拿得到
/// SharedArrayBuffer，没这两个头直接报错。子资源再加一条 CORP，否则 COEP 会把
/// 它们自己挡在外面。
///
/// 单独暴露是因为门禁页、301、举报页这些我们自己渲染的响应没有「文件路径」，
/// 但同样在这个作品的源下，隔离头必须一致——点了开始之后才隔离等于没隔离。
pub fn isolation(headers: &mut HeaderMap, isolated: bool, resource: bool) {
    if !isolated {
        return;
    }
    put(headers, "cross-origin-opener-policy", "same-origin");
    put(headers, "cross-origin-embedder-policy", "require-corp");
    if resource {
        put(headers, "cross-origin-resource-policy", "same-origin");
    }
}

/// Unity 的 Decompression Fallback 产物是 `.unityweb`：字节里已经是 brotli 或 gzip，
/// 但解压是 Unity 的加载器在 JS 里做的，浏览器不能插手。给它加 `Content-Encoding`，
/// 浏览器会先解一遍、加载器再解一遍，直接崩——Unity 的两种导出正好要求相反的服务器配置
/// （`.br` 要配头，Decompression Fallback 要不配头），配反是这一格最常见的坑。
///
/// 判据是请求路径最后一个扩展名：`Build/x.wasm.unityweb` 命中；
/// `x.unityweb.br`（谁把 fallback 产物又压了一遍）不命中，那种情况出去的确实是 brotli 流。
pub fn may_set_encoding(path: &str) -> bool {
    !last_ext(path).eq_ignore_ascii_case("unityweb")
}

fn content_type(headers: &mut HeaderMap, path: &str, source: Source, encoding: Option<&str>) {
    let ours = content_type_for(mime_path(path, encoding));
    match source {
        Source::Ours => put(headers, "content-type", &ours),
        Source::Upstream => {
            let existing = headers
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            // 上游什么都没说，或者把 wasm 说成了别的：`.wasm` 不给 `application/wasm`
            // 就没有 `WebAssembly.instantiateStreaming`，加载器要么退回慢路径要么直接报错。
            // 其余类型上游说什么算什么——dev server 给 `.js` 加自己的 charset 是常事。
            if existing.is_empty() || (ours == "application/wasm" && existing != ours) {
                put(headers, "content-type", &ours);
            }
        }
    }
}

fn encoding(headers: &mut HeaderMap, path: &str, source: Source, encoding: Option<&'static str>) {
    if source == Source::Upstream {
        // 体是上游压的（或没压），它自己已经写过这个头了。我们插手只会写错。
        return;
    }
    let Some(value) = encoding else { return };
    if !may_set_encoding(path) {
        tracing::debug!(path, value, "Decompression Fallback 产物不加 Content-Encoding");
        return;
    }
    put(headers, "content-encoding", value);
}

/// 算 MIME 时看的是「解压之后是什么」。Unity 的加载器直接请求 `Build/x.wasm.br`，
/// 出去的 `Content-Type` 得是里层的 `application/wasm`，不是 `.br` 猜出来的东西。
fn mime_path<'a>(path: &'a str, encoding: Option<&str>) -> &'a str {
    let suffix = match encoding {
        Some("br") => ".br",
        Some("gzip") => ".gz",
        _ => return path,
    };
    path.strip_suffix(suffix).unwrap_or(path)
}

fn last_ext(path: &str) -> &str {
    let file = path.rsplit('/').next().unwrap_or(path);
    file.rsplit_once('.').map(|(_, ext)| ext).unwrap_or("")
}

fn put(headers: &mut HeaderMap, name: &'static str, value: &str) {
    match HeaderValue::from_str(value) {
        Ok(v) => {
            headers.insert(name, v);
        }
        Err(err) => tracing::warn!(name, value, %err, "响应头的值不合法，丢掉"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers() -> HeaderMap {
        HeaderMap::new()
    }

    fn get<'a>(h: &'a HeaderMap, name: &str) -> Option<&'a str> {
        h.get(name).and_then(|v| v.to_str().ok())
    }

    #[test]
    fn wasm_gets_the_streaming_compilation_mime() {
        let mut h = headers();
        apply(&mut h, "/Build/game.wasm", false, Source::Ours, Opts::default());
        assert_eq!(get(&h, "content-type"), Some("application/wasm"));
        assert_eq!(get(&h, "content-encoding"), None);
        assert_eq!(get(&h, "accept-ranges"), None);
    }

    #[test]
    fn precompressed_keeps_the_inner_type() {
        let mut h = headers();
        apply(
            &mut h,
            "/Build/game.wasm.br",
            false,
            Source::Ours,
            Opts {
                encoding: Some("br"),
                ranges: true,
                ..Opts::default()
            },
        );
        assert_eq!(get(&h, "content-type"), Some("application/wasm"));
        assert_eq!(get(&h, "content-encoding"), Some("br"));
        assert_eq!(get(&h, "accept-ranges"), Some("bytes"));

        // 请求的是未压缩的名字、我们出的是 .gz 那份：路径上没有后缀可剥，类型照样对。
        let mut h = headers();
        apply(
            &mut h,
            "/app.js",
            false,
            Source::Ours,
            Opts {
                encoding: Some("gzip"),
                vary_encoding: true,
                ..Opts::default()
            },
        );
        assert_eq!(get(&h, "content-type"), Some("text/javascript; charset=utf-8"));
        assert_eq!(get(&h, "content-encoding"), Some("gzip"));
        assert_eq!(get(&h, "vary"), Some("Accept-Encoding"));
    }

    #[test]
    fn unityweb_never_gets_content_encoding() {
        // Decompression Fallback 的产物。就算调用方递了一个编码进来也要丢掉。
        for path in ["/Build/x.wasm.unityweb", "/Build/x.data.unityweb", "/a.UNITYWEB"] {
            let mut h = headers();
            apply(
                &mut h,
                path,
                false,
                Source::Ours,
                Opts {
                    encoding: Some("br"),
                    ..Opts::default()
                },
            );
            assert_eq!(get(&h, "content-encoding"), None, "{path}");
            assert!(!may_set_encoding(path), "{path}");
        }
        assert_eq!(
            {
                let mut h = headers();
                apply(
                    &mut h,
                    "/Build/x.wasm.unityweb",
                    false,
                    Source::Ours,
                    Opts::default(),
                );
                get(&h, "content-type").map(str::to_string)
            },
            Some("application/octet-stream".to_string())
        );

        // 反过来：谁真的把 fallback 产物又 brotli 了一遍，出去的确实是 brotli 流。
        assert!(may_set_encoding("/Build/x.unityweb.br"));
        assert!(may_set_encoding("/Build/x.wasm.br"));
    }

    #[test]
    fn isolation_is_all_or_nothing() {
        let mut h = headers();
        isolation(&mut h, false, true);
        assert!(h.is_empty());

        let mut h = headers();
        isolation(&mut h, true, false);
        assert_eq!(get(&h, "cross-origin-opener-policy"), Some("same-origin"));
        assert_eq!(get(&h, "cross-origin-embedder-policy"), Some("require-corp"));
        // 顶层文档不给 CORP。
        assert_eq!(get(&h, "cross-origin-resource-policy"), None);

        let mut h = headers();
        isolation(&mut h, true, true);
        assert_eq!(get(&h, "cross-origin-resource-policy"), Some("same-origin"));
    }

    #[test]
    fn upstream_responses_are_only_corrected_where_it_breaks_the_game() {
        // 隧道回来的响应：dev server 什么都没说 → 补上。
        let mut h = headers();
        apply(&mut h, "/game.wasm", true, Source::Upstream, Opts { resource: true, ..Opts::default() });
        assert_eq!(get(&h, "content-type"), Some("application/wasm"));
        assert_eq!(get(&h, "cross-origin-embedder-policy"), Some("require-corp"));
        assert_eq!(get(&h, "cross-origin-resource-policy"), Some("same-origin"));

        // dev server 把 wasm 说成了字节流 → 换掉，否则没有流式编译。
        let mut h = headers();
        h.insert("content-type", HeaderValue::from_static("application/octet-stream"));
        apply(&mut h, "/game.wasm", false, Source::Upstream, Opts::default());
        assert_eq!(get(&h, "content-type"), Some("application/wasm"));

        // 别的类型上游说了算，不替它改。
        let mut h = headers();
        h.insert("content-type", HeaderValue::from_static("application/javascript"));
        apply(&mut h, "/main.js", false, Source::Upstream, Opts::default());
        assert_eq!(get(&h, "content-type"), Some("application/javascript"));

        // 体是上游的，Content-Encoding 一个字都不动。
        let mut h = headers();
        h.insert("content-encoding", HeaderValue::from_static("gzip"));
        apply(
            &mut h,
            "/main.js",
            false,
            Source::Upstream,
            Opts {
                encoding: Some("br"),
                ..Opts::default()
            },
        );
        assert_eq!(get(&h, "content-encoding"), Some("gzip"));
    }

    #[test]
    fn engine_export_types() {
        for (path, want) in [
            ("/index.html", "text/html; charset=utf-8"),
            ("/game.pck", "application/octet-stream"),
            ("/Build/x.data", "application/octet-stream"),
            ("/s.css", "text/css; charset=utf-8"),
            ("/LICENSE", "application/octet-stream"),
        ] {
            let mut h = headers();
            apply(&mut h, path, false, Source::Ours, Opts::default());
            assert_eq!(get(&h, "content-type"), Some(want), "{path}");
        }
    }
}
