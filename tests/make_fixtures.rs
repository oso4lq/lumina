// Запускается вручную: `cargo run --bin make_fixtures`
// Генерирует tests/fixtures/red_2x3.png и red_2x3.jpg
fn main() {
    let mut img = image::RgbImage::new(2, 3);
    for p in img.pixels_mut() {
        *p = image::Rgb([255, 0, 0]);
    }
    std::fs::create_dir_all("tests/fixtures").unwrap();
    img.save("tests/fixtures/red_2x3.png").unwrap();
    img.save("tests/fixtures/red_2x3.jpg").unwrap();
    println!("fixtures готовы");
}
