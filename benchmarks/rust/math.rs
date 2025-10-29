// Equivalent Rust implementation for benchmarking
// Compile with: rustc -O -C target-cpu=native math.rs

fn compute(n: i64) -> i64 {
    let mut result = 0;
    let mut i = 0;

    while i < n {
        result = result + i * 2 - i / 2;
        i += 1;
    }

    result
}

fn main() {
    let iterations = 1000;
    let n = 10000;

    let mut i = 0;
    let mut result = 0;

    while i < iterations {
        result = compute(n);
        i += 1;
    }

    println!("{}", result);
}
