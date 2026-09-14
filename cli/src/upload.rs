//! `playtest ./dist`：整理目录 → 问服务器缺哪些 → 只传缺的 → 提交 → 打印链接。

use std::collections::{HashMap, HashSet};
use std::io::{IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use playtest_common::api::{CreateSiteRequest, ErrorCode, PrepareUploadRequest, UpdateSiteRequest};
use playtest_common::limits::{self, ANON_MAX_VERSION_BYTES};
use playtest_common::manifest::{self, validate_files, ChapterEntry, Cover, FileEntry, WorkKind};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::args::{self, Isolation, UploadArgs};
use crate::client::{Client, OnBytes};
use crate::config::{self, Config};
use crate::output::{self, Finding, PlazaOut, Timings, UploadReport};
use crate::scan::{self, ScannedFile};
use crate::{card, clock, inspect, ui};

/// 同时传几个。再多在家宽上行（普遍 30–50 Mbps）上不会更快，只会让进度条更跳。
const UPLOAD_CONCURRENCY: usize = 4;

/// 一个文件最多试几次。
const MAX_ATTEMPTS: u32 = 3;

/// 第一次重试等多久，之后每次翻倍。
const FIRST_BACKOFF: Duration = Duration::from_millis(500);

/// slug 是从哪来的。只有「上次记住的」那种在服务器不认时才好自动换新——
/// 用户自己用 `--site` 指定的，悄悄换成另一个作品是不能接受的。
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SlugSource {
    Explicit,
    Remembered,
    Fresh,
}

struct UploadInput {
    root: PathBuf,
    remember_key: String,
    files: Vec<ScannedFile>,
    kind: WorkKind,
    entry: Option<String>,
    title_hint: Option<String>,
    checked: inspect::Report,
    chapters: Vec<ChapterEntry>,
}

/// 做完了返回这一次的全部结果，由调用方决定说给谁听——终端、`--json`、还是 MCP 的对话框。
pub async fn run(cli_args: &UploadArgs, shown: &str) -> Result<UploadReport> {
    let mut timings = Timings::default();

    let hashing = Instant::now();
    let input = prepare_input(shown, cli_args.backend.is_some())
        .await
        .map_err(output::as_bad_input)?;
    timings.hash_ms = output::ms_since(hashing);
    let UploadInput {
        root,
        remember_key,
        files,
        kind,
        entry,
        title_hint,
        mut checked,
        chapters,
    } = input;
    if files.is_empty() {
        return Err(output::bad_input(format!(
            "{shown} 是空的，没有可以上传的文件。（以「.」开头的文件和目录不会上传。）"
        )));
    }

    let entries: Vec<FileEntry> = files.iter().map(|f| f.entry.clone()).collect();
    let paths: Vec<String> = entries.iter().map(|e| e.path.clone()).collect();
    let total_bytes: u64 = entries.iter().map(|e| e.size).sum();
    ui::say(&format!(
        "正在整理 {shown}：{} 个文件，{}",
        entries.len(),
        ui::bytes(total_bytes)
    ));

    check_limits(&entries, &paths).map_err(output::as_bad_input)?;
    check_kind_options(cli_args, kind).map_err(output::as_bad_input)?;
    // 「传上去一定打不开」的就别传了：匿名作品只有三个名额，一个 404 的链接会白占一个。
    // 拦下时只报这一条（它就是错误本身），别的发现留给下一次。
    if let Some(blocker) = checked
        .findings
        .iter()
        .find(|f| f.level == output::Level::Blocker)
        .filter(|_| !cli_args.force)
    {
        let hint = match &blocker.hint {
            Some(h) => format!("{h}。确定要照传就加 -y。"),
            None => "确定要照传就加 -y。".to_string(),
        };
        return Err(output::bad_input_with_hint(blocker.message.clone(), hint));
    }
    output::say_findings(&checked.findings);
    let isolated = if kind == WorkKind::Web {
        choose_isolated(&mut checked, cli_args)
    } else {
        false
    };

    let title = match kind {
        WorkKind::Web => title_for(cli_args, &root, checked.page_title.as_deref()),
        _ => title_for_file(cli_args, title_hint.as_deref()),
    }
    .map_err(output::as_bad_input)?;
    check_note(cli_args).map_err(output::as_bad_input)?;
    check_plaza_inputs(cli_args).map_err(output::as_bad_input)?;
    let cover = match &cli_args.cover {
        Some(path) => Some(read_cover(path).map_err(output::as_bad_input)?),
        None => None,
    };

    let preparing = Instant::now();
    let config_path = config::default_path()?;
    let mut config = config::load(&config_path)?;
    let api = args::api_base(cli_args.api.as_deref(), config.api.as_deref());
    let mut client = Client::new(&api)?;
    ensure_token(&mut client, &mut config, &config_path, &api).await?;

    let (mut slug, source) = choose_site(
        &mut client,
        cli_args,
        &mut config,
        &config_path,
        &api,
        &remember_key,
        &title,
    )
    .await?;

    let request = PrepareUploadRequest {
        files: entries.clone(),
        kind,
        entry,
        title: Some(title.clone()),
        note: cli_args.note.clone(),
        summary: cli_args.summary.clone(),
        cover: cover.as_ref().map(|c| c.cover.clone()),
        gate: manifest::GateMode::Once,
        isolated,
        spa: kind == WorkKind::Web && cli_args.spa,
        engine: (kind == WorkKind::Web)
            .then(|| checked.manifest_engine())
            .flatten(),
        chapters,
    };

    let prepared = match client.prepare_upload(&slug, &request).await {
        Ok(prepared) => prepared,
        Err(e) if e.means_token_gone() && config.is_logged_in() => {
            return Err(anyhow::anyhow!(
                "登录已经失效（{e}）。重新运行 playtest login。"
            ));
        }
        Err(e) if source == SlugSource::Remembered && e.means_anonymous_link_gone() => {
            if e.means_token_gone() {
                ui::say("上次的匿名链接已过期（匿名链接只保留 24 小时），这是一个新链接。");
                new_anon_session(&mut client, &mut config, &config_path, &api).await?;
            } else {
                ui::say("上次的作品已经不在了（匿名作品只保留 24 小时），这是一个新链接。");
            }
            slug = create_site(&client, &title).await?;
            config.remember(remember_key.clone(), slug.clone());
            config::save(&config_path, &config)?;
            client.prepare_upload(&slug, &request).await?
        }
        Err(e) => return Err(e.into()),
    };
    timings.prepare_ms = output::ms_since(preparing);

    // 令牌和作品都定下来了，后面只读，装进 Arc 好分给并发的上传任务。
    let client = Arc::new(client);
    let sending = Instant::now();
    // 封面和目录里的文件走同一条上传路（DESIGN §3.3），进度和「N 个文件」把它一起数。
    let mut to_upload: Vec<ScannedFile> = files.clone();
    if let Some(cover) = &cover {
        to_upload.push(cover.as_scanned());
    }
    send_missing(
        Arc::clone(&client),
        &to_upload,
        &prepared.missing,
        to_upload.len(),
    )
    .await?;
    timings.upload_ms = output::ms_since(sending);

    let committing = Instant::now();
    let committed = client.commit_upload(&slug, &prepared.upload_id).await?;
    timings.commit_ms = output::ms_since(committing);

    // 广场（DESIGN §3.8）：版本发出去之后再改状态，广场上出现的一定是能玩的东西。
    // 只给 `--community` 不上广场——群是给已经点进来的玩家看的（DESIGN §3.3），
    // 不能因为填了个群号就把作品挂出去。
    let wants_plaza = cli_args.wants_plaza();
    let updated = if wants_plaza || cli_args.community.is_some() {
        Some(
            client
                .update_site(
                    &slug,
                    &UpdateSiteRequest {
                        public: wants_plaza.then_some(true),
                        seeking: wants_plaza.then_some(cli_args.seeking()),
                        // 「想让人看什么」就是这一版的话（REWRITE §9.6）。
                        seek_note: cli_args.note.clone(),
                        seats: cli_args.seats,
                        community_url: cli_args.community.clone(),
                        // 「让玩家看到彼此的反馈」这一轮只在控制台里改（DESIGN §3.5）。
                        feedback_public: None,
                    },
                )
                .await?,
        )
    } else {
        None
    };

    let door_url = playtest_common::door_url(&committed.url, &committed.slug);
    let qr_text = if cli_args.no_qr {
        None
    } else {
        ui::qr_text(&door_url)
    };
    let mut report = UploadReport::new(
        committed.slug,
        door_url,
        title,
        committed.version,
        timings,
        committed.expires_at,
        qr_text,
        checked.findings,
    );
    report.kind = kind;
    if wants_plaza {
        if let Some(site) = &updated {
            report.on_plaza(PlazaOut {
                url: plaza_url(&report.url),
                public: site.listing.public,
                seeking: site.listing.seeking,
                seek_note: site.listing.seek_note.clone(),
            });
        }
    }
    // 名额报服务器认下来的那个数，不报命令行里写的那个：控制面还没认这一项时，
    // 门禁页上不会有「还差几位」，这里也就不该说「想找 10 位」。
    report.seats = updated.as_ref().and_then(|site| site.listing.seats);

    report.with_card(
        card::take(
            &report.url,
            &report.title,
            &report.slug,
            cli_args.card_path().flatten().as_deref(),
            !cli_args.wants_card(),
        )
        .await,
    );
    report.elapsed_ms = output::elapsed_ms();
    Ok(report)
}

/// 广场的地址就是玩家链接去掉 slug 那一级：`https://brisk-otter-41.playtest.run` → `https://playtest.run/`。
pub(crate) fn plaza_url(site_url: &str) -> String {
    playtest_common::root_url_from_site(site_url)
}

/// 读进来的封面：清单里那条引用，加上上传时按哪个路径读。
pub(crate) struct CoverFile {
    pub cover: Cover,
    pub source: PathBuf,
}

impl CoverFile {
    fn as_scanned(&self) -> ScannedFile {
        ScannedFile {
            entry: FileEntry {
                path: "（封面）".to_string(),
                hash: self.cover.hash.clone(),
                size: self.cover.size,
            },
            source: self.source.clone(),
        }
    }
}

/// `--cover` 指的那个文件：看开头几个字节认类型（扩展名会骗人），算哈希，检查大小。
pub(crate) fn read_cover(path: &Path) -> Result<CoverFile> {
    let shown = path.display();
    let meta = match std::fs::metadata(path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            bail!("找不到封面 {shown}。检查一下路径。")
        }
        Err(e) => return Err(e).with_context(|| format!("看不了封面 {shown}")),
    };
    if !meta.is_file() {
        bail!("封面 {shown} 不是一个文件。要一张 PNG、JPEG 或 WebP 图片。");
    }
    if meta.len() > limits::MAX_COVER_BYTES {
        bail!(
            "封面 {shown} 有 {}，最多 {} MB。缩一下再来——广场上一屏十几张封面，大了每个翻广场的人都在替它付流量。",
            ui::bytes(meta.len()),
            limits::MAX_COVER_BYTES / limits::MIB
        );
    }
    let bytes = std::fs::read(path).with_context(|| format!("读不了封面 {shown}"))?;
    let Some(mime) = manifest::sniff_image_mime(&bytes) else {
        bail!("封面 {shown} 不是 PNG、JPEG 或 WebP（看的是文件内容，不是扩展名）。SVG 和 GIF 都不收。");
    };
    let cover = Cover {
        hash: playtest_common::hash::hash_bytes(&bytes),
        size: bytes.len() as u64,
        mime: mime.to_string(),
    };
    manifest::validate_cover(&cover).map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(CoverFile {
        cover,
        source: std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()),
    })
}

