// Hide console on Windows
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    #[cfg(target_os = "linux")]
    {
        // Prevent WebKitGTK EGL_BAD_PARAMETER / DMA-BUF crash on Wayland (KDE Plasma 6, Mesa)
        if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
    }

    mindsnap_lib::run();
}
