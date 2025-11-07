use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::{
    download::{
        DownloadResult::{Cached, Created, Replaced},
        cache_dir, cross_prefix, decompress_tar_xz, download,
    },
    make::{run_make_in, run_make_with_env_in},
    profile::kernel_arch,
};

pub fn download_linux(version: impl AsRef<str>) -> Result<PathBuf> {
    println!("=> download linux");

    let version = version.as_ref();
    let major = version.split(".").next().unwrap();
    let tarball = format!("linux-{version}.tar.xz");
    let url = format!(
        "https://cdn.kernel.org/pub/linux/kernel/v{major}.x/{tarball}",
        tarball = &tarball
    );
    let linux_dir = cache_dir()?.join(format!("linux-{version}"));

    // download the linux source if the tarball doesn't exist.
    if !linux_dir.exists() {
        let download_result =
            download(&url, &tarball, true).context(format!("failed to download {tarball}"))?;

        let path = match download_result {
            Replaced(p) | Created(p) | Cached(p) => p,
        };

        decompress_tar_xz(path, cache_dir()?)?;
    }

    Ok(linux_dir)
}

pub fn install_headers(arch: impl AsRef<str>, sysroot: impl AsRef<Path>) -> Result<()> {
    println!("=> install linux headers");
    let kernel_src = download_linux("6.17.7")?;

    run_make_in(
        kernel_src,
        &[
            format!("ARCH={}", kernel_arch(arch.as_ref())).as_str(),
            "headers_install",
            format!("INSTALL_HDR_PATH={}/usr", sysroot.as_ref().display()).as_str(),
        ],
    )?;
    Ok(())
}

pub fn defconfig(arch_triple: impl AsRef<str>, workdir: PathBuf, out: PathBuf) -> Result<()> {
    println!("=> kernel defconfig");

    let env: Vec<(String, String)> = vec![(
        "PATH".into(),
        format!(
            "{}:{}",
            cross_prefix()?.join("bin").display(),
            std::env::var("PATH")?
        ),
    )];

    let arch = kernel_arch(arch_triple.as_ref());

    let defconfig = if arch == "x86" {
        "i386_defconfig"
    } else {
        "defconfig"
    };

    run_make_with_env_in(
        &workdir,
        &[format!("ARCH={}", arch).as_str(), "mrproper"],
        env.clone(),
    )?;

    run_make_with_env_in(
        workdir,
        &[
            format!("ARCH={}", arch).as_str(),
            format!("O={}", out.display()).as_str(),
            format!("CROSS_COMPILE={}-", arch_triple.as_ref()).as_str(),
            defconfig,
        ],
        env,
    )?;
    Ok(())
}

pub fn build(arch: impl AsRef<str>, workdir: PathBuf, jobs: u64, out: PathBuf) -> Result<()> {
    println!("=> kerenl build");

    let env: Vec<(String, String)> = vec![(
        "PATH".into(),
        format!(
            "{}:{}",
            cross_prefix()?.join("bin").display(),
            std::env::var("PATH")?
        ),
    )];

    run_make_with_env_in(
        &workdir,
        &[
            format!("O={}", out.display()).as_str(),
            format!("ARCH={}", kernel_arch(arch.as_ref())).as_str(),
            format!("CROSS_COMPILE={}-", arch.as_ref()).as_str(),
            format!("-j{}", jobs).as_str(),
        ],
        env,
    )?;
    Ok(())
}

pub fn build_out(version: impl AsRef<str>, architecture: impl AsRef<str>) -> Result<PathBuf> {
    Ok(cache_dir()?.join("linux-images").join(format!(
        "{}-{}",
        architecture.as_ref(),
        version.as_ref()
    )))
}

pub fn get_image(
    version: impl AsRef<str>,
    architecture: impl AsRef<str>,
    jobs: u64,
) -> Result<PathBuf> {
    println!("=> kernel image");

    let out = build_out(&version, &architecture)?;
    let image = out
        .join("arch")
        .join(kernel_arch(architecture.as_ref()))
        .join("boot");

    let arch = kernel_arch(architecture.as_ref());
    let image = match arch {
        "x86" => image.join("bzImage"),
        "arm" => image.join("zImage"),
        _ => image.join("Image"),
    };

    if image.exists() {
        return Ok(image);
    }

    let workdir = download_linux(&version)?;
    defconfig(&architecture, workdir.clone(), out.clone())?;
    build(&architecture, workdir.clone(), jobs, out)?;

    Ok(image)
}
