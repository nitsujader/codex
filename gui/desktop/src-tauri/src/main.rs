use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlaceholderResponse {
    ok: bool,
    message: String,
}

#[tauri::command]
fn paste_from_clipboard_placeholder() -> PlaceholderResponse {
    PlaceholderResponse {
        ok: false,
        message: "Clipboard paste placeholder: wire native clipboard integration.".to_string(),
    }
}

#[tauri::command]
fn screenshot_placeholder() -> PlaceholderResponse {
    PlaceholderResponse {
        ok: false,
        message: "Screenshot placeholder: wire window capture integration.".to_string(),
    }
}

#[tauri::command]
fn notification_placeholder(title: String, body: String) -> PlaceholderResponse {
    PlaceholderResponse {
        ok: true,
        message: format!("Notification placeholder received title='{title}' body='{body}'."),
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            paste_from_clipboard_placeholder,
            screenshot_placeholder,
            notification_placeholder
        ])
        .run(tauri::generate_context!())
        .expect("error while running desktop companion");
}
