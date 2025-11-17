use anyhow::{Context, Result};
use std::ffi::OsString;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::{fs::OpenOptions, path::PathBuf};

use crate::commands::run_command_in;
use crate::cpio::pack_rootfs;
use crate::download::cache_dir;
use crate::download::download_and_decompress;
use crate::profile::Toolchain;

pub fn download_busybox() -> Result<PathBuf> {
    log::info!("=> downloading busybox");

    // using the github mirror because busybox.net is super slow and times out most of the time.
    download_and_decompress(
        "https://github.com/mirror/busybox/archive/refs/tags/1_36_1.tar.gz",
        "busybox-1_36_1",
        true,
    )
}

/// Returns rootfs image
pub fn build_rootfs(toolchain: &Toolchain) -> Result<PathBuf> {
    let busybox_dir = download_busybox()?;
    let rootfs_dir = cache_dir()?.join(format!("rootfs-{}", toolchain.target));
    let cpio_gz = cache_dir()?.join(format!("rootfs-{}.cpio.gz", toolchain.target));
    if cpio_gz.exists() {
        return Ok(cpio_gz);
    }

    log::info!("=> busybox");

    std::fs::create_dir_all(&rootfs_dir)?;
    std::fs::create_dir_all(&rootfs_dir.join("proc"))?;
    std::fs::create_dir_all(&rootfs_dir.join("sys"))?;
    std::fs::create_dir_all(&rootfs_dir.join("dev"))?;
    std::fs::create_dir_all(&rootfs_dir.join("etc"))?;

    let init_script = r"#!/bin/sh
mount -t proc proc /proc
mount -t sysfs sysfs /sys
mount -t devtmpfs devtmpfs /dev 2>/dev/null || mount -t tmpfs tmpfs /dev
[ -c /dev/console ] || mknod -m 600 /dev/console c 5 1
exec setsid cttyhack /bin/sh
";
    let mut init = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o755)
        .open(rootfs_dir.join("init"))
        .unwrap();
    init.write_all(init_script.as_bytes())?;

    let env: Vec<(OsString, OsString)> = vec![("PATH".into(), toolchain.env_path()?)];

    run_command_in(
        &busybox_dir,
        "make",
        "make",
        &[
            format!("CROSS_COMPILE={}-", toolchain.target).as_str(),
            "defconfig",
        ],
        Some(env.clone()),
    )?;
    fix_busybox_config(busybox_dir.join(".config"))?;
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(busybox_dir.join(".config"))
        .unwrap();

    // static build
    writeln!(f, "CONFIG_STATIC=y").unwrap();
    // workaround: https://forum.beagleboard.org/t/errors-during-busybox-compilation/38969/6
    writeln!(f, "# CONFIG_TC is not set").unwrap();

    run_command_in(
        &busybox_dir,
        "make",
        "make",
        &[
            format!("CROSS_COMPILE={}-", toolchain.target).as_str(),
            format!("CONFIG_PREFIX={}", &rootfs_dir.display()).as_str(),
            "install",
        ],
        Some(env.clone()),
    )?;

    let sysroot = toolchain.sysroot()?;

    if sysroot.join("lib").exists() {
        copy_dir_to(&sysroot.join("lib"), &rootfs_dir).context("copying sysroot/lib")?;
    }
    if sysroot.join("lib64").exists() {
        copy_dir_to(&sysroot.join("lib64"), &rootfs_dir).context("copying sysroot/lib64")?;
    }

    copy_dir_to(&sysroot.join("usr"), &rootfs_dir)?;

    log::info!("=> packing");
    pack_rootfs(&rootfs_dir, &cpio_gz)?;

    Ok(cpio_gz)
}

/// Copy directory into another one.
///
/// This is a naive implementation that doesn't take cyclic symlinks or other edge cases into
/// account. Only use for copying sysroot to rootfs.
fn copy_dir_to<P: AsRef<Path>>(src: P, target_root: P) -> Result<()> {
    let src = src.as_ref();
    let target_root = target_root.as_ref();

    let target_dir = target_root.join(src.file_name().unwrap());
    std::fs::create_dir_all(&target_dir)?;

    // Recursively walk through all entries
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let target_path = target_dir.join(entry.file_name());

        if path.is_dir() {
            copy_dir_to(&path, &target_dir)?;
        } else {
            std::fs::copy(&path, &target_path)?;
        }
    }

    Ok(())
}

pub fn fix_busybox_config(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    let contents = std::fs::read_to_string(path)?;

    let mut out = String::new();
    for line in contents.lines() {
        // remove any previous STATIC setting
        if line.starts_with("CONFIG_STATIC=") || line == "# CONFIG_STATIC is not set" {
            continue;
        }
        // remove any TC setting
        if line.starts_with("CONFIG_TC=") || line == "# CONFIG_TC is not set" {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }

    out.push_str("CONFIG_STATIC=y\n");
    out.push_str("# CONFIG_TC is not set\n");

    std::fs::write(path, out)?;

    Ok(())
}
