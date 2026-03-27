use crate::core::stage::Stage;
use std::sync::Mutex;
use std::io::Write;

static LOG_BUFFER: Mutex<Vec<String>> = Mutex::new(Vec::new());

const LOG_FILE: &str = "cosyn_telemetry.log";

pub fn log_stage(stage: Stage, passed: bool, detail: &str) {
    let status = if passed { "PASS" } else { "FAIL" };
    let line = format!("[{}] {} — {}", stage.label(), status, detail);
    println!("  {}", line);
    if let Ok(mut buf) = LOG_BUFFER.lock() {
        buf.push(line);
    }
}

pub fn take_log() -> Vec<String> {
    if let Ok(mut buf) = LOG_BUFFER.lock() {
        std::mem::take(&mut *buf)
    } else {
        Vec::new()
    }
}

pub fn flush_to_file(stage_lines: &[String], dcc_lines: &[String]) {
    let timestamp = {
        let d = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let secs = d.as_secs();
        // UTC timestamp: YYYY-MM-DDThh:mm:ssZ
        let (days, rem) = (secs / 86400, secs % 86400);
        let (h, rem2) = (rem / 3600, rem % 3600);
        let (m, s) = (rem2 / 60, rem2 % 60);
        // Days since 1970-01-01 to date
        let (y, mo, da) = days_to_ymd(days);
        format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, mo, da, h, m, s)
    };
    let mut file = match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG_FILE)
    {
        Ok(f) => f,
        Err(e) => {
            eprintln!("telemetry: failed to open {}: {}", LOG_FILE, e);
            return;
        }
    };

    let _ = writeln!(file, "=== CoSyn Run {} ===", timestamp);
    for line in stage_lines {
        let _ = writeln!(file, "{}", line);
    }
    for line in dcc_lines {
        let _ = writeln!(file, "{}", line);
    }
    let _ = writeln!(file);
}

pub fn log_event(event: &str, detail: &str) {
    let line = format!("[event] {} — {}", event, detail);
    println!("  {}", line);
    if let Ok(mut buf) = LOG_BUFFER.lock() {
        buf.push(line);
    }
}

fn days_to_ymd(days_since_epoch: u64) -> (u64, u64, u64) {
    // Civil calendar conversion from days since 1970-01-01
    let z = days_since_epoch + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}
