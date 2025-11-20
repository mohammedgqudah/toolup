use std::str::FromStr;

use anyhow::Result;
use serial_test::serial;
use toolup::{
    config::ToolchainConfigResult,
    packages::{
        binutils::{Binutils, BinutilsVersion},
        gcc::{GCC, GCCVersion},
        glibc::GlibcVersion,
    },
    profile::{Libc, Target, Toolchain},
};

fn test_config_dir() -> tempfile::TempDir {
    let test_home = tempfile::TempDir::new().expect("failed to create temp dir");
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", test_home.path());
    };
    test_home
}

#[test]
#[serial]
fn test_global_default_target_toolchain() -> Result<()> {
    let test_config = test_config_dir();
    let global_config = test_config.path().join("toolup.toml");

    // it should create a `toolup.toml` file and initialize it with a default toolchain for the
    // target.
    let toolchain = toolup::config::resolve_target_toolchain("aarch64-unknown-linux-gnu")?;
    assert!(matches!(toolchain, ToolchainConfigResult::GlobalCreated(_)));
    let toolchain = toolup::config::resolve_target_toolchain("aarch64-unknown-linux-gnu")?;
    assert!(matches!(toolchain, ToolchainConfigResult::GlobalFound(_)));
    assert!(global_config.exists());

    let expected = toml::toml! {
        [toolchain.aarch64-unknown-linux-gnu]
        gcc = "15.2.0"
        binutils = "2.45"
        libc = "2.42"
    };

    similar_asserts::assert_eq!(
        expected.to_string(),
        std::fs::read_to_string(&global_config)?
    );

    // `toolup.toml` exists but this target is not configured, it should initialize it without
    // affecting existing toolchains.
    toolup::config::resolve_target_toolchain("x86_64-unknown-linux-gnu")?;

    let expected = toml::toml! {
        [toolchain.aarch64-unknown-linux-gnu]
        gcc = "15.2.0"
        binutils = "2.45"
        libc = "2.42"

        [toolchain.x86_64-unknown-linux-gnu]
        gcc = "15.2.0"
        binutils = "2.45"
        libc = "2.42"
    };

    similar_asserts::assert_eq!(
        expected.to_string(),
        std::fs::read_to_string(&global_config)?
    );

    Ok(())
}

#[test]
#[serial]
fn test_local_takes_precedence_over_global() -> Result<()> {
    let test_config = test_config_dir();
    let global_config = test_config.path().join("toolup.toml");

    let working_dir = tempfile::TempDir::new().expect("failed to create temp dir");
    let local_config = working_dir.path().join("toolup.toml");
    std::env::set_current_dir(working_dir.path())?;

    let global = toml::toml! {
        [toolchain.aarch64-unknown-linux-gnu]
        gcc = "13.2.0"
        binutils = "2.45"
        libc = "2.42"
    };
    std::fs::write(&global_config, global.to_string())?;

    let local = toml::toml! {
        [toolchain.aarch64-unknown-linux-gnu]
        gcc = "15.2.0"
        binutils = "2.20"
        libc = "2.10"
    };
    std::fs::write(&local_config, local.to_string())?;

    let toolchain = toolup::config::resolve_target_toolchain("aarch64-unknown-linux-gnu")?;
    let target = Target::from_str("aarch64-unknown-linux-gnu")?;
    let binutils = Binutils::new(BinutilsVersion(2, 20, 0));
    let gcc = GCC::new(GCCVersion(15, 2, 0));
    let libc = Libc::Glibc(GlibcVersion(2, 10, 0));

    similar_asserts::assert_eq!(
        ToolchainConfigResult::LocalFound(Toolchain::new(target, binutils, gcc, libc)),
        toolchain
    );
    Ok(())
}
