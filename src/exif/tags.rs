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
}
