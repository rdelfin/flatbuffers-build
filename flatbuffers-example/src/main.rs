#[allow(warnings)]
pub mod defs {
    include!(concat!(env!("OUT_DIR"), "/example_generated.rs"));
}

fn main() {
    println!("Hello, world!");
}