/// `--summary` `--seek` `--seats` `--community` 本地先拦一道：这些是纯粹的输入错误，
/// 不值得先把几十 MB 传上去再被服务器退回来。
pub(crate) fn check_plaza_inputs(cli_args: &UploadArgs) -> Result<()> {
    if let Some(summary) = &cli_args.summary {
        if summary.chars().count() > limits::MAX_SUMMARY_CHARS {
            bail!(
                "「一句话介绍」太长了，最多 {} 个字——广场卡片上只放得下两行。",
                limits::MAX_SUMMARY_CHARS
            );
        }
    }
    if let Some(note) = &cli_args.note {
        if note.trim().is_empty() {
            bail!("-m 后面要跟一句话：这版改了什么，或者你想让来的人重点看什么。");
        }
        if note.chars().count() > limits::MAX_NOTE_CHARS {
            bail!("这一版的话太长了，最多 {} 个字。", limits::MAX_NOTE_CHARS);
        }
    }
    if let Some(seats) = cli_args.seats {
        // 0 位试玩者是一句自相矛盾的话，多半是手滑；上限说出具体数字，不让人再试一次。
        if seats == 0 {
            bail!("--seats 至少是 1。想取消找人测就别加这个参数。");
        }
        if seats > limits::MAX_SEATS {
            bail!(
                "--seats 最多 {}，你写了 {seats}。真要这么多人，分几批发更好组织。",
                limits::MAX_SEATS
            );
        }
    }
    if let Some(community) = &cli_args.community {
        check_community(community)?;
    }
    Ok(())
}

