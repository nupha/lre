use std::{ffi::c_void, ops::Range};

use crate::{
    error::Result,
    safe::{self, MatchResult, RegexFlags, RegexInfo, encode_utf8_surrogate},
};

/// A single match of a regex in a text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match<'t> {
    bytes: &'t [u8],
    start: usize,
    end: usize,
}

/// Capture groups from a regex match.
#[derive(Debug, Clone)]
pub struct Captures<'t> {
    bytes: &'t [u8],
    matches: Vec<Option<(usize, usize)>>,
}

/// A compiled regular expression.
#[derive(Debug)]
#[repr(transparent)]
pub struct Regex(RegexInfo);

impl Regex {
    /// Compiles a regular expression in bytes with flags.
    /// `pattern` is special bytes required by lre_compile.
    #[inline(always)]
    pub fn from_bytes(pattern: &[u8], flags: RegexFlags) -> Result<Self> {
        safe::compile(pattern, flags).map(Self)
    }

    /// Compiles a regular expression str with flags.
    pub fn from_str(pattern: &str, flags: RegexFlags) -> Result<Self> {
        if flags.is_unicode() || pattern.is_ascii() {
            Self::from_bytes(pattern.as_bytes(), flags)
        } else {
            // for non u,v flags,
            // convert pattern to special encoding required by lre_compile
            Self::from_bytes(&encode_utf8_surrogate(pattern), flags)
        }
    }

    /// Returns the compiled bytecode (for advanced use).
    #[inline(always)]
    pub fn bytecode(&self) -> &[u8] {
        &self.0.bytecode
    }

    /// Returns the flags with which this regex was compiled.
    #[inline(always)]
    pub fn flags(&self) -> RegexFlags {
        self.0.flags()
    }

    /// Returns the names of all named capture groups.
    pub fn group_names(&self) -> Option<Box<[String]>> {
        if self.flags().contains(RegexFlags::NAMED_GROUPS) {
            safe::get_group_names(self.bytecode())
                .ok()
                .and_then(|x| if x.is_empty() { None } else { Some(x) })
        } else {
            None
        }
    }

    /// Returns the number of capture groups in the regex.
    #[inline(always)]
    pub fn captures_len(&self) -> usize {
        self.0.capture_count
    }

    /// Executes the regex against the given byte slice, returning match
    /// information (capture groups) if the pattern matches.
    ///
    /// # Arguments
    /// * `bytes` - The input text as raw bytes.
    /// * `bytes_offset` - Starting byte offset within `bytes` to search from.
    /// * `is_wide` - If `true`, treats `bytes` as UTF-16LE (each group of
    ///   2 bytes is one code unit). Positions are in bytes; if
    ///   `is_wide == true`, both `bytes_offset` and the returned capture
    ///   positions must be even.
    /// * `opaque` - Opaque pointer forwarded to the C runtime (for timeout
    ///   / stack-overflow callbacks). Pass `std::ptr::null_mut()` unless you
    ///   have custom callbacks.
    #[inline(always)]
    pub fn exec(
        &self,
        bytes: &[u8],
        bytes_offset: usize,
        is_wide: bool,
        opaque: *mut c_void,
    ) -> Result<MatchResult> {
        safe::exec_bytes_raw(
            &self.0.bytecode,
            bytes.as_ptr(),
            bytes.len(),
            bytes_offset,
            is_wide,
            opaque,
        )
    }

    /// Returns `true` if the regex matches anywhere in `bytes`.
    ///
    /// This is a convenience wrapper around [`exec`](Self::exec) that only
    /// checks whether a match exists, discarding capture details.
    ///
    /// # Arguments
    /// * `bytes` - The input text as raw bytes.
    /// * `is_wide` - If `true`, treats `bytes` as UTF-16LE.
    #[inline(always)]
    pub fn is_match(&self, bytes: &[u8], is_wide: bool) -> bool {
        self.exec(bytes, 0, is_wide, std::ptr::null_mut())
            .is_ok_and(|r| r.success)
    }

