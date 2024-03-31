use flatbuffers_build::BuilderOptions;

fn main() {
    BuilderOptions::new_with_files(["weapon.fbs", "example.fbs"])
        .set_symlink_directory("src/generated")
        .compile()
        .expect("flatbuffer compilation failed")
}
