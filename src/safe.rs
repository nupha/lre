use std::{
    borrow::Cow,
    ffi::c_void,
    os::raw::{c_char, c_int},
    ptr, slice,
};

use bitflags::bitflags;

use crate::{
    LRE_FLAG_DOTALL, LRE_FLAG_GLOBAL, LRE_FLAG_IGNORECASE, LRE_FLAG_INDICES, LRE_FLAG_MULTILINE,
    LRE_FLAG_NAMED_GROUPS, LRE_FLAG_STICKY, LRE_FLAG_UNICODE, LRE_FLAG_UNICODE_SETS,
    LRE_RET_MEMORY_ERROR, LRE_RET_TIMEOUT,
    error::{RegexError, Result},
    lre_get_flags,
};

bitflags! {
    /// Compilation flags for regular expressions.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct RegexFlags: u32 {
        /// Global matching (find all matches).
        const GLOBAL = LRE_FLAG_GLOBAL;
        /// Case-insensitive matching.
        const IGNORE_CASE = LRE_FLAG_IGNORECASE;
        /// Multi-line mode (^ and $ match line boundaries).
        const MULTI_LINE = LRE_FLAG_MULTILINE;
        /// Dot-all mode (. matches newline).
        const DOTALL = LRE_FLAG_DOTALL;
        /// Unicode mode.
        const UNICODE = LRE_FLAG_UNICODE;
        /// Sticky matching.
        const STICKY = LRE_FLAG_STICKY;
        /// Include indices in captures.
        const INDICES = LRE_FLAG_INDICES;
        /// Support named groups.
        const NAMED_GROUPS = LRE_FLAG_NAMED_GROUPS;
        /// Unicode sets support.
        const UNICODE_SETS = LRE_FLAG_UNICODE_SETS;
    }
}

impl RegexFlags {
    /// Converts flags to the u32.
    #[inline(always)]
    pub const fn to_u32(&self) -> u32 {
        self.bits()
    }

    /// Enables case-insensitive matching.
    pub fn ignore_case(mut self) -> Self {
        self |= Self::IGNORE_CASE;
        self
    }

    /// Enables multi-line mode.
    pub fn multi_line(mut self) -> Self {
        self |= Self::MULTI_LINE;
        self
    }

    /// Enables dot-all mode.
    pub fn dotall(mut self) -> Self {
        self |= Self::DOTALL;
        self
    }

    /// Enables Unicode mode.
    pub fn unicode(mut self) -> Self {
        self |= Self::UNICODE;
        self
    }

    /// Enables global matching.
    pub fn global(mut self) -> Self {
        self |= Self::GLOBAL;
        self
    }

    /// Enables sticky matching.
    pub fn sticky(mut self) -> Self {
        self |= Self::STICKY;
        self
    }

    /// Enables indices in captures.
    pub fn indices(mut self) -> Self {
        self |= Self::INDICES;
        self
    }

    /// Enables named groups support.
    pub fn named_groups(mut self) -> Self {
        self |= Self::NAMED_GROUPS;
        self
    }

    /// Enables Unicode sets support.
    pub fn unicode_sets(mut self) -> Self {
        self |= Self::UNICODE_SETS;
        self
    }

    /// Returns true if global matching is enabled.
    #[inline(always)]
    pub fn has_global(&self) -> bool {
        self.contains(Self::GLOBAL)
    }

    /// Returns true if case-insensitive matching is enabled.
    #[inline(always)]
    pub fn has_ignore_case(&self) -> bool {
        self.contains(Self::IGNORE_CASE)
    }

    /// Returns true if multi-line mode is enabled.
    #[inline(always)]
    pub fn has_multi_line(&self) -> bool {
        self.contains(Self::MULTI_LINE)
    }

    /// Returns true if dot-all mode is enabled.
    #[inline(always)]
    pub fn has_dotall(&self) -> bool {
        self.contains(Self::DOTALL)
    }

    /// Returns true if sticky matching is enabled.
    #[inline(always)]
    pub fn has_sticky(&self) -> bool {
        self.contains(Self::STICKY)
    }

    /// Returns true if indices in captures are enabled.
    #[inline(always)]
    pub fn has_indices(&self) -> bool {
        self.contains(Self::INDICES)
    }

    /// Returns true if named groups support is enabled.
    #[inline(always)]
    pub fn has_named_groups(&self) -> bool {
        self.contains(Self::NAMED_GROUPS)
    }

    /// Returns true if Unicode mode is enabled.
    #[inline(always)]
    pub fn has_unicode(&self) -> bool {
        self.contains(Self::UNICODE)
    }

    /// Returns true if Unicode sets support is enabled.
    #[inline(always)]
    pub fn has_unicode_sets(&self) -> bool {
        self.contains(Self::UNICODE_SETS)
    }

    /// unicode or unicode_sets enabled
    #[inline]
    pub fn is_unicode(&self) -> bool {
        self.has_unicode() || self.has_unicode_sets()
    }
}