/// 群链接。玩家会从门禁页点出去，所以只收 http(s)——`qq://` 这类只有装了客户端的人点得动，
/// 在浏览器里就是一个死链。
fn check_community(url: &str) -> Result<()> {
    let trimmed = url.trim();
    if !(trimmed.starts_with("https://") || trimmed.starts_with("http://")) {
        bail!(
            "--community 要一条 http:// 或 https:// 开头的链接，你给的是「{trimmed}」。\n微信群、QQ 群没有网址的话，把群二维码传成一张图片放在网上，给那张图的地址。"
        );
    }
    if trimmed.chars().count() > limits::MAX_COMMUNITY_URL_CHARS {
        bail!(
            "群链接太长了，最多 {} 个字符。",
            limits::MAX_COMMUNITY_URL_CHARS
        );
    }
    Ok(())
}

fn resolve_dir(shown: &str) -> Result<PathBuf> {
    let given = Path::new(shown);
    let meta = match std::fs::metadata(given) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            bail!("找不到 {shown}。检查一下路径；如果还没构建，先构建出这个目录。")
        }
        Err(e) => return Err(e).with_context(|| format!("看不了 {shown}")),
    };
    if !meta.is_dir() {
        bail!(
            "{shown} 是一个文件。请给目录，不是文件——引擎导出的那个文件夹，里面通常有 index.html。"
        );
    }
    std::fs::canonicalize(given).with_context(|| format!("看不了 {shown}"))
}