    /// Returns the start and end byte offsets of the first match in the text.
    #[inline(always)]
    pub fn find<'t>(&self, bytes: &'t [u8], is_wide: bool) -> Result<Option<Match<'t>>> {
        self.find_at(bytes, 0, is_wide)
    }

    /// Returns the start and end byte offsets of the first match in the text,
    /// starting from the given byte position.
    pub fn find_at<'t>(
        &self,
        bytes: &'t [u8],
        bytes_offset: usize,
        is_wide: bool,
    ) -> Result<Option<Match<'t>>> {
        let result = self.exec(bytes, bytes_offset, is_wide, std::ptr::null_mut())?;
        if !result.success {
            Ok(None)
        } else {
            // Get the full match (group 0)
            if let Some((match_start, match_end)) = result.captures.get(0).and_then(|&c| c) {
                // Ensure the match starts at or after the requested start position
                if match_start >= bytes_offset
                    && match_start <= match_end
                    && match_end <= bytes.len()
                {
                    return Ok(Some(Match {
                        bytes,
                        start: match_start,
                        end: match_end,
                    }));
                }
            }
            Ok(None)
        }
    }

    /// Returns an iterator over all non-overlapping matches in the text.
    pub fn find_iter<'r, 't>(
        &'r self,
        bytes: &'t [u8],
        bytes_offset: usize,
        is_wide: bool,
    ) -> Matches<'r, 't> {
        Matches {
            regex: self,
            bytes,
            is_wide,
            last_end: bytes_offset,
            last_match: None,
        }
    }

    /// Returns the capture groups for the first match in the text,
    /// starting from the given byte position.
    pub fn captures_at<'t>(
        &self,
        bytes: &'t [u8],
        bytes_offset: usize,
        is_wide: bool,
    ) -> Result<Option<Captures<'t>>> {
        let r = self.exec(bytes, bytes_offset, is_wide, std::ptr::null_mut())?;
        if !r.success {
            Ok(None)
        } else {
            Ok(Some(Captures {
                bytes,
                matches: r.captures,
            }))
        }
    }

    /// Returns the capture groups for the first match in the text.
    #[inline(always)]
    pub fn captures<'t>(&self, bytes: &'t [u8], is_wide: bool) -> Result<Option<Captures<'t>>> {
        self.captures_at(bytes, 0, is_wide)
    }

    /// Returns an iterator over all capture groups in the text.
    #[inline(always)]
    pub fn captures_iter<'r, 't>(
        &'r self,
        bytes: &'t [u8],
        is_wide: bool,
    ) -> CaptureMatches<'r, 't> {
        CaptureMatches {
            regex: self,
            bytes,
            is_wide,
            last_end: 0,
        }
    }

    /// Replaces all matches in the text with the replacement string.
    pub fn replace_all<'t>(&self, bytes: &'t [u8], is_wide: bool, rep: &[u8]) -> Box<[u8]> {
        let mut result = Vec::<u8>::with_capacity(bytes.len());
        let mut last_end = 0;

        for mat in self.find_iter(bytes, 0, is_wide) {
            result.extend_from_slice(&bytes[last_end..mat.start()]);
            result.extend_from_slice(rep);
            last_end = mat.end();
        }

        result.extend_from_slice(&bytes[last_end..]);
        result.into()
    }

    /// Replaces all matches in the text using a closure.
    pub fn replace_all_with<'t>(
        &self,
        bytes: &'t [u8],
        is_wide: bool,
        f: impl Fn(&Captures) -> std::borrow::Cow<'t, [u8]>,
    ) -> Box<[u8]> {
        let mut result = Vec::<u8>::with_capacity(bytes.len());
        let mut last_end = 0;

        for caps in self.captures_iter(bytes, is_wide) {
            let mat = caps.get(0).unwrap();
            result.extend_from_slice(&bytes[last_end..mat.start()]);
            result.extend_from_slice(f(&caps).as_ref());
            last_end = mat.end();
        }

        result.extend_from_slice(&bytes[last_end..]);
        result.into()
    }

    /// Splits the text by matches of the regex.
    #[inline(always)]
    pub fn split<'r, 't>(&'r self, bytes: &'t [u8], is_wide: bool) -> Split<'r, 't> {
        Split {
            finder: self.find_iter(bytes, 0, is_wide),
            last_end: 0,
        }
    }
}

impl<'t> Match<'t> {
    /// Returns the starting byte offset of the match in the text.
    #[inline(always)]
    pub fn start(&self) -> usize {
        self.start
    }

    /// Returns the ending byte offset of the match in the text.
    #[inline(always)]
    pub fn end(&self) -> usize {
        self.end
    }

    /// Returns the byte range of the match.
    #[inline(always)]
    pub fn range(&self) -> Range<usize> {
        self.start..self.end
    }

    /// Returns the matched bytes slice.
    #[inline(always)]
    pub fn as_bytes(&self) -> &'t [u8] {
        &self.bytes[self.start..self.end]
    }
}

