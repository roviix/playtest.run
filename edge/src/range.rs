//! 单区间 `Range`。
//!
//! 引擎的加载器和浏览器的音视频都会用它：Unity 的 `.data` 断了要续，
//! `<audio>` 拖进度条会直接请求中间一段。多区间不支持——按 RFC 9110 当没收到这个头，
//! 客户端会退回整包下载，比回一个我们拼不对的 multipart 强。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Range {
    /// 没有 Range 头，或者头写错了、是多区间——都当整包出。
    Whole,
    /// 闭区间，两端都含。
    Part { start: u64, end: u64 },
    /// 越界，回 416。
    Unsatisfiable,
}

pub fn parse(header: Option<&str>, len: u64) -> Range {
    let Some(raw) = header else { return Range::Whole };
    let Some(spec) = raw.trim().strip_prefix("bytes=") else {
        return Range::Whole;
    };
    let spec = spec.trim();
    if spec.contains(',') || spec.is_empty() {
        return Range::Whole;
    }
    let Some((first, last)) = spec.split_once('-') else {
        return Range::Whole;
    };
    let (first, last) = (first.trim(), last.trim());

    // 空文件对任何区间都不满足，只能回 416。
    if len == 0 {
        return Range::Unsatisfiable;
    }

    if first.is_empty() {
        // `bytes=-n`：最后 n 个字节。
        let Ok(suffix) = last.parse::<u64>() else {
            return Range::Whole;
        };
        if suffix == 0 {
            return Range::Unsatisfiable;
        }
        return Range::Part {
            start: len.saturating_sub(suffix),
            end: len - 1,
        };
    }

    let Ok(start) = first.parse::<u64>() else {
        return Range::Whole;
    };
    if start >= len {
        return Range::Unsatisfiable;
    }
    if last.is_empty() {
        return Range::Part { start, end: len - 1 };
    }
    let Ok(end) = last.parse::<u64>() else {
        return Range::Whole;
    };
    if end < start {
        return Range::Unsatisfiable;
    }
    Range::Part {
        start,
        end: end.min(len - 1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_header_is_whole() {
        assert_eq!(parse(None, 100), Range::Whole);
    }

    #[test]
    fn closed_open_and_suffix_forms() {
        assert_eq!(parse(Some("bytes=0-9"), 100), Range::Part { start: 0, end: 9 });
        assert_eq!(parse(Some("bytes=10-"), 100), Range::Part { start: 10, end: 99 });
        assert_eq!(parse(Some("bytes=-20"), 100), Range::Part { start: 80, end: 99 });
        // 末端超出就夹到文件尾，这是 RFC 要求的，不是错误。
        assert_eq!(parse(Some("bytes=90-999"), 100), Range::Part { start: 90, end: 99 });
        // 后缀比文件还长就是整个文件。
        assert_eq!(parse(Some("bytes=-500"), 100), Range::Part { start: 0, end: 99 });
    }

    #[test]
    fn out_of_bounds_is_416() {
        assert_eq!(parse(Some("bytes=999999999-"), 100), Range::Unsatisfiable);
        assert_eq!(parse(Some("bytes=100-200"), 100), Range::Unsatisfiable);
        assert_eq!(parse(Some("bytes=9-3"), 100), Range::Unsatisfiable);
        assert_eq!(parse(Some("bytes=-0"), 100), Range::Unsatisfiable);
        assert_eq!(parse(Some("bytes=0-0"), 0), Range::Unsatisfiable);
    }

    #[test]
    fn unsupported_forms_fall_back_to_whole() {
        assert_eq!(parse(Some("bytes=0-1,5-9"), 100), Range::Whole);
        assert_eq!(parse(Some("items=0-1"), 100), Range::Whole);
        assert_eq!(parse(Some("bytes=abc"), 100), Range::Whole);
        assert_eq!(parse(Some("bytes="), 100), Range::Whole);
        assert_eq!(parse(Some("bytes=1-x"), 100), Range::Whole);
    }
}
