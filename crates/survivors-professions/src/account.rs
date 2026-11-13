//! The account: what every run so far has earned, and what it was spent on.
//!
//! Points are never stored. Earned points are the sum of the achievements earned, spent points the
//! sum of the recipes crafted, so a save cannot drift out of step with itself and a rebalanced cost
//! applies to old saves too.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use crate::achievements::{Achievement, Lifetime, RunRecord};
use crate::professions::{find_recipe, Bonuses, Profession, Recipe, PROFESSIONS};

/// Why a recipe cannot be crafted.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CraftError {
    /// No such profession or tier.
    Unknown,
    AlreadyKnown,
    /// The recipe before it in the chain comes first.
    NeedsPrevious(&'static Recipe),
    NotEnoughPoints {
        need: u32,
        have: u32,
    },
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Account {
    /// Keys of the achievements earned.
    earned: BTreeSet<String>,
    /// Keys of the recipes crafted.
    crafted: BTreeSet<String>,
    pub lifetime: Lifetime,
}

/// The save file's first line: its format, so a later one can read this one.
const HEADER: &str = "azeroth-survivors-account 1";

impl Account {
    pub fn has_earned(&self, achievement: &str) -> bool {
        self.earned.contains(achievement)
    }

    pub fn knows(&self, recipe: &str) -> bool {
        self.crafted.contains(recipe)
    }

    pub fn earned_count(&self, catalog: &[Achievement]) -> usize {
        catalog.iter().filter(|a| self.has_earned(&a.key)).count()
    }

    /// Points from every achievement earned that the game still has.
    pub fn points_earned(&self, catalog: &[Achievement]) -> u32 {
        catalog
            .iter()
            .filter(|a| self.has_earned(&a.key))
            .map(|a| a.points)
            .sum()
    }

    /// Points spent on every recipe crafted that the game still has.
    pub fn points_spent(&self) -> u32 {
        self.crafted
            .iter()
            .filter_map(|k| find_recipe(k))
            .map(|(_, _, r)| r.cost)
            .sum()
    }

    pub fn points_available(&self, catalog: &[Achievement]) -> u32 {
        self.points_earned(catalog)
            .saturating_sub(self.points_spent())
    }

    /// How far into `profession`'s chain the account is: the tier of the best recipe whose every
    /// predecessor is known too, or `None` before the first. A recipe with a gap below it gives
    /// nothing until the gap is filled.
    pub fn tier(&self, profession: &Profession) -> Option<usize> {
        profession
            .recipes
            .iter()
            .take_while(|r| self.knows(r.key))
            .count()
            .checked_sub(1)
    }

    /// Whether the recipe at `tier` of the profession at index `profession` can be crafted now.
    pub fn check_craft(
        &self,
        profession: usize,
        tier: usize,
        catalog: &[Achievement],
    ) -> Result<&'static Recipe, CraftError> {
        let prof = PROFESSIONS.get(profession).ok_or(CraftError::Unknown)?;
        let recipe = prof.recipes.get(tier).ok_or(CraftError::Unknown)?;
        if self.knows(recipe.key) {
            return Err(CraftError::AlreadyKnown);
        }
        if let Some(prev) = tier.checked_sub(1).map(|t| &prof.recipes[t]) {
            if !self.knows(prev.key) {
                return Err(CraftError::NeedsPrevious(prev));
            }
        }
        let have = self.points_available(catalog);
        if have < recipe.cost {
            return Err(CraftError::NotEnoughPoints {
                need: recipe.cost,
                have,
            });
        }
        Ok(recipe)
    }

    /// Craft the recipe: its points are spent and its bonus applies from the next run.
    pub fn craft(
        &mut self,
        profession: usize,
        tier: usize,
        catalog: &[Achievement],
    ) -> Result<&'static Recipe, CraftError> {
        let recipe = self.check_craft(profession, tier, catalog)?;
        self.crafted.insert(recipe.key.to_string());
        Ok(recipe)
    }

    /// What the account adds to a run: each profession's best recipe, not their sum.
    pub fn bonuses(&self) -> Bonuses {
        let mut b = Bonuses::default();
        for p in PROFESSIONS {
            if let Some(t) = self.tier(p) {
                b.add(p.stat, p.recipes[t].value);
            }
        }
        b
    }

    /// Count a finished run and earn what it achieved. Returns the indexes in `catalog` of the
    /// achievements this run earned, in catalog order.
    pub fn record_run(&mut self, run: &RunRecord, catalog: &[Achievement]) -> Vec<usize> {
        self.lifetime.add(run);
        let mut new = Vec::new();
        for (i, a) in catalog.iter().enumerate() {
            if !self.has_earned(&a.key) && a.condition.met(run, &self.lifetime) {
                self.earned.insert(a.key.clone());
                new.push(i);
            }
        }
        new
    }

    /// The save file: one fact per line, so it reads by eye and merges by hand.
    pub fn to_text(&self) -> String {
        let l = &self.lifetime;
        let mut out = String::new();
        let _ = writeln!(out, "{HEADER}");
        let _ = writeln!(out, "lifetime runs {}", l.runs);
        let _ = writeln!(out, "lifetime kills {}", l.kills);
        let _ = writeln!(out, "lifetime bosses {}", l.bosses_slain);
        let _ = writeln!(out, "lifetime seconds {:.1}", l.seconds);
        let _ = writeln!(out, "lifetime wins {}", l.wins);
        for k in &self.earned {
            let _ = writeln!(out, "achievement {k}");
        }
        for k in &self.crafted {
            let _ = writeln!(out, "recipe {k}");
        }
        out
    }

    /// Read a save. Nothing in it is fatal: a line this build does not understand is skipped, so a
    /// damaged or newer file still keeps everything it can.
    pub fn from_text(text: &str) -> Account {
        let mut a = Account::default();
        for line in text.lines() {
            let mut words = line.split_whitespace();
            match (words.next(), words.next(), words.next()) {
                (Some("achievement"), Some(k), None) => {
                    a.earned.insert(k.to_string());
                }
                (Some("recipe"), Some(k), None) => {
                    a.crafted.insert(k.to_string());
                }
                (Some("lifetime"), Some(field), Some(v)) => {
                    let l = &mut a.lifetime;
                    match field {
                        "runs" => l.runs = v.parse().unwrap_or(l.runs),
                        "kills" => l.kills = v.parse().unwrap_or(l.kills),
                        "bosses" => l.bosses_slain = v.parse().unwrap_or(l.bosses_slain),
                        "wins" => l.wins = v.parse().unwrap_or(l.wins),
                        "seconds" => {
                            if let Ok(s) = v.parse::<f64>() {
                                if s.is_finite() && s >= 0.0 {
                                    l.seconds = s;
                                }
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        a
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::achievements::tests::{CLASSES, ZONES};
    use crate::achievements::{catalog, Category, Condition};
    use crate::BonusStat;

    const ENCHANTING: usize = 2;
    const ENGINEERING: usize = 3;

    /// A catalog of one achievement per 10 points wanted, all earned by any run.
    fn rich(points: u32) -> (Account, Vec<Achievement>) {
        let cat: Vec<Achievement> = (0..points / 10)
            .map(|i| Achievement {
                key: format!("test-{i}"),
                name: String::new(),
                description: String::new(),
                points: 10,
                category: Category::Survival,
                condition: Condition::Survive(0.0),
            })
            .collect();
        let mut a = Account::default();
        a.record_run(&RunRecord::default(), &cat);
        (a, cat)
    }

    #[test]
    fn a_recipe_needs_the_one_before_it() {
        let (mut a, cat) = rich(100);
        let wand = &PROFESSIONS[ENCHANTING].recipes[0];
        assert_eq!(
            a.craft(ENCHANTING, 1, &cat),
            Err(CraftError::NeedsPrevious(wand))
        );
        assert_eq!(
            a.craft(ENCHANTING, 0, &cat).map(|r| r.key),
            Ok("lesser-magic-wand")
        );
        assert_eq!(
            a.craft(ENCHANTING, 1, &cat).map(|r| r.key),
            Ok("greater-magic-wand")
        );
        assert_eq!(a.craft(ENCHANTING, 1, &cat), Err(CraftError::AlreadyKnown));
        assert_eq!(a.craft(ENCHANTING, 9, &cat), Err(CraftError::Unknown));
        assert_eq!(a.craft(99, 0, &cat), Err(CraftError::Unknown));
    }

    #[test]
    fn crafting_spends_points_and_stops_when_they_run_out() {
        let (mut a, cat) = rich(50);
        assert_eq!(a.points_available(&cat), 50);
        a.craft(ENGINEERING, 0, &cat).unwrap();
        a.craft(ENGINEERING, 1, &cat).unwrap();
        assert_eq!((a.points_spent(), a.points_available(&cat)), (40, 10));
        assert_eq!(
            a.craft(ENGINEERING, 2, &cat),
            Err(CraftError::NotEnoughPoints { need: 30, have: 10 })
        );
    }

    #[test]
    fn the_best_recipe_counts_not_the_sum() {
        let (mut a, cat) = rich(200);
        assert!(a.bonuses().is_empty());
        a.craft(ENCHANTING, 0, &cat).unwrap();
        assert_eq!(a.bonuses().luck, 5.0);
        a.craft(ENCHANTING, 1, &cat).unwrap();
        a.craft(ENGINEERING, 0, &cat).unwrap();
        let b = a.bonuses();
        assert_eq!(b.luck, 10.0, "the greater wand replaces the lesser");
        assert_eq!(b.area, 0.10);
        assert_eq!(b.get(BonusStat::Damage), 0.0);
        assert_eq!(a.tier(&PROFESSIONS[ENCHANTING]), Some(1));
        assert_eq!(a.tier(&PROFESSIONS[0]), None);
    }

    #[test]
    fn a_gap_in_the_chain_gives_nothing_above_it() {
        // Only reachable by editing the save: the top dynamite without the four below it.
        let a = Account::from_text("recipe dense-dynamite\n");
        assert_eq!(a.tier(&PROFESSIONS[ENGINEERING]), None);
        assert!(a.bonuses().is_empty());
    }

    #[test]
    fn runs_earn_achievements_once_and_add_up() {
        let cat = catalog(CLASSES, ZONES);
        let mut a = Account::default();
        let run = RunRecord {
            class_id: 8,
            zone: 0,
            seconds: 400.0,
            level: 14,
            kills: 700,
            bosses_slain: 1,
            won: false,
        };
        let names = |idx: &[usize]| idx.iter().map(|&i| cat[i].key.as_str()).collect::<Vec<_>>();
        let first = a.record_run(&run, &cat);
        assert_eq!(
            names(&first),
            [
                "survive-3m",
                "survive-5m",
                "level-10",
                "kills-500",
                "boss-1",
                "survive-in-the-barrens"
            ]
        );
        assert_eq!(a.points_earned(&cat), 10 + 10 + 10 + 10 + 10 + 5);
        // The same run again earns none of its own achievements twice, but its kills add to the
        // lifetime count, which now passes 1,000.
        let second = a.record_run(&run, &cat);
        assert_eq!(names(&second), ["total-kills-1k"]);
        assert_eq!(a.lifetime.kills, 1400);
        assert_eq!(a.lifetime.runs, 2);
        let won = RunRecord {
            seconds: 900.0,
            won: true,
            ..run
        };
        let third = a.record_run(&won, &cat);
        assert_eq!(
            names(&third),
            ["survive-10m", "win", "win-as-mage", "win-in-the-barrens"]
        );
        assert_eq!(a.earned_count(&cat), 11);
    }

    #[test]
    fn the_save_round_trips() {
        let cat = catalog(CLASSES, ZONES);
        let mut a = Account::default();
        a.record_run(
            &RunRecord {
                class_id: 1,
                zone: 3,
                seconds: 912.25,
                level: 31,
                kills: 3200,
                bosses_slain: 6,
                won: true,
            },
            &cat,
        );
        a.craft(ENCHANTING, 0, &cat).unwrap();
        a.craft(ENCHANTING, 1, &cat).unwrap();
        a.craft(ENGINEERING, 0, &cat).unwrap();
        let text = a.to_text();
        assert!(text.starts_with(HEADER));
        let back = Account::from_text(&text);
        // Seconds are kept to a tenth.
        assert!((back.lifetime.seconds - 912.25).abs() < 0.06);
        let mut a_rounded = a.clone();
        a_rounded.lifetime.seconds = back.lifetime.seconds;
        assert_eq!(back, a_rounded);
        assert_eq!(back.bonuses(), a.bonuses());
        assert_eq!(back.points_available(&cat), a.points_available(&cat));
    }

    #[test]
    fn a_damaged_or_newer_save_keeps_what_it_can() {
        let a = Account::from_text(
            "azeroth-survivors-account 7\n\
             lifetime kills 12x\n\
             lifetime kills 40\n\
             lifetime seconds -5\n\
             lifetime seconds NaN\n\
             lifetime wins 2\n\
             lifetime glory 9000\n\
             achievement win\n\
             achievement\n\
             recipe rough-dynamite extra-word\n\
             recipe lesser-magic-wand\n\
             pet-battles enabled\n\
             \u{0}\u{1}garbage",
        );
        assert_eq!(a.lifetime.kills, 40);
        assert_eq!(a.lifetime.seconds, 0.0);
        assert_eq!(a.lifetime.wins, 2);
        assert!(a.has_earned("win"));
        assert!(a.knows("lesser-magic-wand"));
        assert!(!a.knows("rough-dynamite"));
        assert_eq!(Account::from_text(""), Account::default());
    }

    #[test]
    fn points_ignore_what_the_game_no_longer_has() {
        let cat = catalog(CLASSES, ZONES);
        let a = Account::from_text("achievement retired-achievement\nrecipe retired-recipe\n");
        assert_eq!(a.points_earned(&cat), 0);
        assert_eq!(a.points_spent(), 0);
    }

    #[test]
    fn spending_more_than_earned_reads_as_nothing_left() {
        // A recipe kept after the achievement that paid for it left the game.
        let cat = catalog(CLASSES, ZONES);
        let a = Account::from_text("recipe lesser-magic-wand\n");
        assert_eq!(a.points_available(&cat), 0);
        assert_eq!(a.bonuses().luck, 5.0, "what was crafted stays crafted");
    }
}
