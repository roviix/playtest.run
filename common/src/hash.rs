//! 内容哈希：SHA-256，小写十六进制，64 个字符。
//!
//! 选 SHA-256 而不是更快的 BLAKE3：浏览器里 `crypto.subtle.digest` 原生支持，
//! 将来网页端上传不用带 wasm；对象存储服务普遍能校验它。上传路径的瓶颈在网络不在哈希。

use sha2::{Digest, Sha256};

/// 哈希字符串的长度。
pub const HEX_LEN: usize = 64;

pub fn hash_bytes(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}

/// 流式哈希，给大文件用。
pub struct Hasher(Sha256);

impl Hasher {
    pub fn new() -> Self {
        Self(Sha256::new())
    }

    pub fn update(&mut self, chunk: &[u8]) {
        self.0.update(chunk);
    }

    pub fn finish(self) -> String {
        hex::encode(self.0.finalize())
    }
}

impl Default for Hasher {
    fn default() -> Self {
        Self::new()
    }
}

/// 只接受我们自己会产生的形态，避免把用户输入直接拼进对象存储的路径。
pub fn is_valid_hex(hash: &str) -> bool {
    hash.len() == HEX_LEN && hash.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_matches_known_digest() {
        assert_eq!(
            hash_bytes(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn streaming_equals_one_shot() {
        let mut h = Hasher::new();
        h.update(b"hello ");
        h.update(b"world");
        assert_eq!(h.finish(), hash_bytes(b"hello world"));
    }

    #[test]
    fn rejects_uppercase_and_wrong_length() {
        assert!(is_valid_hex(&hash_bytes(b"x")));
        assert!(!is_valid_hex(&hash_bytes(b"x").to_uppercase()));
        assert!(!is_valid_hex("abc"));
    }
}
