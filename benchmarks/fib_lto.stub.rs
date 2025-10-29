extern crate tea_runtime;

extern "C" {
    fn tea_main() -> i32;
}

fn main() {
    std::process::exit(unsafe { tea_main() });
}
