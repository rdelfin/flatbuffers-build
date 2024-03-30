use flate2::read::GzDecoder;
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
const EXTRACT_DIRECTORY_PREFIX: &str = "flatbuffers-{version}";

fn main() {
    #[cfg(feature = "vendored")]
    vendor_flatc().expect("failed to vendor flatc");
}

fn vendor_flatc() -> anyhow::Result<()> {
    let tmpdir = tempfile::tempdir()?;

    let tarball_path = download_source_tarball(&tmpdir)?;
    // Extract the source tarball
    let extract_path = tmpdir.path().join("flatbuffers");
    unpack_tarball(tarball_path, &extract_path)?;
    Ok(())
}

fn download_source_tarball<P: AsRef<Path>>(dir: P) -> anyhow::Result<PathBuf> {
    let tarball_path = dir.as_ref().join("flatbuffers.tar.gz");
    let mut file = File::create(&tarball_path)?;
    let mut response = reqwest::blocking::get(get_full_source_url())?;
    response.copy_to(&mut file)?;
    Ok(tarball_path)
}

fn unpack_tarball<P: AsRef<Path>, Q: AsRef<Path>>(
    tarball_path: P,
    extraction_path: Q,
) -> anyhow::Result<()> {
    let tar_gz = File::open(tarball_path)?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    archive.unpack(extraction_path)?;
    Ok(())
}

fn get_full_source_url() -> String {
    SOURCE_URL.replace("{version}", SUPPORTED_FLATC_VERSION)
}
