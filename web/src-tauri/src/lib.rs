#[tauri::command]
fn running_processes() -> Vec<String> {
    #[cfg(not(target_os = "android"))]
    {
        let mut system = sysinfo::System::new();
        system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        let mut names: Vec<String> = system
            .processes()
            .values()
            .map(|p| p.exe().and_then(|e| e.file_name()).unwrap_or(p.name()).to_string_lossy().into_owned())
            .collect();
        names.sort();
        names.dedup();
        names
    }
    #[cfg(target_os = "android")]
    {
        Vec::new()
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![running_processes])
        .run(tauri::generate_context!())
        .expect("failed to start Hearth");
}
