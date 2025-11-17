use std::{ffi::OsString, fmt::Display, path::PathBuf, str::FromStr};

use anyhow::{Context, Result, anyhow};

use crate::{
    commands::run_command_in,
    download::download_and_decompress,
    profile::{Libc, Toolchain},
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

pub fn install_musl_sysroot(toolchain: &Toolchain) -> Result<()> {
    log::info!("=> install musl");

    let Libc::Musl(musl_version) = toolchain.libc else {
        return Err(anyhow!(
            "`install_musl_sysroot` called with a glibc toolchain"
        ));
    };

    let musl_dir = download_musl(musl_version.to_string())?;
    let objdir = musl_dir.join(format!("objdir-arch-{}", toolchain.id()));
    std::fs::create_dir_all(&objdir)?;

    let args = vec![
        format!("--host={}", toolchain.target),
        "--prefix=/usr".into(),
        "--syslibdir=/lib".into(),
        "--disable-werror".into(),
    ];
    let prefix = toolchain.target;

    let env: Vec<(OsString, OsString)> = vec![
        ("BUILD_CC".into(), "gcc".into()),
        ("BUILD_CXX".into(), "g++".into()),
        ("BUILD_AR".into(), "ar".into()),
        ("BUILD_RANLIB".into(), "ranlib".into()),
        ("CC".into(), format!("{prefix}-gcc").into()),
        ("CXX".into(), format!("{prefix}-g++").into()),
        ("AR".into(), format!("{prefix}-ar").into()),
        ("RANLIB".into(), format!("{prefix}-ranlib").into()),
        ("LD".into(), format!("{prefix}-ld").into()),
        ("READELF".into(), format!("{prefix}-readelf").into()),
        ("PATH".into(), toolchain.env_path()?),
    ];
    run_command_in(
        &objdir,
        "configure",
        objdir.parent().unwrap().join("configure"),
        &args,
        Some(env.clone()),
    )?;

    run_command_in(&objdir, "make", "make", &["-j", "28"], Some(env.clone()))?;
    run_command_in(
        &objdir,
        "make",
        "make",
        &[
            "install",
            &format!("DESTDIR={}", toolchain.sysroot()?.display()),
            "-j",
            "28",
        ],
        Some(env.clone()),
    )?;

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MuslVersion(u64, u64, u64);

impl FromStr for MuslVersion {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split(".").collect();

        fn parse_part(s: &str) -> anyhow::Result<u64> {
            s.parse().context(format!("`{}` is not a number", s))
        }

        match parts.as_slice() {
            [major, minor, patch] => Ok(MuslVersion(
                parse_part(major)?,
                parse_part(minor)?,
                parse_part(patch)?,
            )),
            _ => Err(anyhow!("`{}` is an invalid musl version", s)),
        }
    }
}

impl Display for MuslVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.0, self.1, self.2)
    }
}
