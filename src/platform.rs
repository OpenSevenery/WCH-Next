use std::{
    env,
    path::Path,
    process::{Command, Stdio},
};

use windows::{
    Win32::{
        Foundation::{CloseHandle, HWND, MAX_PATH, RECT},
        Graphics::{
            Dwm::{DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute},
            Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow},
        },
        System::Threading::{
            OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
            QueryFullProcessImageNameW,
        },
        UI::WindowsAndMessaging::{
            GetForegroundWindow, GetWindowRect, GetWindowThreadProcessId, IsZoomed, SWP_NOACTIVATE,
            SWP_NOSIZE, SWP_NOZORDER, SetWindowPos,
        },
    },
    core::PWSTR,
};

// 活动窗口信息
pub struct ActiveWindow {
    pub hwnd: HWND,
    pub name: String,
}

// 平台能力抽象
pub trait PlatformApi {
    /// 获取当前活动窗口的信息
    fn get_active_window(&self) -> windows::core::Result<ActiveWindow>;

    /// 将指定窗口居中到其所在显示器的工作区
    fn center_window(&self, hwnd: HWND) -> windows::core::Result<()>;

    // 添加自动启动
    fn enable_autorun(&self) -> windows::core::Result<()>;
    // 移除自动启动
    fn disable_autorun(&self) -> windows::core::Result<()>;
}

// Windows 平台实现
#[derive(Copy, Clone)]
pub struct WindowsPlatformApi;

impl PlatformApi for WindowsPlatformApi {
    fn get_active_window(&self) -> windows::core::Result<ActiveWindow> {
        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.is_invalid() {
            return Err(windows::core::Error::from_thread());
        }

        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        let name = get_process_name(pid)?;

        Ok(ActiveWindow { hwnd, name })
    }

    fn center_window(&self, hwnd: HWND) -> windows::core::Result<()> {
        // 判断窗口是否最大化
        if unsafe { IsZoomed(hwnd) }.as_bool() {
            return Ok(());
        }

        // 精确窗口边界（排除 Windows 10/11 的不可见调整边框）
        let rect = get_window_position(hwnd)?;
        let win_w = rect.right - rect.left;
        let win_h = rect.bottom - rect.top;

        // 获取显示器工作区（排除任务栏）
        let hmonitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        let _ = unsafe { GetMonitorInfoW(hmonitor, &mut info) };
        let work = info.rcWork;

        // 计算居中坐标
        let x = work.left + (work.right - work.left - win_w) / 2;
        let y = work.top + (work.bottom - work.top - win_h) / 2;

        // 移动窗口，保持大小和 Z 序不变
        unsafe {
            SetWindowPos(
                hwnd,
                None,
                x,
                y,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            )?
        };

        Ok(())
    }

    fn enable_autorun(&self) -> windows::core::Result<()> {
        let exe_path = env::current_exe()
            .expect("Failed to get exe path")
            .display()
            .to_string();

        Command::new("reg")
            .args([
                "add",
                r"HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run",
                "/v",
                "WCH-Next",
                "/t",
                "REG_SZ",
                "/d",
                &exe_path,
                "/f",
            ])
            .stdout(Stdio::null())
            .status()
            .expect("Failed to create task");

        Ok(())
    }

    fn disable_autorun(&self) -> windows::core::Result<()> {
        Command::new("reg")
            .args([
                "delete",
                r"HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run",
                "/v",
                "WCH-Next",
                "/f",
            ])
            .stdout(Stdio::null())
            .status()
            .expect("Failed to delete task");

        Ok(())
    }
}

// 获取进程的名称
fn get_process_name(pid: u32) -> windows::core::Result<String> {
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)? };

    let mut buf = [0u16; MAX_PATH as usize];
    let mut size = MAX_PATH;

    let result = unsafe {
        QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(buf.as_mut_ptr()),
            &mut size,
        )
    };

    unsafe {
        let _ = CloseHandle(handle);
    }
    result?;

    let path = String::from_utf16_lossy(&buf[..size as usize]);
    let name = Path::new(&path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();

    Ok(name)
}

// 获取窗口的精确位置
fn get_window_position(hwnd: HWND) -> windows::core::Result<RECT> {
    let mut rect = RECT::default();

    let dwm_ok = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &raw mut rect as *mut _,
            size_of::<RECT>() as u32,
        )
    };

    if dwm_ok.is_err() {
        unsafe { GetWindowRect(hwnd, &mut rect)? };
    }

    Ok(rect)
}
