// Equivalent Rust implementation for benchmarking
// Compile with: rustc -O -C target-cpu=native structs.rs

struct Point {
    x: i64,
    y: i64,
}

fn make_points(n: i64) -> Vec<Point> {
    let mut result = Vec::new();
    let mut i = 0;

    while i < n {
        result.push(Point { x: i, y: i * 2 });
        i += 1;
    }

    result
}

fn sum_points(points: &Vec<Point>) -> i64 {
    let mut total = 0;
    let mut i = 0;

    while i < points.len() {
        let p = &points[i];
        total += p.x + p.y;
        i += 1;
    }

    total
}

fn main() {
    let iterations = 100;
    let point_count = 1000;

    let mut i = 0;
    let mut result = 0;

    while i < iterations {
        let points = make_points(point_count);
        result = sum_points(&points);
        i += 1;
    }

    println!("{}", result);
}
