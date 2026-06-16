// Запускается вручную: `cargo run --bin make_fixtures`
// Генерирует tests/fixtures/red_2x3.{png,jpg} и oriented_3x2_or6.jpg
fn main() {
    let mut img = image::RgbImage::new(2, 3);
    for p in img.pixels_mut() {
        *p = image::Rgb([255, 0, 0]);
    }
    std::fs::create_dir_all("tests/fixtures").unwrap();
    img.save("tests/fixtures/red_2x3.png").unwrap();
    img.save("tests/fixtures/red_2x3.jpg").unwrap();
    println!("fixtures готовы");

    make_oriented_jpeg();
}

/// Записать JPEG 3×2 (ландшафт) с EXIF Orientation=6 (показывать повёрнутым на 90° CW → портрет 2×3).
fn make_oriented_jpeg() {
    use std::io::Write;
    // 1) Базовый JPEG 3×2 через image.
    let mut buf: Vec<u8> = Vec::new();
    {
        let img = image::RgbaImage::from_pixel(3, 2, image::Rgba([10, 200, 30, 255]));
        let dynimg = image::DynamicImage::ImageRgba8(img);
        dynimg
            .write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Jpeg)
            .expect("encode jpeg");
    }
    // 2) Минимальный EXIF APP1 с тегом Orientation(0x0112)=6, big-endian (MM).
    let exif_payload: &[u8] = &[
        0x45, 0x78, 0x69, 0x66, 0x00, 0x00,             // "Exif\0\0"
        0x4D, 0x4D, 0x00, 0x2A, 0x00, 0x00, 0x00, 0x08, // TIFF header (MM, 42, offset=8)
        0x00, 0x01,                                      // IFD0: 1 entry
        0x01, 0x12, 0x00, 0x03, 0x00, 0x00, 0x00, 0x01, // tag=Orientation, type=SHORT, count=1
        0x00, 0x06, 0x00, 0x00,                          // value=6: big-endian SHORT в байтах [0..2], [2..4] — паддинг
        0x00, 0x00, 0x00, 0x00,                          // next IFD offset = 0
    ];
    // 3) Собрать APP1-сегмент: маркер FFE1 + длина (вкл. 2 байта длины) + payload.
    let app1_len = (exif_payload.len() + 2) as u16;
    let mut app1 = vec![0xFF, 0xE1, (app1_len >> 8) as u8, (app1_len & 0xFF) as u8];
    app1.extend_from_slice(exif_payload);
    // 4) Вставить APP1 сразу после SOI (первые 2 байта FFD8).
    let mut out = Vec::with_capacity(buf.len() + app1.len());
    out.extend_from_slice(&buf[..2]); // SOI
    out.extend_from_slice(&app1);
    out.extend_from_slice(&buf[2..]);
    let path = std::path::Path::new("tests/fixtures/oriented_3x2_or6.jpg");
    std::fs::File::create(path).unwrap().write_all(&out).unwrap();
    println!("written {path:?}");
}
