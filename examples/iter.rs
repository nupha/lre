//! Example demonstrating iterator behavior with surrogate pairs
//! Shows how find_iter and find_at work with UTF-16 surrogate pairs

use lre::{Regex, RegexFlags};
use zerocopy::IntoBytes;

fn main() {
    println!("=== Iterator Tests for Surrogate Pairs ===\n");

    // Create regex with unicode flag
    let flags = RegexFlags::empty().unicode();
    let re = Regex::from_bytes(br"^|\udf06", flags).unwrap();

    // Input string: surrogate pair for U+1D306
    let text_utf16: Vec<u16> = vec![0xd834, 0xdf06]; // "\ud834\udf06"
    let bytes = text_utf16.as_bytes();

    println!("Regex pattern: ^|\\udf06");
    println!("Input UTF-16: {:?}", text_utf16);
    println!("Input bytes: {:?}", bytes);
    println!("Bytes length: {}", bytes.len());

    // Part 1: Manual simulation of find_iter behavior
    println!("\n--- Part 1: Manual simulation of find_iter ---");
    let mut last_end = 0;
    let mut last_match: Option<usize> = None;
    let is_wide = true;

    for i in 0..10 {
        println!(
            "Iteration {}: last_end={}, last_match={:?}",
            i, last_end, last_match
        );

        // Avoid infinite loop on zero-length matches
        if let Some(start) = last_match {
            if start == last_end {
                // Advance cursor
                if is_wide {
                    last_end += 2;
                } else {
                    last_end += 1;
                }
                println!("  Advanced last_end to {}", last_end);
            }
        }

        if last_end > bytes.len() {
            println!("  last_end > bytes.len(), stopping");
            break;
        }

        match re.find_at(bytes, last_end, is_wide) {
            Ok(Some(mat)) => {
                println!(
                    "  Found match: start={}, end={}, bytes={:?}",
                    mat.start(),
                    mat.end(),
                    mat.as_bytes()
                );
                last_match = Some(mat.start());
                last_end = mat.end();
            }
            Ok(None) => {
                println!("  No match found");
                break;
            }
            Err(e) => {
                println!("  Error: {:?}", e);
                break;
            }
        }
    }

    // Part 2: Test find_at at different positions
    println!("\n--- Part 2: Testing find_at at specific positions ---");
    for start in [0, 2, 4] {
        println!("\nfind_at(start={}):", start);
        match re.find_at(bytes, start, true) {
            Ok(Some(mat)) => {
                println!(
                    "  Match: start={}, end={}, bytes={:?}",
                    mat.start(),
                    mat.end(),
                    mat.as_bytes()
                );
            }
            Ok(None) => {
                println!("  No match");
            }
            Err(e) => {
                println!("  Error: {:?}", e);
            }
        }
    }

    // Part 3: Test find_at at valid UTF-16 boundary positions
    println!("\n--- Part 3: Testing find_at at valid UTF-16 boundary positions ---");
    for start in [0, 2, 4] {
        let result = re.find_at(bytes, start, true);
        println!("find_at(start={}): {:?}", start, result);
        if let Ok(Some(mat)) = result {
            println!(
                "  Match: start={}, end={}, bytes={:?}",
                mat.start(),
                mat.end(),
                mat.as_bytes()
            );
        }
    }

    // Part 4: Test find_iter
    println!("\n--- Part 4: Testing find_iter ---");
    let matches: Vec<_> = re.find_iter(bytes, 0, true).collect();
    println!("Found {} matches", matches.len());
    for (i, mat) in matches.iter().enumerate() {
        println!(
            "Match {}: start={}, end={}, bytes={:?}",
            i,
            mat.start(),
            mat.end(),
            mat.as_bytes()
        );
    }

    // Part 5: Using Regex exec method (public API)
    println!("\n--- Part 5: Using Regex exec method ---");
    let result = re.exec(bytes, 0, true, std::ptr::null_mut()).unwrap();
    println!(
        "re.exec(start=0): success={}, captures={:?}",
        result.success, result.captures
    );

    let result2 = re.exec(bytes, 2, true, std::ptr::null_mut()).unwrap();
    println!(
        "re.exec(start=2): success={}, captures={:?}",
        result2.success, result2.captures
    );

    // Part 6: Test with just low surrogate
    println!("\n--- Part 6: Testing with just low surrogate ---");
    let low_surrogate: Vec<u16> = vec![0xdf06];
    let low_bytes = low_surrogate.as_bytes();
    println!("Low surrogate bytes: {:?}", low_bytes);

    for start in [0, 2] {
        let result = re.find_at(low_bytes, start, true);
        println!("find_at(start={}): {:?}", start, result);
    }

    // Part 7: Test with empty string
    println!("\n--- Part 7: Testing with empty string ---");
    let empty: Vec<u16> = vec![];
    let empty_bytes = empty.as_bytes();
    let matches: Vec<_> = re.find_iter(empty_bytes, 0, true).collect();
    println!("Empty string matches: {}", matches.len());

    println!("\n=== All iterator tests completed ===");
}
