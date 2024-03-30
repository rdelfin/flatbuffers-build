#![warn(clippy::all, clippy::pedantic)]

//! This crate provides a set of functions to facilitate compiling flatbuffers to Rust from within
//! Rust. This is particularly helpful for use in `build.rs` scripts. Please note that for
//! compatiblity this crate will only support a single version of the `flatc` compiler. Please
//! check what version that is against whatever version is installed on your system.That said, due
//! to flatbuffers' versioning policy, it could be ok to mix patch and even minor versions.
//!
//! If you're not sure where to start, take a look at [`BuilderOptions`]. Please also look at the
//! [`flatbuffers-example`](https://github.com/rdelfin/flatbuffers-build/tree/main/flatbuffers-example)
//! folder in the repo for an example. However, we'll explain the full functionality here.
//!
//! As an example, imagine a crate with the following folder structure:
//! ```bash
//! ├── build.rs
//! ├── Cargo.toml
//! ├── example.fbs
//! └── src
//!     └── lib.rs
//! ```
//! In order to compile and use the code generated from `example.fbs` code, first you need to add
//! `flatbuffers-build` to your build dependencies, as well as a matching version of `flatbuffers`:
//! ```toml
//! # Cargo.toml
//! # [...]
//! [dependencies]
//! flatbuffers = "=23.5.26"
//!
//! [build-dependencies]
//! flatbuffers-build = "=23.5.26"
//! # [...]
//! ```
//!
//! You can then have a very simple `build.rs` as follows:
//! ```no_run
//! use flatbuffers_build::BuilderOptions;
//!
//! BuilderOptions::new_with_files(["example.fbs"])
//!     .compile()
//!     .expect("flatbuffer compilation failed");
//! ```
//!
//! Note here that `example.fbs` is the same one provided by `flatbuffers` as an example. The
//! namespace is `MyGame.Sample` and it contains multiple tables and structs, including a `Monster`
//! table.
//!
//! This will just compile the flatbuffers and drop them in `OUT_DIR`. You can then pull them in in
//! `lib.rs` like so:
//!
//! ```rust,ignore
//! #[allow(warnings)]
//! pub mod defs {
//!     include!(concat!(env!("OUT_DIR"), "/example_generated.rs"));
//! }
//!
//! use defs::my_game::sample::Monster;
//!
//! fn some_fn() {
//!     // Make use of `Monster`
//! }
//! ```

use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    process::Command,
};

const FLATC_VERSION_PREFIX: &str = "flatc version ";
const FLATC_BUILD_PATH: Option<&str> = option_env!("FLATC_PATH");

/// Version of `flatc` supported by this library. Make sure this matches exactly with the `flatc`
/// binary you're using and the version of the `flatbuffers` rust library.
pub const SUPPORTED_FLATC_VERSION: &str = "23.5.26";

/// Primary error type returned when you compile your flatbuffer specifications to Rust.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Returned when `flatc` returns with an non-zero status code for a reason not covered
    /// elsewhere in this enum.
    #[error("flatc exited unexpectedly with status code {status_code:?}\n-- stdout:\n{stdout}\n-- stderr:\n{stderr}\n")]
    FlatcErrorCode {
        /// Status code returned by `flatc` (none if program was terminated by a signal).
        status_code: Option<i32>,
        /// Standard output stream contents of the program
        stdout: String,
        /// Standard error stream contents of the program
        stderr: String,
    },
    /// Returned if `flatc --version` generates output we cannot parse. Usually means that the
    /// binary requested is not, in fact, flatc.
    #[error("flatc returned invalid output for --version: {0}")]
    InvalidFlatcOutput(String),
    /// Returned if the version of `flatc` does not match the supported version. Please refer to
    /// [`SUPPORTED_FLATC_VERSION`] for that.
    #[error("flatc version '{0}' is unsupported by this version of the library. Please match your library with your flatc version")]
    UnsupportedFlatcVersion(String),
    /// Returned if we fail to spawn a process with `flatc`. Usually means the supplied path to
    /// flatc does not exist.
    #[error("flatc failed to spawn: {0}")]
    FlatcSpawnFailure(#[from] std::io::Error),
    /// Returned if you failed to set either the output path or the `OUT_DIR` environment variable.
    #[error(
        "Output directory was not set. Either call .set_output_path() or set the `OUT_DIR` env var"
    )]
    OutputDirNotSet,
}

/// Alias for a Result that uses [`Error`] as the default error type.
pub type Result<T = (), E = Error> = std::result::Result<T, E>;

