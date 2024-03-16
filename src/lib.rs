use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Command,
};

const FLATC_VERSION_PREFIX: &str = "flatc version ";
const SUPPORTED_FLATC_VERSION: &str = "23.5.26";

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("flatc exited unexpectedly with status code {status_code:?}\n-- stdout:\n{stdout}\n-- stderr:\n{stderr}\n")]
    FlatcErrorCode {
        status_code: Option<i32>,
        stdout: String,
        stderr: String,
    },
    #[error("flatc returned invalid output for --version: {0}")]
    InvalidFlatcOutput(String),
    #[error("flatc version '{0}' is unsupported by this version of the library. Please match your library with your flatc version")]
    UnsupportedFlatcVersion(String),
}

pub type Result<T = (), E = Error> = std::result::Result<T, E>;

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

    pub fn compile(self) -> Result {
        compile(self)
    }
}

fn compile(builder_options: BuilderOptions) -> Result {
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

    confirm_flatc_version(&compiler)?;

    let mut args = vec!["--rust", "-o", &output_path];
    args.extend(files_str.iter().map(|s| &s[..]));
    run_flatc(&compiler, &args)?;
    Ok(())
}

fn confirm_flatc_version(compiler: &str) -> Result {
    // Output shows up in stdout
    let output = run_flatc(compiler, ["--version"])?;
    if !output.stdout.starts_with(FLATC_VERSION_PREFIX) {
        Err(Error::InvalidFlatcOutput(output.stdout))
    } else {
        let version_str = output.stdout[FLATC_VERSION_PREFIX.len()..].trim_end();
        if version_str != SUPPORTED_FLATC_VERSION {
            Err(Error::UnsupportedFlatcVersion(version_str.into()))
        } else {
            Ok(())
        }
    }
}

struct ProgramOutput {
    pub stdout: String,
    pub _stderr: String,
}

fn run_flatc<I: IntoIterator<Item = S>, S: AsRef<OsStr>>(
    compiler: &str,
    args: I,
) -> Result<ProgramOutput> {
    let output = Command::new(compiler).args(args).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    if !output.status.success() {
        Err(Error::FlatcErrorCode {
            status_code: output.status.code(),
            stdout,
            stderr,
        })
    } else {
        Ok(ProgramOutput {
            stdout,
            _stderr: stderr,
        })
    }
}
