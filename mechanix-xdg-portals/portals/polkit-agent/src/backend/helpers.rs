use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use zbus::blocking::Connection;
use zbus::zvariant::OwnedObjectPath;

/// Find the polkit-agent-helper-1 binary.
/// polkit ships it at one of these locations depending on the distro.
pub fn find_helper_binary() -> Option<PathBuf> {
    let candidates = [
        "/usr/lib/polkit-1/polkit-agent-helper-1",
        "/usr/libexec/polkit-agent-helper-1",
        "/lib/polkit-1/polkit-agent-helper-1",
    ];
    for path in &candidates {
        let p = PathBuf::from(path);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

/// Result from a helper run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HelperResult {
    Success,
    Failure(Option<String>),
    HelperNotFound,
}

/// Authenticate a user via `polkit-agent-helper-1` and PAM.
///
/// Spawns the setuid helper binary, passes the cookie and password via stdin,
/// and returns whether PAM authentication succeeded (exit code 0).
pub fn authenticate_with_pam(username: &str, cookie: &str, password: &str) -> HelperResult {
    let helper = match find_helper_binary() {
        Some(path) => path,
        None => return HelperResult::HelperNotFound,
    };

    println!("[polkit-helpers] Spawning helper: {} for user: {}", helper.display(), username);

    let mut child = match Command::new(&helper)
        .arg(username)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[polkit-helpers] Failed to spawn helper binary: {e}");
            return HelperResult::Failure(Some(format!("Failed to spawn helper binary: {e}")));
        }
    };

    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    let stdout = child.stdout.take().expect("Failed to open stdout");
    let mut reader = BufReader::new(stdout);

    // 1. Write the cookie immediately on stdin
    if let Err(e) = writeln!(stdin, "{}", cookie) {
        eprintln!("[polkit-helpers] Failed to write cookie to helper: {e}");
        return HelperResult::Failure(Some(format!("Failed to write cookie to helper: {e}")));
    }
    if let Err(e) = stdin.flush() {
        eprintln!("[polkit-helpers] Failed to flush cookie to helper: {e}");
        return HelperResult::Failure(Some(format!("Failed to flush cookie to helper: {e}")));
    }

    let mut result = HelperResult::Failure(None);
    let mut last_error_msg = None;
    let mut line = String::new();

    // 2. Read stdout line-by-line
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.starts_with("PAM_PROMPT_ECHO_OFF") {
                    // Write password
                    if let Err(e) = writeln!(stdin, "{}", password) {
                        eprintln!("[polkit-helpers] Failed to write password to helper: {e}");
                        break;
                    }
                    if let Err(e) = stdin.flush() {
                        eprintln!("[polkit-helpers] Failed to flush password to helper: {e}");
                        break;
                    }
                } else if trimmed.starts_with("PAM_PROMPT_ECHO_ON") {
                    // Write password as fallback
                    if let Err(e) = writeln!(stdin, "{}", password) {
                        eprintln!("[polkit-helpers] Failed to write response to helper: {e}");
                        break;
                    }
                    if let Err(e) = stdin.flush() {
                        eprintln!("[polkit-helpers] Failed to flush response to helper: {e}");
                        break;
                    }
                } else if trimmed.starts_with("PAM_ERROR_MSG") {
                    let msg = trimmed.strip_prefix("PAM_ERROR_MSG ").unwrap_or(trimmed);
                    eprintln!("[polkit-helpers] PAM Error: {}", msg);
                    last_error_msg = Some(msg.to_string());
                } else if trimmed.starts_with("PAM_TEXT_INFO") {
                    let msg = trimmed.strip_prefix("PAM_TEXT_INFO ").unwrap_or(trimmed);
                    println!("[polkit-helpers] PAM Info: {}", msg);
                } else if trimmed == "SUCCESS" {
                    result = HelperResult::Success;
                    break;
                } else if trimmed == "FAILURE" {
                    result = HelperResult::Failure(last_error_msg.clone());
                    break;
                }
            }
            Err(e) => {
                eprintln!("[polkit-helpers] Error reading helper stdout: {e}");
                break;
            }
        }
    }

    // Wait for the process to exit
    let exit_status = child.wait();
    if result != HelperResult::Success {
        let stderr = child.stderr.take().map(|err| {
            let mut err_reader = BufReader::new(err);
            let mut err_line = String::new();
            let _ = err_reader.read_line(&mut err_line);
            err_line.trim().to_string()
        });
        if let Some(ref err_str) = stderr {
            if !err_str.is_empty() {
                eprintln!("[polkit-helpers] Helper stderr: {}", err_str);
                if last_error_msg.is_none() {
                    result = HelperResult::Failure(Some(err_str.clone()));
                }
            }
        }
        eprintln!(
            "[polkit-helpers] Helper authentication failed. Exit status: {:?}",
            exit_status
        );
    } else {
        println!("[polkit-helpers] Helper authentication completed successfully.");
    }

    result
}

