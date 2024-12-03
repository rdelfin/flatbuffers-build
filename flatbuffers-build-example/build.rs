use flatbuffers_build::BuilderOptions;

fn main() {
    BuilderOptions::new_with_files(["schemas/weapon.fbs", "schemas/example.fbs"])
        .compile()
        .expect("flatbuffer compilation failed")
}
