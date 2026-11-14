//! The trainer's brackets, the six professions and their recipe chains.
//!
//! Every number a player could want to tune lives here: what a bracket costs ([`Bracket::cost`]),
//! what a recipe costs ([`RECIPE_COST`]), and in [`PROFESSIONS`] each recipe's bracket and the
//! bonus it gives. Bonuses do not stack within a profession: the best
//! recipe crafted is the one carried (a Greater Magic Wand replaces the Lesser one), so a
//! recipe's value is the profession's total once it is crafted.

use crate::num;

/// A stat a profession raises for every run on the account.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BonusStat {
    /// Percent chance that a level-up deals a fourth card to choose from.
    Luck,
    /// All damage dealt, as a fraction: `0.05` is +5%.
    Damage,
    /// Maximum health, as a fraction.
    MaxHealth,
    /// The size of every area of effect, as a fraction.
    Area,
    /// Damage blocked from every hit taken, flat.
    Armor,
    /// The range experience is picked up from, as a fraction.
    PickupRange,
}

impl BonusStat {
    pub const ALL: [BonusStat; 6] = [
        BonusStat::Luck,
        BonusStat::Damage,
        BonusStat::MaxHealth,
        BonusStat::Area,
        BonusStat::Armor,
        BonusStat::PickupRange,
    ];

    pub fn label(self) -> &'static str {
        match self {
            BonusStat::Luck => "Luck",
            BonusStat::Damage => "Damage",
            BonusStat::MaxHealth => "Max health",
            BonusStat::Area => "Area of effect",
            BonusStat::Armor => "Armor",
            BonusStat::PickupRange => "Pickup range",
        }
    }

    /// What the stat does in a run, for the profession's header.
    pub fn effect(self) -> &'static str {
        match self {
            BonusStat::Luck => "Chance of a 4th level-up card",
            BonusStat::Damage => "All damage you deal",
            BonusStat::MaxHealth => "Your maximum health",
            BonusStat::Area => "Size of your areas of effect",
            BonusStat::Armor => "Damage blocked per hit",
            BonusStat::PickupRange => "Experience pickup range",
        }
    }

    /// Flat stats print as points; the rest are fractions printed as percent.
    fn is_flat(self) -> bool {
        matches!(self, BonusStat::Luck | BonusStat::Armor)
    }

    /// `+5 Luck`, `+20% Area of effect`.
    pub fn format(self, v: f32) -> String {
        if self.is_flat() {
            format!("+{} {}", num(v), self.label())
        } else {
            format!("+{}% {}", num(v * 100.0), self.label())
        }
    }
}

/// The trainer's ranks, as in WoW Classic: a recipe can only be crafted once its bracket is
/// unlocked. Brackets are for the whole account: unlocking Journeyman opens the Journeyman
/// recipes of every profession.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Bracket {
    /// Known from the start.
    Apprentice,
    Journeyman,
    Expert,
    Artisan,
}

impl Bracket {
    pub const ALL: [Bracket; 4] = [
        Bracket::Apprentice,
        Bracket::Journeyman,
        Bracket::Expert,
        Bracket::Artisan,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Bracket::Apprentice => "Apprentice",
            Bracket::Journeyman => "Journeyman",
            Bracket::Expert => "Expert",
            Bracket::Artisan => "Artisan",
        }
    }

    /// Stable id in the save file.
    pub fn key(self) -> &'static str {
        match self {
            Bracket::Apprentice => "apprentice",
            Bracket::Journeyman => "journeyman",
            Bracket::Expert => "expert",
            Bracket::Artisan => "artisan",
        }
    }

    pub fn from_key(key: &str) -> Option<Bracket> {
        Bracket::ALL.into_iter().find(|b| b.key() == key)
    }

    /// Achievement points to unlock it.
    pub fn cost(self) -> u32 {
        match self {
            Bracket::Apprentice => 0,
            Bracket::Journeyman => 50,
            Bracket::Expert => 100,
            Bracket::Artisan => 150,
        }
    }

    /// The Classic trainer's skill cap for the rank, for the screens' flavour text.
    pub fn skill_cap(self) -> u32 {
        match self {
            Bracket::Apprentice => 75,
            Bracket::Journeyman => 150,
            Bracket::Expert => 225,
            Bracket::Artisan => 300,
        }
    }

    /// The bracket that must be unlocked first.
    pub fn previous(self) -> Option<Bracket> {
        match self {
            Bracket::Apprentice => None,
            Bracket::Journeyman => Some(Bracket::Apprentice),
            Bracket::Expert => Some(Bracket::Journeyman),
            Bracket::Artisan => Some(Bracket::Expert),
        }
    }
}

