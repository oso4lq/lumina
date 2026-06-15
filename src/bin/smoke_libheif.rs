// Проверка, что libheif линкуется и инициализируется.
// Запуск: cargo run --bin smoke_libheif
fn main() {
    let _lib = libheif_rs::LibHeif::new();
    println!("libheif слинкован и инициализирован успешно");
}
