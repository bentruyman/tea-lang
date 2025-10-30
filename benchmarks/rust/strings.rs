// Equivalent Rust implementation for benchmarking
// Compile with: rustc -O -C target-cpu=native strings.rs

fn build_string(count: i64) -> String {
    let mut result = String::new();
    let mut i = 0;

    while i < count {
        result.push('x');
        i += 1;
    }

    result
}

fn main() {
    let iterations = 1000;
    let string_length = 1000;

    let mut i = 0;
    let mut result = String::new();

    while i < iterations {
        result = build_string(string_length);
        i += 1;
    }

    println!("Built string of length {}", result.len());
}
