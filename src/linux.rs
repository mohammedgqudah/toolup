use std::{
    fs::OpenOptions,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::{Context, Result};

use crate::{
    download::{cross_prefix, download_and_decompress, linux_images_dir},
    make::{run_make_in, run_make_with_env_in},
    profile::{Arch, Target},
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

pub fn install_headers(target: &Target, sysroot: impl AsRef<Path>) -> Result<()> {
    println!("=> install linux headers");
    let kernel_src = download_linux("6.17.7")?;

    run_make_in(
        kernel_src,
        &[
            format!("ARCH={}", target.arch.to_kernel_arch()).as_str(),
            "headers_install",
            format!("INSTALL_HDR_PATH={}/usr", sysroot.as_ref().display()).as_str(),
        ],
    )?;
    Ok(())
}

pub fn config(
    target: &Target,
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

    //let defconfig = if arch == "x86" {
    //    "i386_defconfig"
    //} else if arch == "mips" {
    //    if arch_triple.as_ref().starts_with("mips64") {
    //        "defconfig"
    //    } else {
    //        "malta_defconfig"
    //    }
    //} else {
    //    "defconfig"
    //};
    let defconfig = match target.arch {
        Arch::I686 => "i386_defconfig",
        _ => "defconfig",
    };

    let force_defconfig = if out.join(".config").exists() {
        false
    } else {
        true
    };

    if use_defconfig || force_defconfig {
        run_make_with_env_in(
            &workdir,
            &[
                format!("ARCH={}", target.arch.to_kernel_arch()).as_str(),
                "mrproper",
            ],
            env.clone(),
        )?;

        run_make_with_env_in(
            &workdir,
            &[
                format!("ARCH={}", target.arch.to_kernel_arch()).as_str(),
                format!("O={}", out.display()).as_str(),
                format!("CROSS_COMPILE={}-", target.to_string()).as_str(),
                defconfig,
            ],
            env.clone(),
        )?;
    }
    if menuconfig {
        Command::new("make")
            .args(&[
                format!("ARCH={}", target.arch.to_kernel_arch()).as_str(),
                format!("O={}", out.display()).as_str(),
                format!("CROSS_COMPILE={}-", target.to_string()).as_str(),
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

pub fn build(target: &Target, workdir: PathBuf, jobs: u64, out: PathBuf) -> Result<()> {
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
            format!("ARCH={}", target.arch.to_kernel_arch()).as_str(),
            format!("CROSS_COMPILE={}-", target.to_string()).as_str(),
            format!("-j{}", jobs).as_str(),
        ],
        env,
    )?;
    Ok(())
}

pub fn build_out(version: impl AsRef<str>, target: &Target) -> Result<PathBuf> {
    Ok(linux_images_dir()?.join(format!("{}-{}", target.to_string(), version.as_ref())))
}

pub fn get_image(
    target: &Target,
    version: impl AsRef<str>,
    jobs: u64,
    menuconfig: bool,
    defconfig: bool,
) -> Result<PathBuf> {
    println!("=> kernel image");

    let out = build_out(&version, target)?;
    let boot_dir = out
        .join("arch")
        .join(target.arch.to_kernel_arch())
        .join("boot");

    let out_image = match target.arch {
        Arch::X86_64 | Arch::I686 => boot_dir.join("bzImage"),
        Arch::Armv7 => boot_dir.join("zImage"),
        Arch::Aarch64 => boot_dir.join("Image"),
        // for mips and ppc, the image is at the top level
        Arch::Ppc64Le | Arch::Ppc64 => boot_dir
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
    config(target, workdir.clone(), out.clone(), menuconfig, defconfig)?;

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
        return Ok(toolup_image);
    }

    build(target, workdir.clone(), jobs, out)?;

    std::fs::copy(out_image, &toolup_image).context("failed to copy kernel image")?;

    Ok(toolup_image)
}
