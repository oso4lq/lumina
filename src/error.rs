use std::path::PathBuf;

#[derive(thiserror::Error, Debug)]
pub enum LuminaError {
    #[error("не удалось прочитать файл {0}: {1}")]
    Io(PathBuf, #[source] std::io::Error),

    #[error("не удалось декодировать {0}: {1}")]
    Decode(PathBuf, #[source] image::ImageError),

    #[error("формат не поддерживается: {0}")]
    Unsupported(PathBuf),

    #[error("ошибка инициализации GPU: {0}")]
    Gpu(String),

    #[error("ошибка HEIC {0}: {1}")]
    Heic(std::path::PathBuf, String),

    #[error("ошибка RAW {0}: {1}")]
    Raw(std::path::PathBuf, String),

    #[error("ошибка платформы: {0}")]
    Platform(String),
}

pub type Result<T> = std::result::Result<T, LuminaError>;
