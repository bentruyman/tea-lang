// Reference implementation in Rust for comparison
// Compile with: rustc -O reference_loops.rs

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
    let iterations = 10000;
    let n = 1000;
    
    let mut i = 0;
    let mut result = 0;
    while i < iterations {
        result = sum_to_n(n);
        i = i + 1;
    }
    
    println!("{}", result);
}
