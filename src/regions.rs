use serde::Serialize;

/// TF2 calls each data center's value `health_ratio`. It moves in steps of 1/800 and stops at
/// 1.0, so this tool reads ratio times 800 as an approximate number searching, and 800 as
/// "at least 800". See docs/protocol.md.
pub const SEARCHING_LIMIT: u32 = 800;

// Groups follow TF2's ETFMatchGroup enum: 0 MvM Practice (Boot Camp), 1 MvM Mann Up,
// 2 to 4 Ladder (Competitive 6v6, 9v9, 12v12), 5 to 7 Casual (6v6, 9v9, 12v12), 8 Event
// placeholder. Only 0, 1, 2 and 7 are in use, so the others are left out.
const BOOT_CAMP_GROUP: usize = 0;
const MANN_UP_GROUP: usize = 1;
const COMPETITIVE_GROUP: usize = 2;
const CASUAL_GROUP: usize = 7;

const DATA_CENTERS: &[(&str, &str, &str)] = &[
    ("sea", "Seattle", "us"),
    ("lax", "Los Angeles", "us"),
    ("ord", "Chicago", "us"),
    ("dfw", "Dallas", "us"),
    ("atl", "Atlanta", "us"),
    ("iad", "Virginia", "us"),
    ("eat", "Moses Lake", "us"),
    ("lim", "Lima", "pe"),
    ("gru", "São Paulo", "br"),
    ("scl", "Santiago", "cl"),
    ("eze", "Buenos Aires", "ar"),
    ("lhr", "London", "gb"),
    ("ams", "Amsterdam", "nl"),
    ("par", "Paris", "fr"),
    ("mad", "Madrid", "es"),
    ("fra", "Frankfurt", "de"),
    ("fsn", "Falkenstein", "de"),
    ("vie", "Vienna", "at"),
    ("waw", "Warsaw", "pl"),
    ("sto", "Stockholm", "se"),
    ("hel", "Helsinki", "fi"),
    ("dxb", "Dubai", "ae"),
    ("jnb", "Johannesburg", "za"),
    ("maa", "Chennai", "in"),
    ("bom", "Mumbai", "in"),
    ("sgp", "Singapore", "sg"),
    ("hkg", "Hong Kong", "hk"),
    ("seo", "Seoul", "kr"),
    ("tyo", "Tokyo", "jp"),
    ("syd", "Sydney", "au"),
];

#[derive(Debug, Clone, Serialize)]
pub struct DataCenterStat {
    pub data_center: String,
    pub city: Option<String>,
    pub country: Option<String>,
    /// Approximate number of players searching; `SEARCHING_LIMIT` when `at_limit` is true.
    pub searching: u32,
    pub at_limit: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Regions {
    pub casual: Vec<DataCenterStat>,
    pub mann_up: Vec<DataCenterStat>,
    pub mvm_boot_camp: Vec<DataCenterStat>,
    pub competitive: Vec<DataCenterStat>,
}

impl Regions {
    /// Every matchmaking type with its stable id, in display order.
    pub fn groups(&self) -> [(&'static str, &[DataCenterStat]); 4] {
        [
            ("casual", &self.casual),
            ("mann_up", &self.mann_up),
            ("mvm_boot_camp", &self.mvm_boot_camp),
            ("competitive", &self.competitive),
        ]
    }
}

fn data_center_stat(code: &str, health_ratio: f32) -> Option<DataCenterStat> {
    // NaN fails the range check too.
    if code.is_empty() || !(0.0..=1.0).contains(&health_ratio) {
        return None;
    }
    // Checked to be within 0.0..=1.0 above, so the product fits in u32 without sign loss.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let searching = (f64::from(health_ratio) * f64::from(SEARCHING_LIMIT)).round() as u32;
    if searching == 0 {
        return None;
    }
    // "maa2" and "bom2" are second sites in the same city as "maa" and "bom".
    let base = code.trim_end_matches(|character: char| character.is_ascii_digit());
    let known = DATA_CENTERS.iter().find(|(known, _, _)| *known == base);
    Some(DataCenterStat {
        data_center: code.to_owned(),
        city: known.map(|(_, city, _)| (*city).to_owned()),
        country: known.map(|(_, _, country)| (*country).to_owned()),
        searching,
        at_limit: searching >= SEARCHING_LIMIT,
    })
}

/// Turns decoded match groups into per data center numbers, busiest first.
/// Returns None when there are too few groups to hold Casual, so a protocol change
/// leaves regions out instead of mislabeling them.
pub fn build_regions(groups: &[Vec<(String, f32)>]) -> Option<Regions> {
    if groups.len() <= CASUAL_GROUP {
        return None;
    }
    let group = |index: usize| {
        let mut stats = groups[index]
            .iter()
            .filter_map(|(code, ratio)| data_center_stat(code, *ratio))
            .collect::<Vec<_>>();
        stats.sort_by(|left, right| {
            right
                .searching
                .cmp(&left.searching)
                .then_with(|| left.data_center.cmp(&right.data_center))
        });
        stats
    };
    Some(Regions {
        casual: group(CASUAL_GROUP),
        mann_up: group(MANN_UP_GROUP),
        mvm_boot_camp: group(BOOT_CAMP_GROUP),
        competitive: group(COMPETITIVE_GROUP),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries(values: &[(&str, f32)]) -> Vec<(String, f32)> {
        values
            .iter()
            .map(|(code, ratio)| ((*code).to_owned(), *ratio))
            .collect()
    }

    #[test]
    fn maps_match_groups_and_scales_ratios() {
        let mut groups = vec![Vec::new(); 9];
        groups[0] = entries(&[("fra", 0.00375)]);
        groups[1] = entries(&[("lhr", 0.0025)]);
        groups[2] = entries(&[("sto", 0.00125)]);
        groups[7] = entries(&[("eat", 0.5), ("maa2", 1.0), ("syd", 0.0), ("", 0.5)]);
        let regions = build_regions(&groups).unwrap();
        let casual = &regions.casual;
        assert_eq!(casual.len(), 2);
        assert_eq!(casual[0].data_center, "maa2");
        assert_eq!(casual[0].city.as_deref(), Some("Chennai"));
        assert!(casual[0].at_limit);
        assert_eq!(casual[1].searching, 400);
        assert!(!casual[1].at_limit);
        assert_eq!(regions.mvm_boot_camp[0].searching, 3);
        assert_eq!(regions.mann_up[0].searching, 2);
        assert_eq!(regions.competitive[0].searching, 1);
    }

    #[test]
    fn rejects_unexpected_layouts_and_values() {
        assert!(build_regions(&[Vec::new(), Vec::new(), Vec::new()]).is_none());
        let mut groups = vec![Vec::new(); 8];
        groups[7] = entries(&[
            ("fra", 1.5),
            ("sto", 1.001),
            ("lhr", f32::NAN),
            ("Pw-New", 0.01),
        ]);
        let casual = build_regions(&groups).unwrap().casual;
        assert_eq!(casual.len(), 1);
        assert_eq!(casual[0].data_center, "Pw-New");
        assert_eq!(casual[0].city, None);
    }
}
