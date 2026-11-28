//! Auto-start on Windows login via the HKCU "Run" registry key.
//! On non-Windows targets these are no-ops so the rest of the crate
//! stays portable.

#[cfg(windows)]
use winreg::enums::HKEY_CURRENT_USER;
#[cfg(windows)]
use winreg::RegKey;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "OCRScanTool";

/// Ambil path executable dari command Run key, baik `"C:\path\app.exe" --flag`
/// maupun `C:\path\app.exe --flag`.
fn executable_from_command(value: &str) -> &str {
    let value = value.trim();
    if let Some(rest) = value.strip_prefix('"') {
        if let Some(end) = rest.find('"') {
            return &rest[..end];
        }
    }
    value.split_whitespace().next().unwrap_or("")
}

fn trim_path(s: &str) -> &str {
    s.trim().trim_end_matches(['\\', '/'])
}

/// Autostart harus masuk tray, bukan memunculkan jendela saat login.
fn command_for(exe: &std::path::Path) -> String {
    format!("\"{}\" --background", exe.display())
}

fn same_exe(a: &str, b: &str) -> bool {
    // Path Windows case-insensitive.
    trim_path(a).eq_ignore_ascii_case(trim_path(b))
}

#[cfg(windows)]
fn run_key() -> Result<RegKey, String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    Ok(hkcu.create_subkey(RUN_KEY).map_err(|e| e.to_string())?.0)
}

#[cfg(windows)]
pub fn set(enabled: bool) -> Result<(), String> {
    let key = run_key()?;
    if enabled {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        key.set_value(VALUE_NAME, &command_for(&exe))
            .map_err(|e| e.to_string())?;
    } else {
        match key.delete_value(VALUE_NAME) {
            Ok(_) => {}
            Err(e) if e.raw_os_error() == Some(2) => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}

#[cfg(windows)]
pub fn get() -> Result<bool, String> {
    let key = run_key()?;
    match key.get_value::<String, _>(VALUE_NAME) {
        // Jangan laporkan aktif kalau value-nya dipakai program lain.
        Ok(value) => {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            Ok(same_exe(
                executable_from_command(&value),
                &exe.to_string_lossy(),
            ))
        }
        Err(e) if e.raw_os_error() == Some(2) => Ok(false),
        Err(e) => Err(e.to_string()),
    }
}

#[cfg(not(windows))]
pub fn set(_enabled: bool) -> Result<(), String> {
    Ok(())
}

#[cfg(not(windows))]
pub fn get() -> Result<bool, String> {
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quoted_command_with_arguments() {
        assert_eq!(
            executable_from_command(r#"  "C:\Program Files\App\app.exe" --minimized "#),
            r"C:\Program Files\App\app.exe"
        );
    }

    #[test]
    fn parses_unquoted_command() {
        assert_eq!(
            executable_from_command(r"C:\Apps\app.exe --flag"),
            r"C:\Apps\app.exe"
        );
    }

    #[test]
    fn compares_paths_case_insensitively() {
        assert!(same_exe(r"C:\Apps\App.EXE", r"c:\apps\app.exe"));
        assert!(!same_exe(r"C:\Apps\other.exe", r"C:\Apps\app.exe"));
    }

    #[test]
    fn autostart_command_keeps_background_flag() {
        let exe = std::path::Path::new(r"C:\Apps\OCR Scan Tool\ocr.exe");
        let command = command_for(exe);
        assert_eq!(command, r#""C:\Apps\OCR Scan Tool\ocr.exe" --background"#);
        assert_eq!(
            executable_from_command(&command),
            r"C:\Apps\OCR Scan Tool\ocr.exe"
        );
    }
}