/// Builder for options to the flatc compiler options. When consumed using
/// [`BuilderOptions::compile`], this generates rust code from the flatbuffer definition files
/// provided. The basic usage for this struct looks something like this:
/// ```no_run
/// use flatbuffers_build::BuilderOptions;
///
/// BuilderOptions::new_with_files(["some_file.fbs", "some_other_file.fbs"])
///     .compile()
///     .expect("flatbuffer compilation failed");
/// ```
///
/// This struct operates as a builder pattern, so you can do things like set the `flatc` path:
/// ```no_run
/// # use flatbuffers_build::BuilderOptions;
/// BuilderOptions::new_with_files(["some_file.fbs", "some_other_file.fbs"])
///     .set_compiler("/some/path/to/flatc")
///     .compile()
///     .expect("flatbuffer compilation failed");
/// ```
///
/// Consult the functions bellow for more details.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuilderOptions {
    files: Vec<PathBuf>,
    compiler: Option<String>,
    output_path: Option<PathBuf>,
}

impl BuilderOptions {
    /// Create a new builder for the compiler options. We purely initialise with an iterable of
    /// files to compile. To actually build, refer to the [`Self::compile`] function.
    ///
    /// # Arguments
    /// * `files` - An iterable of files that should be compiled into rust code. No glob resolution
    ///             happens here, and all paths MUST match to real files, either as absolute paths
    ///             or relative to the current working directory.
    #[must_use]
    pub fn new_with_files<P: AsRef<Path>, I: IntoIterator<Item = P>>(files: I) -> Self {
        BuilderOptions {
            files: files.into_iter().map(|f| f.as_ref().into()).collect(),
            compiler: None,
            output_path: None,
        }
    }

    /// Set the path of the `flatc` binary to use as a compiler. If no such path is provided, we
    /// will default to first using whatever's set in the `FLATC_PATH` environment variable, or if
    /// that's not set, we will let the system resolve using standard `PATH` resolution.
    ///
    /// # Arguments
    /// * `compiler` - Path to the compiler to run. This can also be a name that we should resolve
    ///                using standard `PATH` resolution.
    #[must_use]
    pub fn set_compiler<S: AsRef<str>>(self, compiler: S) -> Self {
        BuilderOptions {
            compiler: Some(compiler.as_ref().into()),
            ..self
        }
    }

    /// Call this to set the output directory of the protobufs. If you don't set this, we will
    /// default to writing to whatever the `OUT_DIR` environment variable is set to.
    ///
    /// # Arguments
    /// * `output_path` - The directory to write the files to.
    #[must_use]
    pub fn set_output_path<P: AsRef<Path>>(self, output_path: P) -> Self {
        BuilderOptions {
            output_path: Some(output_path.as_ref().into()),
            ..self
        }
    }

    /// Call this function to trigger compilation. Will write the compiled protobufs to the
    /// specified directoyr, or to `OUT_DIR` by default.
    ///
    /// # Errors
    /// Will fail if any error happens during compilation, including:
    /// - Invalid protoc files
    /// - Unsupported flatc version
    /// - flatc exiting with a non-zero error code
    /// For more details, see [`Error`].
    pub fn compile(self) -> Result {
        compile(self)
    }
}

fn compile(builder_options: BuilderOptions) -> Result {
    let files_str: Vec<_> = builder_options
        .files
        .iter()
        .map(|p| p.clone().into_os_string())
        .collect();
    let compiler = builder_options.compiler.unwrap_or_else(|| {
        if let Some(build_flatc) = FLATC_BUILD_PATH {
            build_flatc.to_owned()
        } else {
            std::env::var("FLATC_PATH").unwrap_or("flatc".into())
        }
    });
    let output_path = builder_options.output_path.map_or_else(
        || std::env::var_os("OUT_DIR").ok_or(Error::OutputDirNotSet),
        |p| Ok(p.into_os_string()),
    )?;

    confirm_flatc_version(&compiler)?;

    let mut args = vec![OsString::from("--rust"), OsString::from("-o"), output_path];
    args.extend(files_str);
    run_flatc(&compiler, &args)?;
    Ok(())
}

fn confirm_flatc_version(compiler: &str) -> Result {
    // Output shows up in stdout
    let output = run_flatc(compiler, ["--version"])?;
    if output.stdout.starts_with(FLATC_VERSION_PREFIX) {
        let version_str = output.stdout[FLATC_VERSION_PREFIX.len()..].trim_end();
        if version_str == SUPPORTED_FLATC_VERSION {
            Ok(())
        } else {
            Err(Error::UnsupportedFlatcVersion(version_str.into()))
        }
    } else {
        Err(Error::InvalidFlatcOutput(output.stdout))
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
    let output = Command::new(compiler)
        .args(args)
        .output()
        .map_err(Error::FlatcSpawnFailure)?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    if output.status.success() {
        Ok(ProgramOutput {
            stdout,
            _stderr: stderr,
        })
    } else {
        Err(Error::FlatcErrorCode {
            status_code: output.status.code(),
            stdout,
            stderr,
        })
    }
}