/// Get the logind session id by calling logind over D-Bus system bus.
/// Attempts `GetSessionByPID` first, and if that fails (e.g. running in systemd user manager),
/// falls back to retrieving the user's active graphical display session.
pub fn get_session_id() -> String {
    if let Some(id) = get_session_id_via_logind() {
        println!("[polkit-agent] Got session ID via GetSessionByPID: {}", id);
        return id;
    }
    if let Some(id) = get_session_id_via_user_display() {
        println!("[polkit-agent] Got session ID via active user Display: {}", id);
        return id;
    }
    println!("[polkit-agent] Failed to retrieve any active logind session ID");
    String::new()
}

pub fn get_session_id_via_logind() -> Option<String> {
    let conn = Connection::system().ok()?;
    let path: OwnedObjectPath = conn
        .call_method(
            Some("org.freedesktop.login1"),
            "/org/freedesktop/login1",
            Some("org.freedesktop.login1.Manager"),
            "GetSessionByPID",
            &(std::process::id(),),
        )
        .ok()?
        .body()
        .deserialize()
        .ok()?;
    let last_segment = path.as_str().rsplit('/').next()?;
    let session_id = last_segment.strip_prefix('_').unwrap_or(last_segment);
    Some(session_id.to_string())
}

pub fn get_session_id_via_user_display() -> Option<String> {
    let conn = Connection::system().ok()?;
    let uid = unsafe { libc::getuid() };
    
    let user_path: OwnedObjectPath = conn
        .call_method(
            Some("org.freedesktop.login1"),
            "/org/freedesktop/login1",
            Some("org.freedesktop.login1.Manager"),
            "GetUser",
            &(uid,),
        )
        .ok()?
        .body()
        .deserialize()
        .ok()?;

    let display_variant: zbus::zvariant::OwnedValue = conn
        .call_method(
            Some("org.freedesktop.login1"),
            &user_path,
            Some("org.freedesktop.DBus.Properties"),
            "Get",
            &("org.freedesktop.login1.User", "Display"),
        )
        .ok()?
        .body()
        .deserialize()
        .ok()?;

    let val: zbus::zvariant::Value = display_variant.into();
    let inner_val = match val {
        zbus::zvariant::Value::Value(inner) => *inner,
        other => other,
    };
    
    let session_id = match inner_val {
        zbus::zvariant::Value::Structure(structure) => {
            let fields = structure.fields();
            if !fields.is_empty() {
                match &fields[0] {
                    zbus::zvariant::Value::Str(s) => s.as_str().to_string(),
                    _ => return None,
                }
            } else {
                return None;
            }
        }
        _ => return None,
    };

    if session_id.is_empty() {
        None
    } else {
        Some(session_id)
    }
}

pub fn process_start_time(pid: u32) -> u64 {
    // /proc/<pid>/stat field 22 (starttime), in clock ticks since boot.
    // Fields are space-separated but field 2 (comm) can itself contain spaces
    // and is wrapped in parentheses, so split after the last ')'.
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).unwrap_or_default();
    stat.rsplit_once(')')
        .and_then(|(_, rest)| rest.split_whitespace().nth(19)) // 22 - 3(pid,comm,state) = 19 → field 22 overall
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0)
}

