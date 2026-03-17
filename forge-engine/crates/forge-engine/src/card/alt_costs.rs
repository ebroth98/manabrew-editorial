//! Alternative cost getters for CardInstance.
//!
//! These methods extract cost information from keywords following the "Keyword:cost" pattern.
//! They're used by the casting/playability system to present alternative casting options.

use super::CardInstance;

impl CardInstance {
    // ── Keyword cost helpers (pattern: "Keyword:cost_string") ────────

    /// Get buyback cost (e.g. "Buyback:2" → Some("2")).
    pub fn get_buyback_cost(&self) -> Option<String> {
        self.get_keyword_cost("Buyback")
    }

    /// Get spectacle cost (e.g. "Spectacle:B R" → Some("B R")).
    pub fn get_spectacle_cost(&self) -> Option<String> {
        self.get_keyword_cost("Spectacle")
    }

    /// Get GainLife alternative cost info.
    ///
    /// Stored as keyword `AltCostGainLife:N:IsPresent` where N is the life amount
    /// and IsPresent is the condition string (e.g. `Forest.YouCtrl`).
    /// Returns `Some((life_amount, condition))` if present.
    pub fn get_gainlife_alt_cost(&self) -> Option<(i32, String)> {
        for kw in self.keywords.iter().chain(self.granted_keywords.iter()) {
            if let Some(rest) = kw.strip_prefix("AltCostGainLife:") {
                let mut parts = rest.splitn(2, ':');
                let amount = parts
                    .next()
                    .and_then(|s| s.parse::<i32>().ok())
                    .unwrap_or(0);
                let condition = parts.next().unwrap_or("").to_string();
                return Some((amount, condition));
            }
        }
        None
    }

    /// Get evoke cost (e.g. "Evoke:2 B" → Some("2 B")).
    pub fn get_evoke_cost(&self) -> Option<String> {
        self.get_keyword_cost("Evoke")
    }

    /// Get bestow cost (e.g. "Bestow:3 G G" → Some("3 G G")).
    pub fn get_bestow_cost(&self) -> Option<String> {
        self.get_keyword_cost("Bestow")
    }

    /// Get dash cost (e.g. "Dash:1 R" → Some("1 R")).
    pub fn get_dash_cost(&self) -> Option<String> {
        self.get_keyword_cost("Dash")
    }

    /// Get blitz cost (e.g. "Blitz:1 R" → Some("1 R")).
    pub fn get_blitz_cost(&self) -> Option<String> {
        self.get_keyword_cost("Blitz")
    }

    /// Get warp cost (e.g. "Warp:1 B" → Some("1 B")).
    pub fn get_warp_cost(&self) -> Option<String> {
        self.get_keyword_cost("Warp")
    }

    /// Get multikicker cost (e.g. "Multikicker:1 G" → Some("1 G")).
    pub fn get_multikicker_cost(&self) -> Option<String> {
        self.get_keyword_cost("Multikicker")
    }

    /// Get replicate cost (e.g. "Replicate:U" → Some("U")).
    pub fn get_replicate_cost(&self) -> Option<String> {
        self.get_keyword_cost("Replicate")
    }

    /// Get entwine cost (e.g. "Entwine:2" → Some("2")).
    pub fn get_entwine_cost(&self) -> Option<String> {
        self.get_keyword_cost("Entwine")
    }

    /// Get escalate cost (e.g. "Escalate:1" → Some("1")).
    pub fn get_escalate_cost(&self) -> Option<String> {
        self.get_keyword_cost("Escalate")
    }

    /// Get escape cost and exile count (e.g. "Escape:1 B B:4" → Some(("1 B B", 4))).
    pub fn get_escape_cost(&self) -> Option<(String, i32)> {
        for kw in self.keywords.iter().chain(self.granted_keywords.iter()) {
            if let Some(rest) = kw.strip_prefix("Escape:") {
                // Format: "mana_cost:exile_count"
                if let Some(colon_pos) = rest.rfind(':') {
                    let mana = rest[..colon_pos].trim().to_string();
                    let exile = rest[colon_pos + 1..].trim().parse().unwrap_or(0);
                    return Some((mana, exile));
                }
            }
        }
        None
    }

    /// Get overload cost (e.g. "Overload:3 R" → Some("3 R")).
    pub fn get_overload_cost(&self) -> Option<String> {
        self.get_keyword_cost("Overload")
    }

    /// Get madness cost (e.g. "Madness:1 R" → Some("1 R")).
    pub fn get_madness_cost(&self) -> Option<String> {
        self.get_keyword_cost("Madness")
    }

    /// Get strive cost (e.g. "Strive:1 W" → Some("1 W")).
    pub fn get_strive_cost(&self) -> Option<String> {
        self.get_keyword_cost("Strive")
    }

    /// Get suspend cost and time counters (e.g. "Suspend:1 U:3" → Some(("1 U", 3))).
    pub fn get_suspend_cost(&self) -> Option<(String, i32)> {
        for kw in self.keywords.iter().chain(self.granted_keywords.iter()) {
            if let Some(rest) = kw.strip_prefix("Suspend:") {
                if let Some(colon_pos) = rest.rfind(':') {
                    let mana = rest[..colon_pos].trim().to_string();
                    let counters = rest[colon_pos + 1..].trim().parse().unwrap_or(0);
                    return Some((mana, counters));
                }
            }
        }
        None
    }

    /// Get foretell cost (e.g. "Foretell:W W" → Some("W W")).
    pub fn get_foretell_cost(&self) -> Option<String> {
        self.get_keyword_cost("Foretell")
    }

    /// Get emerge cost (e.g. "Emerge:5 U U" → Some("5 U U")).
    pub fn get_emerge_cost(&self) -> Option<String> {
        self.get_keyword_cost("Emerge")
    }

    /// Get offering type (e.g. "Offering:Snake" → Some("Snake")).
    pub fn get_offering_type(&self) -> Option<String> {
        self.get_keyword_cost("Offering")
    }

    /// Generic keyword cost parser — looks for "Keyword:cost" in keywords vec.
    pub fn get_keyword_cost(&self, keyword: &str) -> Option<String> {
        let prefix = format!("{}:", keyword);
        for kw in self.keywords.iter().chain(self.granted_keywords.iter()) {
            if let Some(cost) = kw.strip_prefix(&prefix) {
                return Some(cost.to_string());
            }
        }
        None
    }

    /// Get Ward cost (e.g. "Ward:2" → Some("2"), "Ward:{U}" → Some("{U}")).
    pub fn get_ward_cost(&self) -> Option<String> {
        for kw in self.keywords.iter().chain(self.granted_keywords.iter()) {
            if let Some(cost) = kw.strip_prefix("Ward:") {
                return Some(cost.to_string());
            }
        }
        None
    }

    /// Get Flashback cost (e.g. "Flashback:2 R" → Some("2 R")).
    pub fn get_flashback_cost(&self) -> Option<String> {
        self.get_keyword_cost("Flashback")
    }

    /// Get Kicker cost (e.g. "Kicker:W" → Some("W")).
    pub fn get_kicker_cost(&self) -> Option<String> {
        self.get_keyword_cost("Kicker")
    }
}
