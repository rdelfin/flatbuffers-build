use flatbuffers_gen::BuilderOptions;

fn main() {
    BuilderOptions::new_with_files(&["example.fbs"]).compile()
}
