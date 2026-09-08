//! 请求路径 → 清单里的一条 → 要出的 blob 和响应头（DESIGN §4.2）。
//!
//! 这里是「对游戏有感」的一半：预压缩产物怎么挑、`.wasm` 给什么 MIME、
//! 目录要不要 301。另一半在 [`crate::range`] 和响应头组装里。

use percent_encoding::{percent_decode_str, utf8_percent_encode, AsciiSet, CONTROLS};
use playtest_common::manifest::{FileEntry, Manifest};

/// 重新拼 `Location` 时要编码的字符。`/` 不在里面，路径分隔符要保留原样；
/// `%` 在里面，因为待编码的字符串是解码过的，里面的 `%` 是字面量。
const LOCATION_ESCAPE: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'`')
    .add(b'{')
    .add(b'}')
    .add(b'%')
    .add(b'\\');

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AcceptEncoding {
    pub br: bool,
    pub gzip: bool,
}

/// 只认 `br` 与 `gzip`，`q=0` 当没写。`*` 按 RFC 9110 算「其余都接受」。
pub fn parse_accept_encoding(raw: Option<&str>) -> AcceptEncoding {
    let mut out = AcceptEncoding::default();
    let Some(raw) = raw else { return out };
    for part in raw.split(',') {
        let mut fields = part.split(';');
        let token = fields.next().unwrap_or("").trim().to_ascii_lowercase();
        let refused = fields.any(|f| {
            f.trim()
                .strip_prefix("q=")
                .and_then(|q| q.trim().parse::<f32>().ok())
                .is_some_and(|q| q <= 0.0)
        });
        if refused {
            continue;
        }
        match token.as_str() {
            "br" => out.br = true,
            "gzip" => out.gzip = true,
            "*" => {
                out.br = true;
                out.gzip = true;
            }
            _ => {}
        }
    }
    out
}

/// 解码并检查过的请求路径。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Normalized {
    /// 去掉前导斜杠、解码后的相对路径。`/` 得到空串，`/sub/` 得到 `sub/`。
    pub rel: String,
    /// 要去清单里找的路径，已经补过 `index.html`。
    pub candidate: String,
    /// 请求写成了目录形式。
    pub directory: bool,
}

/// 百分号解码 + 形态检查。返回 `None` 就是 404，不再往下走。
pub fn normalize(raw_path: &str) -> Option<Normalized> {
    let decoded = percent_decode_str(raw_path).decode_utf8().ok()?;
    let directory = decoded.is_empty() || decoded.ends_with('/');
    let rel = decoded.trim_start_matches('/').to_string();
    if rel.chars().any(char::is_control) || rel.contains('\\') {
        return None;
    }
    // 穿越只可能来自这两种段。清单里的路径是 api 校验过的，不会有它们；
    // 取 blob 也是按哈希拼路径不是按请求路径，这一道是双保险。
    if rel.split('/').any(|seg| seg == "." || seg == "..") {
        return None;
    }
    let candidate = if directory {
        format!("{rel}index.html")
    } else {
        rel.clone()
    };
    Some(Normalized {
        rel,
        candidate,
        directory,
    })
}

/// 选中的那个 blob，以及由它决定的响应头。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Served {
    pub entry: FileEntry,
    pub content_type: String,
    /// `Content-Encoding` 的值，出预压缩产物时才有。
    pub encoding: Option<&'static str>,
    /// 这条路径的响应会随 `Accept-Encoding` 变。
    pub vary_encoding: bool,
    pub is_html: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolved {
    File(Box<Served>),
    /// 301 到目录形式，否则页面里的相对链接全部指错一层。
    Redirect(String),
    NotFound,
}

pub fn resolve(
    manifest: &Manifest,
    norm: &Normalized,
    accept: AcceptEncoding,
    navigation: bool,
) -> Resolved {
    if let Some(served) = select(manifest, &norm.candidate, accept) {
        return Resolved::File(Box::new(served));
    }
    if !norm.directory && !norm.rel.is_empty() {
        let as_dir = format!("{}/index.html", norm.rel);
        if manifest.find(&as_dir).is_some() {
            let encoded = utf8_percent_encode(&norm.rel, LOCATION_ESCAPE).to_string();
            return Resolved::Redirect(format!("/{encoded}/"));
        }
    }
    // SPA 回退只给导航请求：一个 404 的贴图回退成 HTML 只会让加载器报更奇怪的错。
    if manifest.spa && navigation {
        if let Some(served) = select(manifest, "index.html", accept) {
            return Resolved::File(Box::new(served));
        }
    }
    Resolved::NotFound
}

