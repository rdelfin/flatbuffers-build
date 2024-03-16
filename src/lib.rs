use std::{
    path::{Path, PathBuf},
    process::Command,
};

#[derive(thiserror::Error, Debug)]
pub enum Error {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuilderOptions {
    files: Vec<PathBuf>,
    compiler: Option<String>,
    output_path: Option<PathBuf>,
}

impl BuilderOptions {
    pub fn new_with_files<P: AsRef<Path>, I: IntoIterator<Item = P>>(files: I) -> Self {
        BuilderOptions {
            files: files.into_iter().map(|f| f.as_ref().into()).collect(),
            compiler: None,
            output_path: None,
        }
    }

    pub fn set_compiler(self, compiler: String) -> Self {
        BuilderOptions {
            compiler: Some(compiler),
            ..self
        }
    }

    pub fn set_output_path<P: AsRef<Path>>(self, output_path: P) -> Self {
        BuilderOptions {
            output_path: Some(output_path.as_ref().into()),
            ..self
        }
    }

    pub fn compile(self) {
        compile(self)
    }
}

fn compile(builder_options: BuilderOptions) {
    let files_str: Vec<_> = builder_options
        .files
        .iter()
        .map(|p| p.clone().into_os_string().into_string().unwrap())
        .collect();
    let compiler = builder_options
        .compiler
        .unwrap_or_else(|| std::env::var("FLATC_PATH").unwrap_or("flatc".into()));
    let output_path = builder_options
        .output_path
        .map(|p| Ok(p.into_os_string().into_string().unwrap()))
        .unwrap_or_else(|| std::env::var("OUT_DIR"))
        .unwrap();

    let mut args = vec!["--rust", "-o", &output_path];
    args.extend(files_str.iter().map(|s| &s[..]));

    let mut child = Command::new(compiler).args(&args).spawn().unwrap();

    child.wait().unwrap();
}
