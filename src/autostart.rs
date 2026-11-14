use windows::core::{w, PCWSTR};
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_SZ,
};

const SUBKEY: PCWSTR = w!(r"Software\Microsoft\Windows\CurrentVersion\Run");
const VALUE_NAME: PCWSTR = w!("Tendedero");

/// 检测是否已开启开机自启动
pub fn is_autostart_enabled() -> bool {
    unsafe {
        let mut hkey = HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, SUBKEY, 0, KEY_READ, &mut hkey).is_ok() {
            let res = RegQueryValueExW(hkey, VALUE_NAME, None, None, None, None);
            let _ = RegCloseKey(hkey);
            res.is_ok()
        } else {
            false
        }
    }
}

/// 开启开机自启动
pub fn enable_autostart() -> Result<(), String> {
    let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe_dir = current_exe.parent().unwrap_or(std::path::Path::new(""));
    let vbs_path = exe_dir.join("Start Tendedero.vbs");
    let cmd = if vbs_path.exists() {
        format!(r#"wscript.exe "{}""#, vbs_path.to_string_lossy())
    } else {
        format!(r#""{}""#, current_exe.to_string_lossy())
    };

    let wide_cmd: Vec<u16> = cmd.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        let mut hkey = HKEY::default();
        let err = RegOpenKeyExW(HKEY_CURRENT_USER, SUBKEY, 0, KEY_WRITE, &mut hkey);
        if err.is_err() {
            return Err(format!("无法打开注册表键: {:?}", err));
        }

        let bytes = std::slice::from_raw_parts(
            wide_cmd.as_ptr() as *const u8,
            wide_cmd.len() * std::mem::size_of::<u16>(),
        );

        let res = RegSetValueExW(
            hkey,
            VALUE_NAME,
            0,
            REG_SZ,
            Some(bytes),
        );
        let _ = RegCloseKey(hkey);
        if res.is_err() {
            return Err(format!("写入自启注册表失败: {:?}", res));
        }
    }
    Ok(())
}

/// 关闭开机自启动
pub fn disable_autostart() -> Result<(), String> {
    unsafe {
        let mut hkey = HKEY::default();
        if RegOpenKeyExW(HKEY_CURRENT_USER, SUBKEY, 0, KEY_WRITE, &mut hkey).is_ok() {
            let _ = RegDeleteValueW(hkey, VALUE_NAME);
            let _ = RegCloseKey(hkey);
        }
    }
    Ok(())
}

/// 切换开机自启动状态，返回切换后的最新状态
pub fn toggle_autostart() -> Result<bool, String> {
    if is_autostart_enabled() {
        disable_autostart()?;
        Ok(false)
    } else {
        enable_autostart()?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_autostart_lifecycle() {
        let _ = disable_autostart();
        assert!(!is_autostart_enabled());

        assert!(enable_autostart().is_ok());
        assert!(is_autostart_enabled());

        assert!(disable_autostart().is_ok());
        assert!(!is_autostart_enabled());
    }
}
