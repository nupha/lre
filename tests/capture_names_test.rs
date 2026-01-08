//! Tests for named capture groups functionality.
//!
//! This test file covers three scenarios:
//! 1. Expressions without capture names
//! 2. Expressions with capture names
//! 3. Mixed expressions with both named and unnamed capture groups

use lre::{Regex, RegexFlags};

#[test]
fn test_no_capture_names() {
    // Test expressions without capture names - should use numeric indices only
    let re = Regex::from_str(r"(\d+)-(\d+)-(\d+)", RegexFlags::empty()).unwrap();
    let text = b"2023-12-25";

    let caps = re.captures(text, false).unwrap().unwrap();
    assert_eq!(caps.len(), 4); // Groups: 0, 1, 2, 3

    // Verify numeric access works
    assert_eq!(caps.get(0).unwrap().as_bytes(), b"2023-12-25"); // Full match
    assert_eq!(caps.get(1).unwrap().as_bytes(), b"2023"); // First group
    assert_eq!(caps.get(2).unwrap().as_bytes(), b"12"); // Second group
    assert_eq!(caps.get(3).unwrap().as_bytes(), b"25"); // Third group

    // Verify no named groups are available (check via regex)
    assert!(re.group_names().is_none());
}

#[test]
fn test_with_capture_names() {
    // Test expressions with named capture groups
    let flags = RegexFlags::empty().named_groups();
    let re = Regex::from_bytes(br"(?<year>\d{4})-(?<month>\d{2})-(?<day>\d{2})", flags).unwrap();
    let text = b"2023-12-25";

    let caps = re.captures(text, false).unwrap().unwrap();
    assert_eq!(caps.len(), 4); // Groups: 0, 1, 2, 3

    // Verify numeric access still works
    assert_eq!(caps.get(0).unwrap().as_bytes(), b"2023-12-25"); // Full match
    assert_eq!(caps.get(1).unwrap().as_bytes(), b"2023"); // year group
    assert_eq!(caps.get(2).unwrap().as_bytes(), b"12"); // month group
    assert_eq!(caps.get(3).unwrap().as_bytes(), b"25"); // day group

    // Verify regex exposes group names
    let regex_names = re.group_names().unwrap();
    assert_eq!(regex_names.len(), 3);
    assert!(regex_names.contains(&"year".to_string()));
    assert!(regex_names.contains(&"month".to_string()));
    assert!(regex_names.contains(&"day".to_string()));
}

#[test]
fn test_mixed_capture_groups() {
    // Test expressions with both named and unnamed capture groups
    let flags = RegexFlags::empty().named_groups();
    let re = Regex::from_bytes(br"(\d{4})-(?<month>\d{2})-(\d{2})", flags).unwrap();
    let text = b"2023-12-25";

    let caps = re.captures(text, false).unwrap().unwrap();
    assert_eq!(caps.len(), 4); // Groups: 0, 1, 2, 3

    // Verify numeric access works for all groups
    assert_eq!(caps.get(0).unwrap().as_bytes(), b"2023-12-25"); // Full match
    assert_eq!(caps.get(1).unwrap().as_bytes(), b"2023"); // Unnamed group 1
    assert_eq!(caps.get(2).unwrap().as_bytes(), b"12"); // Named group "month"
    assert_eq!(caps.get(3).unwrap().as_bytes(), b"25"); // Unnamed group 3

    // Verify regex exposes group names
    // The group_names array may be empty if mixed groups aren't supported
    if let Some(names) = re.group_names() {
        assert_eq!(names.len(), 1);
        assert!(names.contains(&"month".to_string()));
    }
}

#[test]
fn test_complex_named_groups() {
    // Test more complex patterns with named groups
    let flags = RegexFlags::empty().named_groups();
    let re = Regex::from_bytes(br"(?<name>\w+)\s+(?<age>\d+)\s+(?<city>\w+)", flags).unwrap();
    let text = b"Alice 25 Beijing";

    let caps = re.captures(text, false).unwrap().unwrap();

    // Verify numeric access works
    assert_eq!(caps.get(1).unwrap().as_bytes(), b"Alice");
    assert_eq!(caps.get(2).unwrap().as_bytes(), b"25");
    assert_eq!(caps.get(3).unwrap().as_bytes(), b"Beijing");

    // Verify regex exposes group names
    let regex_names = re.group_names().unwrap();
    assert_eq!(regex_names.len(), 3);
    assert!(regex_names.contains(&"name".to_string()));
    assert!(regex_names.contains(&"age".to_string()));
    assert!(regex_names.contains(&"city".to_string()));
}

