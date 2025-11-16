use std::{fmt::Display, str::FromStr};

use anyhow::{Context, Result, anyhow};

use crate::{
    download::download_and_decompress,
    make::{run_configure_in, run_make_in},
    profile::Toolchain,
};

/// Download and build binutils.
pub fn install_binutils(toolchain: &Toolchain, jobs: u64) -> Result<()> {
    log::info!("=> install binutils {}", toolchain.binutils.version);

    let binutils_dir = download_and_decompress(
        format!(
            "https://ftp.gnu.org/gnu/binutils/binutils-{}.tar.xz",
            toolchain.binutils.version
        ),
        format!("binutils-{}", toolchain.binutils.version),
        true,
    )
    .context("failed to download binutils")?;

    let arch_dir = binutils_dir.join(format!("objdir-arch-{}", toolchain.target));

    std::fs::create_dir_all(&arch_dir).context("failed to create an objdir for the arch")?;

    run_configure_in(
        &arch_dir,
        &[
            "--target",
            toolchain.target.to_target_string().as_str(),
            "--prefix",
            toolchain
                .dir()?
                .to_str()
                .expect("toolchain dir is a valid UTF8 string"),
            "--disable-nls",
            "--disable-werror",
        ],
    )?;
    let jobs = jobs.to_string();
    run_make_in(&arch_dir, &["-j", jobs.as_str()])?;
    run_make_in(&arch_dir, &["install", "-j", jobs.as_str()])?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BinutilsVersion {
    major: u64,
    minor: u64,
    patch: u64,
}

impl FromStr for BinutilsVersion {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split(".").collect();

        fn parse_part(s: &str) -> anyhow::Result<u64> {
            s.parse().context(format!("`{}` is not a number", s))
        }

        match parts.as_slice() {
            [major, minor, patch] => Ok(BinutilsVersion {
                major: parse_part(major)?,
                minor: parse_part(minor)?,
                patch: parse_part(patch)?,
            }),
            [major, minor] => Ok(BinutilsVersion {
                major: parse_part(major)?,
                minor: parse_part(minor)?,
                patch: 0,
            }),
            _ => Err(anyhow!("`{}` is an invalid version", s)),
        }
    }
}

impl Display for BinutilsVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.patch == 0 {
            write!(f, "{}.{}", self.major, self.minor)
        } else {
            write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
        }
    }
}

pub struct Binutils {
    pub version: BinutilsVersion,
}

impl Binutils {
    pub fn new(version: BinutilsVersion) -> Self {
        Self { version }
    }
}
