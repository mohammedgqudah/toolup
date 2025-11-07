use anyhow::Result;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::{fs::OpenOptions, path::PathBuf};

use crate::cpio::pack_rootfs;
use crate::download::cache_dir;
use crate::download::cross_prefix;
use crate::make::run_make_with_env_in;

pub fn download_busybox() -> Result<PathBuf> {
    // TODO: decompress bz2 https://busybox.net/downloads/busybox-1.36.1.tar.bz2
    Ok(cache_dir()?.join("busybox-1.36.1"))
}

/// Returns rootfs image
pub fn build_rootfs(architecture: impl AsRef<str>) -> Result<PathBuf> {
    let busybox_dir = download_busybox()?;
    let rootfs_dir = cache_dir()?.join(format!("rootfs-{}", architecture.as_ref()));
    let cpio_gz = cache_dir()?.join(format!("rootfs-{}.cpio.gz", architecture.as_ref()));
    if cpio_gz.exists() {
        return Ok(cpio_gz);
    }

    println!("=> busybox");

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

    let env: Vec<(String, String)> = vec![(
        "PATH".into(),
        format!(
            "{}:{}",
            cross_prefix()?.join("bin").display(),
            std::env::var("PATH")?
        ),
    )];

    run_make_with_env_in(
        &busybox_dir,
        &[
            format!("CROSS_COMPILE={}-", architecture.as_ref()).as_str(),
            "defconfig",
        ],
        env.clone(),
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

    run_make_with_env_in(
        &busybox_dir,
        &[
            format!("CROSS_COMPILE={}-", architecture.as_ref()).as_str(),
            format!("CONFIG_PREFIX={}", &rootfs_dir.display()).as_str(),
            "install",
        ],
        env,
    )?;

    pack_rootfs(&rootfs_dir, &cpio_gz)?;

    Ok(cpio_gz)
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
