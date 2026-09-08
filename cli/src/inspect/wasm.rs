//! 从 `.wasm` 的字节里看出这个构建要不要多线程。
//!
//! 只回答一个问题：模块有没有要一块**共享内存**（shared memory）。要了就是多线程构建，
//! 浏览器只在跨源隔离的页面里才给它 `SharedArrayBuffer`，也就是上传时要加 `--isolated`；
//! 没有这两个响应头的话，Godot 4 的线程导出一打开就报错。
//!
//! 自己按 wasm 的二进制格式往下走，不为这几十行引一条解析库。要看的东西全在文件最前面：
//! 8 字节的 magic 和版本，然后是按 id 排好的段——2 号是 import，5 号是模块自己声明的内存。
//! **任何看不懂的字节都返回 [`Memory::Unknown`]，绝不 panic**：上传目录里什么东西都可能
//! 叫 `.wasm`，压缩过的、传了一半的、随手改名的都有。

/// 一个 `.wasm` 看下来的结论。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Memory {
    /// 要一块共享内存：多线程构建。
    Shared,
    /// 读明白了，没有共享内存。
    Plain,
    /// 看不出来：不是 wasm、被截断、或者有我们不认识的写法。
    Unknown,
}

/// wasm 的 magic 加版本号：`\0asm` 和小端的 1。
const HEADER: &[u8] = b"\0asm\x01\x00\x00\x00";

/// import 段。
const IMPORT_SECTION: u8 = 2;

/// 模块自己声明内存的那一段。
const MEMORY_SECTION: u8 = 5;

/// 看这段字节（可以只是文件开头的一部分，要看的段都在最前面）。
pub fn memory_kind(bytes: &[u8]) -> Memory {
    let mut cursor = Cursor::new(bytes);
    if cursor.take(HEADER.len()) != Some(HEADER) {
        return Memory::Unknown;
    }
    // 段按 id 从小到大排（0 号自定义段可以夹在任何位置）。走过 5 号段就说明内存都看过了，
    // 后面即使因为只读了开头而断掉，也能下结论。
    let mut memory_settled = false;
    while let Some(id) = cursor.byte() {
        let Some(size) = cursor.leb_u32() else {
            return Memory::Unknown;
        };
        // 看到排在 5 号之后的段，说明内存该出现的地方都过去了；这一段本身读不全也没关系。
        if id > MEMORY_SECTION {
            memory_settled = true;
        }
        let Some(body) = cursor.take(size as usize) else {
            return if memory_settled {
                Memory::Plain
            } else {
                Memory::Unknown
            };
        };
        let shared = match id {
            IMPORT_SECTION => imported_memory(body),
            MEMORY_SECTION => declared_memory(body),
            _ => Some(false),
        };
        match shared {
            Some(true) => return Memory::Shared,
            Some(false) => {}
            None => return Memory::Unknown,
        }
        if id == MEMORY_SECTION {
            memory_settled = true;
        }
    }
    Memory::Plain
}

/// import 段里有没有共享内存。`None` 表示读不懂，别猜。
fn imported_memory(body: &[u8]) -> Option<bool> {
    let mut cursor = Cursor::new(body);
    let count = cursor.leb_u32()?;
    for _ in 0..count {
        cursor.skip_name()?; // 从哪个模块导入
        cursor.skip_name()?; // 导入的名字
        match cursor.byte()? {
            0x00 => {
                cursor.leb_u32()?; // 函数：类型下标
            }
            0x01 => {
                cursor.byte()?; // 表：元素类型
                limits(&mut cursor)?;
            }
            0x02 => {
                if limits(&mut cursor)? {
                    return Some(true);
                }
            }
            0x03 => {
                cursor.byte()?; // 全局：值类型
                cursor.byte()?; // 可不可变
            }
            _ => return None,
        }
    }
    Some(false)
}

/// 模块自己声明的内存里有没有共享的。
fn declared_memory(body: &[u8]) -> Option<bool> {
    let mut cursor = Cursor::new(body);
    let count = cursor.leb_u32()?;
    for _ in 0..count {
        if limits(&mut cursor)? {
            return Some(true);
        }
    }
    Some(false)
}