/// One craftable step of a profession.
#[derive(Debug, PartialEq)]
pub struct Recipe {
    /// Stable id in the save file; never reuse or rename one.
    pub key: &'static str,
    pub name: &'static str,
    /// The trainer rank it needs: where the recipe is learned in WoW Classic.
    pub bracket: Bracket,
    /// Achievement points spent to craft it.
    pub cost: u32,
    /// The profession's bonus once this is its best recipe, in [`BonusStat`]'s units.
    pub value: f32,
}

/// A chain of recipes raising one stat.
#[derive(Debug, PartialEq)]
pub struct Profession {
    pub key: &'static str,
    pub name: &'static str,
    /// The profession's Apprentice spell, whose icon the screens show.
    pub icon_spell: u32,
    pub stat: BonusStat,
    /// Weakest first; each needs the one before it.
    pub recipes: &'static [Recipe],
}

impl Profession {
    pub fn recipe(&self, key: &str) -> Option<(usize, &'static Recipe)> {
        self.recipes.iter().enumerate().find(|(_, r)| r.key == key)
    }
}

/// What every recipe costs, whatever its bracket: the bracket is the gate that grows.
pub const RECIPE_COST: u32 = 20;

const fn r(key: &'static str, name: &'static str, bracket: Bracket, value: f32) -> Recipe {
    Recipe {
        key,
        name,
        bracket,
        cost: RECIPE_COST,
        value,
    }
}

use Bracket::{Apprentice as A, Artisan as R, Expert as E, Journeyman as J};

