//! Basic usage example for libregexp-rs

use lre::{Regex, RegexFlags};
use std::borrow::Cow;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Basic Regex Matching ===");

    // Create a regex for matching digits
    let re = Regex::from_str(r"\d+", RegexFlags::empty())?;
    let text = "There are 123 apples and 456 oranges.";
    let text_bytes = text.as_bytes();

    println!("Text: {}", text);
    println!("Pattern: {}", r"\d+");

    // Check if the pattern matches
    if re.is_match(text_bytes, false) {
        println!("Pattern matches the text!");
    }

    // Find all matches
    println!("\nAll matches:");
    for mat in re.find_iter(text_bytes, 0, false) {
        let matched_str = String::from_utf8_lossy(mat.as_bytes());
        println!(
            "  Found '{}' at position {}-{}",
            matched_str,
            mat.start(),
            mat.end()
        );
    }

    println!("\n=== Capture Groups ===");

    // Create a regex with capture groups
    let re = Regex::from_str(r"(\d+)\s+(\w+)", RegexFlags::empty())?;
    let text = "123 apples, 456 oranges, 789 bananas";
    let text_bytes = text.as_bytes();

    println!("Text: {}", text);
    println!("Pattern: {}", r"(\d+)\s+(\w+)");

    // Find all captures
    for caps in re.captures_iter(text_bytes, false) {
        let full_match = String::from_utf8_lossy(caps.get(0).unwrap().as_bytes());
        let number = String::from_utf8_lossy(caps.get(1).unwrap().as_bytes());
        let fruit = String::from_utf8_lossy(caps.get(2).unwrap().as_bytes());
        println!("  Full match: {}", full_match);
        println!("    Number: {}", number);
        println!("    Fruit: {}", fruit);
    }

    println!("\n=== Replacement ===");

    let re = Regex::from_str(r"\d+", RegexFlags::empty())?;
    let text = "I have 3 apples and 5 oranges";
    let text_bytes = text.as_bytes();
    let replaced = re.replace_all(text_bytes, false, b"N");
    let replaced_str = String::from_utf8_lossy(&replaced);

    println!("Original: {}", text);
    println!("Replaced: {}", replaced_str);

    println!("\n=== Case Insensitive Matching ===");

    let flags = RegexFlags::empty().ignore_case();
    let re = Regex::from_bytes(b"hello", flags)?;

    let texts = ["HELLO", "Hello", "hello", "HeLlO"];
    for text in &texts {
        if re.is_match(text.as_bytes(), false) {
            println!("  '{}' matches!", text);
        }
    }

    println!("\n=== Splitting ===");

    let re = Regex::from_str(r"\s*,\s*", RegexFlags::empty())?;
    let text = "apple, banana, cherry, date";
    let text_bytes = text.as_bytes();
    let parts: Vec<&[u8]> = re.split(text_bytes, false).collect();
    let parts_str: Vec<Cow<'_, str>> = parts
        .iter()
        .map(|&bytes| String::from_utf8_lossy(bytes))
        .collect();

    println!("Text: {}", text);
    println!("Split parts: {:?}", parts_str);

    Ok(())
}
