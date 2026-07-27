use std::collections::BTreeMap;

use anyhow::{Result, bail};

const SEASONAL_TAGS: [&str; 2] = ["christmas", "halloween"];

#[derive(Debug, Clone, PartialEq, Eq)]
enum Value {
    Scalar(String),
    Block(BTreeMap<String, Value>),
}

#[derive(Debug, Clone)]
pub struct MapDefinition {
    pub index: usize,
    pub name: String,
    pub tags: Vec<String>,
    pub queue_eligible: bool,
}

fn tokenize(input: &str) -> Result<Vec<String>> {
    let bytes = input.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index].is_ascii_whitespace() {
            index += 1;
        } else if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'/') {
            index += 2;
            while index < bytes.len() && bytes[index] != b'\n' {
                index += 1;
            }
        } else if matches!(bytes[index], b'{' | b'}') {
            tokens.push((bytes[index] as char).to_string());
            index += 1;
        } else if bytes[index] == b'"' {
            index += 1;
            let mut value = Vec::new();
            while index < bytes.len() && bytes[index] != b'"' {
                if bytes[index] == b'\\' && index + 1 < bytes.len() {
                    index += 1;
                }
                value.push(bytes[index]);
                index += 1;
            }
            if index == bytes.len() {
                bail!("unterminated quoted KeyValues string");
            }
            index += 1;
            tokens.push(String::from_utf8(value).map_err(|error| {
                anyhow::anyhow!("quoted KeyValues string is not valid UTF-8: {error}")
            })?);
        } else {
            let start = index;
            while index < bytes.len()
                && !bytes[index].is_ascii_whitespace()
                && !matches!(bytes[index], b'{' | b'}')
            {
                index += 1;
            }
            tokens.push(String::from_utf8_lossy(&bytes[start..index]).into_owned());
        }
    }
    Ok(tokens)
}

fn master_maps_fragment(items_game: &str) -> Result<&str> {
    let name_offset = items_game
        .find("master_maps_list")
        .ok_or_else(|| anyhow::anyhow!("items_game.txt has no master_maps_list block"))?;
    let bytes = items_game.as_bytes();
    let start = bytes[name_offset..]
        .iter()
        .position(|byte| *byte == b'{')
        .map(|offset| name_offset + offset)
        .ok_or_else(|| anyhow::anyhow!("master_maps_list has no opening brace"))?;
    let mut depth = 0_u32;
    let mut quoted = false;
    let mut escaped = false;
    let mut line_comment = false;
    let mut index = start;
    while index < bytes.len() {
        let byte = bytes[index];
        if line_comment {
            if byte == b'\n' {
                line_comment = false;
            }
        } else if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
        } else if byte == b'"' {
            quoted = true;
        } else if byte == b'/' && bytes.get(index + 1) == Some(&b'/') {
            line_comment = true;
            index += 1;
        } else if byte == b'{' {
            depth += 1;
        } else if byte == b'}' {
            depth = depth
                .checked_sub(1)
                .ok_or_else(|| anyhow::anyhow!("unexpected KeyValues block end"))?;
            if depth == 0 {
                return Ok(&items_game[start..=index]);
            }
        }
        index += 1;
    }
    bail!("unterminated master_maps_list block")
}

fn parse_block(tokens: &[String], offset: &mut usize) -> Result<BTreeMap<String, Value>> {
    if tokens.get(*offset).map(String::as_str) != Some("{") {
        bail!("expected KeyValues block");
    }
    *offset += 1;
    let mut entries = BTreeMap::new();
    while tokens.get(*offset).map(String::as_str) != Some("}") {
        let key = tokens
            .get(*offset)
            .ok_or_else(|| anyhow::anyhow!("unterminated KeyValues block"))?
            .clone();
        *offset += 1;
        let token = tokens
            .get(*offset)
            .ok_or_else(|| anyhow::anyhow!("missing KeyValues value"))?;
        if token == "{" {
            entries.insert(key, Value::Block(parse_block(tokens, offset)?));
        } else if token == "}" {
            bail!("unexpected KeyValues block end");
        } else {
            entries.insert(key, Value::Scalar(token.clone()));
            *offset += 1;
        }
    }
    *offset += 1;
    Ok(entries)
}

fn as_block(value: Option<&Value>) -> Option<&BTreeMap<String, Value>> {
    match value {
        Some(Value::Block(block)) => Some(block),
        _ => None,
    }
}

fn scalar<'a>(block: &'a BTreeMap<String, Value>, key: &str) -> Option<&'a str> {
    match block.get(key) {
        Some(Value::Scalar(value)) => Some(value),
        _ => None,
    }
}

pub fn parse_master_maps_list(items_game: &str) -> Result<Vec<MapDefinition>> {
    let tokens = tokenize(master_maps_fragment(items_game)?)?;
    let mut offset = 0;
    let master = parse_block(&tokens, &mut offset)?;
    let mut maps = Vec::new();
    for (index_text, value) in master {
        let Ok(index) = index_text.parse::<usize>() else {
            continue;
        };
        let Value::Block(map) = value else {
            continue;
        };
        let Some(name) = scalar(&map, "name") else {
            continue;
        };
        let tags = as_block(map.get("rolling_match_tags"))
            .map(|tag_block| {
                tag_block
                    .iter()
                    .filter_map(|(tag, value)| match value {
                        Value::Scalar(enabled)
                            if enabled != "0" && !enabled.eq_ignore_ascii_case("false") =>
                        {
                            Some(tag.to_ascii_lowercase())
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        maps.push(MapDefinition {
            index,
            name: name.to_owned(),
            queue_eligible: index > 0 && name != "Missing" && !tags.is_empty(),
            tags,
        });
    }
    maps.sort_by_key(|map| map.index);
    Ok(maps)
}

pub fn is_seasonal(map: &MapDefinition) -> bool {
    map.tags
        .iter()
        .any(|tag| SEASONAL_TAGS.contains(&tag.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_master_map_catalogue() {
        let maps = parse_master_maps_list(
            r#""items_game" { "master_maps_list" {
                "0" { "name" "Missing" }
                "1" { "name" "pl_badwater" "rolling_match_tags" { "payload" "1" "disabled" "0" } }
            } }"#,
        )
        .unwrap();
        assert_eq!(maps.len(), 2);
        assert_eq!(maps[1].name, "pl_badwater");
        assert_eq!(maps[1].tags, ["payload"]);
        assert!(maps[1].queue_eligible);
    }

    #[test]
    fn isolates_the_master_block_and_preserves_utf8() {
        let maps = parse_master_maps_list(
            r#""ignored" { "large" { "prefix" "value" } }
            "master_maps_list" {
                "1" { "name" "cp_café" "rolling_match_tags" { "control_point" "1" } }
            }
            "ignored_after" { "broken-looking" "{ still quoted }" }"#,
        )
        .unwrap();
        assert_eq!(maps.len(), 1);
        assert_eq!(maps[0].name, "cp_café");
    }
}
