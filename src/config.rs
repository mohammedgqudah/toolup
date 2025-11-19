//! `toolup.toml` parsing and handling.
//!
//! All commands will check the current working directory for a toolup configuration file or fallback to
//! The global configuration file. Local configuration will take precedence over the global configuration, for example
//! The toolchain version specified in the local configuration will be used instead of the version
//! Specified in the global configuration.
//!
//! # Example configuration
//! ```toml
//!  [toolchain.x86_64-unknown-linux-gnu]
//!  gcc = "15.2.0"
//!  binutils = "2.45"
//!  libc = "2.42"
//! ```
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use toml_edit::DocumentMut;

use crate::{
    packages::{
        binutils::{Binutils, BinutilsVersion},
        gcc::{GCC, GCCVersion},
        glibc::GlibcVersion,
        musl::MuslVersion,
    },
    profile::{Libc, Target, Toolchain},
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolchainConfig {
    binutils: String,
    gcc: String,
    libc: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    toolchain: HashMap<String, ToolchainConfig>,
}

impl From<&Toolchain> for ToolchainConfig {
    fn from(value: &Toolchain) -> Self {
        Self {
            binutils: value.binutils.version.to_string(),
            gcc: value.gcc.version.to_string(),
            libc: match value.libc {
                Libc::Musl(musl) => musl.to_string(),
                Libc::Glibc(glibc) => glibc.to_string(),
            },
        }
    }
}

impl ToolchainConfig {
    /// Convert the toolchain configuration from TOML to a `Toolchain`
    fn to_toolchain(self: &ToolchainConfig, target: impl AsRef<str>) -> Result<Toolchain> {
        let target = Target::from_str(target.as_ref())?;
        let binutils = Binutils {
            version: BinutilsVersion::from_str(&self.binutils)?,
        };
        let gcc = GCC {
            version: GCCVersion::from_str(&self.gcc)?,
        };
        let libc = if target.is_musl() {
            Libc::Musl(MuslVersion::from_str(self.libc.as_str())?)
        } else {
            Libc::Glibc(GlibcVersion::from_str(self.libc.as_str())?)
        };
        Ok(Toolchain::new(target.into(), binutils, gcc, libc))
    }
}

/// Load configuration in `filepath`.
pub fn load_config(filepath: impl AsRef<Path>) -> Result<Option<Config>> {
    if !filepath.as_ref().exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&filepath).context(format!(
        "failed to read file at `{}`",
        filepath.as_ref().display()
    ))?;

    Ok(toml::from_str(content.as_str()).context(format!(
        "failed to parse TOML in `{}`",
        filepath.as_ref().display()
    ))?)
}

fn global_config_path() -> PathBuf {
    Path::new(&std::env::var("XDG_CONFIG_HOME").unwrap()).join("toolup.toml")
}

/// Load configuration from the global `toolup.toml`.
fn load_global_config() -> Result<Config> {
    let global_config = global_config_path();

    match load_config(&global_config)? {
        None => {
            let default_config = Config::default();
            std::fs::write(global_config, toml::to_string(&default_config)?)
                .context("failed to write out default global config")?;

            Ok(default_config)
        }
        Some(config) => Ok(config),
    }
}

/// Load configuration `toolup.toml` in the current working directory.
fn load_local_config() -> Result<Option<Config>> {
    load_config(Path::new("toolup.toml"))
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ToolchainConfigResult {
    /// From the local configuration file
    LocalFound(Toolchain),
    /// From the global configuration (already existed)
    GlobalFound(Toolchain),
    /// Toolchain was never configured but a default global configuration was created for it
    GlobalCreated(Toolchain),
}

impl From<ToolchainConfigResult> for Toolchain {
    fn from(value: ToolchainConfigResult) -> Self {
        match value {
            ToolchainConfigResult::LocalFound(t) => t,
            ToolchainConfigResult::GlobalFound(t) => t,
            ToolchainConfigResult::GlobalCreated(t) => t,
        }
    }
}

/// Updates the toolchain configuration for a target in the global configuration. This will
/// preserve comments and the original layout of the file.
fn set_global_toolchain(toolchain: &Toolchain) -> Result<()> {
    let global_config = global_config_path();
    let target = toolchain.target.to_string();

    let toml_str = std::fs::read_to_string(&global_config)
        .context(format!("failed to read `{}`", global_config.display()))?;

    let mut doc: DocumentMut = toml_str.parse().context("failed to parse TOML")?;
    let toolchain_tbl = doc
        .entry("toolchain")
        .or_insert(toml_edit::table())
        .as_table_mut()
        .expect("`toolchain` is a table");

    // do not add an empty [toolchain] header
    toolchain_tbl.set_implicit(true);

    if toolchain_tbl.contains_key(&target) {
        log::debug!("updating the global toolchain for {target}");
    }

    let item = toml_edit::ser::to_document(&ToolchainConfig::from(toolchain))?.into_item();
    toolchain_tbl[&target] = item;

    std::fs::write(&global_config, doc.to_string())
        .context(format!("failed to write to `{}`", global_config.display()))?;

    Ok(())
}

pub enum GlobalToolchain {
    /// Target toolchain was already configured in the global config
    Found(Toolchain),
    /// Target toolchain was initialized to the default
    Created(Toolchain),
}

/// Ensure a global toolchain is configured for `target`.
///
/// If a toolchain is already configured globally, return it.
/// Otherwise, initialize it to the default and return that.
fn ensure_global_toolchain(target_str: impl AsRef<str>) -> Result<GlobalToolchain> {
    let global = load_global_config()?;
    let target = Target::from_str(target_str.as_ref())?;
    let default = Toolchain::target_default(&target);

    Ok(match global.toolchain.get(target_str.as_ref()) {
        Some(existing_config) => {
            GlobalToolchain::Found(existing_config.to_toolchain(target_str.as_ref())?)
        }
        None => {
            // A toolchain for `target` was never configured, edit the file and set a default toolchain for
            // `target`.
            set_global_toolchain(&default)?;
            GlobalToolchain::Created(default.into())
        }
    })
}

/// Returns the toolchain configuration for `target`.
///
/// Precedence:
/// - Local configuration `toolup.toml`
//  - Global configuration
//  - Otherwise, initialize the global configuration with a default toolchain for target.
pub fn resolve_target_toolchain(target: impl AsRef<str>) -> Result<ToolchainConfigResult> {
    let local = load_local_config()?;
    match local {
        None => {
            log::debug!(
                "no `toolup.toml` detected in current directory. Using the global toolchain for `{}`",
                target.as_ref()
            );
        }
        Some(local_config) => {
            if let Some(toolchain_config) = local_config.toolchain.get(target.as_ref()) {
                return Ok(ToolchainConfigResult::LocalFound(
                    toolchain_config.to_toolchain(target.as_ref())?,
                ));
            }
            log::debug!(
                "`toolup.toml` doesn't specify a toolchain for target `{}`. Using the global toolchain",
                target.as_ref()
            );
        }
    };

    // fallback to global configuration
    Ok(match ensure_global_toolchain(target)? {
        GlobalToolchain::Found(toolchain) => ToolchainConfigResult::GlobalFound(toolchain),
        GlobalToolchain::Created(toolchain) => ToolchainConfigResult::GlobalCreated(toolchain),
    })
}
