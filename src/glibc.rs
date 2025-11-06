use std::{path::PathBuf, process::Command};

use anyhow::{Context, Result};

use crate::{
    download::{
        DownloadResult::{Cached, Created, Replaced},
        cache_dir, cross_prefix, decompress_tar_xz, download,
    },
    gcc::Sysroot,
    make::{run_configure_with_env_in, run_make_with_env_in},
    profile::Profile,
};

pub fn download_glibc(version: impl AsRef<str>) -> Result<PathBuf> {
    println!("=> download glibc");
    let version = version.as_ref();
    let tarball = format!("glibc-{version}.tar.xz");
    let url = format!(
        "https://ftp.gnu.org/gnu/glibc/{tarball}",
        tarball = &tarball
    );
    let glibc_dir = cache_dir()?.join(format!("glibc-{version}"));

    // download the glibc source if the tarball doesn't exist.
    if !glibc_dir.exists() {
        let download_result =
            download(&url, &tarball, true).context(format!("failed to download {tarball}"))?;

        let path = match download_result {
            Replaced(p) | Created(p) | Cached(p) => p,
        };

        decompress_tar_xz(path, cache_dir()?)?;
    }

    Ok(glibc_dir)
}

pub fn install_glibc_sysroot(
    sysroot: Sysroot,
    profile: Profile,
    architecture: impl AsRef<str>,
) -> Result<()> {
    println!("=> install glibc");

    let architecture = architecture.as_ref();
    let glibc_dir = download_glibc("2.42")?;
    let objdir = glibc_dir.join(format!(
        "objdir-arch-{arch}-{profile:#?}",
        arch = architecture,
        profile = profile
    ));
    std::fs::create_dir_all(&objdir)?;

    let stdout = Command::new(glibc_dir.join("scripts").join("config.guess"))
        .output()?
        .stdout;
    let guess = String::from_utf8(stdout)?;

    let libdir = if architecture.contains("x86_64")
        || architecture.contains("ppc64")
        || architecture.contains("s390x")
    {
        "/usr/lib64"
    } else {
        "/usr/lib"
    };

    let args = vec![
        format!("--host={}", architecture),
        format!("--build={}", guess.trim()),
        "--prefix=/usr".into(),
        format!("--with-headers={}/usr/include", sysroot.display()),
        "--disable-werror".into(),
        format!("--libdir={}", libdir),
    ];
    let env: Vec<(String, String)> = vec![
        ("BUILD_CC".into(), "gcc".into()),
        ("BUILD_CXX".into(), "g++".into()),
        ("BUILD_AR".into(), "ar".into()),
        ("BUILD_RANLIB".into(), "ranlib".into()),
        ("CC".into(), format!("{architecture}-gcc")),
        ("CXX".into(), format!("{architecture}-g++")),
        ("AR".into(), format!("{architecture}-ar")),
        ("RANLIB".into(), format!("{architecture}-ranlib")),
        ("LD".into(), format!("{architecture}-ld")),
        ("READELF".into(), format!("{architecture}-readelf")),
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