async fn prepare_input(shown: &str, has_backend: bool) -> Result<UploadInput> {
    let given = Path::new(shown);
    let meta = match std::fs::symlink_metadata(given) {
        Ok(meta) => meta,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            bail!("找不到 {shown}。检查一下路径；如果还没构建，先构建出要发布的内容。")
        }
        Err(error) => return Err(error).with_context(|| format!("看不了 {shown}")),
    };
    if meta.file_type().is_symlink() {
        bail!("{shown} 是符号链接。请直接给真实的目录、Markdown 或 MP4 文件。")
    }
    if meta.is_dir() {
        let root = resolve_dir(shown)?;
        let files = scan::scan_dir(&root).await?;
        let entries = files
            .iter()
            .map(|file| file.entry.clone())
            .collect::<Vec<_>>();

        let has_any_html = entries
            .iter()
            .any(|e| e.path == "index.html" || e.path.ends_with("/index.html") || e.path.ends_with(".htm") || e.path.ends_with("/index.htm"));
        let has_web_assets = entries.iter().any(|e| {
            e.path.ends_with(".js")
                || e.path.ends_with(".mjs")
                || e.path.ends_with(".wasm")
                || e.path.ends_with(".pck")
                || e.path.ends_with(".unityweb")
                || e.path == "package.json"
                || e.path == "Cargo.toml"
                || e.path.starts_with("node_modules/")
                || e.path.starts_with("src/")
        });
        let mut md_files: Vec<&ScannedFile> = files
            .iter()
            .filter(|f| f.entry.path.ends_with(".md"))
            .collect();

        if !has_any_html && !has_web_assets && !md_files.is_empty() {
            md_files.sort_by(|a, b| a.entry.path.cmp(&b.entry.path));
            let mut chapters = Vec::new();
            let mut book_title = None;

            for (idx, sf) in md_files.iter().enumerate() {
                let content = std::fs::read_to_string(&sf.source).unwrap_or_default();
                let inspected = playtest_common::article::inspect(&content, &sf.entry.path).ok();
                let title = inspected
                    .as_ref()
                    .and_then(|i| i.title.clone())
                    .unwrap_or_else(|| {
                        Path::new(&sf.entry.path)
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("未命名章节")
                            .to_string()
                    });

                let stem = Path::new(&sf.entry.path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");

                if (stem.eq_ignore_ascii_case("readme") || stem.eq_ignore_ascii_case("index"))
                    && book_title.is_none()
                {
                    if let Some(ref ins) = inspected {
                        if ins.title.is_some() {
                            book_title = ins.title.clone();
                        }
                    }
                }

                let chapter_id = if !stem.is_empty()
                    && stem
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                {
                    stem.to_string()
                } else {
                    format!("c{}", idx + 1)
                };

                chapters.push(ChapterEntry {
                    id: chapter_id,
                    title,
                    path: sf.entry.path.clone(),
                    hash: sf.entry.hash.clone(),
                    size: sf.entry.size,
                });
            }

            let entry = chapters.first().map(|c| c.path.clone());
            let fallback_title = root
                .file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string());
            let title_hint = book_title.or(fallback_title);

            let mut checked = inspect::Report::default();
            checked.findings.push(Finding::note(format!(
                "这是一部连载作品：包含 {} 个章节，按文件名自然序编排",
                chapters.len()
            )));

            return Ok(UploadInput {
                remember_key: root.to_string_lossy().into_owned(),
                root,
                files,
                kind: WorkKind::Article,
                entry,
                title_hint,
                checked,
                chapters,
            });
        }

        let checked = inspect_dir(&root, &files, &entries, has_backend);
        return Ok(UploadInput {
            remember_key: root.to_string_lossy().into_owned(),
            root,
            files,
            kind: WorkKind::Web,
            entry: None,
            title_hint: None,
            checked,
            chapters: vec![],
        });
    }
    if !meta.is_file() {
        bail!("{shown} 不是普通文件或目录，不能发布。")
    }

    let source = std::fs::canonicalize(given).with_context(|| format!("看不了 {shown}"))?;
    let root = source
        .parent()
        .ok_or_else(|| anyhow::anyhow!("{shown} 没有可用的所在目录"))?
        .to_path_buf();
    let entry = source
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("{shown} 没有文件名"))?;
    let entry = scan::manifest_path(Path::new(entry)).map_err(anyhow::Error::msg)?;
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let fallback = source
        .file_stem()
        .and_then(|value| value.to_str())
        .map(str::to_string);

    let (kind, paths, title_hint, finding) = match extension.as_str() {
        "md" => {
            if meta.len() > limits::MAX_ARTICLE_SOURCE_BYTES {
                bail!(
                    "Markdown 原稿太大了：{}；首批文章上限是 {} MiB。",
                    ui::bytes(meta.len()),
                    limits::MAX_ARTICLE_SOURCE_BYTES / limits::MIB
                );
            }
            let markdown = std::fs::read_to_string(&source)
                .with_context(|| format!("{shown} 不是可读取的 UTF-8 Markdown"))?;
            let article = playtest_common::article::inspect(&markdown, &entry)?;
            let mut paths = Vec::with_capacity(article.images.len() + 1);
            paths.push(entry.clone());
            paths.extend(article.images);
            (
                WorkKind::Article,
                paths,
                article.title.or(fallback),
                Finding::note("这是一篇 Markdown 文章；只会上传正文明确引用的本地图片"),
            )
        }
        "mp4" => {
            let check = source.clone();
            let info = tokio::task::spawn_blocking(move || {
                let mut file = std::fs::File::open(&check)
                    .with_context(|| format!("读不了 {}", check.display()))?;
                playtest_common::video::inspect(&mut file).map_err(anyhow::Error::from)
            })
            .await
            .context("视频检查任务没跑完")??;
            (
                WorkKind::Video,
                vec![entry.clone()],
                fallback,
                Finding::note(format!(
                    "这是 H.264 视频{}；播放器按需读取，不会自动播放",
                    if info.audio_tracks > 0 {
                        "，带 AAC 音轨"
                    } else {
                        "，没有音轨"
                    }
                )),
            )
        }
        _ => {
            bail!(
                "{shown} 是一个文件。首批单文件作品只接受 .md 文章和 H.264/AAC 的 .mp4 视频；网页作品请给包含 index.html 的目录。"
            )
        }
    };
    let files = scan::scan_selected(&root, &paths).await?;
    if kind == WorkKind::Article {
        for file in files.iter().filter(|file| file.entry.path != entry) {
            let mut head = [0u8; 16];
            let read = std::fs::File::open(&file.source)
                .and_then(|mut source| source.read(&mut head))
                .with_context(|| format!("读不了配图 {}", file.entry.path))?;
            let actual = manifest::sniff_image_mime(&head[..read]);
            let expected = playtest_common::article::image_mime(&file.entry.path);
            if actual.is_none() || actual != expected {
                bail!(
                    "{} 的扩展名和实际图片格式对不上（实际是 {}），不能只改扩展名。",
                    file.entry.path,
                    actual.unwrap_or("无法识别的格式")
                );
            }
        }
    }
    let mut checked = inspect::Report::default();
    checked.findings.push(finding);
    ui::say(match kind {
        WorkKind::Article => "文章发布路径正在实现验证中；尚未完成真实手机验收。",
        WorkKind::Video => "视频发布路径正在实现验证中；尚未完成真实手机与移动网络验收。",
        WorkKind::Web => unreachable!(),
    });
    Ok(UploadInput {
        remember_key: source.to_string_lossy().into_owned(),
        root,
        files,
        kind,
        entry: Some(entry),
        title_hint,
        checked,
        chapters: vec![],
    })
}

