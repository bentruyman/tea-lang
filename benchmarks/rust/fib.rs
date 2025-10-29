// Equivalent Rust implementation for benchmarking
// Compile with: rustc -O -C target-cpu=native fib.rs

fn fib(n: i64) -> i64 {
    if n <= 1 {
        return n;
    }
    fib(n - 1) + fib(n - 2)
}

fn main() {
    let n = 40;
    let result = fib(n);
    println!("{}", result);
}
