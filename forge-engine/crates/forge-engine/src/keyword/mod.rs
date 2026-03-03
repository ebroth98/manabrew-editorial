/// Keyword cost parsing utilities.
/// Mirrors Java's KeywordInterface + specific keyword parsers.

/// Info about a card's kicker cost(s).
#[derive(Debug, Clone)]
pub struct KickerInfo {
    /// First kicker cost string (e.g. "1 R").
    pub cost1: String,
    /// Optional second kicker cost string for cards with two kicker costs.
    pub cost2: Option<String>,
}

/// Info about a card's escape cost.
#[derive(Debug, Clone)]
pub struct EscapeInfo {
    /// Mana cost for escape (e.g. "3 B B").
    pub mana_cost: String,
    /// Number of other cards to exile from graveyard.
    pub exile_count: i32,
}

/// Parse a keyword cost from a card's keywords list.
/// E.g. keywords contains "Flashback:2 R", name = "Flashback" -> Some("2 R")
pub fn parse_keyword_cost(keywords: &[String], name: &str) -> Option<String> {
    let prefix = format!("{}:", name);
    for kw in keywords {
        if let Some(cost) = kw.strip_prefix(&prefix) {
            return Some(cost.to_string());
        }
    }
    None
}

/// Parse kicker info from keywords.
/// Supports single kicker ("Kicker:1 R") and double kicker ("Kicker:1 R:1 G").
pub fn parse_kicker(keywords: &[String]) -> Option<KickerInfo> {
    for kw in keywords {
        if let Some(rest) = kw.strip_prefix("Kicker:") {
            // Check for double kicker (two costs separated by ":")
            // E.g. "1 R:1 G"
            let parts: Vec<&str> = rest.splitn(2, ':').collect();
            if parts.len() == 2 {
                return Some(KickerInfo {
                    cost1: parts[0].to_string(),
                    cost2: Some(parts[1].to_string()),
                });
            } else {
                return Some(KickerInfo {
                    cost1: rest.to_string(),
                    cost2: None,
                });
            }
        }
    }
    None
}

/// Parse escape info from keywords.
/// Format: "Escape:MANA_COST:EXILE_COUNT" e.g. "Escape:3 B B:4"
pub fn parse_escape(keywords: &[String]) -> Option<EscapeInfo> {
    for kw in keywords {
        if let Some(rest) = kw.strip_prefix("Escape:") {
            // Split from right to find the exile count (last segment)
            if let Some(last_colon) = rest.rfind(':') {
                let mana_cost = &rest[..last_colon];
                let exile_str = &rest[last_colon + 1..];
                let exile_count = exile_str.parse::<i32>().unwrap_or(0);
                return Some(EscapeInfo {
                    mana_cost: mana_cost.to_string(),
                    exile_count,
                });
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_keyword_cost() {
        let keywords = vec!["Flashback:2 R".to_string(), "Flying".to_string()];
        assert_eq!(
            parse_keyword_cost(&keywords, "Flashback"),
            Some("2 R".to_string())
        );
        assert_eq!(parse_keyword_cost(&keywords, "Evoke"), None);
    }

    #[test]
    fn test_parse_kicker_single() {
        let keywords = vec!["Kicker:1 R".to_string()];
        let info = parse_kicker(&keywords).unwrap();
        assert_eq!(info.cost1, "1 R");
        assert!(info.cost2.is_none());
    }

    #[test]
    fn test_parse_kicker_double() {
        let keywords = vec!["Kicker:1 R:1 G".to_string()];
        let info = parse_kicker(&keywords).unwrap();
        assert_eq!(info.cost1, "1 R");
        assert_eq!(info.cost2, Some("1 G".to_string()));
    }

    #[test]
    fn test_parse_escape() {
        let keywords = vec!["Escape:3 B B:4".to_string()];
        let info = parse_escape(&keywords).unwrap();
        assert_eq!(info.mana_cost, "3 B B");
        assert_eq!(info.exile_count, 4);
    }
}
