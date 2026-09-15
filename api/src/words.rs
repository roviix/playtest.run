//! 拼给人看的那几句话时会用到的小零件。
//!
//! 只放「怎么说」，不放句子——整句在报错的地方拼，那里才知道上下文。

/// `1 file` / `3 files`。中文不分单复数，英文要分；数的都是规则名词，加个 s 就够。
pub fn count(n: u64, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_is_singular_and_everything_else_is_not() {
        assert_eq!(count(1, "file"), "1 file");
        assert_eq!(count(0, "file"), "0 files");
        assert_eq!(count(7, "project"), "7 projects");
    }
}
