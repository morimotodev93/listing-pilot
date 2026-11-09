#[tauri::command]
pub fn greet() -> String {
    "Listing Pilot の準備ができました。Rustとの接続に成功しました。".to_owned()
}
pub mod product;
