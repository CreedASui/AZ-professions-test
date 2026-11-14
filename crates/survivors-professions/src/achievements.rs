//! Achievements: what a run (or every run so far) has to do to earn achievement points.
//!
//! Each is earned once per account. Most are fixed ([`GENERAL`]); one per class and two per zone
//! are made from the game's own class and zone lists by [`catalog`], so a new zone brings its
//! achievements with it.

/// One run as it ended, the only input an achievement is judged on.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RunRecord {
    /// The class's `ChrClasses.dbc` id.
    pub class_id: u8,
    /// An index into the game's zone list.
    pub zone: usize,
    pub seconds: f32,
    pub level: u32,
    pub kills: u32,
    /// Bosses killed, not merely met.
    pub bosses_slain: u32,
    pub won: bool,
}

/// Every run on the account added up.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Lifetime {
    /// Runs of at least [`COUNTED_RUN_SECS`]; a run quit at once does not count.
    pub runs: u32,
    pub kills: u64,
    pub bosses_slain: u64,
    pub seconds: f64,
    pub wins: u32,
}

/// The shortest run [`Lifetime::runs`] counts, so starting and quitting earns nothing.
pub const COUNTED_RUN_SECS: f32 = 60.0;

impl Lifetime {
    pub fn add(&mut self, run: &RunRecord) {
        if run.seconds >= COUNTED_RUN_SECS {
            self.runs += 1;
        }
        self.kills += u64::from(run.kills);
        self.bosses_slain += u64::from(run.bosses_slain);
        self.seconds += f64::from(run.seconds.max(0.0));
        if run.won {
            self.wins += 1;
        }
    }
}

/// What earns an achievement.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Condition {
    /// In one run: survive this many seconds.
    Survive(f32),
    /// In one run: reach this level.
    Level(u32),
    /// In one run: kill this many enemies.
    RunKills(u32),
    /// In one run: kill this many bosses.
    RunBosses(u32),
    /// Win a run.
    Win,
    /// Over every run: enemies killed.
    TotalKills(u64),
    /// Over every run: bosses killed.
    TotalBosses(u64),
    /// Over every run: runs played (see [`COUNTED_RUN_SECS`]).
    TotalRuns(u32),
    /// Over every run: seconds survived.
    TotalTime(f64),
    /// Over every run: runs won.
    TotalWins(u32),
    /// Win a run as the class with this id.
    WinAsClass(u8),
    /// Win a run in the zone at this index.
    WinInZone(usize),
    /// In one run in the zone at this index: survive this many seconds.
    SurviveInZone(usize, f32),
}

impl Condition {
    /// Whether `run`, with `life` already counting it, meets the condition.
    pub fn met(&self, run: &RunRecord, life: &Lifetime) -> bool {
        match *self {
            Condition::Survive(s) => run.seconds >= s,
            Condition::Level(l) => run.level >= l,
            Condition::RunKills(k) => run.kills >= k,
            Condition::RunBosses(b) => run.bosses_slain >= b,
            Condition::Win => run.won,
            Condition::TotalKills(k) => life.kills >= k,
            Condition::TotalBosses(b) => life.bosses_slain >= b,
            Condition::TotalRuns(n) => life.runs >= n,
            Condition::TotalTime(s) => life.seconds >= s,
            Condition::TotalWins(n) => life.wins >= n,
            Condition::WinAsClass(c) => run.won && run.class_id == c,
            Condition::WinInZone(z) => run.won && run.zone == z,
            Condition::SurviveInZone(z, s) => run.zone == z && run.seconds >= s,
        }
    }
}

/// Where an achievement is listed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    /// One great run.
    Survival,
    /// Every run added up.
    Veteran,
    /// One per class.
    Classes,
    /// Two per zone.
    Zones,
}

