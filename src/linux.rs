use std::{
    fs::OpenOptions,
    io::Read,
    path::PathBuf,
    process::{Command, Stdio},
    str::FromStr,
};

use anyhow::{Context, Result, anyhow};

use crate::{
    download::{download_and_decompress, linux_images_dir},
    make::{run_make_in, run_make_with_env_in},
    profile::{Arch, Target, Toolchain},
};

pub fn download_linux(version: impl AsRef<str>) -> Result<PathBuf> {
    log::info!("=> download linux");

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

pub fn install_headers(toolchain: &Toolchain) -> Result<()> {
    log::info!("=> install linux headers");
    let kernel_src = download_linux("6.17.7")?;

    run_make_in(
        kernel_src,
        &[
            format!("ARCH={}", toolchain.target.arch.to_kernel_arch()).as_str(),
            "headers_install",
            format!("INSTALL_HDR_PATH={}/usr", toolchain.sysroot()?.display()).as_str(),
        ],
    )?;

    Ok(())
}

pub fn config(
    toolchain: &Toolchain,
    workdir: PathBuf,
    out: PathBuf,
    menuconfig: bool,
    use_defconfig: bool,
) -> Result<()> {
    log::info!("=> kernel defconfig");

    let env: Vec<(String, String)> = vec![("PATH".into(), toolchain.env_path()?)];

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
    let defconfig = match toolchain.target.arch {
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
                format!("ARCH={}", toolchain.target.arch.to_kernel_arch()).as_str(),
                "mrproper",
            ],
            env.clone(),
        )?;

        run_make_with_env_in(
            &workdir,
            &[
                format!("ARCH={}", toolchain.target.arch.to_kernel_arch()).as_str(),
                format!("O={}", out.display()).as_str(),
                format!("CROSS_COMPILE={}-", toolchain.target).as_str(),
                defconfig,
            ],
            env.clone(),
        )?;
    }
    if menuconfig {
        Command::new("make")
            .args(&[
                format!("ARCH={}", toolchain.target.arch.to_kernel_arch()).as_str(),
                format!("O={}", out.display()).as_str(),
                format!("CROSS_COMPILE={}-", toolchain.target).as_str(),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct KernelVersion {
    major: u64,
    minor: u64,
    patch: u64,
}

impl FromStr for KernelVersion {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split(".").collect();

        match parts.as_slice() {
            [major, minor] => Ok(KernelVersion {
                major: major.parse().context("invalid version")?,
                minor: minor.parse().context("invalid version")?,
                patch: 0,
            }),
            [major, minor, patch] => Ok(KernelVersion {
                major: major.parse().context("invalid version")?,
                minor: minor.parse().context("invalid version")?,
                patch: patch.parse().context("invalid version")?,
            }),
            _ => Err(anyhow!("")),
        }
    }
}
impl ToString for KernelVersion {
    fn to_string(&self) -> String {
        if self.patch == 0 {
            format!("{}.{}", self.major, self.minor)
        } else {
            format!("{}.{}.{}", self.major, self.minor, self.patch)
        }
    }
}

pub fn build(
    version: impl AsRef<str>,
    toolchain: &Toolchain,
    workdir: PathBuf,
    jobs: u64,
    out: PathBuf,
) -> Result<()> {
    log::info!("=> kerenl build");

    let mut env: Vec<(String, String)> = vec![("PATH".into(), toolchain.env_path()?)];
    let mut args: Vec<String> = vec![
        format!("O={}", out.display()),
        format!("ARCH={}", toolchain.target.arch.to_kernel_arch()),
        format!("CROSS_COMPILE={}-", toolchain.target.to_string()),
        format!("-j{}", jobs),
    ];

    let mut kcflags: Vec<&str> = vec![];
    let kernel_version = KernelVersion::from_str(version.as_ref())?;

    // modify compiler flags to compile old kernels with a newer GCC version.
    if kernel_version <= KernelVersion::from_str("6.14").unwrap() {
        // https://gcc.gnu.org/bugzilla/show_bug.cgi?id=117178
        kcflags.push("-Wno-unterminated-string-initialization");
    }

    // 'bool' is a keyword with '-std=c23' onwards
    if kernel_version <= KernelVersion::from_str("6.13").unwrap() {
        kcflags.push("-std=gnu11");

        args.push("CFLAGS_KERNEL=-std=gnu11".into());
        args.push("CFLAGS_MODULE=-std=gnu11".into());
    }

    if kernel_version <= KernelVersion::from_str("6.2").unwrap() {
        // https://lists.linaro.org/archives/list/linux-stable-mirror%40lists.linaro.org/message/7X43AVMPEXUTTYJFHQLJAV5AMZO7PFB3/
        kcflags.push("-Wno-array-bounds");

        args.push("CFLAGS_KERNEL=-std=gnu11".into());
        args.push("CFLAGS_MODULE=-std=gnu11".into());
    }

    if kernel_version <= KernelVersion::from_str("6.0").unwrap() {
        kcflags.push("-Wno-error=format");
    }

    if kernel_version <= KernelVersion::from_str("5.15").unwrap() {
        kcflags.push("-Wno-use-after-free");
        kcflags.push("-fno-analyzer");
        kcflags.push("-Wno-error=use-after-free");
        args.push("CFLAGS_KERNEL=-std=gnu11 -Wno-error=use-after-free -Wno-use-after-free".into());
        args.push("CFLAGS_MODULE=-std=gnu11 -Wno-error=use-after-free -Wno-use-after-free".into());
        args.push("CFLAGS=-Wno-error=use-after-free -Wno-use-after-free".into());
        args.push("EXTRA_CFLAGS=-Wno-error=use-after-free -Wno-use-after-free".into());
    }

    if kernel_version <= KernelVersion::from_str("5.10").unwrap() {
        // TODO: we need an old binutils for objtool to work
        // TODO: I need to work on supporting multiple versions of the same toolchain.
        // Or, pass CONFIG_STACK_VALIDATION=n, but I need to create a function to programmatically
        // configure the kernel.
    }

    if !kcflags.is_empty() {
        env.push(("KCFLAGS".into(), kcflags.join(" ")));
    }
    run_make_with_env_in(&workdir, &args, env)?;
    Ok(())
}

pub fn build_out(version: impl AsRef<str>, target: &Target) -> Result<PathBuf> {
    Ok(linux_images_dir()?.join(format!("{}-{}", target.to_string(), version.as_ref())))
}

pub fn get_image(
    toolchain: &Toolchain,
    version: impl AsRef<str>,
    jobs: u64,
    menuconfig: bool,
    defconfig: bool,
) -> Result<PathBuf> {
    let _ = toolchain;
    log::info!("=> kernel image");

    let out = build_out(&version, &toolchain.target)?;
    let boot_dir = out
        .join("arch")
        .join(toolchain.target.arch.to_kernel_arch())
        .join("boot");

    let out_image = match toolchain.target.arch {
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
    config(
        &toolchain,
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
        return Ok(toolup_image);
    }

    build(&version, &toolchain, workdir.clone(), jobs, out)?;

    std::fs::copy(out_image, &toolup_image).context("failed to copy kernel image")?;

    Ok(toolup_image)
}
