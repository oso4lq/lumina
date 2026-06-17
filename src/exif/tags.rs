//! Чистое ядро: модель EXIF-тегов и разбор JSON-вывода exiftool (`-json -G`).
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub struct TagGroup {
    pub name: String,
    pub tags: Vec<(String, String)>, // (tag без префикса группы, значение)
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExifTags {
    pub groups: Vec<TagGroup>,
}

/// Предпочтительный порядок групп; неизвестные — после, по алфавиту.
const GROUP_ORDER: &[&str] = &[
    "File", "EXIF", "MakerNotes", "Composite", "GPS", "IPTC", "XMP", "ICC_Profile",
];

fn group_rank(name: &str) -> usize {
    GROUP_ORDER.iter().position(|g| *g == name).unwrap_or(GROUP_ORDER.len())
}

/// JSON-значение exiftool → отображаемая строка.
fn value_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        other => other.to_string(), // массивы/объекты — компактный JSON
    }
}

/// Разобрать вывод `exiftool -json -G` (массив с одним объектом).
/// Ключи вида "Group:Tag"; "SourceFile" пропускается. Порядок детерминированный:
/// группы по GROUP_ORDER (неизвестные — алфавит), теги внутри — по алфавиту.
pub fn parse(json: &str) -> ExifTags {
    let Ok(Value::Array(arr)) = serde_json::from_str::<Value>(json) else {
        return ExifTags::default();
    };
    let Some(Value::Object(obj)) = arr.into_iter().next() else {
        return ExifTags::default();
    };
    let mut by_group: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for (key, val) in obj {
        if key == "SourceFile" {
            continue;
        }
        let (group, tag) = match key.split_once(':') {
            Some((g, t)) => (g.to_string(), t.to_string()),
            None => ("Other".to_string(), key.clone()),
        };
        by_group.entry(group).or_default().push((tag, value_to_string(&val)));
    }
    let mut groups: Vec<TagGroup> = by_group
        .into_iter()
        .map(|(name, mut tags)| {
            tags.sort_by(|a, b| a.0.cmp(&b.0));
            TagGroup { name, tags }
        })
        .collect();
    groups.sort_by(|a, b| group_rank(&a.name).cmp(&group_rank(&b.name)).then(a.name.cmp(&b.name)));
    ExifTags { groups }
}

/// Значение тега по группе и имени (первое совпадение).
pub fn get(tags: &ExifTags, group: &str, tag: &str) -> Option<String> {
    tags.groups
        .iter()
        .find(|g| g.name == group)
        .and_then(|g| g.tags.iter().find(|(t, _)| t == tag))
        .map(|(_, v)| v.clone())
}

/// Группы тегов, доступные на запись через exiftool. Остальные (File/Composite/
/// ExifTool/ICC_Profile/…) — вычисляемые/служебные, редактирование запрещено.
pub const WRITABLE_GROUPS: &[&str] = &["EXIF", "XMP", "IPTC", "GPS"];

/// Можно ли редактировать теги этой группы.
pub fn is_editable(group: &str) -> bool {
    WRITABLE_GROUPS.contains(&group)
}

/// Операция над тегом для записи exiftool.
#[derive(Clone, Debug, PartialEq)]
pub enum TagEdit {
    Set { group: String, tag: String, value: String },
    Delete { group: String, tag: String },
    DeleteAllGps,
}

