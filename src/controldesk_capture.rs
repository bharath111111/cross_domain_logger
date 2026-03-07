use std::path::PathBuf;
use std::process::{Command, Output};

#[derive(Copy, Clone)]
pub enum CanLogFormat {
    Text,
    Asc,
}

fn resolve_script_path() -> Result<PathBuf, String> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(explicit) = std::env::var("CONTROLDESK_CAPTURE_SCRIPT") {
        candidates.push(PathBuf::from(explicit));
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("scripts").join("controldesk_capture.py"));
            candidates.push(dir.join("controldesk_capture.py"));
            if let Some(parent) = dir.parent() {
                candidates.push(parent.join("scripts").join("controldesk_capture.py"));
            }
        }
    }

    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("scripts").join("controldesk_capture.py"));
        candidates.push(cwd.join("controldesk_capture.py"));
    }

    if let Some(path) = candidates.into_iter().find(|path| path.exists()) {
        Ok(path)
    } else {
        Err("Unable to locate scripts/controldesk_capture.py. Set CONTROLDESK_CAPTURE_SCRIPT to an absolute path.".to_string())
    }
}

fn run_python_output(extra_args: &[String]) -> Result<Output, String> {
    let script = resolve_script_path()?;

    let mut last_error = String::new();

    for launcher in ["py", "python"] {
        let mut command = Command::new(launcher);
        if launcher == "py" {
            command.arg("-3");
        }

        let output = command
            .arg(&script)
            .args(extra_args)
            .output();

        match output {
            Ok(out) => return Ok(out),
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    last_error = format!("{} launcher not found", launcher);
                    continue;
                }
                return Err(format!("Failed to run {}: {}", launcher, e));
            }
        }
    }

    Err(format!("Python launcher unavailable: {}", last_error))
}

fn run_python_status(extra_args: &[String]) -> Result<(), String> {
    let script = resolve_script_path()?;

    let mut last_error = String::new();

    for launcher in ["py", "python"] {
        let mut command = Command::new(launcher);
        if launcher == "py" {
            command.arg("-3");
        }

        let status = command
            .arg(&script)
            .args(extra_args)
            .status();

        match status {
            Ok(s) => {
                if s.success() {
                    return Ok(());
                }
                return Err(format!("ControlDesk script exited with status {}", s));
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    last_error = format!("{} launcher not found", launcher);
                    continue;
                }
                return Err(format!("Failed to run {}: {}", launcher, e));
            }
        }
    }

    Err(format!("Python launcher unavailable: {}", last_error))
}

pub fn print_can_channel_mapping() -> Result<(), String> {
    let args = vec!["--list-platforms".to_string()];
    let out = run_python_output(&args)?;

    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("ControlDesk platform listing failed with status {}", out.status)
        } else {
            format!("ControlDesk platform listing failed: {}", stderr)
        });
    }

    let stdout = String::from_utf8_lossy(&out.stdout);
    print!("{}", stdout);
    Ok(())
}

pub fn listen_can_all_connected(
    duration_ms: Option<u64>,
    output_dir: Option<&str>,
    format: CanLogFormat,
) -> Result<(), String> {
    let mut args: Vec<String> = vec!["--capture-all".to_string()];

    if let Some(ms) = duration_ms {
        args.push("--duration-ms".to_string());
        args.push(ms.to_string());
    }

    if let Some(dir) = output_dir {
        args.push("--output-dir".to_string());
        args.push(dir.to_string());
    }

    args.push("--format".to_string());
    args.push(match format {
        CanLogFormat::Text => "text".to_string(),
        CanLogFormat::Asc => "asc".to_string(),
    });

    run_python_status(&args)
}
