//! Pins windows into Explorer's desktop, between the icons and the static wallpaper.
//! Two layouts: classic (Win10, Win11 <= 23H2) and raised (Win11 24H2+).

use std::{thread, time::Duration};

use windows::{
    core::{w, BOOL},
    Win32::{
        Foundation::{COLORREF, HWND, LPARAM, RECT, WPARAM},
        UI::WindowsAndMessaging::*,
    },
};

#[derive(Clone, Copy, Debug)]
pub enum Host {
    Classic { workerw: HWND },
    Raised { progman: HWND, defview: HWND, workerw: HWND },
}

// HWNDs are plain handles, safe to move between threads.
unsafe impl Send for Host {}
unsafe impl Sync for Host {}

impl Host {
    /// The window our wallpaper windows are parented to.
    pub fn parent(&self) -> HWND {
        match *self {
            Host::Classic { workerw } => workerw,
            Host::Raised { progman, .. } => progman,
        }
    }

    /// False once Explorer has restarted and the host is gone.
    pub fn is_alive(&self) -> bool {
        unsafe { IsWindow(Some(self.parent())).as_bool() }
    }
}

pub fn find_host() -> Result<Host, String> {
    unsafe {
        let progman = FindWindowW(w!("Progman"), None)
            .map_err(|_| "Explorer's desktop (Progman) was not found.".to_string())?;

        // Ask Explorer to spawn the WorkerW wallpaper layer.
        SendMessageTimeoutW(progman, 0x052C, WPARAM(0xD), LPARAM(1), SMTO_NORMAL, 1000, None);
        thread::sleep(Duration::from_millis(200));

        for _ in 0..5 {
            let ex = GetWindowLongPtrW(progman, GWL_EXSTYLE) as u32;
            if ex & WS_EX_NOREDIRECTIONBITMAP.0 != 0 {
                let defview = FindWindowExW(Some(progman), None, w!("SHELLDLL_DefView"), None);
                let workerw = FindWindowExW(Some(progman), None, w!("WorkerW"), None);
                if let (Ok(defview), Ok(workerw)) = (defview, workerw) {
                    eprintln!("kinora: desktop host: raised layout");
                    return Ok(Host::Raised { progman, defview, workerw });
                }
            } else if let Some(workerw) = classic_workerw() {
                eprintln!("kinora: desktop host: classic layout");
                return Ok(Host::Classic { workerw });
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
    Err("Couldn't find the desktop wallpaper layer. On Windows 10, turn on \
         \"Animate controls and elements inside windows\" in Performance Options."
        .into())
}

/// Classic layout: the top-level WorkerW right after the one holding the icons.
unsafe fn classic_workerw() -> Option<HWND> {
    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        unsafe {
            if FindWindowExW(Some(hwnd), None, w!("SHELLDLL_DefView"), None).is_ok() {
                if let Ok(next) = FindWindowExW(None, Some(hwnd), w!("WorkerW"), None) {
                    *(lparam.0 as *mut HWND) = next;
                }
            }
        }
        true.into()
    }

    let mut found = HWND::default();
    unsafe {
        let _ = EnumWindows(Some(enum_proc), LPARAM(&mut found as *mut HWND as isize));
    }
    (!found.is_invalid()).then_some(found)
}

/// Attach a hidden window to the desktop host, covering `rect` (physical pixels,
/// virtual-screen coordinates), then show it.
pub fn attach(ours: HWND, host: &Host, rect: RECT) -> Result<(), String> {
    unsafe {
        // Style bits are only changed while the window is hidden.
        let style = GetWindowLongPtrW(ours, GWL_STYLE) as u32;
        let style = (style & !(WS_POPUP.0 | WS_CAPTION.0 | WS_THICKFRAME.0 | WS_SYSMENU.0))
            | WS_CHILD.0;
        SetWindowLongPtrW(ours, GWL_STYLE, style as isize);

        // Layered + transparent: click-through so the icons and right-click keep working.
        let ex = GetWindowLongPtrW(ours, GWL_EXSTYLE) as u32;
        let ex = ex | WS_EX_LAYERED.0 | WS_EX_TRANSPARENT.0 | WS_EX_NOACTIVATE.0;
        SetWindowLongPtrW(ours, GWL_EXSTYLE, ex as isize);
        SetLayeredWindowAttributes(ours, COLORREF(0), 255, LWA_ALPHA).map_err(err)?;

        SetParent(ours, Some(host.parent())).map_err(err)?;

        let mut host_rect = RECT::default();
        GetWindowRect(host.parent(), &mut host_rect).map_err(err)?;
        let (x, y) = (rect.left - host_rect.left, rect.top - host_rect.top);
        let (w, h) = (rect.right - rect.left, rect.bottom - rect.top);

        match *host {
            Host::Classic { .. } => {
                SetWindowPos(ours, None, x, y, w, h, SWP_NOZORDER | SWP_NOACTIVATE).map_err(err)?;
            }
            Host::Raised { defview, workerw, .. } => {
                // Insert directly below the icons, push Explorer's wallpaper layer to the bottom.
                SetWindowPos(ours, Some(defview), x, y, w, h, SWP_NOACTIVATE).map_err(err)?;
                let _ = SetWindowPos(
                    workerw,
                    Some(HWND_BOTTOM),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
        }

        // Raw ShowWindow: going through Tauri would make it rewrite our style bits.
        let _ = ShowWindow(ours, SW_SHOWNA);
    }
    Ok(())
}

/// The current static wallpaper image, if there is one (not a solid color).
pub fn static_wallpaper_path() -> Option<String> {
    let mut buf = [0u16; 1024];
    unsafe {
        SystemParametersInfoW(
            SPI_GETDESKWALLPAPER,
            buf.len() as u32,
            Some(buf.as_mut_ptr().cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
        .ok()?;
    }
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    let path = String::from_utf16_lossy(&buf[..len]);
    (!path.is_empty() && std::path::Path::new(&path).is_file()).then_some(path)
}

/// Re-apply the current static wallpaper so no frozen video frame is left behind.
pub fn refresh_static_wallpaper() {
    let mut buf = [0u16; 1024];
    unsafe {
        let got = SystemParametersInfoW(
            SPI_GETDESKWALLPAPER,
            buf.len() as u32,
            Some(buf.as_mut_ptr().cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
        if got.is_ok() {
            let _ = SystemParametersInfoW(
                SPI_SETDESKWALLPAPER,
                0,
                Some(buf.as_mut_ptr().cast()),
                SPIF_UPDATEINIFILE,
            );
        }
    }
}

fn err(e: windows::core::Error) -> String {
    e.message()
}
