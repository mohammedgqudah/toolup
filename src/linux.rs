use std::{
    fs::OpenOptions,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{Context, Result};

use crate::{
    download::{cache_dir, cross_prefix, download_and_decompress},
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

    let linux_dir = download_and_decompress(&url, format!("linux-{version}"), true)
        .context(format!("failed to download {tarball}"))?;

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

pub fn config(
    arch_triple: impl AsRef<str>,
    workdir: PathBuf,
    out: PathBuf,
    menuconfig: bool,
    use_defconfig: bool,
) -> Result<()> {
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
    } else if arch == "mips" {
        if arch_triple.as_ref().starts_with("mips64") {
            "defconfig"
        } else {
            "malta_defconfig"
        }
    } else {
        "defconfig"
    };

    if use_defconfig {
        run_make_with_env_in(
            &workdir,
            &[format!("ARCH={}", arch).as_str(), "mrproper"],
            env.clone(),
        )?;

        run_make_with_env_in(
            &workdir,
            &[
                format!("ARCH={}", arch).as_str(),
                format!("O={}", out.display()).as_str(),
                format!("CROSS_COMPILE={}-", arch_triple.as_ref()).as_str(),
                defconfig,
            ],
            env.clone(),
        )?;
    }
    if menuconfig {
        Command::new("make")
            .args(&[
                format!("ARCH={}", arch).as_str(),
                format!("O={}", out.display()).as_str(),
                format!("CROSS_COMPILE={}-", arch_triple.as_ref()).as_str(),
                "menuconfig",
            ])
            .current_dir(workdir)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .context("running menuconfig")?;
    }
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
    menuconfig: bool,
    defconfig: bool,
) -> Result<PathBuf> {
    println!("=> kernel image");

    let out = build_out(&version, &architecture)?;
    let boot_dir = out
        .join("arch")
        .join(kernel_arch(architecture.as_ref()))
        .join("boot");

    let arch = kernel_arch(architecture.as_ref());
    let out_image = match arch {
        "x86" => boot_dir.join("bzImage"),
        "arm" => boot_dir.join("zImage"),
        // for mips, the image is at the top level
        "mips" | "powerpc" => boot_dir
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("vmlinux"),
        _ => boot_dir.join("Image"),
    };

    let workdir = download_linux(&version)?;
    config(
        &architecture,
        workdir.clone(),
        out.clone(),
        menuconfig,
        defconfig,
    )?;

    let mut config_file = OpenOptions::new()
        .read(true)
        .open(out.join(".config"))
        .context("failed to open config file")?;
    let mut config_buf: Vec<u8> = Vec::new();
    config_file.read_to_end(&mut config_buf)?;

    let config_hash = blake3::hash(config_buf.as_slice()).to_hex();

    let mut toolup_image = out_image.clone();
    toolup_image.add_extension(config_hash.to_string());

    if toolup_image.exists() {
        return Ok(out_image);
    }

    build(&architecture, workdir.clone(), jobs, out)?;

    std::fs::copy(out_image, &toolup_image).context("failed to copy kernel image")?;

    Ok(toolup_image)
}