/// 两条预压缩规则（DESIGN §4.2）：
///
/// (a) 请求本身就带 `.br` / `.gz`（Unity 的加载器直接请求 `Build/x.wasm.br`）：
///     照出，`Content-Type` 按里层扩展名定，别的都不管。
/// (b) 请求 `foo.wasm` 而清单里有 `foo.wasm.br`：客户端接受就优先给压缩的，
///     哪怕未压缩的那份也在清单里——省的是玩家的等待时间。
fn select(manifest: &Manifest, candidate: &str, accept: AcceptEncoding) -> Option<Served> {
    if let Some(inner) = strip_encoding_suffix(candidate) {
        let entry = manifest.find(candidate)?;
        return Some(Served {
            entry: entry.clone(),
            content_type: content_type_for(inner.0),
            encoding: Some(inner.1),
            vary_encoding: false,
            is_html: is_html_path(inner.0),
        });
    }

    let is_html = is_html_path(candidate);
    let content_type = content_type_for(candidate);
    let brotli = format!("{candidate}.br");
    let gzipped = format!("{candidate}.gz");
    let has_brotli = manifest.find(&brotli).is_some();
    let has_gzip = manifest.find(&gzipped).is_some();
    let vary_encoding = has_brotli || has_gzip;

    let picked = if accept.br && has_brotli {
        Some((manifest.find(&brotli)?, Some("br")))
    } else if accept.gzip && has_gzip {
        Some((manifest.find(&gzipped)?, Some("gzip")))
    } else {
        manifest.find(candidate).map(|e| (e, None))
    };

    let (entry, encoding) = picked?;
    Some(Served {
        entry: entry.clone(),
        content_type,
        encoding,
        vary_encoding,
        is_html,
    })
}

fn strip_encoding_suffix(path: &str) -> Option<(&str, &'static str)> {
    if let Some(inner) = path.strip_suffix(".br") {
        Some((inner, "br"))
    } else {
        path.strip_suffix(".gz").map(|inner| (inner, "gzip"))
    }
}

pub fn is_html_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".html") || lower.ends_with(".htm")
}