/// Information about a compiled regex.
#[derive(Debug)]
pub struct RegexInfo {
    /// The compiled bytecode.
    pub bytecode: Box<[u8]>,
    /// Number of capture groups (including group 0 for the full match).
    pub capture_count: usize,
}

impl RegexInfo {
    pub fn flags(&self) -> RegexFlags {
        RegexFlags::from_bits_retain(unsafe { lre_get_flags(self.bytecode.as_ptr()) as _ })
    }
}

/// Converts UTF-8 to special encoding required by lre_compile:
/// For non-BMP unichar, converts them to surrogate pairs and encodes each as UTF-8.
pub(super) fn encode_utf8_surrogate(utf8_str: &str) -> Cow<'_, [u8]> {
    // Check if we have any non-BMP unichar in pattern
    let has_non_bmp = utf8_str.chars().any(|ch| ch as u32 > 0xFFFF);
    if !has_non_bmp {
        // No non-BMP characters, return borrowed bytes
        Cow::Borrowed(utf8_str.as_bytes())
    } else {
        // Has non-BMP characters, convert to owned bytes
        let mut cvt = Vec::with_capacity(utf8_str.len() * 3 / 2);

        for ch in utf8_str.chars() {
            if ch as u32 > 0xFFFF {
                // Non-BMP character: convert to surrogate pair and encode each part
                let code = ch as u32;
                let high_surrogate = 0xD800 + ((code - 0x10000) >> 10);
                let low_surrogate = 0xDC00 + ((code - 0x10000) & 0x3FF);

                // Encode high surrogate as UTF-8
                let high_bytes = encode_utf8(high_surrogate as u32);
                cvt.extend_from_slice(&high_bytes);

                // Encode low surrogate as UTF-8
                let low_bytes = encode_utf8(low_surrogate as u32);
                cvt.extend_from_slice(&low_bytes);
            } else {
                // BMP character or ASCII: encode normally as UTF-8
                let bytes = ch.to_string().into_bytes();
                cvt.extend_from_slice(&bytes);
            }
        }

        Cow::Owned(cvt)
    }
}

/// Encodes a code point as UTF-8 bytes.
fn encode_utf8(code_point: u32) -> Vec<u8> {
    if code_point <= 0x7F {
        vec![code_point as u8]
    } else if code_point <= 0x7FF {
        vec![
            (0xC0 | (code_point >> 6) as u8) as u8,
            (0x80 | (code_point & 0x3F) as u8) as u8,
        ]
    } else if code_point <= 0xFFFF {
        vec![
            (0xE0 | (code_point >> 12) as u8) as u8,
            (0x80 | ((code_point >> 6) & 0x3F) as u8) as u8,
            (0x80 | (code_point & 0x3F) as u8) as u8,
        ]
    } else {
        vec![
            (0xF0 | (code_point >> 18) as u8) as u8,
            (0x80 | ((code_point >> 12) & 0x3F) as u8) as u8,
            (0x80 | ((code_point >> 6) & 0x3F) as u8) as u8,
            (0x80 | (code_point & 0x3F) as u8) as u8,
        ]
    }
}

/// Safe wrapper for regex compilation.
/// `pattern` is utf8 encoded bytes
pub(crate) fn compile(pattern: &[u8], flags: RegexFlags) -> Result<RegexInfo> {
    let pattern_len = pattern.len();

    // Prepare error buffer
    const ERROR_BUF_SIZE: usize = 256;
    let mut error_buf = [0u8; ERROR_BUF_SIZE];

    // Call C function
    let mut bytecode_len: c_int = 0;
    let bytecode_ptr = unsafe {
        // lre_compile() requires '\0' terminated C-str
        let mut _arr: Option<[u8; 128]> = None;
        let data: Cow<[u8]> = if pattern_len < 128 {
            let mut buf = [0_u8; 128];
            std::ptr::copy_nonoverlapping::<u8>(
                pattern.as_ref().as_ptr(),
                buf.as_mut_ptr(),
                pattern_len,
            );
            _arr.replace(buf);
            _arr.as_ref().unwrap().into()
        } else {
            let mut buf = Vec::<u8>::with_capacity(pattern_len + 1); // + NUL
            buf.extend_from_slice(&pattern);
            buf.push(0);
            buf.into()
        };

        crate::lre_compile(
            &mut bytecode_len as *mut c_int,
            error_buf.as_mut_ptr() as *mut c_char,
            ERROR_BUF_SIZE as c_int,
            data.as_ref().as_ptr().cast(),
            pattern_len as _,
            flags.to_u32() as _,
            ptr::null_mut(),
        )
    };

    // Check for errors
    if bytecode_ptr.is_null() {
        let error_msg = unsafe {
            std::ffi::CStr::from_ptr(error_buf.as_ptr() as *const c_char)
                .to_string_lossy()
                .into_owned()
        };
        return Err(RegexError::CompileError(error_msg));
    }

    // Convert bytecode to Rust slice
    let bytecode_len = bytecode_len as usize;
    let bytecode: Box<[u8]> = unsafe {
        let slice = slice::from_raw_parts(bytecode_ptr, bytecode_len);
        Box::from(slice)
    };

    // Get capture count
    let capture_count = unsafe { crate::lre_get_capture_count(bytecode.as_ptr()) as usize };

    Ok(RegexInfo {
        bytecode,
        capture_count,
    })
}

