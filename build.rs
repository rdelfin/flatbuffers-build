#![warn(clippy::all, clippy::pedantic)]

fn main() {
    #[cfg(feature = "vendored")]
    vendored::vendor_flatc().expect("failed to vendor flatc");
}

#[cfg(feature = "vendored")]
mod vendored {
    use flate2::read::GzDecoder;
    use ring::digest::{Context, SHA256};
    use std::{
        fs::File,
        io::{BufReader, Read},
        path::{Path, PathBuf},
    };
    use tar::Archive;

    const SOURCE_URL: &str =
        "https://github.com/google/flatbuffers/archive/refs/tags/v{version}.tar.gz";
    const SUPPORTED_FLATC_VERSION: &str = "25.2.10";
    const CHECKSUM_SHA256: &str =
        "b9c2df49707c57a48fc0923d52b8c73beb72d675f9d44b2211e4569be40a7421";
    const EXTRACT_DIRECTORY_PREFIX: &str = "flatbuffers-{version}";

    pub fn vendor_flatc() -> anyhow::Result<()> {
        let tmpdir = tempfile::tempdir()?;

        let tarball_path = download_source_tarball(&tmpdir)?;
        checksum_check(&tarball_path, CHECKSUM_SHA256)?;

        // Extract the source tarball
        let extract_path = tmpdir.path().join("flatbuffers");
        unpack_tarball(tarball_path, &extract_path)?;

        let source_dir = extract_path
            .join(EXTRACT_DIRECTORY_PREFIX.replace("{version}", SUPPORTED_FLATC_VERSION));
        let dest = compile_flatc(source_dir);
        let flatc_path = dest.join("bin/flatc");
        println!("cargo::rustc-env=FLATC_PATH={}", flatc_path.display());
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

    fn checksum_check<P: AsRef<Path>>(file_path: P, expected_checksum: &str) -> anyhow::Result<()> {
        let mut digester = Context::new(&SHA256);
        let mut file = File::open(file_path)?;
        let mut reader = BufReader::new(&mut file);
        let mut buffer = [0u8; 4096];
        loop {
            let byte_count = reader.read(&mut buffer)?;
            if byte_count == 0 {
                break;
            }
            digester.update(&buffer[..byte_count]);
        }
        let digest = digester.finish();
        let digest_str = hex::encode(digest.as_ref());
        if digest_str == expected_checksum {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "checskum for file did not match; expected {}, got {}",
                expected_checksum,
                digest_str
            ))
        }
    }

    fn compile_flatc<P: AsRef<Path>>(source_dir: P) -> PathBuf {
        cmake::build(source_dir)
    }

    fn get_full_source_url() -> String {
        SOURCE_URL.replace("{version}", SUPPORTED_FLATC_VERSION)
    }
}
