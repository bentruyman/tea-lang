// Equivalent Rust implementation for benchmarking
// Compile with: rustc -O -C target-cpu=native dicts.rs

use std::collections::HashMap;

fn build_dict(n: i64) -> HashMap<String, i64> {
    let mut result = HashMap::new();
    result.insert("_".to_string(), 0);
    let mut i = 0;

    while i < n {
        result.insert(format!("key_{}", i), i);
        i += 1;
    }

    result
}

fn lookup_dict(dict: &HashMap<String, i64>, count: i64) -> i64 {
    let mut total = 0;
    let mut i = 0;

    while i < count {
        let key = format!("key_{}", i);
        total += dict.get(&key).unwrap_or(&0);
        i += 1;
    }

    total
}

fn main() {
    let iterations = 100;
    let dict_size = 500;

    let mut i = 0;
    let mut result = 0;

    while i < iterations {
        let dict = build_dict(dict_size);
        result = lookup_dict(&dict, dict_size);
        i += 1;
    }

    println!("{}", result);
}
