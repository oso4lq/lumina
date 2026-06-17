//! Генератор иконки приложения из мастера app-icon.png.
//! Запуск: cargo run --example make_icon
//! Пишет assets/icon/lumina.ico (многоразмерный) и assets/icon/lumina-256.png (для окна).
use image::imageops::FilterType;

fn main() {
    let master = image::open("app-icon.png").expect("app-icon.png в корне репо");
    std::fs::create_dir_all("assets/icon").expect("создать assets/icon");

    // 256×256 PNG для иконки окна (winit)
    let png256 = master.resize_exact(256, 256, FilterType::Lanczos3).to_rgba8();
    png256.save("assets/icon/lumina-256.png").expect("сохранить lumina-256.png");

    // Многоразмерный ICO для ресурса exe
    let mut dir = ico::IconDir::new(ico::ResourceType::Icon);
    for size in [16u32, 24, 32, 48, 64, 128, 256] {
        let resized = master.resize_exact(size, size, FilterType::Lanczos3).to_rgba8();
        let (w, h) = (resized.width(), resized.height());
        let icon_image = ico::IconImage::from_rgba_data(w, h, resized.into_raw());
        dir.add_entry(ico::IconDirEntry::encode(&icon_image).expect("кодирование ICO-слоя"));
    }
    let file = std::fs::File::create("assets/icon/lumina.ico").expect("создать lumina.ico");
    dir.write(file).expect("записать lumina.ico");

    println!("готово: assets/icon/lumina.ico (7 размеров) + lumina-256.png");
}