impl Category {
    pub fn label(self) -> &'static str {
        match self {
            Category::Survival => "Survival",
            Category::Veteran => "Veteran",
            Category::Classes => "Classes",
            Category::Zones => "Zones",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Achievement {
    /// Stable id in the save file; never reuse or rename one.
    pub key: String,
    pub name: String,
    pub description: String,
    pub points: u32,
    pub category: Category,
    pub condition: Condition,
}

/// A fixed achievement: `(key, name, description, points, category, condition)`.
type General = (
    &'static str,
    &'static str,
    &'static str,
    u32,
    Category,
    Condition,
);

const MIN: f32 = 60.0;
const HOUR: f64 = 3600.0;

/// The achievements every build of the game has, whatever its classes and zones.
#[rustfmt::skip]
pub const GENERAL: &[General] = &[
    // One run.
    ("survive-3m", "Holding the Line", "Survive 3 minutes in one run.", 10, Category::Survival, Condition::Survive(3.0 * MIN)),
    ("survive-5m", "Still Standing", "Survive 5 minutes in one run.", 10, Category::Survival, Condition::Survive(5.0 * MIN)),
    ("survive-10m", "Against the Tide", "Survive 10 minutes in one run.", 20, Category::Survival, Condition::Survive(10.0 * MIN)),
    ("win", "Survivor", "Win a run.", 25, Category::Survival, Condition::Win),
    ("level-10", "Rising Hero", "Reach level 10 in one run.", 10, Category::Survival, Condition::Level(10)),
    ("level-20", "Seasoned Hero", "Reach level 20 in one run.", 10, Category::Survival, Condition::Level(20)),
    ("level-30", "Legendary Hero", "Reach level 30 in one run.", 20, Category::Survival, Condition::Level(30)),
    ("kills-500", "Slayer", "Kill 500 enemies in one run.", 10, Category::Survival, Condition::RunKills(500)),
    ("kills-1500", "Butcher", "Kill 1,500 enemies in one run.", 20, Category::Survival, Condition::RunKills(1500)),
    ("kills-3000", "Unstoppable", "Kill 3,000 enemies in one run.", 25, Category::Survival, Condition::RunKills(3000)),
    ("boss-1", "Giant Slayer", "Kill a boss.", 10, Category::Survival, Condition::RunBosses(1)),
    ("boss-3", "Boss Hunter", "Kill 3 bosses in one run.", 10, Category::Survival, Condition::RunBosses(3)),
    ("boss-5", "Kingslayer", "Kill 5 bosses in one run.", 25, Category::Survival, Condition::RunBosses(5)),
    // Every run.
    ("total-kills-1k", "A Thousand Fallen", "Kill 1,000 enemies in total.", 10, Category::Veteran, Condition::TotalKills(1_000)),
    ("total-kills-10k", "Ten Thousand Fallen", "Kill 10,000 enemies in total.", 20, Category::Veteran, Condition::TotalKills(10_000)),
    ("total-kills-50k", "The Horde Thins", "Kill 50,000 enemies in total.", 25, Category::Veteran, Condition::TotalKills(50_000)),
    ("total-kills-100k", "Bane of the Horde", "Kill 100,000 enemies in total.", 50, Category::Veteran, Condition::TotalKills(100_000)),
    ("total-bosses-10", "Bounty Hunter", "Kill 10 bosses in total.", 10, Category::Veteran, Condition::TotalBosses(10)),
    ("total-bosses-50", "Raid Leader", "Kill 50 bosses in total.", 20, Category::Veteran, Condition::TotalBosses(50)),
    ("total-bosses-150", "Legend of the Arena", "Kill 150 bosses in total.", 25, Category::Veteran, Condition::TotalBosses(150)),
    ("total-runs-10", "Back for More", "Play 10 runs of at least a minute.", 10, Category::Veteran, Condition::TotalRuns(10)),
    ("total-runs-50", "Glutton for Punishment", "Play 50 runs of at least a minute.", 20, Category::Veteran, Condition::TotalRuns(50)),
    ("total-runs-100", "One More Run", "Play 100 runs of at least a minute.", 25, Category::Veteran, Condition::TotalRuns(100)),
    ("total-time-1h", "Time Well Spent", "Survive 1 hour in total.", 10, Category::Veteran, Condition::TotalTime(HOUR)),
    ("total-time-5h", "Dedicated", "Survive 5 hours in total.", 20, Category::Veteran, Condition::TotalTime(5.0 * HOUR)),
    ("total-time-10h", "Devoted", "Survive 10 hours in total.", 25, Category::Veteran, Condition::TotalTime(10.0 * HOUR)),
    ("total-time-24h", "A Day in Azeroth", "Survive 24 hours in total.", 50, Category::Veteran, Condition::TotalTime(24.0 * HOUR)),
    ("total-wins-5", "Proven", "Win 5 runs.", 10, Category::Veteran, Condition::TotalWins(5)),
    ("total-wins-25", "Champion", "Win 25 runs.", 25, Category::Veteran, Condition::TotalWins(25)),
    ("total-wins-100", "Undying", "Win 100 runs.", 50, Category::Veteran, Condition::TotalWins(100)),
];

/// Points for winning as a class, and for winning in and exploring a zone.
pub const CLASS_WIN_POINTS: u32 = 10;
pub const ZONE_WIN_POINTS: u32 = 10;
pub const ZONE_SURVIVE_POINTS: u32 = 5;
/// How long a zone's exploring achievement asks to survive there.
pub const ZONE_SURVIVE_SECS: f32 = 5.0 * MIN;

/// `Gates of Ahn'Qiraj` -> `gates-of-ahnqiraj`: a name as a save-file key.
pub fn slug(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for c in name.chars().filter(|c| *c != '\'') {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

fn article(word: &str) -> &'static str {
    match word.chars().next().map(|c| c.to_ascii_lowercase()) {
        Some('a' | 'e' | 'i' | 'o' | 'u') => "an",
        _ => "a",
    }
}

/// Every achievement: [`GENERAL`], then one per class (`(id, name)`), then two per zone, in the
/// order the game lists them.
pub fn catalog(classes: &[(u8, &str)], zones: &[&str]) -> Vec<Achievement> {
    let mut out: Vec<Achievement> = GENERAL
        .iter()
        .map(
            |&(key, name, description, points, category, condition)| Achievement {
                key: key.into(),
                name: name.into(),
                description: description.into(),
                points,
                category,
                condition,
            },
        )
        .collect();
    for &(id, name) in classes {
        out.push(Achievement {
            key: format!("win-as-{}", slug(name)),
            name: format!("{name} Champion"),
            description: format!("Win a run as {} {name}.", article(name)),
            points: CLASS_WIN_POINTS,
            category: Category::Classes,
            condition: Condition::WinAsClass(id),
        });
    }
    for (i, &zone) in zones.iter().enumerate() {
        out.push(Achievement {
            key: format!("survive-in-{}", slug(zone)),
            name: format!("Explorer: {zone}"),
            description: format!("Survive 5 minutes in {zone}."),
            points: ZONE_SURVIVE_POINTS,
            category: Category::Zones,
            condition: Condition::SurviveInZone(i, ZONE_SURVIVE_SECS),
        });
        out.push(Achievement {
            key: format!("win-in-{}", slug(zone)),
            name: format!("Conqueror: {zone}"),
            description: format!("Win a run in {zone}."),
            points: ZONE_WIN_POINTS,
            category: Category::Zones,
            condition: Condition::WinInZone(i),
        });
    }
    out
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{Bracket, PROFESSIONS};
    use std::collections::HashSet;

    /// The game's nine classes and sixteen zones, as `data::CLASSES` and `data::MAPS` list them.
    pub(crate) const CLASSES: &[(u8, &str)] = &[
        (1, "Warrior"),
        (2, "Paladin"),
        (3, "Hunter"),
        (4, "Rogue"),
        (5, "Priest"),
        (7, "Shaman"),
        (8, "Mage"),
        (9, "Warlock"),
        (11, "Druid"),
    ];
    pub(crate) const ZONES: &[&str] = &[
        "The Barrens",
        "The Dark Portal",
        "Gates of Ahn'Qiraj",
        "Fire Plume Ridge",
        "Gurubashi Arena",
        "Elwynn Forest",
        "Durotar",
        "Teldrassil",
        "Dun Morogh",
        "Mulgore",
        "Tirisfal Glades",
        "Westfall",
        "Duskwood",
        "Redridge Mountains",
        "Mount Hyjal",
        "Naxxramas",
    ];

    #[test]
    fn slugs_are_stable_save_keys() {
        assert_eq!(slug("Gates of Ahn'Qiraj"), "gates-of-ahnqiraj");
        assert_eq!(slug("The Barrens"), "the-barrens");
        assert_eq!(slug("  Dun  Morogh! "), "dun-morogh");
    }

    #[test]
    fn keys_are_unique() {
        let all = catalog(CLASSES, ZONES);
        let mut seen = HashSet::new();
        for a in &all {
            assert!(
                seen.insert(a.key.clone()),
                "duplicate achievement {}",
                a.key
            );
            assert!(!a.key.contains(char::is_whitespace));
        }
        assert_eq!(all.len(), GENERAL.len() + CLASSES.len() + 2 * ZONES.len());
    }

    /// A player who does everything can unlock every bracket and craft every recipe.
    #[test]
    fn there_are_enough_points_to_master_everything() {
        let earnable: u32 = catalog(CLASSES, ZONES).iter().map(|a| a.points).sum();
        let recipes: u32 = PROFESSIONS
            .iter()
            .flat_map(|p| p.recipes)
            .map(|r| r.cost)
            .sum();
        let brackets: u32 = Bracket::ALL.iter().map(|b| b.cost()).sum();
        assert_eq!((recipes, brackets), (640, 300));
        assert_eq!(earnable, 940);
        assert!(earnable >= recipes + brackets);
    }

    #[test]
    fn class_text_reads_naturally() {
        let all = catalog(&[(8, "Mage"), (99, "Illusionist")], &[]);
        let n = all.len();
        assert_eq!(all[n - 2].description, "Win a run as a Mage.");
        assert_eq!(all[n - 1].description, "Win a run as an Illusionist.");
    }

    #[test]
    fn conditions_judge_the_run_and_the_lifetime() {
        let run = RunRecord {
            class_id: 8,
            zone: 5,
            seconds: 320.0,
            level: 12,
            kills: 640,
            bosses_slain: 2,
            won: false,
        };
        let mut life = Lifetime::default();
        life.add(&run);
        assert!(Condition::Survive(300.0).met(&run, &life));
        assert!(!Condition::Survive(600.0).met(&run, &life));
        assert!(Condition::Level(10).met(&run, &life));
        assert!(Condition::RunKills(500).met(&run, &life));
        assert!(!Condition::RunBosses(3).met(&run, &life));
        assert!(!Condition::Win.met(&run, &life));
        assert!(!Condition::WinAsClass(8).met(&run, &life));
        assert!(Condition::SurviveInZone(5, 300.0).met(&run, &life));
        assert!(!Condition::SurviveInZone(4, 300.0).met(&run, &life));
        assert!(Condition::TotalRuns(1).met(&run, &life));

        let won = RunRecord {
            won: true,
            seconds: 900.0,
            ..run
        };
        assert!(Condition::WinAsClass(8).met(&won, &life));
        assert!(Condition::WinInZone(5).met(&won, &life));
        assert!(!Condition::WinInZone(6).met(&won, &life));
    }

    #[test]
    fn a_run_quit_at_once_does_not_count_as_played() {
        let mut life = Lifetime::default();
        life.add(&RunRecord {
            seconds: 12.0,
            kills: 3,
            ..RunRecord::default()
        });
        assert_eq!(life.runs, 0);
        assert_eq!(life.kills, 3);
        life.add(&RunRecord {
            seconds: 61.0,
            ..RunRecord::default()
        });
        assert_eq!(life.runs, 1);
    }
}