/// 引擎导出物踩过的坑都写在这张表里：`.wasm` 不给 `application/wasm` 就没有流式编译；
/// `.js` 有些系统按 `application/javascript` 给会让 `type=module` 挑剔；
/// Unity 的 `.data` / `.unityweb` 和 Godot 的 `.pck` 猜出来的类型经常是错的，一律当字节流。
pub fn content_type_for(path: &str) -> String {
    let file = path.rsplit('/').next().unwrap_or(path);
    let ext = file
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "wasm" => "application/wasm".to_string(),
        "js" | "mjs" => "text/javascript; charset=utf-8".to_string(),
        "html" | "htm" => "text/html; charset=utf-8".to_string(),
        "pck" | "data" | "unityweb" | "bin" => "application/octet-stream".to_string(),
        "json" => "application/json".to_string(),
        _ => match mime_guess::from_path(file).first() {
            Some(mime)
                if mime.type_() == mime_guess::mime::TEXT
                    && mime.get_param("charset").is_none() =>
            {
                format!("{mime}; charset=utf-8")
            }
            Some(mime) => mime.to_string(),
            None => "application/octet-stream".to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use playtest_common::hash::hash_bytes;
    use playtest_common::manifest::{GateMode, SCHEMA};

    fn entry(path: &str) -> FileEntry {
        FileEntry {
            path: path.to_string(),
            hash: hash_bytes(path.as_bytes()),
            size: 1,
        }
    }

    fn manifest(paths: &[&str]) -> Manifest {
        Manifest {
            schema: SCHEMA,
            slug: "brisk-otter-41".into(),
            version: 7,
            title: "测试".into(),
            developer: "某某".into(),
            note: None,
            summary: None,
            cover: None,
            created_at: "2026-09-07T00:00:00Z".into(),
            expires_at: None,
            badge: true,
            gate: GateMode::Once,
            isolated: false,
            spa: false,
            engine: None,
            files: paths.iter().map(|p| entry(p)).collect(),
        }
    }

    #[test]
    fn normalizes_root_and_directories() {
        let n = normalize("/").unwrap();
        assert_eq!(n.candidate, "index.html");
        assert!(n.directory);

        let n = normalize("/sub/").unwrap();
        assert_eq!(n.candidate, "sub/index.html");

        let n = normalize("/Build/game.wasm").unwrap();
        assert_eq!(n.candidate, "Build/game.wasm");
        assert!(!n.directory);

        let n = normalize("/%E4%B8%AD%E6%96%87.html").unwrap();
        assert_eq!(n.candidate, "中文.html");
    }

    #[test]
    fn rejects_traversal_and_control_chars() {
        assert!(normalize("/../etc/passwd").is_none());
        assert!(normalize("/a/../../b").is_none());
        assert!(normalize("/%2e%2e/secret").is_none());
        assert!(normalize("/a/./b").is_none());
        assert!(normalize("/a%00b").is_none());
        assert!(normalize("/a%5Cb").is_none());
        // 文件名里连着两个点不是穿越，别误伤。
        assert_eq!(
            normalize("/game..min.js").unwrap().candidate,
            "game..min.js"
        );
    }

    #[test]
    fn parses_accept_encoding() {
        assert_eq!(parse_accept_encoding(None), AcceptEncoding::default());
        assert_eq!(
            parse_accept_encoding(Some("gzip, deflate, br, zstd")),
            AcceptEncoding {
                br: true,
                gzip: true
            }
        );
        assert_eq!(
            parse_accept_encoding(Some("gzip;q=1.0, br;q=0")),
            AcceptEncoding {
                br: false,
                gzip: true
            }
        );
        assert_eq!(
            parse_accept_encoding(Some("*")),
            AcceptEncoding {
                br: true,
                gzip: true
            }
        );
        assert_eq!(
            parse_accept_encoding(Some("identity")),
            AcceptEncoding::default()
        );
    }

    #[test]
    fn explicit_compressed_path_keeps_inner_type() {
        let m = manifest(&["Build/x.wasm.br", "Build/x.data.br"]);
        let n = normalize("/Build/x.wasm.br").unwrap();
        let Resolved::File(s) = resolve(&m, &n, AcceptEncoding::default(), false) else {
            panic!("应当命中文件");
        };
        assert_eq!(s.encoding, Some("br"));
        assert_eq!(s.content_type, "application/wasm");
        assert!(!s.vary_encoding);

        let n = normalize("/Build/x.data.br").unwrap();
        let Resolved::File(s) = resolve(&m, &n, AcceptEncoding::default(), false) else {
            panic!("应当命中文件");
        };
        assert_eq!(s.content_type, "application/octet-stream");
    }

    #[test]
    fn prefers_precompressed_when_accepted() {
        let m = manifest(&["game.wasm", "game.wasm.br"]);
        let n = normalize("/game.wasm").unwrap();

        let accepted = resolve(
            &m,
            &n,
            AcceptEncoding {
                br: true,
                gzip: true,
            },
            false,
        );
        let Resolved::File(s) = accepted else {
            panic!("应当命中文件")
        };
        assert_eq!(s.entry.path, "game.wasm.br");
        assert_eq!(s.encoding, Some("br"));
        assert_eq!(s.content_type, "application/wasm");
        assert!(s.vary_encoding);

        // 不接受压缩时退回未压缩那份，但仍然要 Vary。
        let plain = resolve(&m, &n, AcceptEncoding::default(), false);
        let Resolved::File(s) = plain else {
            panic!("应当命中文件")
        };
        assert_eq!(s.entry.path, "game.wasm");
        assert_eq!(s.encoding, None);
        assert!(s.vary_encoding);
    }

    #[test]
    fn gzip_only_client_gets_gzip() {
        let m = manifest(&["app.js.br", "app.js.gz"]);
        let n = normalize("/app.js").unwrap();
        let Resolved::File(s) = resolve(
            &m,
            &n,
            AcceptEncoding {
                br: false,
                gzip: true,
            },
            false,
        ) else {
            panic!("应当命中文件");
        };
        assert_eq!(s.entry.path, "app.js.gz");
        assert_eq!(s.encoding, Some("gzip"));
        assert_eq!(s.content_type, "text/javascript; charset=utf-8");
    }

    #[test]
    fn only_compressed_and_client_refuses_is_not_found() {
        let m = manifest(&["game.wasm.br"]);
        let n = normalize("/game.wasm").unwrap();
        assert_eq!(
            resolve(&m, &n, AcceptEncoding::default(), false),
            Resolved::NotFound
        );
    }

    #[test]
    fn directory_without_slash_redirects() {
        let m = manifest(&["index.html", "sub/index.html"]);
        let n = normalize("/sub").unwrap();
        assert_eq!(
            resolve(&m, &n, AcceptEncoding::default(), true),
            Resolved::Redirect("/sub/".into())
        );

        let n = normalize("/深 层").unwrap();
        let m = manifest(&["深 层/index.html"]);
        assert_eq!(
            resolve(&m, &n, AcceptEncoding::default(), true),
            Resolved::Redirect("/%E6%B7%B1%20%E5%B1%82/".into())
        );
    }

    #[test]
    fn spa_fallback_only_for_navigation() {
        let mut m = manifest(&["index.html"]);
        m.spa = true;
        let n = normalize("/level/3").unwrap();
        let Resolved::File(s) = resolve(&m, &n, AcceptEncoding::default(), true) else {
            panic!("导航请求应当回退到 index.html");
        };
        assert_eq!(s.entry.path, "index.html");
        assert_eq!(
            resolve(&m, &n, AcceptEncoding::default(), false),
            Resolved::NotFound
        );
    }

    #[test]
    fn content_types_cover_engine_exports() {
        assert_eq!(content_type_for("a/b.wasm"), "application/wasm");
        assert_eq!(content_type_for("m.mjs"), "text/javascript; charset=utf-8");
        assert_eq!(content_type_for("i.html"), "text/html; charset=utf-8");
        assert_eq!(content_type_for("g.pck"), "application/octet-stream");
        assert_eq!(content_type_for("b.unityweb"), "application/octet-stream");
        assert_eq!(content_type_for("m.json"), "application/json");
        assert_eq!(content_type_for("s.css"), "text/css; charset=utf-8");
        assert_eq!(content_type_for("p.png"), "image/png");
        assert_eq!(content_type_for("LICENSE"), "application/octet-stream");
        assert_eq!(content_type_for("weird.qqq"), "application/octet-stream");
    }
}
