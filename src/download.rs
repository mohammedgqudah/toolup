use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::{
    fs::{self, File},
    io::{self, BufReader},
    path::{Path, PathBuf},
    time::Duration,
};
use tar::Archive;
use xz2::read::XzDecoder;

pub fn cache_dir() -> Result<PathBuf> {
    let cache =
        PathBuf::from(std::env::var("HOME").context("reading $HOME")?).join(".cache/toolup");
    fs::create_dir_all(&cache).context("creating toolup cache")?;
    Ok(cache)
}

pub fn logs_dir() -> Result<PathBuf> {
    let logs = cache_dir()?.join("logs");
    fs::create_dir_all(&logs).context("creating toolup logs dir")?;
    Ok(logs)
}

pub fn cross_prefix() -> Result<PathBuf> {
    let toolchains = PathBuf::from(std::env::var("HOME").context("reading $HOME")?)
        .join(".toolup")
        .join("toolchains");
    fs::create_dir_all(&toolchains).context("creating .toolup/toolchains")?;
    Ok(toolchains)
}

pub enum DownloadResult {
    /// Replaced the cached file (user requested to not use cache)
    Replaced(PathBuf),
    /// File was downloaded for the first time
    Created(PathBuf),
    /// File was found in cache
    Cached(PathBuf),
}

pub fn archives_dir() -> Result<PathBuf> {
    let dir = cache_dir()?.join("archives");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn sysroots_dir() -> Result<PathBuf> {
    let dir = PathBuf::from(std::env::var("HOME").context("reading $HOME")?)
        .join(".toolup")
        .join("sysroot");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn linux_images_dir() -> Result<PathBuf> {
    let dir = PathBuf::from(std::env::var("HOME").context("reading $HOME")?)
        .join(".toolup")
        .join("linux-images");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Download an archive.
pub fn download_archive<S: AsRef<str>>(url: S, use_cache: bool) -> Result<DownloadResult> {
    let filename = url.as_ref().split("/").last().context(format!(
        "couldn't derive a filename from URL: {}",
        url.as_ref()
    ))?;
    let hash = &blake3::hash(url.as_ref().as_bytes()).to_hex()[..12];
    // prepend the url hash to the filename
    let filename = format!("{hash}-{filename}");

    let url = url.as_ref();
    let file_path = archives_dir()?.join(&filename);
    let cache_exists = file_path.exists();

    if use_cache && cache_exists {
        return Ok(DownloadResult::Cached(file_path));
    }
    let response = reqwest::blocking::Client::builder()
        .user_agent("curl/8.5.0")
        .build()?
        .get(url)
        .send()
        .context(format!("sending GET request to {}", url))?
        .error_for_status()
        .context(format!("non-success status from {}", url))?;

    let style = ProgressStyle::with_template(
        "{msg:.dim} {bar:30.green/dim} {binary_bytes:>7}/{binary_total_bytes:7}",
    )
    .expect("this should be a valid template")
    .progress_chars("--");

    let pb = match response.content_length() {
        Some(size) => ProgressBar::new(size),
        None => ProgressBar::new_spinner(),
    };

    pb.set_style(style);
    pb.set_message(filename.clone());

    let mut download_path = file_path.clone();
    download_path.add_extension("download");

    let mut dest = File::create(&download_path).context(format!("creating {}", filename))?;
    let mut source = pb.wrap_read(response);
    io::copy(&mut source, &mut dest).context(format!("writing {}", filename))?;
    std::fs::rename(&download_path, &file_path).context("moving .download file")?;

    pb.finish();

    if cache_exists {
        Ok(DownloadResult::Replaced(file_path))
    } else {
        Ok(DownloadResult::Created(file_path))
    }
}

pub fn decompress_tar<P: AsRef<Path>, Q: AsRef<Path>>(tar_xz_path: P, dest_dir: Q) -> Result<()> {
    let tar_xz_path = tar_xz_path.as_ref();
    let dest_dir = dest_dir.as_ref();

    fs::create_dir_all(dest_dir).context(format!(
        "creating destination directory {}",
        dest_dir.display()
    ))?;

    let file = File::open(tar_xz_path).context(format!("opening {}", tar_xz_path.display()))?;

    let mp = MultiProgress::new();

    let pb_entry = mp.add(ProgressBar::new_spinner());
    pb_entry.set_style(ProgressStyle::with_template("{spinner:.dim} {msg:.dim}")?);
    pb_entry.enable_steady_tick(Duration::from_millis(100));

    // stream-decompress and extract
    let reader = BufReader::new(file);
    let reader = pb_entry.wrap_read(reader);
    let decoder: Box<dyn std::io::Read> = match tar_xz_path.extension().unwrap().to_str().unwrap() {
        "xz" => Box::new(XzDecoder::new(reader)),
        "gz" => Box::new(GzDecoder::new(reader)),
        "bz2" => Box::new(bzip2::read::BzDecoder::new(reader)),
        _ => unimplemented!(),
    };
    let mut archive = Archive::new(decoder);

    for entry_res in archive.entries().context("reading .tar entries")? {
        let mut entry = entry_res.context("reading a .tar entry")?;
        if let Ok(path) = entry.path() {
            pb_entry.set_message(path.display().to_string());
        }
        entry.unpack_in(dest_dir).context("extracting entry")?;
    }

    pb_entry.finish_and_clear();

    Ok(())
}

/// Returns the extracted directory path.
pub fn download_and_decompress(
    url: impl AsRef<str>,
    dirname: impl AsRef<str>,
    use_cache: bool,
) -> Result<PathBuf> {
    if cache_dir()?.join(dirname.as_ref()).exists() {
        return Ok(cache_dir()?.join(dirname.as_ref()));
    }

    let download_result = download_archive(url, use_cache)?;
    let archive_path = match download_result {
        DownloadResult::Cached(p) => {
            log::debug!("=> using cached {}", dirname.as_ref());
            p
        }
        DownloadResult::Replaced(p) | DownloadResult::Created(p) => p,
    };

    decompress_tar(archive_path, cache_dir()?)?;

    Ok(cache_dir()?.join(dirname.as_ref()))
}