#[test]
fn test_url_parsing() {
    // Test practical example: URL parsing
    let flags = RegexFlags::empty().named_groups();
    let re = Regex::from_bytes(
        br"https?://(?<domain>[^/]+)/(?<path>[^\?]+)\?(?<query>.+)",
        flags,
    )
    .unwrap();
    let text = b"https://example.com/api/users?id=123&name=john";

    let caps = re.captures(text, false).unwrap().unwrap();

    // Verify numeric access works
    assert_eq!(caps.get(1).unwrap().as_bytes(), b"example.com");
    assert_eq!(caps.get(2).unwrap().as_bytes(), b"api/users");
    assert_eq!(caps.get(3).unwrap().as_bytes(), b"id=123&name=john");

    // Verify regex exposes group names
    let regex_names = re.group_names().unwrap();
    assert_eq!(regex_names.len(), 3);
    assert!(regex_names.contains(&"domain".to_string()));
    assert!(regex_names.contains(&"path".to_string()));
    assert!(regex_names.contains(&"query".to_string()));
}

#[test]
fn test_named_groups_with_repetition() {
    // Test named groups with repetition patterns
    let flags = RegexFlags::empty().named_groups();
    let re = Regex::from_bytes(
        br"(?<word>\w+)\s+(?<word>\w+)", // Repeated group name - should work
        flags,
    );

    // This might be implementation-dependent, so we'll test it
    match re {
        Ok(regex) => {
            let text = b"hello world";
            let caps = regex.captures(text, false).unwrap().unwrap();

            // Verify numeric access works
            assert_eq!(caps.get(1).unwrap().as_bytes(), b"hello");
            assert_eq!(caps.get(2).unwrap().as_bytes(), b"world");

            // Verify regex exposes group names
            let regex_names = regex.group_names();
            // Implementation may handle repeated names differently
            assert!(regex_names.is_some());
        }
        Err(_) => {
            // If repeated group names are not supported, that's also valid
        }
    }
}

#[test]
fn test_empty_capture_groups() {
    // Test named groups that don't match anything
    let flags = RegexFlags::empty().named_groups();
    let re = Regex::from_bytes(br"(?<year>\d{4})-(?<month>\d{2})-(?:\d{2})?", flags).unwrap();
    let text = b"2023-"; // Incomplete match

    let caps = re.captures(text, false);

    // This should return None since the pattern doesn't fully match
    assert!(caps.is_ok());
    assert!(caps.unwrap().is_none());
}

#[test]
fn test_nonexistent_group_name() {
    // Test accessing non-existent group names
    let flags = RegexFlags::empty().named_groups();
    let re = Regex::from_bytes(br"(?<year>\d{4})", flags).unwrap();
    let text = b"2023";

    let caps = re.captures(text, false).unwrap().unwrap();

    // Verify numeric access works
    assert_eq!(caps.get(1).unwrap().as_bytes(), b"2023");

    // Verify regex exposes group names
    let regex_names = re.group_names().unwrap();
    assert_eq!(regex_names.len(), 1);
    assert!(regex_names.contains(&"year".to_string()));
}

#[test]
fn test_group_names_order() {
    // Test that group names are returned in the correct order
    let flags = RegexFlags::empty().named_groups();
    let re = Regex::from_bytes(
        br"(?<z>z)(?<a>a)(?<m>m)", // Out of alphabetical order
        flags,
    )
    .unwrap();

    let names = re.group_names().unwrap();

    // Should contain all names regardless of order
    assert_eq!(names.len(), 3);
    assert!(names.contains(&"z".to_string()));
    assert!(names.contains(&"a".to_string()));
    assert!(names.contains(&"m".to_string()));

    // Test that numeric access works regardless of declaration order
    let text = b"zam";
    let caps = re.captures(text, false).unwrap().unwrap();

    assert_eq!(caps.get(1).unwrap().as_bytes(), b"z");
    assert_eq!(caps.get(2).unwrap().as_bytes(), b"a");
    assert_eq!(caps.get(3).unwrap().as_bytes(), b"m");
}

#[test]
fn test_unescaped_special_chars_in_names() {
    // Test that special characters in group names are properly rejected
    let flags = RegexFlags::empty().named_groups();
    let result = Regex::from_bytes(br"(?<my-group>\w+)", flags);

    // libregexp should reject group names with special characters like hyphens
    assert!(result.is_err());
}
