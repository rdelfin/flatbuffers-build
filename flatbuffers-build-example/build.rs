use flatbuffers_build::BuilderOptions;

fn main() {
    BuilderOptions::new_with_files(["schemas/weapon.fbs", "schemas/example.fbs"])
        .gen_object_api()
        .compile()
        .expect("flatbuffer compilation failed")
}
