use std::{path::PathBuf, process::Command};

use anyhow::{Context, Result};

use crate::{
    download::{cross_prefix, download_and_decompress},
    gcc::Sysroot,
    make::{run_configure_with_env_in, run_make_with_env_in},
    profile::Target,
};

pub fn download_glibc(version: impl AsRef<str>) -> Result<PathBuf> {
    println!("=> download glibc");
    let version = version.as_ref();
    let tarball = format!("glibc-{version}.tar.xz");
    let url = format!(
        "https://ftp.gnu.org/gnu/glibc/{tarball}",
        tarball = &tarball
    );

    let glibc_dir = download_and_decompress(&url, format!("glibc-{version}"), true)
        .context(format!("failed to download {tarball}"))?;

    Ok(glibc_dir)
}

pub fn install_glibc_sysroot(target: &Target, sysroot: Sysroot) -> Result<()> {
    println!("=> install glibc");

    let glibc_dir = download_glibc("2.42")?;
    let objdir = glibc_dir.join(format!("objdir-arch-{}", target.to_string()));
    std::fs::create_dir_all(&objdir)?;

    let stdout = Command::new(glibc_dir.join("scripts").join("config.guess"))
        .output()?
        .stdout;
    let guess = String::from_utf8(stdout)?;

    let args = vec![
        format!("--host={}", target.to_string()),
        format!("--build={}", guess.trim()),
        "--prefix=/usr".into(),
        format!("--with-headers={}/usr/include", sysroot.display()),
        format!("--with-sysroot={}", sysroot.display()),
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
