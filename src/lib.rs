#![warn(clippy::all, clippy::pedantic, clippy::cargo)]

//! This crate provides a set of functions to facilitate compiling flatbuffers to Rust from within
//! Rust. This is particularly helpful for use in `build.rs` scripts. Please note that for
//! compatiblity this crate will only support a single version of the `flatc` compiler. Please
//! check what version that is against whatever version is installed on your system.That said, due
//! to flatbuffers' versioning policy, it could be ok to mix patch and even minor versions.
//!
//! ## Usage
//!
//! If you're not sure where to start, take a look at [`BuilderOptions`]. Please also look at the
//! [`flatbuffers-build-example`](https://github.com/rdelfin/flatbuffers-build/tree/main/flatbuffers-build-example)
//! folder in the repo for an example. However, we'll explain the full functionality here.
//!
//! As an example, imagine a crate with the following folder structure:
//! ```bash
//! ├── build.rs
//! ├── Cargo.toml
//! ├── schemas
//! │   ├── example.fbs
//! │   └── weapon.fbs
//! └── src
//!     └── main.rs
//! ```
//! In order to compile and use the code generated from both `example.fbs` and `weapon.fbs`, first
//! you need to add `flatbuffers-build` to your build dependencies, as well as a matching version
//! of `flatbuffers`:
//! ```toml
//! # Cargo.toml
//! # [...]
//! [dependencies]
//! flatbuffers = "=24.3.25"
//!
//! [build-dependencies]
//! flatbuffers-build = "=24.3.25"
//! # [...]
//! ```
//!
//! You can then have a very simple `build.rs` as follows:
//! ```no_run
//! use flatbuffers_build::BuilderOptions;
//!
//! BuilderOptions::new_with_files(["schemas/weapon.fbs", "schemas/example.fbs"])
//!     .set_symlink_directory("src/gen_flatbuffers")
//!     .compile()
//!     .expect("flatbuffer compilation failed");
//! ```
//!
//! Note here that `weapon.fbs` and `example.fbs` are based on the schemas provided by
//! `flatbuffers` as an example. The namespace is `MyGame.Sample` and it contains multiple tables
//! and structs, including a `Monster` table.
//!
//! This will just compile the flatbuffers and drop them in `${OUT_DIR}/flatbuffers` and will
//! create a symlink under `src/gen_flatbuffers`. You can then use them in `lib.rs` like so:
//!
//! ```rust,ignore
//! #[allow(warnings)]
//! mod gen_flatbuffers;
//!
//! use gen_flatbuffers::my_game::sample::Monster;
//!
//! fn some_fn() {
//!     // Make use of `Monster`
//! }
//! ```
//!
//! Note that since this will generate a symlink under `src/gen_flatbuffers`, you need to add this
//! file to your gitignore as this symlink will dynamically change at runtime.
//!
//! ## On file ordering
//!
//! Unfortunately due to a quirk in the `flatc` compiler the order you provide the `fbs` files does
//! matter. From some experimentation, the guidance is to always list files _after_ their
//! dependencies. Otherwise, the resulting `mod.rs` will be unusable. As an example, we have a
//! `weapon.fbs` and `example.fbs`. Since the latter has an `include` directive for `weapon.fbs`,
//! it should go after in the list. If you were to put `example.fbs` _before_ `weapon.fbs`, you'd
//! end up only being able to import the contents of `weapon.fbs` and with compilation errors if
//! you tried to use any other components.

use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    process::Command,
};

const FLATC_VERSION_PREFIX: &str = "flatc version ";
const FLATC_BUILD_PATH: Option<&str> = option_env!("FLATC_PATH");

