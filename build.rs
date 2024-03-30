use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::mpsc,
    thread::JoinHandle,
};
use tar::Archive;
use tempfile::TempDir;

const SOURCE_URL: &str =
    "https://github.com/google/flatbuffers/archive/refs/tags/v{version}.tar.gz";
const SUPPORTED_FLATC_VERSION: &str = "23.5.26";
const CHECKSUM_SHA256: &str = "1cce06b17cddd896b6d73cc047e36a254fb8df4d7ea18a46acf16c4c0cd3f3f3";

fn main() {
    #[cfg(feature = "vendored")]
    vendor_flatc().expect("failed to vendor flatc");
}

fn vendor_flatc() -> anyhow::Result<()> {
    let tmpdir = tempfile::tempdir()?;

    let tarball_path = download_source_tarball(&tmpdir)?;
    // Extract the source tarball
    let extract_path = tmpdir.path().join("flatbuffers");
    // let mut ar = Archive::new(File::open(tarball_path)?);
    // ar.unpack(&extract_path)?;
    println!("AAAA we extracted to {}", extract_path.display());
    std::process::exit(1);
    Ok(())
}

fn download_source_tarball<P: AsRef<Path>>(dir: P) -> anyhow::Result<PathBuf> {
    let tarball_path = dir.as_ref().join("flatbuffers.tar.gz");
    let mut file = File::create(&tarball_path)?;
    let mut response = reqwest::blocking::get(get_full_source_url())?;
    response.copy_to(&mut file)?;
    Ok(tarball_path)
}

fn get_full_source_url() -> String {
    SOURCE_URL.replace("{version}", SUPPORTED_FLATC_VERSION)
}
