use std::{ffi::OsString, path::PathBuf};

use anyhow::{Context, Result};

use crate::{commands::run_command_in, download::download_and_decompress, profile::Toolchain};

pub fn download_make(version: impl AsRef<str>) -> Result<PathBuf> {
    log::info!("=> download make {}", version.as_ref());
    let version = version.as_ref();
    let tarball = format!("make-{version}.tar.gz");
    let url = format!("https://ftp.gnu.org/gnu/make/{tarball}", tarball = &tarball);

    let make_dir = download_and_decompress(&url, format!("make-{version}"), true)
        .context(format!("failed to download {tarball}"))?;

    Ok(make_dir)
}

pub fn install_make(version: impl AsRef<str>, toolchain: &Toolchain) -> Result<()> {
    log::info!("=> install make {}", version.as_ref());

    let workdir = download_make(version)?;

    run_command_in(
        &workdir,
        "configure",
        "./configure",
        &[format!("--prefix={}", toolchain.dir()?.display())],
        None::<Vec<(OsString, OsString)>>,
    )?;

    // we can compile Make using the hosts' Make.
    run_command_in(
        &workdir,
        "make",
        "make",
        &["-j10"],
        None::<Vec<(OsString, OsString)>>,
    )?;
    run_command_in(
        &workdir,
        "make",
        "make",
        &["install"],
        None::<Vec<(OsString, OsString)>>,
    )?;

    Ok(())
}