/// 一段 limits：一个标志字节，加最少页数，标志说有上限时再加一个最多页数。
/// 返回的是「这块内存是不是共享的」。
fn limits(cursor: &mut Cursor) -> Option<bool> {
    let flags = cursor.byte()?;
    // 第 0 位：写了上限吗。第 1 位：共享吗。第 2 位：64 位地址空间。别的位我们不认识，
    // 认不出来就说看不出来，不要蒙一个答案。
    const HAS_MAX: u8 = 0b001;
    const SHARED: u8 = 0b010;
    if flags & !0b111 != 0 {
        return None;
    }
    cursor.leb_u64()?;
    if flags & HAS_MAX != 0 {
        cursor.leb_u64()?;
    }
    Some(flags & SHARED != 0)
}

/// 一个往前走的读头。越界一律返回 `None`。
struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    fn byte(&mut self) -> Option<u8> {
        let byte = *self.bytes.get(self.at)?;
        self.at += 1;
        Some(byte)
    }

    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.at.checked_add(n)?;
        let slice = self.bytes.get(self.at..end)?;
        self.at = end;
        Some(slice)
    }

    /// LEB128 变长整数。超过 `groups` 组就是坏数据，不再往下读。
    fn leb(&mut self, groups: u32) -> Option<u64> {
        let mut value: u64 = 0;
        for group in 0..groups {
            let byte = self.byte()?;
            value |= u64::from(byte & 0x7f) << (7 * group);
            if byte & 0x80 == 0 {
                return Some(value);
            }
        }
        None
    }

    fn leb_u32(&mut self) -> Option<u32> {
        u32::try_from(self.leb(5)?).ok()
    }

    fn leb_u64(&mut self) -> Option<u64> {
        self.leb(10)
    }

    /// 跳过一段带长度前缀的名字。
    fn skip_name(&mut self) -> Option<()> {
        let len = self.leb_u32()? as usize;
        self.take(len)?;
        Some(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 拼一个模块。这里造的段都很短，长度一个字节写得下。
    fn module(sections: &[(u8, Vec<u8>)]) -> Vec<u8> {
        let mut out = HEADER.to_vec();
        for (id, body) in sections {
            out.push(*id);
            out.push(u8::try_from(body.len()).expect("测试里的段要短到一个字节写得下"));
            out.extend_from_slice(body);
        }
        out
    }

    /// 一条内存导入：`env.memory`，limits 的标志位由调用方给。
    fn memory_import(flags: u8, pages: &[u8]) -> Vec<u8> {
        let mut body = vec![0x01]; // 一条导入
        body.extend_from_slice(&[0x03, b'e', b'n', b'v']);
        body.extend_from_slice(&[0x06, b'm', b'e', b'm', b'o', b'r', b'y']);
        body.push(0x02); // 导入的是内存
        body.push(flags);
        body.extend_from_slice(pages);
        body
    }

    /// Godot 4 线程导出的样子：共享内存，写了上限。
    #[test]
    fn a_shared_memory_import_is_a_threaded_build() {
        let wasm = module(&[(IMPORT_SECTION, memory_import(0b011, &[0x80, 0x02, 0x80, 0x04]))]);
        assert_eq!(memory_kind(&wasm), Memory::Shared);
    }

    /// 单线程导出：同样导入内存，只是没有共享那一位。
    #[test]
    fn an_ordinary_memory_import_is_not() {
        let wasm = module(&[(IMPORT_SECTION, memory_import(0b001, &[0x80, 0x02, 0x80, 0x04]))]);
        assert_eq!(memory_kind(&wasm), Memory::Plain);
        let no_max = module(&[(IMPORT_SECTION, memory_import(0b000, &[0x80, 0x02]))]);
        assert_eq!(memory_kind(&no_max), Memory::Plain);
    }

    /// import 段里排在内存前面的函数、表、全局都要能正确跳过。
    #[test]
    fn other_kinds_of_imports_are_skipped() {
        let mut body = vec![0x04];
        body.extend_from_slice(&[0x01, b'a', 0x01, b'f', 0x00, 0x07]); // 函数
        body.extend_from_slice(&[0x01, b'a', 0x01, b't', 0x01, 0x70, 0x00, 0x01]); // 表
        body.extend_from_slice(&[0x01, b'a', 0x01, b'g', 0x03, 0x7f, 0x00]); // 全局
        body.extend_from_slice(&[0x01, b'a', 0x01, b'm', 0x02, 0b010, 0x01]); // 共享内存
        let wasm = module(&[(IMPORT_SECTION, body)]);
        assert_eq!(memory_kind(&wasm), Memory::Shared);
    }

    /// 模块自己声明的共享内存也算（Emscripten 多数时候是导入，但不保证）。
    #[test]
    fn a_shared_memory_declared_in_the_module_counts_too() {
        let wasm = module(&[(MEMORY_SECTION, vec![0x01, 0b011, 0x01, 0x02])]);
        assert_eq!(memory_kind(&wasm), Memory::Shared);
        let plain = module(&[(MEMORY_SECTION, vec![0x01, 0b000, 0x01])]);
        assert_eq!(memory_kind(&plain), Memory::Plain);
    }

    /// `fixtures/headers-lab/export/mod.wasm` 那 41 个字节：一个合法的小模块，没有内存。
    #[test]
    fn the_tiny_fixture_module_reads_as_plain() {
        let wasm: Vec<u8> = vec![
            0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x07, 0x01, 0x60, 0x02, 0x7f,
            0x7f, 0x01, 0x7f, 0x03, 0x02, 0x01, 0x00, 0x07, 0x07, 0x01, 0x03, 0x61, 0x64, 0x64,
            0x00, 0x00, 0x0a, 0x09, 0x01, 0x07, 0x00, 0x20, 0x00, 0x20, 0x01, 0x6a, 0x0b,
        ];
        assert_eq!(wasm.len(), 41);
        assert_eq!(memory_kind(&wasm), Memory::Plain);
    }

    /// 不是 wasm 的东西一律「看不出来」，不能 panic，也不能猜。
    #[test]
    fn anything_that_is_not_wasm_is_unknown() {
        assert_eq!(memory_kind(b""), Memory::Unknown);
        assert_eq!(memory_kind(b"\0asm"), Memory::Unknown);
        assert_eq!(memory_kind(b"<!doctype html>"), Memory::Unknown);
        // brotli 压缩过的 wasm：magic 对不上。
        assert_eq!(memory_kind(&[0x1b, 0x28, 0x00, 0x00, 0x04]), Memory::Unknown);
        // 版本号不是 1。
        assert_eq!(memory_kind(b"\0asm\x02\x00\x00\x00"), Memory::Unknown);
    }

    /// 只读了文件开头：内存那几段还没出现就断了，只能说看不出来。
    #[test]
    fn a_prefix_that_stops_before_the_memory_is_unknown() {
        let whole = module(&[
            (0x01, vec![0x00]),
            (IMPORT_SECTION, memory_import(0b010, &[0x01])),
        ]);
        assert_eq!(memory_kind(&whole[..whole.len() - 3]), Memory::Unknown);
    }

    /// 内存那几段读完了才断的，按读到的算。
    #[test]
    fn a_prefix_that_stops_after_the_memory_is_plain() {
        let whole = module(&[
            (IMPORT_SECTION, memory_import(0b000, &[0x01])),
            (0x0a, vec![0x00; 8]), // code 段，读不全
        ]);
        assert_eq!(memory_kind(&whole[..whole.len() - 5]), Memory::Plain);
    }

    /// 段长度乱写（说自己有 4 GB）也只是「看不出来」。
    #[test]
    fn a_bogus_section_length_does_not_blow_up() {
        let mut wasm = HEADER.to_vec();
        wasm.extend_from_slice(&[IMPORT_SECTION, 0xff, 0xff, 0xff, 0xff, 0x0f, 0x01]);
        assert_eq!(memory_kind(&wasm), Memory::Unknown);
    }

    /// 认不出来的 limits 标志位（比如将来新提案）宁可说看不出来。
    #[test]
    fn unknown_limit_flags_are_unknown() {
        let wasm = module(&[(IMPORT_SECTION, memory_import(0b1000, &[0x01]))]);
        assert_eq!(memory_kind(&wasm), Memory::Unknown);
    }
}