fn check_kind_options(cli_args: &UploadArgs, kind: WorkKind) -> Result<()> {
    if kind == WorkKind::Web {
        return Ok(());
    }
    if cli_args.backend.is_some() {
        bail!("--backend 只用于网页目录；文章和视频没有需要转发到本机的后端。")
    }
    if cli_args.spa {
        bail!("--spa 只用于网页目录；文章和视频由平台的阅读器或播放器展示。")
    }
    if cli_args.isolated != Isolation::Auto {
        bail!("--isolated 只用于网页目录；文章和视频不执行作者脚本。")
    }
    // `-y` 只跳过交互确认；文章与视频的格式、安全检查仍在上面无条件执行，控制面还会再查一次。
    Ok(())
}

fn title_for_file(cli_args: &UploadArgs, hint: Option<&str>) -> Result<String> {
    let title = match cli_args.name.as_deref() {
        Some(name) => name.trim().to_string(),
        None => hint
            .map(str::trim)
            .filter(|title| !title.is_empty())
            .unwrap_or("未命名")
            .to_string(),
    };
    if title.is_empty() {
        bail!("作品名不能是空的。");
    }
    if title.chars().count() > limits::MAX_TITLE_CHARS {
        bail!("作品名太长了，最多 {} 个字。", limits::MAX_TITLE_CHARS);
    }
    Ok(title)
}

/// 本地先按匿名档的配额拦一道，把服务器会说的话提前说了，省一次往返。
fn check_limits(entries: &[FileEntry], paths: &[String]) -> Result<()> {
    let Err(e) = validate_files(entries, ANON_MAX_VERSION_BYTES) else {
        return Ok(());
    };
    match inspect::explain_limit(&e, paths) {
        Some(hint) => bail!("{e}\n{hint}"),
        None => bail!("{e}"),
    }
}

/// 上传前把这堆文件从头到尾看一遍。读文件的活儿在这里，看出什么在 [`crate::inspect`]。
///
/// 只读每个文件的开头：要看的东西（wasm 的 import 段、脚本里的引擎名字）都在最前面，
/// 而 Godot、Unity 的 `.wasm` 和 `.data` 动辄几十 MB，为一句提示读完不值得。
fn inspect_dir(
    root: &Path,
    files: &[ScannedFile],
    entries: &[FileEntry],
    has_backend: bool,
) -> inspect::Report {
    let by_path: HashMap<&str, &ScannedFile> =
        files.iter().map(|f| (f.entry.path.as_str(), f)).collect();
    let mut read_prefix = |path: &str, max: usize| {
        let file = by_path.get(path)?;
        let opened = std::fs::File::open(&file.source).ok()?;
        let mut bytes = Vec::new();
        opened.take(max as u64).read_to_end(&mut bytes).ok()?;
        Some(bytes)
    };
    inspect::inspect(
        inspect::Input {
            files: entries,
            // 以「.」开头的目录不上传，所以这一条得直接看磁盘。
            has_git_dir: root.join(".git").exists(),
            has_backend,
        },
        &mut read_prefix,
    )
}

/// 看出这个构建要多线程时，到底开不开跨源隔离。
///
/// 浏览器只在跨源隔离的页面里给 `SharedArrayBuffer`，不开的结果是玩家一点开就报错。所以
/// 有终端就问一句、默认开；没有终端（脚本、`--json`、agent 调用）不问，直接开并说明白，
/// 因为那时候没有人能回答问题；`--no-isolated` 是明确拒绝，照办，但要说清楚后果。
fn choose_isolated(checked: &mut inspect::Report, cli_args: &UploadArgs) -> bool {
    let Some(threads) = checked.threads.clone() else {
        return cli_args.isolated == Isolation::On;
    };
    let head = if threads.is_certain() {
        format!("这个构建用了多线程（{}）", threads.because())
    } else {
        format!("这个构建可能用了多线程（{}）", threads.because())
    };

    if cli_args.isolated == Isolation::Off {
        return decided(
            checked,
            false,
            Finding::warn(format!("{head}；你写了 --isolated=off，那就不开"))
                .hint("玩家打开时多半会报错，去掉 --isolated=off 就能跑"),
        );
    }
    if cli_args.isolated == Isolation::On {
        return decided(
            checked,
            true,
            Finding::note(format!("{head}，--isolated 已经开着")),
        );
    }
    match ask_yes(&format!(
        "{head}，需要 --isolated 才能在浏览器里跑。帮你开吗？[Y/n] "
    )) {
        Some(true) => decided(checked, true, Finding::note("这一版开了 --isolated")),
        Some(false) => decided(
            checked,
            false,
            Finding::warn("那就不开 --isolated").hint("玩家打开时多半会报错，下次加 --isolated"),
        ),
        None => decided(
            checked,
            true,
            Finding::note(format!("{head}，已经自动加上 --isolated（跨源隔离）"))
                .hint("不想要就加 --no-isolated"),
        ),
    }
}