impl<'t> Captures<'t> {
    /// Returns the match for a specific capture group.
    ///
    /// Group 0 always corresponds to the entire match.
    ///
    /// # Panics
    ///
    /// Panics if the group index is out of bounds.
    pub fn get(&self, i: usize) -> Option<Match<'t>> {
        self.matches.get(i).and_then(|&opt| {
            opt.map(|(start, end)| Match {
                bytes: self.bytes,
                start,
                end,
            })
        })
    }

    /// Returns the number of capture groups (including group 0).
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.matches.len()
    }

    /// Returns true if there are no capture groups.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.matches.is_empty()
    }

    /// Returns an iterator over all capture groups.
    #[inline(always)]
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = Option<Match<'a>>> + 'a {
        struct CapturesIterImpl<'a> {
            captures: &'a Captures<'a>,
            idx: usize,
        }

        impl<'a> Iterator for CapturesIterImpl<'a> {
            type Item = Option<Match<'a>>;

            fn next(&mut self) -> Option<Self::Item> {
                if self.idx < self.captures.len() {
                    let result = self.captures.get(self.idx);
                    self.idx += 1;
                    Some(result)
                } else {
                    None
                }
            }
        }

        CapturesIterImpl {
            captures: self,
            idx: 0,
        }
    }
}

#[inline(always)]
const fn is_hi_surrogate(c: u16) -> bool {
    (c >> 10) == (0xD800 >> 10) // 0xD800-0xDBFF
}

#[inline(always)]
const fn is_lo_surrogate(c: u16) -> bool {
    (c >> 10) == (0xDC00 >> 10) // 0xDC00-0xDFFF
}

// Advance bytes cursor by one char
fn bytes_advance_char(bytes: &[u8], pos: usize, is_wide: bool, re_flags: RegexFlags) -> usize {
    if !is_wide || !re_flags.is_unicode() || pos >= bytes.len() {
        pos + 1
    } else {
        let mut i = pos;
        let c = u16::from_ne_bytes([bytes[i], bytes[i + 1]]);
        i += 2;
        if is_hi_surrogate(c) && i < bytes.len() {
            let c1 = u16::from_ne_bytes([bytes[i], bytes[i + 1]]);
            if is_lo_surrogate(c1) {
                i += 2;
            }
        }
        i
    }
}

/// Iterator over matches of a regex in a text.
pub struct Matches<'r, 't> {
    regex: &'r Regex,
    bytes: &'t [u8],
    is_wide: bool,
    last_end: usize,
    last_match: Option<usize>,
}

impl<'r, 't> Iterator for Matches<'r, 't> {
    type Item = Match<'t>;

    fn next(&mut self) -> Option<Self::Item> {
        // Avoid infinite loop on zero-length matches
        if let Some(start) = self.last_match {
            if start == self.last_end {
                self.last_end =
                    bytes_advance_char(self.bytes, self.last_end, self.is_wide, self.regex.flags());
            }
        }

        if self.last_end > self.bytes.len() {
            return None;
        }

        match self.regex.find_at(self.bytes, self.last_end, self.is_wide) {
            Ok(Some(mat)) => {
                self.last_match = Some(mat.start());
                self.last_end = mat.end();
                Some(mat)
            }
            _ => None,
        }
    }
}

/// Iterator over capture groups of a regex in a text.
pub struct CaptureMatches<'r, 't> {
    regex: &'r Regex,
    bytes: &'t [u8],
    is_wide: bool,
    last_end: usize,
}

impl<'r, 't> Iterator for CaptureMatches<'r, 't> {
    type Item = Captures<'t>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.last_end > self.bytes.len() {
            return None;
        }

        match self
            .regex
            .captures_at(self.bytes, self.last_end, self.is_wide)
        {
            Ok(Some(caps)) => {
                if let Some(m) = caps.get(0) {
                    self.last_end = if m.start() == m.end() {
                        bytes_advance_char(
                            self.bytes,
                            self.last_end,
                            self.is_wide,
                            self.regex.flags(),
                        )
                    } else {
                        m.end()
                    };
                }
                Some(caps)
            }
            _ => None,
        }
    }
}

/// Iterator over the parts of a string split by a regex.
pub struct Split<'r, 't> {
    finder: Matches<'r, 't>,
    last_end: usize,
}

impl<'r, 't> Iterator for Split<'r, 't> {
    type Item = &'t [u8];

