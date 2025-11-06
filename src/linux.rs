use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::{
    download::{
        DownloadResult::{Cached, Created, Replaced},
        cache_dir, decompress_tar_xz, download,
    },
    make::{run_configure_in, run_make_in},
    profile::kernel_arch,
};

pub fn download_linux(version: impl AsRef<str>) -> Result<PathBuf> {
    println!("=> download linux");

    let version = version.as_ref();
    let major = version.split(".").next().unwrap();
    let tarball = format!("linux-{version}.tar.xz");
    let url = format!(
        "https://cdn.kernel.org/pub/linux/kernel/v{major}.x/{tarball}",
        tarball = &tarball
    );
    let linux_dir = cache_dir()?.join(format!("linux-{version}"));

    // download the linux source if the tarball doesn't exist.
    if !linux_dir.exists() {
        let download_result =
            download(&url, &tarball, true).context(format!("failed to download {tarball}"))?;

        let path = match download_result {
            Replaced(p) | Created(p) | Cached(p) => p,
        };

        decompress_tar_xz(path, cache_dir()?)?;
    }

    Ok(linux_dir)
}

pub fn install_headers(arch: impl AsRef<str>, sysroot: impl AsRef<Path>) -> Result<()> {
    println!("=> install linux headers");
    let kernel_src = download_linux("6.17.7")?;

    run_make_in(
        kernel_src,
        &[
            format!("ARCH={}", kernel_arch(arch.as_ref())).as_str(),
            "headers_install",
            format!("INSTALL_HDR_PATH={}/usr", sysroot.as_ref().display()).as_str(),
        ],
    )?;
    Ok(())
}
