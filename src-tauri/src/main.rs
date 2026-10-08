// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // WebKitGTK's DMA-BUF renderer can leave the window blank on virtual GPUs.
    // Must be set before the webview initializes, so it happens here in main().
    if soplos_webapp_manager_lib::is_running_in_vm() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    soplos_webapp_manager_lib::run()
}
