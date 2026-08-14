use std::process::Command;
use std::collections::HashMap;
use std::path::Path;
use zbus::zvariant::OwnedValue;

pub fn get_printer_list() -> Vec<String> {
    if let Ok(output) = Command::new("lpstat").arg("-e").output() {
        if output.status.success() {
            let printers: Vec<String> = String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(|line| line.trim().to_string())
                .filter(|line| !line.is_empty())
                .collect();
            if !printers.is_empty() {
                return printers;
            }
        }
    }
    vec![
        "HP LaserJet Professional M1212nf (Default)".to_string(),
        "Office Printer 3F".to_string(),
        "PDF Writer".to_string(),
    ]
}

pub fn print_file(
    printer: &str,
    title: &str,
    file_path: &Path,
    settings: &HashMap<String, OwnedValue>,
    page_setup: &HashMap<String, OwnedValue>,
) -> Result<(), String> {
    let mut cmd = Command::new("lp");
    cmd.arg("-d").arg(printer);
    cmd.arg("-t").arg(title);

    // Map settings to lp options
    if let Some(copies_val) = settings.get("n-copies") {
        if let Some(copies) = format_value_to_string(copies_val) {
            cmd.arg("-n").arg(copies);
        }
    }

    if let Some(duplex_val) = settings.get("duplex") {
        if let Some(duplex) = format_value_to_string(duplex_val) {
            match duplex.to_lowercase().as_str() {
                "simplex" | "one-sided" => {
                    cmd.arg("-o").arg("sides=one-sided");
                }
                "horizontal" | "two-sided-short-edge" => {
                    cmd.arg("-o").arg("sides=two-sided-short-edge");
                }
                "vertical" | "two-sided-long-edge" => {
                    cmd.arg("-o").arg("sides=two-sided-long-edge");
                }
                _ => {}
            }
        }
    }

    if let Some(orientation_val) = page_setup.get("orientation") {
        if let Some(orientation) = format_value_to_string(orientation_val) {
            match orientation.to_lowercase().as_str() {
                "portrait" => {
                    cmd.arg("-o").arg("orientation-requested=3");
                }
                "landscape" => {
                    cmd.arg("-o").arg("orientation-requested=4");
                }
                "reverse-portrait" => {
                    cmd.arg("-o").arg("orientation-requested=5");
                }
                "reverse-landscape" => {
                    cmd.arg("-o").arg("orientation-requested=6");
                }
                _ => {}
            }
        }
    }

    if let Some(collate_val) = settings.get("collate") {
        if let Some(collate) = format_value_to_string(collate_val) {
            cmd.arg("-o").arg(format!("collate={}", collate));
        }
    }

    cmd.arg(file_path);

    println!("[print-helper] Running command: {:?}", cmd);
    match cmd.status() {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(format!("lp exited with non-zero code: {:?}", status.code())),
        Err(e) => Err(format!("failed to execute lp: {e}")),
    }
}

pub fn format_value_to_string(val: &OwnedValue) -> Option<String> {
    use std::ops::Deref;
    let val_ref = val.deref();
    match val_ref {
        zbus::zvariant::Value::Str(s) => Some(s.as_str().to_string()),
        zbus::zvariant::Value::U32(v) => Some(v.to_string()),
        zbus::zvariant::Value::I32(v) => Some(v.to_string()),
        zbus::zvariant::Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}