/// Набор правок → аргументы exiftool (без пути; путь добавляет write::write_edits).
/// `Set` → `-Group:Tag=value`; `Delete` → `-Group:Tag=`; `DeleteAllGps` → `-gps:all=`.
/// Для группы `EXIF` дополнительно очищается IFD1-дубль (`-IFD1:Tag=`): иначе exiftool при
/// чтении `-json -G` на дублированном теге (напр. Fuji зеркалит Artist в thumbnail-IFD) отдаёт
/// устаревшее значение из IFD1, и правка IFD0 «не видна». Для других групп / форматов без IFD1
/// это безопасный no-op (XMP/IPTC/GPS дублей в IFD1 не имеют; отсутствующий тег — удаление-no-op).
pub fn edits_to_args(edits: &[TagEdit]) -> Vec<String> {
    let mut args = Vec::new();
    for e in edits {
        match e {
            TagEdit::Set { group, tag, value } => {
                args.push(format!("-{group}:{tag}={value}"));
                if group == "EXIF" {
                    args.push(format!("-IFD1:{tag}="));
                }
            }
            TagEdit::Delete { group, tag } => {
                args.push(format!("-{group}:{tag}="));
                if group == "EXIF" {
                    args.push(format!("-IFD1:{tag}="));
                }
            }
            TagEdit::DeleteAllGps => args.push("-gps:all=".to_string()),
        }
    }
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"[{
        "SourceFile": "a.jpg",
        "EXIF:Make": "Fujifilm",
        "EXIF:Model": "X-T5",
        "EXIF:ISO": 400,
        "GPS:GPSLatitude": "41.3 N",
        "File:FileSize": "4.9 MB"
    }]"#;

    #[test]
    fn parse_groups_and_values() {
        let t = parse(SAMPLE);
        // File идёт раньше EXIF, EXIF раньше GPS (GROUP_ORDER)
        let names: Vec<&str> = t.groups.iter().map(|g| g.name.as_str()).collect();
        assert_eq!(names, vec!["File", "EXIF", "GPS"]);
        // числовое значение приведено к строке
        let exif = &t.groups[1];
        assert!(exif.tags.contains(&("ISO".to_string(), "400".to_string())));
        // теги внутри группы — по алфавиту
        let exif_tags: Vec<&str> = exif.tags.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(exif_tags, vec!["ISO", "Make", "Model"]);
    }

    #[test]
    fn get_returns_first_match() {
        let t = parse(SAMPLE);
        assert_eq!(get(&t, "EXIF", "Model").as_deref(), Some("X-T5"));
        assert_eq!(get(&t, "EXIF", "Nope"), None);
    }

    #[test]
    fn parse_invalid_is_empty() {
        assert_eq!(parse("not json").groups.len(), 0);
        assert_eq!(parse("[]").groups.len(), 0);
    }

    #[test]
    fn key_without_group_goes_to_other() {
        let t = parse(r#"[{"SourceFile":"a","Weird":"v"}]"#);
        assert_eq!(t.groups.len(), 1);
        assert_eq!(t.groups[0].name, "Other");
    }

    #[test]
    fn editable_groups() {
        assert!(is_editable("EXIF"));
        assert!(is_editable("XMP"));
        assert!(is_editable("IPTC"));
        assert!(is_editable("GPS"));
        assert!(!is_editable("File"));
        assert!(!is_editable("Composite"));
        assert!(!is_editable("ExifTool"));
        assert!(!is_editable("ICC_Profile"));
    }

    #[test]
    fn edits_to_args_set_delete_gps() {
        let edits = vec![
            TagEdit::Set { group: "EXIF".into(), tag: "Artist".into(), value: "Jane".into() },
            TagEdit::Delete { group: "XMP".into(), tag: "Rating".into() },
            TagEdit::DeleteAllGps,
        ];
        let args = edits_to_args(&edits);
        assert_eq!(
            args,
            vec![
                "-EXIF:Artist=Jane".to_string(),
                "-IFD1:Artist=".to_string(),
                "-XMP:Rating=".to_string(),
                "-gps:all=".to_string(),
            ]
        );
    }

    #[test]
    fn edits_to_args_empty_is_empty() {
        assert!(edits_to_args(&[]).is_empty());
    }

    #[test]
    fn edits_to_args_exif_set_clears_ifd1_dup() {
        let edits = vec![TagEdit::Set { group: "EXIF".into(), tag: "Artist".into(), value: "Jane".into() }];
        assert_eq!(
            edits_to_args(&edits),
            vec!["-EXIF:Artist=Jane".to_string(), "-IFD1:Artist=".to_string()]
        );
    }

    #[test]
    fn edits_to_args_exif_delete_clears_ifd1_dup() {
        let edits = vec![TagEdit::Delete { group: "EXIF".into(), tag: "Artist".into() }];
        assert_eq!(
            edits_to_args(&edits),
            vec!["-EXIF:Artist=".to_string(), "-IFD1:Artist=".to_string()]
        );
    }

    #[test]
    fn edits_to_args_non_exif_groups_unchanged() {
        // XMP/IPTC/GPS дублей в IFD1 не имеют — один аргумент на правку
        let edits = vec![
            TagEdit::Set { group: "XMP".into(), tag: "Rating".into(), value: "5".into() },
            TagEdit::Delete { group: "IPTC".into(), tag: "Keywords".into() },
        ];
        assert_eq!(
            edits_to_args(&edits),
            vec!["-XMP:Rating=5".to_string(), "-IPTC:Keywords=".to_string()]
        );
    }
}
