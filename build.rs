//! Встраивание иконки приложения в exe (только Windows) через ресурс.
fn main() {
    #[cfg(windows)]
    {
        let _ = embed_resource::compile("assets/icon/lumina.rc", embed_resource::NONE);
    }
}
