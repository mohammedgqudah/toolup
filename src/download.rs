use anyhow::{Context, Result};
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

/// download a file to cache
pub fn download<S: AsRef<str>>(url: S, filename: S, use_cache: bool) -> Result<DownloadResult> {
    let filename = filename.as_ref();
    let url = url.as_ref();
    let file_path = cache_dir()?.join(filename);
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

    let total_size = response.content_length().expect("should have a length");

    let style = ProgressStyle::with_template(
        "{msg:.dim} {bar:30.green/dim} {binary_bytes:>7}/{binary_total_bytes:7}",
    )
    .expect("this should be a valid template")
    .progress_chars("--");

    let pb = ProgressBar::new(total_size);
    pb.set_style(style);
    pb.set_message(filename.to_string());

    // TODO: download to a *.download file and move to file_path when download is finished
    let mut dest = File::create(&file_path).context(format!("creating {}", filename))?;
    let mut source = pb.wrap_read(response);
    io::copy(&mut source, &mut dest).context(format!("writing {}", filename))?;

    pb.finish();

    if cache_exists {
        Ok(DownloadResult::Replaced(file_path))
    } else {
        Ok(DownloadResult::Created(file_path))
    }
}

pub fn decompress_tar_xz<P: AsRef<Path>, Q: AsRef<Path>>(
    tar_xz_path: P,
    dest_dir: Q,
) -> Result<()> {
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
    let decoder = XzDecoder::new(reader);
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
