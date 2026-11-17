#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() && std::path::Path::new("/proc/driver/nvidia").exists() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
    let builder = tauri::Builder::default();
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_window_state::Builder::default().build());
    builder
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            #[cfg(target_os = "linux")]
            {
                use tauri::Manager;
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.with_webview(|view| {
                        use webkit2gtk::{PermissionRequestExt, SettingsExt, UserMediaPermissionRequest, WebViewExt};
                        use webkit2gtk::glib::object::Cast;
                        let view = view.inner();
                        if let Some(settings) = WebViewExt::settings(&view) {
                            settings.set_enable_media_stream(true);
                            settings.set_enable_webrtc(true);
                            settings.set_enable_mediasource(true);
                            settings.set_media_playback_requires_user_gesture(false);
                        }
                        view.connect_permission_request(|_, request| {
                            if request.downcast_ref::<UserMediaPermissionRequest>().is_some() {
                                request.allow();
                                return true;
                            }
                            false
                        });
                    });
                }
            }
            #[cfg(not(target_os = "linux"))]
            let _ = app;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to start Hearth");
}