/// Version of `flatc` supported by this library. Make sure this matches exactly with the `flatc`
/// binary you're using and the version of the `flatbuffers` rust library.
pub const SUPPORTED_FLATC_VERSION: &str = "24.3.25";

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
    FlatcSpawnFailure(#[source] std::io::Error),
    /// Returned if you failed to set either the output path or the `OUT_DIR` environment variable.
    #[error(
        "output directory was not set. Either call .set_output_path() or set the `OUT_DIR` env var"
    )]
    OutputDirNotSet,
    /// Returned when an issue arrises when creating the symlink. Typically this will be things
    /// like permissions, a directory existing already at the file location, or other filesystem
    /// errors.
    #[error("failed to create symlink path requested: {0}")]
    SymlinkCreationFailure(#[source] std::io::Error),
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
    symlink_path: Option<PathBuf>,
    supress_buildrs_directives: bool,
}

impl BuilderOptions {
    /// Create a new builder for the compiler options. We purely initialise with an iterable of
    /// files to compile. To actually build, refer to the [`Self::compile`] function. Note that the
    /// order of the files is actually important, as incorrect ordering will result in incorrect
    /// generated code with missing components. You should always put dependencies of other files
    /// earlier in the list. In other words, if `schema_a.fbs` imports `schema_b.fbs`, then you'd
    /// want to call this with:
    ///
    /// ```rust
    /// # use flatbuffers_build::BuilderOptions;
    /// BuilderOptions::new_with_files(["schema_b.fbs", "schema_a.fbs"]);
    /// ```
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
            symlink_path: None,
            supress_buildrs_directives: false,
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
    /// default to writing to `${OUT_DIR}/flatbuffers`.
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

    /// Set a path to create a symlink that points to the output files. This is commonly used to
    /// symlink to a folder under `src` so you can normally pull in the generated code as a module.
    /// We recommend always calling this and setting it to `src/generated` or something similar.
    ///
    /// # Arguments
    /// * `symlink_path` - Path to generate the symlink to.
    #[must_use]
    pub fn set_symlink_directory<P: AsRef<Path>>(self, symlink_path: P) -> Self {
        BuilderOptions {
            symlink_path: Some(symlink_path.as_ref().into()),
            ..self
        }
    }

    /// Set this if you're not running from a `build.rs` script and don't want us to print the
    /// build.rs instructions/directives that we would otherwise print in stdout.
    #[must_use]
    pub fn supress_buildrs_directives(self) -> Self {
        BuilderOptions {
            supress_buildrs_directives: true,
            ..self
        }
    }

    /// Call this function to trigger compilation. Will write the compiled protobufs to the
    /// specified directory, or to `${OUT_DIR}/flatbuffers` by default.
    ///
    /// # Errors
    /// Will fail if any error happens during compilation, including:
    /// - Invalid protoc files
    /// - Unsupported flatc version
    /// - flatc exiting with a non-zero error code
    ///
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
        || {
            std::env::var_os("OUT_DIR")
                .ok_or(Error::OutputDirNotSet)
                .map(|mut s| {
                    s.push(OsString::from("/flatbuffers"));
                    s
                })
        },
        |p| Ok(p.into_os_string()),
    )?;

    confirm_flatc_version(&compiler)?;

    let mut args = vec![
        OsString::from("--rust"),
        OsString::from("--rust-module-root-file"),
        OsString::from("-o"),
        output_path.clone(),
    ];
    args.extend(files_str);
    run_flatc(&compiler, &args)?;

    if let Some(symlink_path) = builder_options.symlink_path {
        generate_symlink(&symlink_path, PathBuf::from(output_path))?;
        if !builder_options.supress_buildrs_directives {
            println!("cargo::rerun-if-changed={}", symlink_path.display());
        }
    }

    if !builder_options.supress_buildrs_directives {
        for file in builder_options.files {
            println!("cargo::rerun-if-changed={}", file.display());
        }
    }
    Ok(())
}

fn generate_symlink<P: AsRef<Path>, Q: AsRef<Path>>(symlink_path: P, output_path: Q) -> Result {
    if symlink_path.as_ref().exists() {
        std::fs::remove_file(&symlink_path).map_err(Error::SymlinkCreationFailure)?;
    }
    std::os::unix::fs::symlink(output_path, symlink_path).map_err(Error::SymlinkCreationFailure)?;
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
