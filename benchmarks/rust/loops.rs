// Equivalent Rust implementation for benchmarking
// Compile with: rustc -O -C target-cpu=native loops.rs

fn sum_to_n(n: i64) -> i64 {
    let mut total = 0;
    let mut i = 1;
    while i <= n {
        total = total + i;
        i = i + 1;
    }
    total
}

fn main() {
    let iterations = 20000000;
    let base_n = 1000;
    
    let mut i = 0;
    let mut result = 0;
    while i < iterations {
        result += sum_to_n(base_n + i % 7);
        i = i + 1;
    }
    
    println!("{}", result);
}
