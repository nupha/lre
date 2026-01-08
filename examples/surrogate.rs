//! Comprehensive example for surrogate pair matching
//! Demonstrates various aspects of regex matching with UTF-16 surrogate pairs

use lre::{Regex, RegexFlags};
use zerocopy::IntoBytes;

fn main() {
    println!("=== Surrogate Pair Matching Tests ===\n");

    // Test 1: Basic surrogate pair matching with ^|\udf06 pattern
    println!("Test 1: Matching ^|\\udf06 pattern on surrogate pair");
    let flags = RegexFlags::empty().unicode();
    let re = Regex::from_bytes(br"^|\udf06", flags).unwrap();

    let text_utf16: Vec<u16> = vec![0xd834, 0xdf06]; // "\ud834\udf06" - U+1D306
    let bytes = text_utf16.as_bytes();

    println!(
        "Input UTF-16: {:?} (length: {})",
        text_utf16,
        text_utf16.len()
    );
    println!("Input as bytes: {:?}", bytes);

    // Test is_match
    println!("\nTesting is_match:");
    let is_match = re.is_match(bytes, true);
    println!("is_match result: {}", is_match);

    // Test find
    println!("\nTesting find:");
    match re.find(bytes, true) {
        Ok(Some(mat)) => {
            println!(
                "Found match: start={}, end={}, bytes={:?}",
                mat.start(),
                mat.end(),
                mat.as_bytes()
            );
        }
        Ok(None) => println!("No match found"),
        Err(e) => println!("Error: {:?}", e),
    }

    // Test find_iter with limit
    println!("\nTesting find_iter (first 5 matches):");
    let mut count = 0;
    for mat in re.find_iter(bytes, 0, true) {
        println!(
            "Match {}: start={}, end={}, bytes={:?}",
            count,
            mat.start(),
            mat.end(),
            mat.as_bytes()
        );
        count += 1;
        if count >= 5 {
            println!("Stopping after 5 matches to avoid infinite loop");
            break;
        }
    }
    println!("Total matches found: {}", count);

    // Test find_at at different positions
    println!("\nTesting find_at at different positions:");
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

    // Test 2: Match just the low surrogate
    println!("\n\n=== Test 2: Matching just low surrogate \\udf06 ===");
    let re_low = Regex::from_bytes(br"\udf06", flags).unwrap();
    let low_surrogate: Vec<u16> = vec![0xdf06];
    let low_bytes = low_surrogate.as_bytes();

    println!("Input UTF-16: {:?}", low_surrogate);
    match re_low.find(low_bytes, true) {
        Ok(Some(mat)) => {
            println!(
                "Found match: start={}, end={}, bytes={:?}",
                mat.start(),
                mat.end(),
                mat.as_bytes()
            );
        }
        Ok(None) => println!("No match found"),
        Err(e) => println!("Error: {:?}", e),
    }

    // Test 3: Match low surrogate in surrogate pair
    println!("\n=== Test 3: Matching low surrogate in surrogate pair ===");
    match re_low.find(bytes, true) {
        Ok(Some(mat)) => {
            println!(
                "Found match: start={}, end={}, bytes={:?}",
                mat.start(),
                mat.end(),
                mat.as_bytes()
            );
        }
        Ok(None) => println!("No match found"),
        Err(e) => println!("Error: {:?}", e),
    }

    // Test 4: Test with ^ pattern only
    println!("\n=== Test 4: Testing ^ pattern only ===");
    let re_caret = Regex::from_bytes(b"^", flags).unwrap();
    match re_caret.find(bytes, true) {
        Ok(Some(mat)) => {
            println!(
                "Found ^ match: start={}, end={}, bytes={:?}",
                mat.start(),
                mat.end(),
                mat.as_bytes()
            );
        }
        Ok(None) => println!("No match found"),
        Err(e) => println!("Error: {:?}", e),
    }

    // Test 5: Test with alternative pattern ^|\udf06 using find_iter
    println!("\n=== Test 5: Testing ^|\\udf06 pattern with find_iter ===");
    println!("Using find_iter (first 3 matches):");
    let mut count = 0;
    for mat in re.find_iter(bytes, 0, true) {
        println!(
            "Match {}: start={}, end={}, bytes={:?}",
            count,
            mat.start(),
            mat.end(),
            mat.as_bytes()
        );
        count += 1;
        if count >= 3 {
            break;
        }
    }

    // Test 6: Test with just low surrogate using find_at
    println!("\n=== Test 6: Testing just low surrogate with find_at ===");
    for start in [0, 2] {
        let result = re.find_at(low_bytes, start, true);
        println!("find_at(start={}): {:?}", start, result);
    }

    // Test 7: Test with empty string
    println!("\n=== Test 7: Testing empty string ===");
    let empty: Vec<u16> = vec![];
    let empty_bytes = empty.as_bytes();
    let matches: Vec<_> = re.find_iter(empty_bytes, 0, true).collect();
    println!("Empty string matches: {}", matches.len());

    // Test 8: Using Regex exec method
    println!("\n=== Test 8: Using Regex exec method ===");
    let result = re.exec(bytes, 0, true, std::ptr::null_mut()).unwrap();
    println!(
        "success: {}, captures: {:?}",
        result.success, result.captures
    );
    let result2 = re.exec(bytes, 2, true, std::ptr::null_mut()).unwrap();
    println!(
        "success at start=2: {}, captures: {:?}",
        result2.success, result2.captures
    );

    println!("\n=== All surrogate pair tests completed ===");
}
