#[cfg(desktop)]
mod update {
    use tauri::{AppHandle, Emitter, Manager};
    use tauri_plugin_updater::UpdaterExt;

    pub struct Enabled(pub bool);

    fn enabled(app: &AppHandle) -> Result<(), String> {
        if app.state::<Enabled>().0 {
            Ok(())
        } else {
            Err("updates are not set up for this build".into())
        }
    }

    #[tauri::command]
    pub async fn update_check(app: AppHandle) -> Result<Option<String>, String> {
        enabled(&app)?;
        let update = app.updater().map_err(|e| e.to_string())?.check().await.map_err(|e| e.to_string())?;
        Ok(update.map(|u| u.version))
    }

    #[tauri::command]
    pub async fn update_install(app: AppHandle) -> Result<(), String> {
        enabled(&app)?;
        let Some(update) = app.updater().map_err(|e| e.to_string())?.check().await.map_err(|e| e.to_string())? else {
            return Err("no update available".into());
        };
        let mut done = 0u64;
        update
            .download_and_install(
                |chunk, total| {
                    done += chunk as u64;
                    if let Some(total) = total.filter(|t| *t > 0) {
                        let _ = app.emit("update-progress", done as f64 * 100.0 / total as f64);
                    }
                },
                || {},
            )
            .await
            .map_err(|e| e.to_string())?;
        app.restart();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "linux")]
    if std::env::var_os("WEBKIT_DISABLE_DMABUF_RENDERER").is_none() && std::path::Path::new("/proc/driver/nvidia").exists() {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }
    let builder = tauri::Builder::default();
    #[cfg(desktop)]
    let builder = builder
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .invoke_handler(tauri::generate_handler![update::update_check, update::update_install]);
    builder
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            #[cfg(desktop)]
            {
                use tauri::Manager;
                let on = app.config().plugins.0.contains_key("updater");
                if on {
                    app.handle().plugin(tauri_plugin_updater::Builder::new().build())?;
                }
                app.manage(update::Enabled(on));
            }
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
