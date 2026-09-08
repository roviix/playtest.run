//! 认一认这是哪个引擎导出的。
//!
//! 只看文件名和几个字符串特征。认错了不改上传行为：结果只进清单的 `engine`，门禁页拿它
//! 决定说「邀请你试玩」还是「邀请你体验」（DESIGN §3.3）。所以这里宁可认不出，不硬猜。

use super::Script;

/// 认出来的引擎。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Engine {
    Godot,
    Unity,
    Cocos,
    Construct,
    GDevelop,
    Pico8,
    Twine,
    Bitsy,
    Phaser,
    Three,
    Pixi,
    P5,
    /// 没有引擎特征、但一眼是 Vite 打出来的包：多半是个网页应用，不是游戏。
    Vite,
}

impl Engine {
    /// 打印给人看的名字。
    pub fn label(self) -> &'static str {
        match self {
            Self::Godot => "Godot",
            Self::Unity => "Unity",
            Self::Cocos => "Cocos",
            Self::Construct => "Construct",
            Self::GDevelop => "GDevelop",
            Self::Pico8 => "PICO-8",
            Self::Twine => "Twine",
            Self::Bitsy => "Bitsy",
            Self::Phaser => "Phaser",
            Self::Three => "Three.js",
            Self::Pixi => "PixiJS",
            Self::P5 => "p5.js",
            Self::Vite => "Vite",
        }
    }

    /// 写进清单的标识符。看出什么就写什么，包括 `vite`——「这算不算游戏」由门禁页那边按
    /// [`playtest_common::manifest::GAME_ENGINES`] 判断，那份名单只有一处，不在这里再判一次。
    pub fn manifest_name(self) -> &'static str {
        match self {
            Self::Godot => "godot",
            Self::Unity => "unity",
            Self::Cocos => "cocos",
            Self::Construct => "construct",
            Self::GDevelop => "gdevelop",
            Self::Pico8 => "pico8",
            Self::Twine => "twine",
            Self::Bitsy => "bitsy",
            Self::Phaser => "phaser",
            Self::Three => "three",
            Self::Pixi => "pixi",
            Self::P5 => "p5",
            Self::Vite => "vite",
        }
    }
}

/// 从文件名和读到的那几段文本里认引擎。顺序有讲究：引擎导出物里往往也躺着一个通用打包器的
/// 产物（Phaser 游戏就常常是 Vite 打的），所以专用的先认，通用的垫底。
pub fn identify(paths: &[&str], index: Option<&str>, scripts: &[Script]) -> Option<Engine> {
    let text = Text { index, scripts };
    if is_godot(paths, &text) {
        return Some(Engine::Godot);
    }
    if is_unity(paths) {
        return Some(Engine::Unity);
    }
    if is_cocos(paths, &text) {
        return Some(Engine::Cocos);
    }
    if has_path_part(paths, "c3runtime") || text.any("c3runtime") {
        return Some(Engine::Construct);
    }
    if has_path_part(paths, "gdjs") || text.any("gdjs.") {
        return Some(Engine::GDevelop);
    }
    if has_path_part(paths, "pico8") || text.any("pico8_") {
        return Some(Engine::Pico8);
    }
    if text.any("tw-storydata") {
        return Some(Engine::Twine);
    }
    if has_path_part(paths, "bitsy") || text.any("bitsy") {
        return Some(Engine::Bitsy);
    }
    if has_path_part(paths, "phaser") || text.any("Phaser") {
        return Some(Engine::Phaser);
    }
    if has_path_part(paths, "three.min.js")
        || has_path_part(paths, "three.module")
        || text.any("THREE.WebGLRenderer")
    {
        return Some(Engine::Three);
    }
    if has_path_part(paths, "pixi") || text.any("PIXI.") {
        return Some(Engine::Pixi);
    }
    if has_path_part(paths, "p5.min.js") || has_path_part(paths, "p5.js") {
        return Some(Engine::P5);
    }
    if is_vite(paths) {
        return Some(Engine::Vite);
    }
    None
}

/// 去掉预压缩的后缀：`game.data.br` → `game.data`。
pub fn without_compression(path: &str) -> &str {
    for suffix in [".br", ".gz", ".unityweb"] {
        if let Some(rest) = path.strip_suffix(suffix) {
            return rest;
        }
    }
    path
}

/// Unity 的 Build 目录：`xxx.loader.js` 一定有；另外三件（`.framework.js`、`.data`、`.wasm`）
/// 跟着压缩设置换后缀——原样、`.br`、`.gz`，开了 Decompression Fallback 就全叫 `.unityweb`。
pub fn is_unity(paths: &[&str]) -> bool {
    paths
        .iter()
        .any(|p| without_compression(p).ends_with(".loader.js"))
        || (has_unity_part(paths, ".framework.js") && has_unity_part(paths, ".data"))
}

/// 有没有 Unity 的某一件产物，压缩与否都算。
pub fn has_unity_part(paths: &[&str], suffix: &str) -> bool {
    paths
        .iter()
        .any(|p| without_compression(p).ends_with(suffix))
}

fn is_godot(paths: &[&str], text: &Text) -> bool {
    paths
        .iter()
        .any(|p| without_compression(p).ends_with(".pck"))
        || paths.iter().any(|p| p.ends_with(".audio.worklet.js"))
        || text.any("Godot")
        || text.any("GODOT")
}

fn is_cocos(paths: &[&str], text: &Text) -> bool {
    has_path_part(paths, "cocos")
        || text.any("cocos")
        || (paths.contains(&"application.js") && paths.contains(&"src/settings.json"))
}

