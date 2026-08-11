//! Auto-start on Windows login via the HKCU "Run" registry key.
//! On non-Windows targets these are no-ops so the rest of the crate
//! stays portable.

#[cfg(windows)]
use winreg::enums::HKEY_CURRENT_USER;
#[cfg(windows)]
use winreg::RegKey;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "OCRScanTool";

#[cfg(windows)]
fn run_key() -> Result<RegKey, String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    Ok(hkcu
        .create_subkey(RUN_KEY)
        .map_err(|e| e.to_string())?
        .0)
}

#[cfg(windows)]
pub fn set(enabled: bool) -> Result<(), String> {
    let key = run_key()?;
    if enabled {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        key.set_value(VALUE_NAME, &format!("\"{}\"", exe.display()))
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
        Ok(_) => Ok(true),
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