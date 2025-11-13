use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::{
    download::{cross_prefix, download_and_decompress},
    gcc::Sysroot,
    make::{run_configure_with_env_in, run_make_with_env_in},
    profile::Target,
};

pub fn download_musl(version: impl AsRef<str>) -> Result<PathBuf> {
    log::info!("=> download musl");
    let version = version.as_ref();
    let tarball = format!("musl-{version}.tar.gz");
    let url = format!(
        "https://musl.libc.org/releases/{tarball}",
        tarball = &tarball
    );

    let musl_dir = download_and_decompress(&url, format!("musl-{version}"), true)
        .context(format!("failed to download {tarball}"))?;

    Ok(musl_dir)
}

pub fn install_musl_sysroot(target: &Target, sysroot: Sysroot) -> Result<()> {
    log::info!("=> install musl");

    let musl_dir = download_musl("1.2.5")?;
    let objdir = musl_dir.join(format!("objdir-arch-{}", target.to_string()));
    std::fs::create_dir_all(&objdir)?;

    let args = vec![
        format!("--host={}", target.to_string()),
        "--prefix=/usr".into(),
        "--syslibdir=/lib".into(),
        "--disable-werror".into(),
    ];
    let prefix = target.to_string();

    let env: Vec<(String, String)> = vec![
        ("BUILD_CC".into(), "gcc".into()),
        ("BUILD_CXX".into(), "g++".into()),
        ("BUILD_AR".into(), "ar".into()),
        ("BUILD_RANLIB".into(), "ranlib".into()),
        ("CC".into(), format!("{prefix}-gcc")),
        ("CXX".into(), format!("{prefix}-g++")),
        ("AR".into(), format!("{prefix}-ar")),
        ("RANLIB".into(), format!("{prefix}-ranlib")),
        ("LD".into(), format!("{prefix}-ld")),
        ("READELF".into(), format!("{prefix}-readelf")),
        (
            "PATH".into(),
            format!(
                "{}:{}",
                cross_prefix()?.join("bin").display(),
                std::env::var("PATH")?
            ),
        ),
    ];
    run_configure_with_env_in(&objdir, &args, env.clone())?;

    run_make_with_env_in(&objdir, &["-j", "28"], env.clone())?;
    run_make_with_env_in(
        &objdir,
        &[
            "install",
            &format!("DESTDIR={}", sysroot.display()),
            "-j",
            "28",
        ],
        env.clone(),
    )?;

    Ok(())
}
