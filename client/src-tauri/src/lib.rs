// GROUP: 42
// MEMBERS: Ray Okamoto, Phoenix Pereira, Kayla Rowley, Qi Wu, Ho Yin Li

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use tauri_plugin_sql::{Migration, MigrationKind};

#[tauri::command]
fn greet(name: &str) -> String {
    shared::greet(name).message
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let migrations = vec![
        Migration {
            version: 1,
            description: "create users table",
            sql: r#"
                CREATE TABLE IF NOT EXISTS users (
                    user_id TEXT PRIMARY KEY,
                    pubkey TEXT NOT NULL,
                    privkey_store TEXT NOT NULL,
                    password TEXT NOT NULL,
                    salt TEXT NOT NULL,
                    meta TEXT,
                    version INT NOT NULL
                )
            "#,
            kind: MigrationKind::Up,
        }
    ];

    tauri::Builder::default()
        .plugin(
            tauri_plugin_sql::Builder::default()
                .add_migrations("sqlite:secure_chat.db", migrations)
                .build()
        )
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