/// 把决定说出去，同时留在 `findings` 里，`--json` 和 MCP 拿到的是同一份。
fn decided(checked: &mut inspect::Report, isolated: bool, finding: Finding) -> bool {
    output::say_findings(std::slice::from_ref(&finding));
    checked.findings.push(finding);
    isolated
}

/// 在终端上问一句是不是，回车就是「是」。
///
/// 不是终端就不问，返回 `None`——脚本、`--json`、MCP 的 stdin 是数据通道，读它会把事情弄坏。
/// 和 [`crate::ui::confirm`] 的区别只在默认值：那边默认「否」，这里默认「是」。
fn ask_yes(question: &str) -> Option<bool> {
    if output::is_json() || !std::io::stdin().is_terminal() || !std::io::stderr().is_terminal() {
        return None;
    }
    eprint!("{question}");
    let _ = std::io::stderr().flush();
    let mut answer = String::new();
    if std::io::stdin().read_line(&mut answer).is_err() {
        return None;
    }
    let answer = answer.trim().to_lowercase();
    Some(answer != "n" && answer != "no")
}

/// 作品名：`--name` 最优先；没给就看 `index.html` 的 `<title>`；再没有才用目录名。
/// 目录名叫 `dist` / `export` / `build` / `www` / `public` 时不用它——玩家在门禁页上看到「邀请你试玩《dist》」
/// 会以为链接发错了；这种目录名的作品，页面标题多半才是它真正的名字。
pub(crate) fn title_for(
    cli_args: &UploadArgs,
    root: &Path,
    page_title: Option<&str>,
) -> Result<String> {
    let dir_name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|n| !n.is_empty() && n != ".");
    let generic_dir = dir_name.as_deref().is_none_or(is_generic_dir_name);
    let title = match (&cli_args.name, page_title) {
        (Some(name), _) => name.trim().to_string(),
        (None, Some(page)) if generic_dir => page.to_string(),
        (None, _) => dir_name.unwrap_or_else(|| "未命名".to_string()),
    };
    if title.is_empty() {
        bail!("作品名不能是空的。");
    }
    if title.chars().count() > limits::MAX_TITLE_CHARS {
        bail!("作品名太长了，最多 {} 个字。", limits::MAX_TITLE_CHARS);
    }
    Ok(title)
}

/// 目录名不像作品名的那几类：构建产物的惯用名、版本号（`v2`、`2.0`、`v0.1.0`）、单个字母或数字。
/// 拿自己的作品试的时候撞到的：目录叫 `v2`，门禁页就写「邀请你体验《v2》」。
fn is_generic_dir_name(name: &str) -> bool {
    let n = name.trim().to_ascii_lowercase();
    if matches!(
        n.as_str(),
        "dist"
            | "export"
            | "exports"
            | "build"
            | "builds"
            | "www"
            | "public"
            | "out"
            | "html"
            | "web"
            | "webgl"
            | "release"
            | "output"
            | "site"
            | "static"
            | "game"
            | "app"
            | "src"
            | "prod"
            | "production"
            | "latest"
            | "final"
            | "new"
            | "old"
            | "tmp"
            | "temp"
            | "test"
    ) {
        return true;
    }
    // v2 / v0.1.0 / 2.0 / 20260908 这类：去掉前导 v，剩下全是数字和点。
    let rest = n.strip_prefix('v').unwrap_or(&n);
    !rest.is_empty()
        && rest
            .chars()
            .all(|c| c.is_ascii_digit() || c == '.' || c == '_' || c == '-')
        || n.chars().count() <= 1
}

fn check_note(cli_args: &UploadArgs) -> Result<()> {
    if let Some(note) = &cli_args.note {
        if note.chars().count() > limits::MAX_NOTE_CHARS {
            bail!(
                "「这版改了什么」太长了，最多 {} 个字。",
                limits::MAX_NOTE_CHARS
            );
        }
    }
    Ok(())
}

pub(crate) async fn ensure_token(
    client: &mut Client,
    config: &mut Config,
    config_path: &Path,
    api: &str,
) -> Result<()> {
    if let Ok(env_tok) = std::env::var("PLAYTEST_TOKEN") {
        let trimmed = env_tok.trim();
        if !trimmed.is_empty() {
            client.set_token(Some(trimmed.to_string()));
            return Ok(());
        }
    }
    if let Some(token) = config.usable_token(api, clock::now()) {
        client.set_token(Some(token.to_string()));
        return Ok(());
    }
    new_anon_session(client, config, config_path, api).await
}