/// Every profession, in the trainer's alphabetical order: one item per bracket, each a permanent
/// upgrade for the whole account, like the examples the feature was asked with (Lesser Magic Wand
/// +5 Luck, then Greater Magic Wand +10 Luck). Each item is one WoW Classic's trainers teach in
/// that bracket, after the WoW-Professions Classic guides; the comment gives its skill level, and
/// where the guides craft an item later than it is learned, that too (Greater Magic Wand is learned
/// at 70 and crafted at 101-110, so it is the Journeyman step).
///
/// 24 items at 20 points and the brackets' 300 come to 780, of the 940 the achievements give with
/// the game's 9 classes and 16 zones: a player who does everything can unlock everything with
/// points to spare, and anyone else chooses what to learn first.
#[rustfmt::skip]
pub static PROFESSIONS: &[Profession] = &[
    Profession {
        key: "alchemy",
        name: "Alchemy",
        icon_spell: 2259,
        stat: BonusStat::MaxHealth,
        recipes: &[
            r("minor-healing-potion", "Minor Healing Potion", A, 0.10),     // skill 1
            r("healing-potion", "Healing Potion", J, 0.20),                 // 110
            r("greater-healing-potion", "Greater Healing Potion", E, 0.30), // 155
            r("major-healing-potion", "Major Healing Potion", R, 0.40),     // 275
        ],
    },
    Profession {
        key: "blacksmithing",
        name: "Blacksmithing",
        icon_spell: 2018,
        stat: BonusStat::Damage,
        recipes: &[
            r("rough-sharpening-stone", "Rough Sharpening Stone", A, 0.05), // 1
            r("heavy-sharpening-stone", "Heavy Sharpening Stone", J, 0.10), // 125
            r("solid-sharpening-stone", "Solid Sharpening Stone", E, 0.15), // 200
            r("dense-sharpening-stone", "Dense Sharpening Stone", R, 0.20), // 250
        ],
    },
    Profession {
        key: "enchanting",
        name: "Enchanting",
        icon_spell: 7411,
        stat: BonusStat::Luck,
        recipes: &[
            r("lesser-magic-wand", "Lesser Magic Wand", A, 5.0),      // 10
            r("greater-magic-wand", "Greater Magic Wand", J, 10.0),   // 70, crafted 101-110
            r("greater-mystic-wand", "Greater Mystic Wand", E, 15.0), // 175
            r("runed-arcanite-rod", "Runed Arcanite Rod", R, 20.0),   // 285
        ],
    },
    Profession {
        key: "engineering",
        name: "Engineering",
        icon_spell: 4036,
        stat: BonusStat::Area,
        recipes: &[
            r("rough-dynamite", "Rough Dynamite", A, 0.10),   // 30
            r("coarse-dynamite", "Coarse Dynamite", J, 0.20), // 75, crafted 90-100
            r("solid-dynamite", "Solid Dynamite", E, 0.30),   // 175
            r("dense-dynamite", "Dense Dynamite", R, 0.40),   // 250
        ],
    },
    Profession {
        key: "leatherworking",
        name: "Leatherworking",
        icon_spell: 2108,
        stat: BonusStat::Armor,
        recipes: &[
            r("light-armor-kit", "Light Armor Kit", A, 0.5),   // 1
            r("medium-armor-kit", "Medium Armor Kit", J, 1.0), // 100
            r("heavy-armor-kit", "Heavy Armor Kit", E, 1.5),   // 150
            r("rugged-armor-kit", "Rugged Armor Kit", R, 2.0), // 250
        ],
    },
    Profession {
        key: "tailoring",
        name: "Tailoring",
        icon_spell: 3908,
        stat: BonusStat::PickupRange,
        recipes: &[
            r("linen-bag", "Linen Bag", A, 0.20),             // 45
            r("woolen-bag", "Woolen Bag", J, 0.40),           // 80
            r("small-silk-pack", "Small Silk Pack", E, 0.60), // 150
            r("runecloth-bag", "Runecloth Bag", R, 0.80),     // 260
        ],
    },
];

/// Look a recipe up by its save-file key: `(profession index, tier, recipe)`.
pub fn find_recipe(key: &str) -> Option<(usize, usize, &'static Recipe)> {
    PROFESSIONS
        .iter()
        .enumerate()
        .find_map(|(p, prof)| prof.recipe(key).map(|(t, r)| (p, t, r)))
}

/// What every profession adds to a run, in [`BonusStat`]'s units.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Bonuses {
    pub luck: f32,
    pub damage: f32,
    pub max_health: f32,
    pub area: f32,
    pub armor: f32,
    pub pickup_range: f32,
}

impl Bonuses {
    pub fn get(&self, stat: BonusStat) -> f32 {
        match stat {
            BonusStat::Luck => self.luck,
            BonusStat::Damage => self.damage,
            BonusStat::MaxHealth => self.max_health,
            BonusStat::Area => self.area,
            BonusStat::Armor => self.armor,
            BonusStat::PickupRange => self.pickup_range,
        }
    }

    pub fn add(&mut self, stat: BonusStat, v: f32) {
        let slot = match stat {
            BonusStat::Luck => &mut self.luck,
            BonusStat::Damage => &mut self.damage,
            BonusStat::MaxHealth => &mut self.max_health,
            BonusStat::Area => &mut self.area,
            BonusStat::Armor => &mut self.armor,
            BonusStat::PickupRange => &mut self.pickup_range,
        };
        *slot += v;
    }

    pub fn is_empty(&self) -> bool {
        BonusStat::ALL.iter().all(|&s| self.get(s) == 0.0)
    }