/// Vite 打出来的包：`assets/index-<哈希>.js`。只在别的都不像的时候才轮到它。
fn is_vite(paths: &[&str]) -> bool {
    paths
        .iter()
        .any(|p| p.starts_with("assets/index-") && (p.ends_with(".js") || p.ends_with(".css")))
}

fn has_path_part(paths: &[&str], needle: &str) -> bool {
    paths
        .iter()
        .any(|p| p.to_ascii_lowercase().contains(needle))
}

/// 读到的那几段文本，用来找特征词。
struct Text<'a> {
    index: Option<&'a str>,
    scripts: &'a [Script],
}

impl Text<'_> {
    /// 特征词是照原样找的：读到的脚本可能有几 MB，为了大小写各找一遍去复制一份不值得。
    fn any(&self, needle: &str) -> bool {
        self.index.is_some_and(|t| t.contains(needle))
            || self.scripts.iter().any(|s| s.text.contains(needle))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn script(path: &str, text: &str) -> Script {
        Script {
            path: path.to_string(),
            text: text.to_string(),
        }
    }

    fn engine_of(paths: &[&str]) -> Option<Engine> {
        identify(paths, None, &[])
    }

    #[test]
    fn a_pck_file_is_enough_to_call_it_godot() {
        assert_eq!(
            engine_of(&["index.html", "game.pck", "game.wasm"]),
            Some(Engine::Godot)
        );
        // Godot 4 的音频线程脚本，名字很有辨识度。
        assert_eq!(
            engine_of(&["index.html", "index.audio.worklet.js"]),
            Some(Engine::Godot)
        );
        assert_eq!(
            identify(&["index.html"], Some("<title>GODOT engine</title>"), &[]),
            Some(Engine::Godot)
        );
    }

    #[test]
    fn unity_is_recognised_through_all_its_compression_variants() {
        assert_eq!(
            engine_of(&["Build/webgl.loader.js", "Build/webgl.data", "index.html"]),
            Some(Engine::Unity)
        );
        assert_eq!(
            engine_of(&[
                "Build/webgl.framework.js.br",
                "Build/webgl.data.br",
                "Build/webgl.wasm.br",
            ]),
            Some(Engine::Unity)
        );
        assert_eq!(
            engine_of(&[
                "Build/webgl.framework.js.unityweb",
                "Build/webgl.data.unityweb",
            ]),
            Some(Engine::Unity)
        );
        assert_eq!(
            engine_of(&["Build/webgl.loader.js.gz"]),
            Some(Engine::Unity)
        );
    }

    /// Phaser 游戏也常常是 Vite 打的包，认的得是 Phaser。
    #[test]
    fn an_engine_beats_the_bundler_that_packed_it() {
        let paths = ["index.html", "assets/index-vqYE1ClE.js"];
        let bundle = script("assets/index-vqYE1ClE.js", "…Phaser v3.60.0…");
        assert_eq!(identify(&paths, None, &[bundle]), Some(Engine::Phaser));
        assert_eq!(engine_of(&paths), Some(Engine::Vite));
    }

    #[test]
    fn the_smaller_engines_have_their_own_marks() {
        assert_eq!(
            identify(&["index.html"], Some("<div id=\"c3runtime\">"), &[]),
            Some(Engine::Construct)
        );
        assert_eq!(
            identify(
                &["index.html", "code0.js"],
                None,
                &[script("code0.js", "gdjs.evtsExt__…")]
            ),
            Some(Engine::GDevelop)
        );
        assert_eq!(
            identify(&["index.html"], Some("pico8_buttons = [0]"), &[]),
            Some(Engine::Pico8)
        );
        assert_eq!(
            identify(&["index.html"], Some("<tw-storydata name=\"故事\">"), &[]),
            Some(Engine::Twine)
        );
        assert_eq!(
            engine_of(&["index.html", "js/pixi.min.js"]),
            Some(Engine::Pixi)
        );
        assert_eq!(engine_of(&["index.html", "p5.min.js"]), Some(Engine::P5));
    }

    #[test]
    fn a_plain_page_is_not_any_engine() {
        assert_eq!(engine_of(&["index.html", "main.js", "style.css"]), None);
    }

    /// 清单里写的名字要么在门禁页那份「算游戏」的名单里，要么是 `vite`——写出一个两边都
    /// 不认得的词，玩家那一页就会莫名其妙地从「试玩」变成「体验」。
    #[test]
    fn every_name_we_write_is_a_name_the_gate_knows() {
        use playtest_common::manifest::GAME_ENGINES;
        for engine in [
            Engine::Godot,
            Engine::Unity,
            Engine::Cocos,
            Engine::Construct,
            Engine::GDevelop,
            Engine::Pico8,
            Engine::Twine,
            Engine::Bitsy,
            Engine::Phaser,
            Engine::Three,
            Engine::Pixi,
            Engine::P5,
            Engine::Vite,
        ] {
            let name = engine.manifest_name();
            let known = GAME_ENGINES.contains(&name) || name == "vite";
            assert!(known, "{name} 门禁页不认识");
        }
        assert_eq!(Engine::Godot.manifest_name(), "godot");
        // Vite 只说明作品是打包出来的，说明不了它是不是游戏；门禁页会说「体验」。
        assert_eq!(Engine::Vite.manifest_name(), "vite");
    }

    #[test]
    fn compression_suffixes_come_off_one_layer() {
        assert_eq!(without_compression("Build/a.data.br"), "Build/a.data");
        assert_eq!(without_compression("Build/a.data.gz"), "Build/a.data");
        assert_eq!(without_compression("Build/a.data.unityweb"), "Build/a.data");
        assert_eq!(without_compression("Build/a.data"), "Build/a.data");
    }
}
