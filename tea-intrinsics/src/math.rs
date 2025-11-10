/// Rounds a float down to the nearest integer
pub fn floor(value: f64) -> i64 {
    value.floor() as i64
}

/// Rounds a float up to the nearest integer
pub fn ceil(value: f64) -> i64 {
    value.ceil() as i64
}

/// Rounds a float to the nearest integer
pub fn round(value: f64) -> i64 {
    value.round() as i64
}

/// Returns the absolute value of a float
pub fn abs(value: f64) -> f64 {
    value.abs()
}

/// Returns the square root of a float
pub fn sqrt(value: f64) -> f64 {
    value.sqrt()
}

/// Returns the minimum of two floats
pub fn min(a: f64, b: f64) -> f64 {
    a.min(b)
}

/// Returns the maximum of two floats
pub fn max(a: f64, b: f64) -> f64 {
    a.max(b)
}