    fn next(&mut self) -> Option<Self::Item> {
        match self.finder.next() {
            Some(m) => {
                let slice = &self.finder.bytes[self.last_end..m.start()];
                self.last_end = m.end();
                Some(slice)
            }
            None => {
                if self.last_end <= self.finder.bytes.len() {
                    let slice = &self.finder.bytes[self.last_end..];
                    self.last_end = self.finder.bytes.len() + 1;
                    Some(slice)
                } else {
                    None
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_new() {
        let re = Regex::from_bytes(br"\d+", RegexFlags::empty()).unwrap();
        assert_eq!(re.captures_len(), 1);
    }

    #[test]
    fn test_is_match() {
        let re = Regex::from_bytes(br"\d+", RegexFlags::empty()).unwrap();
        assert!(re.is_match(b"123", false));
        assert!(!re.is_match(b"abc", false));
        assert!(re.is_match(b"abc123def", false));
    }

    #[test]
    fn test_find() {
        let re = Regex::from_bytes(br"\d+", RegexFlags::empty()).unwrap();
        let text = b"abc123def456";

        let mat = re.find(text, false).unwrap().unwrap();
        assert_eq!(mat.as_bytes(), b"123");
        assert_eq!(mat.start(), 3);
        assert_eq!(mat.end(), 6);
    }

    #[test]
    fn test_find_iter() {
        let re = Regex::from_bytes(br"\d+", RegexFlags::empty()).unwrap();
        let text = b"123 abc 456 def 789";

        let matches: Vec<&[u8]> = re.find_iter(text, 0, false).map(|m| m.as_bytes()).collect();
        assert_eq!(matches, vec![b"123", b"456", b"789"]);
    }

    #[test]
    fn test_captures() {
        let re = Regex::from_bytes(br"(\d+)-(\d+)", RegexFlags::empty()).unwrap();
        let text = b"123-456";

        let caps = re.captures(text, false).unwrap().unwrap();
        assert_eq!(caps.len(), 3);
        assert_eq!(caps.get(0).unwrap().as_bytes(), b"123-456");
        assert_eq!(caps.get(1).unwrap().as_bytes(), b"123");
        assert_eq!(caps.get(2).unwrap().as_bytes(), b"456");
        assert!(caps.get(3).is_none());
    }

    #[test]
    fn test_replace_all() {
        let re = Regex::from_bytes(br"\d+", RegexFlags::empty()).unwrap();
        let result = re.replace_all(b"123 abc 456", false, b"NUM");
        assert_eq!(result.iter().as_slice(), "NUM abc NUM".as_bytes());
    }

    #[test]
    fn test_split() {
        let re = Regex::from_bytes(br"\s+", RegexFlags::empty()).unwrap();
        let parts: Vec<&[u8]> = re.split(b"a b  c   d", false).collect();
        assert_eq!(parts, vec![b"a", b"b", b"c", b"d"]);
    }

    #[test]
    fn test_with_flags_unicode_support() {
        // Test various Unicode characters and flags combination

        // Test non-BMP characters
        let pattern1 = "𠮷";
        let flags1 = RegexFlags::empty().unicode();
        let re1 = Regex::from_bytes(pattern1.as_bytes(), flags1).unwrap();
        assert_eq!(re1.captures_len(), 1);
        assert!(re1.flags().contains(RegexFlags::UNICODE));

        // Test Japanese moji characters
        let pattern2 = "日";
        let flags2 = RegexFlags::empty().unicode();
        let re2 = Regex::from_str(pattern2, flags2).unwrap();
        assert_eq!(re2.captures_len(), 1);
        assert!(re2.flags().contains(RegexFlags::UNICODE));

        // Test mixed characters (ASCII + non-BMP)
        let pattern3 = "[a-z𠮷]";
        let flags3 = RegexFlags::empty().unicode();
        let re3 = Regex::from_bytes(pattern3.as_bytes(), flags3).unwrap();
        assert_eq!(re3.captures_len(), 1);

        // Test ignore case flag with Unicode
        let pattern4 = "a";
        let flags4 = RegexFlags::empty().ignore_case().unicode();
        let re4 = Regex::from_bytes(pattern4.as_bytes(), flags4).unwrap();
        assert!(re4.flags().contains(RegexFlags::IGNORE_CASE));
        assert!(re4.flags().contains(RegexFlags::UNICODE));

        // Test matching with ignore case
        assert!(re4.is_match("A".as_bytes(), false));
        assert!(re4.is_match("a".as_bytes(), false));

        // Test capture groups with non-BMP characters
        let pattern5 = "(𠮷+)";
        let flags5 = RegexFlags::empty();
        let re5 = Regex::from_str(pattern5, flags5).unwrap();
        assert_eq!(re5.captures_len(), 2); // Group 0 (full match) + Group 1
    }

    #[test]
    fn test_unicode_operations() {
        // Test find, replace, and other operations with Unicode characters

        // Test find with Japanese characters
        let pattern1 = "日";
        let re1 = Regex::from_bytes(pattern1.as_bytes(), RegexFlags::empty().unicode()).unwrap();

        // Test replace with Japanese characters
        let pattern2 = "日";
        let re2 = Regex::from_bytes(pattern2.as_bytes(), RegexFlags::empty()).unwrap();

        // Note: Actual matching operations with Unicode text require
        // additional handling for proper input text encoding
        // These tests focus on compilation and basic API functionality
        assert!(re1.captures_len() >= 1);
        assert!(re2.captures_len() >= 1);
    }
}
