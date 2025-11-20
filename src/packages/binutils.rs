use std::{fmt::Display, str::FromStr};

use anyhow::{Context, Result, anyhow};

use crate::{
    commands::{run_configure_in, run_make_in},
    download::download_and_decompress,
    profile::Toolchain,
};

/// Download and build binutils.
pub fn install_binutils(toolchain: &Toolchain, jobs: u64) -> Result<()> {
    log::info!("=> install binutils {}", toolchain.binutils.version);

    let tarball = if toolchain.binutils.version <= BinutilsVersion(2, 28, 1) {
        format!("{}.tar.gz", toolchain.binutils.version)
    } else {
        format!("{}.tar.xz", toolchain.binutils.version)
    };

    let binutils_dir = download_and_decompress(
        format!("https://ftp.gnu.org/gnu/binutils/binutils-{tarball}",),
        format!("binutils-{}", toolchain.binutils.version),
        true,
    )
    .context("failed to download binutils")?;

    let arch_dir = binutils_dir.join(format!("objdir-arch-{}", toolchain.id()));

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
pub struct BinutilsVersion(pub u64, pub u64, pub u64);

impl FromStr for BinutilsVersion {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split(".").collect();

        fn parse_part(s: &str) -> anyhow::Result<u64> {
            s.parse().context(format!("`{}` is not a number", s))
        }

        match parts.as_slice() {
            [major, minor, patch] => Ok(BinutilsVersion(
                parse_part(major)?,
                parse_part(minor)?,
                parse_part(patch)?,
            )),
            [major, minor] => Ok(BinutilsVersion(parse_part(major)?, parse_part(minor)?, 0)),
            _ => Err(anyhow!("`{}` is an invalid binutils version", s)),
        }
    }
}

impl Display for BinutilsVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.2 == 0 {
            write!(f, "{}.{}", self.0, self.1)
        } else {
            write!(f, "{}.{}.{}", self.0, self.1, self.2)
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct Binutils {
    pub version: BinutilsVersion,
}

impl Binutils {
    pub fn new(version: BinutilsVersion) -> Self {
        Self { version }
    }
}
impl Default for Binutils {
    fn default() -> Self {
        Self {
            version: BinutilsVersion(2, 45, 0),
        }
    }
}
