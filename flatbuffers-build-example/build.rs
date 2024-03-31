use flatbuffers_build::BuilderOptions;

fn main() {
    BuilderOptions::new_with_files(["schemas/weapon.fbs", "schemas/example.fbs"])
        .set_symlink_directory("src/gen_flatbuffers")
        .compile()
        .expect("flatbuffer compilation failed")
}