async fn new_anon_session(
    client: &mut Client,
    config: &mut Config,
    config_path: &Path,
    api: &str,
) -> Result<()> {
    client.set_token(None);
    let session = client.anon_session().await?;
    client.set_token(Some(session.token.clone()));
    config.api = Some(api.to_string());
    config.token = Some(session.token);
    config.token_expires_at = Some(session.expires_at);
    config::save(config_path, config)?;
    Ok(())
}

pub(crate) async fn choose_site(
    client: &mut Client,
    cli_args: &UploadArgs,
    config: &mut Config,
    config_path: &Path,
    api: &str,
    dir_key: &str,
    title: &str,
) -> Result<(String, SlugSource)> {
    match cli_args.to.as_deref() {
        // `--to new` 是「不要发到上次那个，给我一个新链接」。
        Some("new") => {}
        Some(slug) => return Ok((slug.to_string(), SlugSource::Explicit)),
        None => {
            if let Some(slug) = config.remembered_slug(dir_key) {
                return Ok((slug.to_string(), SlugSource::Remembered));
            }
        }
    }
    let slug = match create_site(client, title).await {
        Ok(slug) => slug,
        // 本地记着的令牌还没到期，服务器却不认了（比如服务器重置过）：换一个匿名会话再来一次，
        // 否则用户每次重跑都撞同一个旧令牌。
        Err(e) if e.means_token_gone() && config.is_logged_in() => {
            // 登录令牌被拒不能悄悄换成匿名的：那会把新作品发到一个 24 小时的身份下。
            return Err(anyhow::anyhow!(
                "登录已经失效（{}）。重新运行 playtest login。",
                e
            ));
        }
        Err(e) if e.means_token_gone() => {
            ui::say("上次的匿名身份已经失效，换了一个新的。");
            new_anon_session(client, config, config_path, api).await?;
            create_site(client, title).await?
        }
        Err(e) => return Err(e.into()),
    };
    // 先记下来再上传：万一上传失败，下次重来还是同一个链接，不会攒出一堆空作品。
    config.remember(dir_key.to_string(), slug.clone());
    config::save(config_path, config)?;
    Ok((slug, SlugSource::Fresh))
}

async fn create_site(client: &Client, title: &str) -> crate::client::Result<String> {
    let site = client
        .create_site(&CreateSiteRequest {
            slug: None,
            title: Some(title.to_string()),
        })
        .await?;
    Ok(site.slug)
}