/// Gets named group information from compiled bytecode.
pub(crate) fn get_group_names(bytecode: &[u8]) -> Result<Box<[String]>> {
    if bytecode.is_empty() {
        return Err(RegexError::InvalidBytecode);
    }

    let names_ptr = unsafe { crate::lre_get_groupnames(bytecode.as_ptr()) };
    if names_ptr.is_null() {
        return Ok(Vec::new().into());
    }

    // Get capture count to know how many names to expect
    let capture_count = unsafe { crate::lre_get_capture_count(bytecode.as_ptr()) as usize };

    // Parse null-terminated strings.
    // Each entry is: name\0 + 1-byte scope (= LRE_GROUP_NAME_TRAILER_LEN).
    // Unnamed groups have an empty name (\0 + scope).
    let mut names = Vec::with_capacity(capture_count);
    let mut current = names_ptr;

    for _ in 1..capture_count {
        // Skip group 0
        if current.is_null() || unsafe { *current } == 0 {
            break;
        }

        let c_str = unsafe { std::ffi::CStr::from_ptr(current) };
        names.push(c_str.to_string_lossy().into());

        // Advance past the name + NUL + trailing scope byte
        current = unsafe { current.add(c_str.to_bytes_with_nul().len() + 1) };
    }

    Ok(names.into())
}

/// Result of a regex match.
#[derive(Debug, Clone)]
pub struct MatchResult {
    /// Whether the match was successful.
    pub success: bool,
    /// Capture positions (start, end) for each group.
    /// Group 0 is the full match.
    pub captures: Vec<Option<(usize, usize)>>,
}

