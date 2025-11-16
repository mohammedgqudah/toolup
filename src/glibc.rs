use std::{fmt::Display, path::PathBuf, process::Command, str::FromStr};

use anyhow::{Context, Result, anyhow};

use crate::{
    download::download_and_decompress,
    gnu_make::install_make,
    make::{run_configure_with_env_in, run_make_with_env_in},
    profile::{Libc, Toolchain},
};

pub fn download_glibc(version: impl AsRef<str>) -> Result<PathBuf> {
    log::info!("=> download glibc");
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

pub fn install_glibc_sysroot(toolchain: &Toolchain) -> Result<()> {
    log::info!("=> install glibc");

    let Libc::Glibc(glibc_version) = toolchain.libc else {
        return Err(anyhow!(
            "`install_glibc_sysroot` called with a musl toolchain"
        ));
    };

    // workaround: we need an old Make version to compile this glibc version.
    // see: https://stackoverflow.com/a/77107152/8701101
    if glibc_version <= GlibcVersion::from_str("2.30").unwrap() {
        install_make("4.3", toolchain)?;
    }

    let glibc_dir = download_glibc(glibc_version.to_string())?;
    let objdir = glibc_dir.join(format!("objdir-arch-{}", toolchain.id()));
    std::fs::create_dir_all(&objdir)?;

    let stdout = Command::new(glibc_dir.join("scripts").join("config.guess"))
        .output()?
        .stdout;
    let guess = String::from_utf8(stdout)?;

    let args = vec![
        format!("--host={}", toolchain.target),
        format!("--build={}", guess.trim()),
        "--prefix=/usr".into(),
        format!(
            "--with-headers={}/usr/include",
            toolchain.sysroot()?.display()
        ),
        format!("--with-sysroot={}", toolchain.sysroot()?.display()),
        "--disable-werror".into(),
    ];
    let prefix = toolchain.target;

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
        ("PATH".into(), toolchain.env_path()?),
    ];
    run_configure_with_env_in(&objdir, &args, env.clone())?;

    run_make_with_env_in(&objdir, &["-j", "28"], env.clone())?;
    run_make_with_env_in(
        &objdir,
        &[
            "install",
            &format!("DESTDIR={}", toolchain.sysroot()?.display()),
            "-j",
            "28",
        ],
        env.clone(),
    )?;

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GlibcVersion {
    major: u64,
    minor: u64,
    patch: u64,
}

impl FromStr for GlibcVersion {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split(".").collect();

        fn parse_part(s: &str) -> anyhow::Result<u64> {
            s.parse().context(format!("`{}` is not a number", s))
        }

        match parts.as_slice() {
            [major, minor, patch] => Ok(GlibcVersion {
                major: parse_part(major)?,
                minor: parse_part(minor)?,
                patch: parse_part(patch)?,
            }),
            [major, minor] => Ok(GlibcVersion {
                major: parse_part(major)?,
                minor: parse_part(minor)?,
                patch: 0,
            }),
            _ => Err(anyhow!("`{}` is an invalid version", s)),
        }
    }
}

impl Display for GlibcVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 2.16.0 is the only version that has a `.0` in the FTP server
        if (self.patch == 0) && (self.major, self.minor) != (2, 16) {
            write!(f, "{}.{}", self.major, self.minor)
        } else {
            write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
        }
    }
}