async fn send_missing(
    client: Arc<Client>,
    files: &[ScannedFile],
    missing: &[String],
    total_files: usize,
) -> Result<()> {
    let missing: HashSet<&str> = missing.iter().map(String::as_str).collect();
    // 内容一样的文件在服务器上是同一个对象，只传一次。
    let mut seen: HashSet<&str> = HashSet::new();
    let mut to_send: Vec<&ScannedFile> = Vec::new();
    let mut files_pending = 0usize;
    for file in files {
        if missing.contains(file.entry.hash.as_str()) {
            files_pending += 1;
            if seen.insert(file.entry.hash.as_str()) {
                to_send.push(file);
            }
        }
    }

    if to_send.is_empty() {
        ui::say(&format!("{total_files} 个文件服务器上都已经有了，不用传。"));
        return Ok(());
    }

    // 用自己算出来的字节数而不是服务器给的 missing_bytes：这是真正会写出去的量，
    // 两者对不上时进度条该跟着实际走。
    let bytes_to_send: u64 = to_send.iter().map(|f| f.entry.size).sum();
    ui::say(&format!(
        "需要上传 {files_pending} 个文件（{}），其余 {} 个服务器上已有",
        ui::bytes(bytes_to_send),
        total_files - files_pending
    ));

    let progress = new_progress(bytes_to_send);
    let permits = Arc::new(Semaphore::new(UPLOAD_CONCURRENCY));
    let mut tasks = JoinSet::new();
    for file in to_send {
        let client = Arc::clone(&client);
        let permits = Arc::clone(&permits);
        let progress = progress.clone();
        let hash = file.entry.hash.clone();
        let source = file.source.clone();
        let shown = file.entry.path.clone();
        tasks.spawn(async move {
            let _permit = permits.acquire_owned().await;
            put_with_retry(&client, &hash, &source, &shown, &progress).await
        });
    }

    let mut failure: Option<anyhow::Error> = None;
    while let Some(joined) = tasks.join_next().await {
        match joined {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                if failure.is_none() {
                    failure = Some(e);
                    tasks.abort_all();
                }
            }
            Err(e) if e.is_cancelled() => {}
            Err(e) => {
                if failure.is_none() {
                    failure = Some(anyhow::anyhow!("上传任务没跑完：{e}"));
                }
            }
        }
    }
    progress.finish_and_clear();
    match failure {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

fn new_progress(total: u64) -> ProgressBar {
    if !std::io::stderr().is_terminal() {
        return ProgressBar::hidden();
    }
    let bar = ProgressBar::new(total);
    bar.set_style(
        ProgressStyle::with_template(
            "{bar:28} {bytes}/{total_bytes}  {binary_bytes_per_sec}  剩 {eta}",
        )
        .unwrap_or_else(|_| ProgressStyle::default_bar()),
    );
    bar
}

/// 边构建边上传时最常见的一种失败：文件在传的过程中被写了。
fn changed_mid_upload(shown: &str) -> anyhow::Error {
    output::bad_input(format!("上传时文件变了（{shown}），请等构建完成再运行。"))
}

async fn put_with_retry(
    client: &Client,
    hash: &str,
    source: &Path,
    shown: &str,
    progress: &ProgressBar,
) -> Result<()> {
    let mut backoff = FIRST_BACKOFF;
    let mut attempt = 0u32;
    loop {
        attempt += 1;
        let sent = Arc::new(AtomicU64::new(0));
        let on_bytes: OnBytes = {
            let progress = progress.clone();
            let sent = Arc::clone(&sent);
            Arc::new(move |n| {
                sent.fetch_add(n, Ordering::Relaxed);
                progress.inc(n);
            })
        };

        let Err(e) = client.put_blob(hash, source, on_bytes).await else {
            return Ok(());
        };

        // 这一趟白传了，进度条退回去，免得重试时超过总数。
        let undo = sent.load(Ordering::Relaxed);
        progress.set_position(progress.position().saturating_sub(undo));

        let mismatched = e.code() == Some(ErrorCode::HashMismatch);
        if mismatched && scan::rehash(source).await? != hash {
            return Err(changed_mid_upload(shown));
        }
        if attempt >= MAX_ATTEMPTS || !(mismatched || e.worth_retrying()) {
            if mismatched {
                return Err(changed_mid_upload(shown));
            }
            // 套一层「哪个文件」，但让底下那个错保持原样——归类要靠它。
            return Err(anyhow::Error::from(e).context(format!("传不上去 {shown}")));
        }
        tokio::time::sleep(backoff).await;
        backoff *= 2;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args_with(seats: Option<u32>, community: Option<&str>) -> UploadArgs {
        UploadArgs {
            seats,
            community: community.map(str::to_string),
            ..UploadArgs::default()
        }
    }

    #[test]
    fn seats_has_to_be_a_number_of_people_we_could_actually_find() {
        assert!(check_plaza_inputs(&args_with(Some(1), None)).is_ok());
        assert!(check_plaza_inputs(&args_with(Some(limits::MAX_SEATS), None)).is_ok());

        let err = check_plaza_inputs(&args_with(Some(0), None)).unwrap_err();
        assert!(err.to_string().contains("至少是 1"), "{err}");

        // 超了要把上限那个数说出来，不然只能再猜一次。
        let err = check_plaza_inputs(&args_with(Some(limits::MAX_SEATS + 1), None)).unwrap_err();
        assert!(
            err.to_string().contains(&limits::MAX_SEATS.to_string()),
            "{err}"
        );
    }

    #[test]
    fn a_group_link_has_to_be_something_a_browser_can_open() {
        for good in [
            "https://t.me/playtest",
            "http://127.0.0.1:8080/qq",
            "https://example.com/微信群.png",
        ] {
            assert!(
                check_plaza_inputs(&args_with(None, Some(good))).is_ok(),
                "{good}"
            );
        }
        // qq:// 这类只有装了客户端的人点得动，在门禁页上就是一个死链。
        for bad in ["qq://group/12345", "微信群 abc", "t.me/playtest", ""] {
            let err = check_plaza_inputs(&args_with(None, Some(bad))).unwrap_err();
            assert!(err.to_string().contains("http"), "{bad} → {err}");
        }
    }

    #[test]
    fn a_group_link_that_long_is_not_a_link() {
        let long = format!(
            "https://example.com/{}",
            "x".repeat(limits::MAX_COMMUNITY_URL_CHARS)
        );
        let err = check_plaza_inputs(&args_with(None, Some(&long))).unwrap_err();
        assert!(err.to_string().contains("太长"), "{err}");
    }

    #[tokio::test]
    async fn markdown_input_only_collects_the_source_and_explicit_images() {
        let dir = tempfile::tempdir().unwrap();
        let images = dir.path().join("images");
        std::fs::create_dir_all(&images).unwrap();
        let article = dir.path().join("post.md");
        std::fs::write(
            &article,
            "# 一篇文章\n\n![配图](images/inside.png)\n\n[外链](https://example.com)",
        )
        .unwrap();
        std::fs::write(images.join("inside.png"), b"\x89PNG\r\n\x1a\nfixture").unwrap();
        std::fs::write(dir.path().join("private.txt"), b"do not upload").unwrap();

        let input = prepare_input(article.to_str().unwrap(), false)
            .await
            .unwrap();
        assert_eq!(input.kind, WorkKind::Article);
        assert_eq!(input.entry.as_deref(), Some("post.md"));
        assert_eq!(input.title_hint.as_deref(), Some("一篇文章"));
        assert_eq!(
            input
                .files
                .iter()
                .map(|file| file.entry.path.as_str())
                .collect::<Vec<_>>(),
            ["images/inside.png", "post.md"]
        );
    }

    #[tokio::test]
    async fn markdown_native_html_is_rejected_before_networking() {
        let dir = tempfile::tempdir().unwrap();
        let article = dir.path().join("unsafe.md");
        std::fs::write(&article, "# 标题\n\n<script>alert(1)</script>").unwrap();
        let error = match prepare_input(article.to_str().unwrap(), false).await {
            Ok(_) => panic!("带原生 HTML 的文章不该通过"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("原生 HTML"), "{error}");
    }
}