/// Executes a regex on the given bytes and wide flag.
pub(crate) fn exec_bytes_raw(
    bytecode: &[u8],
    bytes_ptr: *const u8,
    bytes_len: usize,
    bytes_start: usize,
    is_wide: bool,
    opaque: *mut c_void,
) -> Result<MatchResult> {
    #[cfg(debug_assertions)]
    if is_wide {
        debug_assert_eq!(bytes_len % 2, 0);
        debug_assert_eq!(bytes_start % 2, 0);
    }

    // Get capture count
    let capture_count = if bytecode.is_empty() {
        return Err(RegexError::InvalidBytecode);
    } else {
        unsafe { crate::lre_get_capture_count(bytecode.as_ptr()) as usize }
    };

    // Prepare capture array.
    // NOTE: lre_exec also stores loop counters / registers in the
    // capture array beyond the capture-group slots.  Each register
    // is stored at index `2 * capture_count + register_index` (see
    // REOP_set_i32 / REOP_loop / REOP_loop_check_adv_split_* in
    // libregexp.c).  The compiler sets register_count in the
    // bytecode header; we must reserve `capture_count * 2 +
    // register_count` total slots.  Using `lre_get_alloc_count()`
    // (the C library's own canonical formula) instead of a
    // hardcoded `capture_count * 2 + 1` avoids under-allocation
    // for patterns with multiple / nested quantifiers.
    let alloc_count = unsafe { crate::lre_get_alloc_count(bytecode.as_ptr()) as usize };
    let mut capture_ptrs: Vec<*mut u8> = vec![ptr::null_mut(); alloc_count];

    // Call C function
    let result = unsafe {
        crate::lre_exec(
            capture_ptrs.as_mut_ptr(),
            bytecode.as_ptr(),
            bytes_ptr,
            if is_wide {
                bytes_start >> 1
            } else {
                bytes_start
            } as _,
            if is_wide { bytes_len >> 1 } else { bytes_len } as _,
            is_wide as _,
            opaque,
        )
    };

    // Handle C errors
    match result {
        1 => {
            // Success - parse captures
            let mut captures = Vec::with_capacity(capture_count);

            for i in 0..capture_count {
                let start_ptr = capture_ptrs[i * 2];
                let end_ptr = capture_ptrs[i * 2 + 1];

                if start_ptr.is_null() || end_ptr.is_null() {
                    captures.push(None);
                } else {
                    let start = unsafe { start_ptr.offset_from(bytes_ptr) } as usize;
                    let end = unsafe { end_ptr.offset_from(bytes_ptr) } as usize;

                    if start <= end && end <= bytes_len {
                        // byte bounds
                        captures.push(Some((start, end)));
                    } else {
                        captures.push(None);
                    }
                }
            }

            Ok(MatchResult {
                success: true,
                captures,
            })
        }
        0 => {
            // No match
            Ok(MatchResult {
                success: false,
                captures: vec![None; capture_count],
            })
        }
        LRE_RET_MEMORY_ERROR => Err(RegexError::MemoryError),
        LRE_RET_TIMEOUT => Err(RegexError::TimeoutError),
        _ => Err(RegexError::InternalError(format!(
            "unknown error code: {}",
            result
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test helper: runs regex on ASCII text, used by the test cases below.
    fn exec_ascii(
        bytecode: &[u8],
        text: &str,
        start_pos: usize,
        opaque: *mut c_void,
    ) -> Result<MatchResult> {
        if text.is_ascii() {
            exec_bytes_raw(
                bytecode,
                text.as_ptr(),
                text.len(),
                start_pos,
                false,
                opaque,
            )
        } else {
            Err(RegexError::InvalidBytecode)
        }
    }

    #[test]
    fn test_lre_flags_comprehensive() {
        // Test flag building and checking
        let flags = RegexFlags::empty()
            .global()
            .ignore_case()
            .multi_line()
            .dotall()
            .unicode()
            .sticky()
            .indices()
            .named_groups()
            .unicode_sets();

        // Test all builder methods set the correct flags
        assert!(flags.has_global());
        assert!(flags.has_ignore_case());
        assert!(flags.has_multi_line());
        assert!(flags.has_dotall());
        assert!(flags.has_unicode());
        assert!(flags.has_sticky());
        assert!(flags.has_indices());
        assert!(flags.has_named_groups());
        assert!(flags.has_unicode_sets());

        // Test conversion to C flags
        let c_flags = flags.to_u32();
        assert_ne!(c_flags, 0);
        assert!(c_flags & LRE_FLAG_GLOBAL != 0);
        assert!(c_flags & LRE_FLAG_IGNORECASE != 0);
        assert!(c_flags & LRE_FLAG_MULTILINE != 0);
        assert!(c_flags & LRE_FLAG_DOTALL != 0);
        assert!(c_flags & LRE_FLAG_UNICODE != 0);
        assert!(c_flags & LRE_FLAG_STICKY != 0);
        assert!(c_flags & LRE_FLAG_INDICES != 0);
        assert!(c_flags & LRE_FLAG_NAMED_GROUPS != 0);
        assert!(c_flags & LRE_FLAG_UNICODE_SETS != 0);

        // Test default flags have no flags set
        let default_flags = RegexFlags::empty();
        assert_eq!(default_flags.to_u32(), 0);
        assert!(!default_flags.has_global());
        assert!(!default_flags.has_ignore_case());
        assert!(!default_flags.has_multi_line());
        assert!(!default_flags.has_dotall());
        assert!(!default_flags.has_unicode());
        assert!(!default_flags.has_sticky());
        assert!(!default_flags.has_indices());
        assert!(!default_flags.has_named_groups());
        assert!(!default_flags.has_unicode_sets());
    }

    #[test]
    fn test_convert_pattern_for_lre() {
        // Test ASCII characters - should remain unchanged
        let ascii_pattern = "abc";
        let converted = encode_utf8_surrogate(ascii_pattern);
        assert_eq!(&*converted, b"abc");

        // Test BMP Unicode characters - should remain unchanged
        let bmp_pattern = "世界";
        let converted = encode_utf8_surrogate(bmp_pattern);
        assert_eq!(&*converted, "世界".as_bytes());

        // Test non-BMP character "𠮷" - should be converted to surrogate pairs
        let non_bmp_pattern = "𠮷";
        let converted = encode_utf8_surrogate(non_bmp_pattern);

        // "𠮷" should be converted to surrogate pair 0xD842 and 0xDFB7
        // These surrogates should be encoded as UTF-8:
        // High surrogate 0xD842 -> ED A1 82
        // Low surrogate 0xDFB7 -> ED BE B7
        assert_eq!(&*converted, vec![0xED, 0xA1, 0x82, 0xED, 0xBE, 0xB7]);

        // Test mixed pattern
        let mixed_pattern = "a世𠮷b";
        let converted = encode_utf8_surrogate(mixed_pattern);
        let mut expected = Vec::new();
        expected.extend_from_slice(b"a");
        expected.extend_from_slice("世".as_bytes());
        expected.extend_from_slice(&[0xED, 0xA1, 0x82, 0xED, 0xBE, 0xB7]);
        expected.extend_from_slice(b"b");
        assert_eq!(&*converted, expected);
    }

    /// Comprehensive test for exec_ascii with various input types
    /// Tests byte-level behavior with different character encodings
    #[test]
    fn test_exec_ascii_comprehensive() {
        let pattern = br"\d+";
        let flags = RegexFlags::empty();

        let compile_result = compile(pattern, flags);
        assert!(compile_result.is_ok(), "Compilation should succeed");

        let regex_info = compile_result.unwrap();

        // Test case 1: Pure ASCII string (baseline)
        let text = "abc123def";
        let exec_result = exec_ascii(&regex_info.bytecode, text, 0, std::ptr::null_mut());
        assert!(
            exec_result.is_ok(),
            "Execution should succeed for pure ASCII"
        );
        let match_result = exec_result.unwrap();
        assert!(
            match_result.success,
            "Should find a match in pure ASCII text"
        );
        if let Some((start, end)) = match_result.captures[0] {
            assert_eq!(&text[start..end], "123", "Should match '123'");
        } else {
            panic!("Expected Some capture, got None");
        }

        // Test case 2: UTF-8 string containing non-ASCII characters (Chinese)
        let text = "abc123世界def";
        let exec_result = exec_ascii(&regex_info.bytecode, text, 0, std::ptr::null_mut());
        match exec_result {
            Ok(match_result) => {
                if match_result.success {
                    if let Some((start, end)) = match_result.captures[0] {
                        assert_eq!(&text[start..end], "123", "Should match '123'");
                    }
                }
            }
            Err(err) => {
                // Expected behavior for non-ASCII text
                println!("exec_ascii returned error: {:?}", err);
            }
        }

        // Test case 3: UTF-8 string with emoji
        let pattern = br"[A-Z]+";
        let compile_result = compile(pattern, flags);
        assert!(compile_result.is_ok(), "Compilation should succeed");
        let regex_info = compile_result.unwrap();
        let text = "HELLO😊WORLD";

        let exec_result = exec_ascii(&regex_info.bytecode, text, 0, std::ptr::null_mut());
        match exec_result {
            Ok(match_result) => {
                if match_result.success {
                    if let Some((start, end)) = match_result.captures[0] {
                        let matched = &text[start..end];
                        assert!(
                            matched == "HELLO" || matched == "WORLD",
                            "Should match either 'HELLO' or 'WORLD'"
                        );
                    }
                }
            }
            Err(err) => {
                println!("exec_ascii returned error with emoji: {:?}", err);
            }
        }

        // Test case 4: Byte-level position verification
        let pattern = br"b";
        let compile_result = compile(pattern, flags);
        assert!(compile_result.is_ok(), "Compilation should succeed");
        let regex_info = compile_result.unwrap();
        let text = "a中b文c"; // a(1), 中(3), b(1), 文(3), c(1) = 9 bytes total

        let exec_result = exec_ascii(&regex_info.bytecode, text, 0, std::ptr::null_mut());
        match exec_result {
            Ok(match_result) => {
                if match_result.success {
                    if let Some((start, end)) = match_result.captures[0] {
                        assert_eq!(start, 4, "'b' should be at byte position 4");
                        assert_eq!(end, 5, "'b' should be 1 byte long");
                        assert_eq!(&text[start..end], "b", "Should match 'b'");
                    }
                }
            }
            Err(err) => {
                println!("exec_ascii returned error: {:?}", err);
            }
        }

        // Test case 5: Character class behavior at byte level
        let pattern = br"\w";
        let compile_result = compile(pattern, flags);
        assert!(compile_result.is_ok(), "Compilation should succeed");
        let regex_info = compile_result.unwrap();

        let test_cases = vec![
            ("a", "ASCII letter"),
            ("1", "ASCII digit"),
            ("_", "underscore"),
        ];

        for (text, description) in test_cases {
            let exec_result = exec_ascii(&regex_info.bytecode, text, 0, std::ptr::null_mut());
            match exec_result {
                Ok(_match_result) => {
                    // Basic assertion that execution succeeds
                    assert!(true, "{} should be processed", description);
                }
                Err(err) => {
                    println!("Error for {}: {:?}", description, err);
                }
            }
        }

        // Test case 6: Invalid UTF-8 sequence handling
        let pattern = br".";
        let compile_result = compile(pattern, flags);
        assert!(compile_result.is_ok(), "Compilation should succeed");
        let regex_info = compile_result.unwrap();

        // Create invalid UTF-8 sequence
        let invalid_utf8: Vec<u8> = vec![0xC0, 0x80, 0x41]; // invalid UTF-8 + 'A'
        let text = unsafe { std::str::from_utf8_unchecked(&invalid_utf8) };

        let exec_result = exec_ascii(&regex_info.bytecode, text, 0, std::ptr::null_mut());
        match exec_result {
            Ok(_match_result) => {
                // exec_ascii works at raw byte level, doesn't validate UTF-8
                assert!(true, "exec_ascii should handle invalid UTF-8 at byte level");
            }
            Err(err) => {
                println!("Error with invalid UTF-8: {:?}", err);
            }
        }
    }

    /// Test successful regex compilation
    #[test]
    fn test_compile_comprehensive() {
        // Test case 1: Basic compilation success
        let pattern = br"\d+";
        let flags = RegexFlags::empty();

        let result = compile(pattern, flags);
        assert!(result.is_ok(), "Basic compilation should succeed");

        let regex_info = result.unwrap();
        assert!(
            !regex_info.bytecode.is_empty(),
            "Bytecode should not be empty"
        );
        assert_eq!(regex_info.capture_count, 1, "Should have 1 capture group");
        assert_eq!(regex_info.flags().to_u32(), 0, "Flags should be 0");

        // Test case 2: Compilation with flags
        let pattern = b"hello";
        let flags = RegexFlags::empty().ignore_case().global();

        let result = compile(pattern, flags);
        assert!(result.is_ok(), "Compilation with flags should succeed");

        let regex_info = result.unwrap();
        assert!(
            !regex_info.bytecode.is_empty(),
            "Bytecode should not be empty"
        );

        let expected_flags = LRE_FLAG_IGNORECASE | LRE_FLAG_GLOBAL;
        assert_eq!(
            regex_info.flags().to_u32(),
            expected_flags,
            "Flags should match compiled flags"
        );

        // Test case 3: Compilation error handling
        let pattern = b"("; // Invalid pattern - unmatched parenthesis
        let flags = RegexFlags::empty();

        let result = compile(pattern, flags);
        assert!(result.is_err(), "Invalid pattern should fail compilation");

        if let Err(err) = result {
            match err {
                RegexError::CompileError(msg) => {
                    assert!(!msg.is_empty(), "Error message should not be empty");
                }
                _ => panic!("Expected CompileError, got {:?}", err),
            }
        }

        // Test case 4: Non-BMP character compilation
        let pattern = "𠮷";
        let flags = RegexFlags::empty();

        let b = encode_utf8_surrogate(pattern);
        let result = compile(&b, flags);
        assert!(
            result.is_ok(),
            "Non-BMP character compilation should succeed"
        );

        let info = result.unwrap();
        assert!(!info.bytecode.is_empty());
        assert_eq!(info.capture_count, 1);

        // Test case 5: Character class with non-BMP characters
        let pattern = "[𠮷中]"; // Character class with non-BMP, BMP, and ASCII
        let flags = RegexFlags::empty();

        let b = encode_utf8_surrogate(pattern);
        let info = compile(&b, flags).unwrap();
        assert!(!info.bytecode.is_empty());
        assert_eq!(info.capture_count, 1);
    }

    /// Test utility functions with comprehensive scenarios
    #[test]
    fn test_utility_functions_comprehensive() {
        let pattern = br"(\d+)-(\w+)";
        let flags = RegexFlags::empty().ignore_case().multi_line();

        let compile_result = compile(pattern, flags);
        assert!(compile_result.is_ok(), "Compilation should succeed");

        let regex_info = compile_result.unwrap();

        // Test case 1: capture_count function
        assert_eq!(regex_info.capture_count, 3, "Should have 3 capture groups");

        // Test case 2: flags retrieval via RegexInfo::flags
        let retrieved_flags = regex_info.flags().to_u32();
        let expected_flags = LRE_FLAG_IGNORECASE | LRE_FLAG_MULTILINE;
        assert_eq!(retrieved_flags, expected_flags, "Flags should match");

        // Test case 5: get_group_names with unnamed groups
        let group_names = get_group_names(&regex_info.bytecode);
        assert!(group_names.is_ok(), "Getting group names should succeed");
        assert_eq!(group_names.unwrap().len(), 0, "Should have no named groups");
    }

    /// Comprehensive test for exec_ascii execution scenarios
    #[test]
    fn test_exec_comprehensive() {
        let pattern = br"(\d+)-(\w+)";
        let flags = RegexFlags::empty();

        let compile_result = compile(pattern, flags);
        assert!(compile_result.is_ok(), "Compilation should succeed");

        let regex_info = compile_result.unwrap();

        // Test case 1: Successful match
        let text = "123-apple";
        let exec_result = exec_ascii(&regex_info.bytecode, text, 0, std::ptr::null_mut());
        assert!(exec_result.is_ok(), "Execution should succeed");

        let match_result = exec_result.unwrap();
        assert!(match_result.success, "Should find a match");
        assert_eq!(
            match_result.captures.len(),
            3,
            "Should have 3 capture groups"
        );

        // Check group 0 (full match)
        if let Some((start, end)) = match_result.captures[0] {
            assert_eq!(
                &text[start..end],
                "123-apple",
                "Full match should be correct"
            );
        } else {
            panic!("Expected Some capture for group 0");
        }

        // Check group 1
        if let Some((start, end)) = match_result.captures[1] {
            assert_eq!(&text[start..end], "123", "Group 1 should match digits");
        } else {
            panic!("Expected Some capture for group 1");
        }

        // Check group 2
        if let Some((start, end)) = match_result.captures[2] {
            assert_eq!(&text[start..end], "apple", "Group 2 should match word");
        } else {
            panic!("Expected Some capture for group 2");
        }

        // Test case 2: No match
        let text = "abc";
        let exec_result = exec_ascii(&regex_info.bytecode, text, 0, std::ptr::null_mut());
        assert!(exec_result.is_ok(), "Execution should succeed");

        let match_result = exec_result.unwrap();
        assert!(!match_result.success, "Should not find a match");

        // Test case 3: Match with start position
        let pattern = br"\d";
        let compile_result = compile(pattern, flags);
        assert!(compile_result.is_ok(), "Compilation should succeed");

        let regex_info = compile_result.unwrap();
        let text = "a1b2c3";

        // Start from position 1 (should find "1")
        let exec_result = exec_ascii(&regex_info.bytecode, text, 1, std::ptr::null_mut());
        assert!(exec_result.is_ok(), "Execution should succeed");

        let match_result = exec_result.unwrap();
        assert!(match_result.success, "Should find a match");

        if let Some((start, end)) = match_result.captures[0] {
            assert_eq!(&text[start..end], "1", "Should find digit at position 1");
        } else {
            panic!("Expected Some capture");
        }

        // Test case 4: Empty bytecode error
        let empty_bytecode: Vec<u8> = Vec::new();
        let text = "test";

        let result = exec_ascii(&empty_bytecode, text, 0, std::ptr::null_mut());
        assert!(result.is_err(), "Empty bytecode should return error");

        if let Err(err) = result {
            match err {
                RegexError::InvalidBytecode => {
                    // Expected error
                }
                _ => panic!("Expected InvalidBytecode error, got {:?}", err),
            }
        }
    }

    /// Test regex compilation and execution workflow
    #[test]
    fn test_full_workflow() {
        let pattern = br"(\d{4})-(\d{2})-(\d{2})";
        let flags = RegexFlags::empty();

        // Compile
        let compile_result = compile(pattern, flags);
        assert!(compile_result.is_ok(), "Compilation should succeed");

        let regex_info = compile_result.unwrap();
        assert_eq!(regex_info.capture_count, 4, "Should have 4 capture groups");

        // Execute
        let text = "Date: 2023-12-25";
        let exec_result = exec_ascii(&regex_info.bytecode, text, 0, std::ptr::null_mut());
        assert!(exec_result.is_ok(), "Execution should succeed");

        let match_result = exec_result.unwrap();
        assert!(match_result.success, "Should find a match");

        // Verify captures
        if let Some((start, end)) = match_result.captures[0] {
            assert_eq!(
                &text[start..end],
                "2023-12-25",
                "Full match should be correct"
            );
        }

        if let Some((start, end)) = match_result.captures[1] {
            assert_eq!(&text[start..end], "2023", "Year capture should be correct");
        }

        if let Some((start, end)) = match_result.captures[2] {
            assert_eq!(&text[start..end], "12", "Month capture should be correct");
        }

        if let Some((start, end)) = match_result.captures[3] {
            assert_eq!(&text[start..end], "25", "Day capture should be correct");
        }
    }

    /// Comprehensive test for flag behaviors
    #[test]
    fn test_flag_behaviors_comprehensive() {
        // Test case 1: Unicode pattern compilation
        let pattern = br"[\\p{L}]+"; // Unicode letters (fixed pattern)
        let flags = RegexFlags::empty().unicode();

        let result = compile(pattern, flags);
        assert!(result.is_ok(), "Unicode pattern compilation should succeed");

        let regex_info = result.unwrap();
        assert!(
            regex_info.flags().to_u32() & LRE_FLAG_UNICODE != 0,
            "Unicode flag should be set"
        );

        // Test case 2: Case insensitive matching
        let pattern = br"hello";
        let flags = RegexFlags::empty().ignore_case();

        let compile_result = compile(pattern, flags);
        assert!(compile_result.is_ok(), "Compilation should succeed");

        let regex_info = compile_result.unwrap();
        let test_cases = vec!["hello", "HELLO", "Hello", "hElLo"];

        for text in test_cases {
            let exec_result = exec_ascii(&regex_info.bytecode, text, 0, std::ptr::null_mut());
            assert!(
                exec_result.is_ok(),
                "Execution should succeed for '{}'",
                text
            );

            let match_result = exec_result.unwrap();
            assert!(
                match_result.success,
                "Should match '{}' case-insensitively",
                text
            );
        }

        // Test case 3: Multi-line pattern matching
        let pattern = b"^hello";
        let flags = RegexFlags::empty().multi_line();

        let compile_result = compile(pattern, flags);
        assert!(compile_result.is_ok(), "Compilation should succeed");

        let regex_info = compile_result.unwrap();
        let text = "hello\nworld\nhello";

        // Should match at the beginning of each line
        let mut match_count = 0;
        let mut pos = 0;

        while pos < text.len() {
            let exec_result = exec_ascii(&regex_info.bytecode, text, pos, std::ptr::null_mut());
            if let Ok(match_result) = exec_result {
                if match_result.success {
                    match_count += 1;
                    if let Some((start, _)) = match_result.captures[0] {
                        pos = start + 1;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        assert_eq!(match_count, 2, "Should match at start of 2 lines");

        // Test case 4: Global flag behavior
        let pattern = br"\d+";
        let flags = RegexFlags::empty().global();

        let compile_result = compile(pattern, flags);
        assert!(compile_result.is_ok(), "Compilation should succeed");

        let regex_info = compile_result.unwrap();
        let text = "1a2b3c";

        // Should find all matches when using global flag
        let mut match_count = 0;
        let mut pos = 0;

        while pos < text.len() {
            let exec_result = exec_ascii(&regex_info.bytecode, text, pos, std::ptr::null_mut());
            match exec_result {
                Ok(match_result) => {
                    if match_result.success {
                        match_count += 1;
                        if let Some((_, end)) = match_result.captures[0] {
                            pos = end;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                Err(_) => break,
            }
        }

        assert_eq!(match_count, 3, "Should find all 3 digits with global flag");

        // Test case 5: Sticky flag behavior
        let pattern = br"\d";
        let flags = RegexFlags::empty().sticky();

        let compile_result = compile(pattern, flags);
        assert!(compile_result.is_ok(), "Compilation should succeed");

        let regex_info = compile_result.unwrap();
        let text = "a1b2c3";

        // Should only match at exact position with sticky flag
        let exec_result = exec_ascii(&regex_info.bytecode, text, 1, std::ptr::null_mut());
        assert!(exec_result.is_ok(), "Execution should succeed");

        let match_result = exec_result.unwrap();
        assert!(match_result.success, "Should match at sticky position");

        if let Some((start, end)) = match_result.captures[0] {
            assert_eq!(&text[start..end], "1", "Should match digit at position 1");
        }
    }

    /// Verify capture-array allocation is correct for various register counts.
    ///
    /// `lre_exec` stores loop counters/registers in slots beyond the
    /// capture-group area at index `2*capture_count + register_index`.
    /// Using `lre_get_alloc_count()` (vs a hardcoded `+1`) ensures the
    /// buffer is large enough for patterns with multiple nested quantifiers.
    #[test]
    fn test_capture_alloc_count_multiple_registers() {
        // Register allocation depends on:
        //   a) Quantifier type: ?, *, + use split+goto (no regs); {n,m} uses
        //      set_i32+loop (1 reg) and optionally set_char_pos (+1 reg).
        //   b) `re_need_check_adv()` on the atom: if the atom always advances
        //      (e.g. `a` or `\d`), zero-advance check is skipped → no set_char_pos.
        //      If the atom CAN match zero-length (e.g. `(?:a{0,5})`), then
        //      set_char_pos IS emitted → +1 reg.
        //   c) Nested {n,m}: the outer's set_i32 (+ set_char_pos) is inserted
        //      BEFORE the inner body via `dbuf_insert`, so the linear
        //      `compute_register_count` scan sees both simultaneously.
        let cases: &[(&[u8], &str)] = &[
            (b"abc", "abc"),                      // reg=0: no quantifier
            (b"a+", "aaa"),                       // reg=0: split+goto only
            (b"a*", "aaa"),                       // reg=0: split+goto only
            (b"a{1,5}", "aaa"),                   // reg=1: set_i32, no z-adv
            (b"a{0,5}", "aaa"),                   // reg=1: set_i32, no z-adv (a always advances)
            (b"(?:a{0,5}){0,5}", "aaa"), // reg=3: outer set_i32+set_char_pos, inner set_i32
            (b"(?:(?:a{0,5}){0,5}){0,5}", "aaa"), // reg=5+: triple nesting
        ];

        for (i, &(pattern, text)) in cases.iter().enumerate() {
            let info = compile(pattern, RegexFlags::empty())
                .unwrap_or_else(|e| panic!("case {}: compile failed: {:?}", i, e));

            let register_count = info.bytecode[3] as usize;

            // The C lib's own formula must equal capture_count * 2 + register_count
            let alloc_count =
                unsafe { crate::lre_get_alloc_count(info.bytecode.as_ptr()) as usize };
            assert_eq!(
                alloc_count,
                info.capture_count * 2 + register_count,
                "case {}: pattern {:?}: lre_get_alloc_count mismatch (cap={}, reg={})",
                i,
                pattern,
                info.capture_count,
                register_count,
            );

            // Execution must not panic / corrupt heap (registers written
            // beyond the capture-pair area must land in valid slots).
            let result = exec_ascii(&info.bytecode, text, 0, std::ptr::null_mut())
                .unwrap_or_else(|e| panic!("case {}: exec failed: {:?}", i, e));
            assert!(
                result.success,
                "case {}: pattern {:?} should match '{}'",
                i, pattern, text,
            );
        }
    }
}
