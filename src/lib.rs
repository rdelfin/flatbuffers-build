use std::{
    path::{Path, PathBuf},
    process::Command,
};

#[derive(thiserror::Error, Debug)]
pub enum Error {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuilderOptions {
    files: Vec<PathBuf>,
    compiler: String,
    output_path: PathBuf,
}

impl BuilderOptions {
    pub fn new_with_files<P: AsRef<Path>, I: IntoIterator<Item = P>>(files: I) -> Self {
        BuilderOptions {
            files: files.into_iter().map(|f| f.as_ref().into()).collect(),
            compiler: std::env::var("FLATC_PATH").unwrap_or("flatc".into()),
            output_path: PathBuf::from("."),
        }
    }

    pub fn set_compiler(self, compiler: String) -> Self {
        BuilderOptions { compiler, ..self }
    }

    pub fn set_output_path<P: AsRef<Path>>(self, output_path: P) -> Self {
        BuilderOptions {
            output_path: output_path.as_ref().into(),
            ..self
        }
    }

    pub fn compile(self) {
        compile(self)
    }
}

fn compile(builder_options: BuilderOptions) {
    let mut child = Command::new(builder_options.compiler)
        .args(&[
            "--rust",
            "-o",
            &builder_options
                .output_path
                .into_os_string()
                .into_string()
                .unwrap(),
        ])
        .spawn()
        .unwrap();

    child.wait().unwrap();
}