    /// Each non-zero bonus as the screens print it, in [`BonusStat::ALL`]'s order.
    pub fn lines(&self) -> Vec<String> {
        BonusStat::ALL
            .iter()
            .filter(|&&s| self.get(s) != 0.0)
            .map(|&s| s.format(self.get(s)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// One item per bracket, Apprentice to Artisan, each better than the last.
    #[test]
    fn every_profession_has_one_item_per_bracket_each_better() {
        assert_eq!(PROFESSIONS.len(), 6);
        for p in PROFESSIONS {
            let brackets: Vec<Bracket> = p.recipes.iter().map(|r| r.bracket).collect();
            assert_eq!(brackets, Bracket::ALL, "{}", p.name);
            for pair in p.recipes.windows(2) {
                assert!(
                    pair[1].value > pair[0].value,
                    "{} must improve",
                    pair[1].name
                );
            }
        }
    }

    #[test]
    fn brackets_climb_in_cost_and_order() {
        for b in Bracket::ALL {
            assert_eq!(Bracket::from_key(b.key()), Some(b));
        }
        // The costs asked for: Journeyman 50, Expert 100, and on in steps of 50.
        assert_eq!(Bracket::ALL.map(Bracket::cost), [0, 50, 100, 150]);
        assert_eq!(Bracket::Expert.previous(), Some(Bracket::Journeyman));
        assert_eq!(Bracket::from_key("grandmaster"), None);
    }

    #[test]
    fn keys_are_unique_and_save_safe() {
        let mut seen = HashSet::new();
        for b in Bracket::ALL {
            assert!(seen.insert(b.key()));
        }
        for p in PROFESSIONS {
            assert!(seen.insert(p.key), "duplicate profession key {}", p.key);
            for r in p.recipes {
                assert!(seen.insert(r.key), "duplicate recipe key {}", r.key);
                assert!(
                    r.key
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                    "{} would not survive the save file",
                    r.key
                );
            }
        }
    }

    /// The examples the feature was asked for, exactly as asked.
    #[test]
    fn the_requested_examples_hold() {
        let (p, t, wand) = find_recipe("lesser-magic-wand").unwrap();
        assert_eq!((PROFESSIONS[p].name, t), ("Enchanting", 0));
        assert_eq!(
            (wand.cost, wand.value, wand.bracket),
            (20, 5.0, Bracket::Apprentice)
        );
        assert_eq!(PROFESSIONS[p].stat.format(wand.value), "+5 Luck");
        let (_, t, greater) = find_recipe("greater-magic-wand").unwrap();
        assert_eq!((t, greater.cost, greater.value), (1, 20, 10.0));
        assert_eq!(greater.bracket, Bracket::Journeyman);

        let (p, t, rough) = find_recipe("rough-dynamite").unwrap();
        assert_eq!((PROFESSIONS[p].name, t), ("Engineering", 0));
        assert_eq!((rough.cost, rough.bracket), (20, Bracket::Apprentice));
        assert_eq!(
            PROFESSIONS[p].stat.format(rough.value),
            "+10% Area of effect"
        );
        let (_, t, coarse) = find_recipe("coarse-dynamite").unwrap();
        assert_eq!(
            (t, coarse.cost, coarse.bracket),
            (1, 20, Bracket::Journeyman)
        );
        assert_eq!(
            PROFESSIONS[p].stat.format(coarse.value),
            "+20% Area of effect"
        );
    }

    #[test]
    fn bonuses_print_in_their_units() {
        assert_eq!(BonusStat::Armor.format(0.5), "+0.5 Armor");
        assert_eq!(BonusStat::Damage.format(0.25), "+25% Damage");
        let mut b = Bonuses::default();
        assert!(b.is_empty());
        b.add(BonusStat::Luck, 10.0);
        b.add(BonusStat::Area, 0.2);
        assert_eq!(b.lines(), vec!["+10 Luck", "+20% Area of effect"]);
    }
}
