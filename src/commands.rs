use std::{
    ffi::OsStr,
    fs::File,
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::{Context, Result, bail};
use chrono::{Local, SecondsFormat};
use indicatif::{ProgressBar, ProgressStyle};

use crate::download::logs_dir;

pub fn log_filename(id: impl AsRef<str>) -> String {
    let ts = Local::now()
        .to_rfc3339_opts(SecondsFormat::Millis, true)
        .replace(':', "-");

    format!("{}-{}.log", id.as_ref(), ts)
}

pub fn run_make_in<P: AsRef<Path>>(workdir: P, args: &[&str]) -> Result<()> {
    _run_make_in(workdir, args, None)
}

pub fn _run_make_in<P: AsRef<Path>>(
    workdir: P,
    args: &[impl AsRef<OsStr>],
    env: Option<Vec<(String, String)>>,
) -> Result<()> {
    run_command_in(workdir, "make", "make", args, env)
}

pub fn run_configure_in<P: AsRef<Path>, S: AsRef<OsStr>>(workdir: P, args: &[S]) -> Result<()> {
    _run_configure_in(workdir, args, None)
}

pub fn _run_configure_in<P: AsRef<Path>, S: AsRef<OsStr>>(
    workdir: P,
    args: &[S],
    env: Option<Vec<(String, String)>>,
) -> Result<()> {
    run_command_in(
        &workdir,
        "configure",
        workdir.as_ref().parent().unwrap().join("configure"),
        args,
        env,
    )
}

/// Run a command in directory and show output in a spinner.
///
/// If the command doesn't finish successfuly the full output will saved to a file and the path
/// will be printed.
pub fn run_command_in(
    workdir: impl AsRef<Path>,
    title: &'static str,
    command: impl AsRef<OsStr>,
    args: &[impl AsRef<OsStr>],
    env: Option<Vec<(impl AsRef<OsStr>, impl AsRef<OsStr>)>>,
) -> Result<()> {
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::with_template("{spinner:.dim} {msg:.dim}")?);
    pb.enable_steady_tick(Duration::from_millis(80));
    pb.set_message(title);

    let mut _cmd = Command::new(command);
    _cmd.args(args)
        .current_dir(workdir.as_ref())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(_env) = env {
        _cmd.envs(_env);
    }
    let mut child = _cmd.spawn().context(format!("spawning `{title}`"))?;

    let stdout = child.stdout.take().expect("stdout is not None");
    let stderr = child.stderr.take().expect("stderr is not None");

    let log_path = logs_dir()?.join(log_filename(title));
    log::trace!("{}", log_path.display());

    let log = Arc::new(Mutex::new(File::create(&log_path)?));

    let t_out = {
        // stream stdout
        let pb_out = pb.clone();
        let log_out = log.clone();
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().flatten() {
                pb_out.set_message(line.chars().take(80).collect::<String>());
                if let Ok(mut f) = log_out.lock() {
                    let _ = f.write_all(line.as_bytes());
                    let _ = f.write_all("\n".as_bytes());
                }
            }
        })
    };

    let t_err = {
        // stream stderr
        let pb_err = pb.clone();
        let log_out = log.clone();
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().flatten() {
                pb_err.set_message(line.chars().take(80).collect::<String>());
                if let Ok(mut f) = log_out.lock() {
                    let _ = f.write_all(line.as_bytes());
                    let _ = f.write_all("\n".as_bytes());
                }
            }
        })
    };

    let status = child
        .wait()
        .context(format!("waiting for `{title}` to finish"))?;
    let _ = t_out.join();
    let _ = t_err.join();

    if status.success() {
        pb.finish_with_message(format!("{title} finished successfully"));
        Ok(())
    } else {
        pb.finish();
        bail!(
            "{title} exited with status {}\nFull output is available at {}",
            status,
            log_path.display()
        );
    }
}
