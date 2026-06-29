// Prevent console window in addition to Slint window in Windows release builds when, e.g., starting the app via file manager. Ignored on other platforms.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod platform;

use std::{
    cell::{Cell, RefCell},
    collections::HashSet,
    error::Error,
    rc::Rc,
    time::Duration,
};

use crate::platform::{PlatformApi, WindowsPlatformApi};

slint::include_modules!();

fn main() -> Result<(), Box<dyn Error>> {
    let ui = AppWindow::new()?;
    let ti = TrayIcon::new()?;

    // 系统托盘功能
    let ui_handle = ui.as_weak();
    ti.on_show_window(move || {
        if let Some(w) = ui_handle.upgrade() {
            let _ = w.show();
        }
    });
    let ui_handle = ui.as_weak();
    ti.on_hide_window(move || {
        if let Some(w) = ui_handle.upgrade() {
            let _ = w.hide();
        }
    });
    ti.on_quit_program(|| {
        let _ = slint::quit_event_loop();
    });

    // 自动窗口居中的开关回调
    let acw_status = Rc::new(Cell::new(true));
    {
        let ui_handle = ui.as_weak();
        let acw_status = Rc::clone(&acw_status);

        ui.on_auto_center_window(move |enabled| {
            acw_status.set(enabled);
            if let Some(ui) = ui_handle.upgrade() {
                ui.set_acw_status(enabled);
            }
        });
    }

    // 定时检测活动窗口是否需要居中
    let _timer;
    {
        let ui_handle = ui.as_weak();
        let acw_status = Rc::clone(&acw_status);
        let api = WindowsPlatformApi;
        let seen_hwnds: Rc<RefCell<HashSet<isize>>> = Rc::new(RefCell::new(HashSet::new()));

        _timer = slint::Timer::default();
        _timer.start(
            slint::TimerMode::Repeated,
            Duration::from_millis(300),
            move || {
                let Ok(win) = api.get_active_window() else {
                    return;
                };

                if let Some(ui) = ui_handle.upgrade() {
                    ui.set_active_app(win.name.as_str().into());
                }
                if seen_hwnds.borrow().len() > 1000 {
                    seen_hwnds.borrow_mut().clear();
                }

                let current = win.hwnd.0 as isize;
                let is_new_window = seen_hwnds.borrow_mut().insert(current);
                if is_new_window && acw_status.get() {
                    let _ = api.center_window(win.hwnd);
                }
            },
        );
    }

    ui.run()?;

    Ok(())
}
