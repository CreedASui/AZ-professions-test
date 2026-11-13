//! The six professions and their recipe chains.
//!
//! Every number a player could want to tune lives in [`PROFESSIONS`]: a recipe's cost in
//! achievement points and the bonus it gives. Bonuses do not stack within a profession: the best
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
            BonusStat::MaxHealth => "Maximum health",
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
            BonusStat::PickupRange => "Range you collect experience from",
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

/// One craftable step of a profession.
#[derive(Debug, PartialEq)]
pub struct Recipe {
    /// Stable id in the save file; never reuse or rename one.
    pub key: &'static str,
    pub name: &'static str,
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

const fn r(key: &'static str, name: &'static str, cost: u32, value: f32) -> Recipe {
    Recipe {
        key,
        name,
        cost,
        value,
    }
}

/// Every profession, in the trainer's alphabetical order.
///
/// Costs run 20, 20, 30, 30, 40: 140 points to master one profession, 840 for all six, against
/// the 940 the achievements give with the game's 9 classes and 16 zones, so a player who does
/// everything can master everything, and anyone else chooses what to learn first.
pub static PROFESSIONS: &[Profession] = &[
    Profession {
        key: "alchemy",
        name: "Alchemy",
        icon_spell: 2259,
        stat: BonusStat::MaxHealth,
        recipes: &[
            r("minor-healing-potion", "Minor Healing Potion", 20, 0.10),
            r("lesser-healing-potion", "Lesser Healing Potion", 20, 0.20),
            r("healing-potion", "Healing Potion", 30, 0.30),
            r("greater-healing-potion", "Greater Healing Potion", 30, 0.40),
            r(
                "superior-healing-potion",
                "Superior Healing Potion",
                40,
                0.50,
            ),
        ],
    },
    Profession {
        key: "blacksmithing",
        name: "Blacksmithing",
        icon_spell: 2018,
        stat: BonusStat::Damage,
        recipes: &[
            r("rough-sharpening-stone", "Rough Sharpening Stone", 20, 0.05),
            r(
                "coarse-sharpening-stone",
                "Coarse Sharpening Stone",
                20,
                0.10,
            ),
            r("heavy-sharpening-stone", "Heavy Sharpening Stone", 30, 0.15),
            r("solid-sharpening-stone", "Solid Sharpening Stone", 30, 0.20),
            r("dense-sharpening-stone", "Dense Sharpening Stone", 40, 0.25),
        ],
    },
    Profession {
        key: "enchanting",
        name: "Enchanting",
        icon_spell: 7411,
        stat: BonusStat::Luck,
        recipes: &[
            r("lesser-magic-wand", "Lesser Magic Wand", 20, 5.0),
            r("greater-magic-wand", "Greater Magic Wand", 20, 10.0),
            r("lesser-mystic-wand", "Lesser Mystic Wand", 30, 15.0),
            r("greater-mystic-wand", "Greater Mystic Wand", 30, 20.0),
            r("runed-arcanite-rod", "Runed Arcanite Rod", 40, 25.0),
        ],
    },
    Profession {
        key: "engineering",
        name: "Engineering",
        icon_spell: 4036,
        stat: BonusStat::Area,
        recipes: &[
            r("rough-dynamite", "Rough Dynamite", 20, 0.10),
            r("coarse-dynamite", "Coarse Dynamite", 20, 0.20),
            r("heavy-dynamite", "Heavy Dynamite", 30, 0.30),
            r("solid-dynamite", "Solid Dynamite", 30, 0.40),
            r("dense-dynamite", "Dense Dynamite", 40, 0.50),
        ],
    },
    Profession {
        key: "leatherworking",
        name: "Leatherworking",
        icon_spell: 2108,
        stat: BonusStat::Armor,
        recipes: &[
            r("light-armor-kit", "Light Armor Kit", 20, 0.5),
            r("medium-armor-kit", "Medium Armor Kit", 20, 1.0),
            r("heavy-armor-kit", "Heavy Armor Kit", 30, 1.5),
            r("thick-armor-kit", "Thick Armor Kit", 30, 2.0),
            r("rugged-armor-kit", "Rugged Armor Kit", 40, 2.5),
        ],
    },
    Profession {
        key: "tailoring",
        name: "Tailoring",
        icon_spell: 3908,
        stat: BonusStat::PickupRange,
        recipes: &[
            r("linen-bag", "Linen Bag", 20, 0.20),
            r("woolen-bag", "Woolen Bag", 20, 0.40),
            r("small-silk-pack", "Small Silk Pack", 30, 0.60),
            r("mageweave-bag", "Mageweave Bag", 30, 0.80),
            r("runecloth-bag", "Runecloth Bag", 40, 1.00),
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

    #[test]
    fn every_profession_is_a_five_step_chain_that_only_improves() {
        assert_eq!(PROFESSIONS.len(), 6);
        for p in PROFESSIONS {
            assert_eq!(p.recipes.len(), 5, "{}", p.name);
            for pair in p.recipes.windows(2) {
                assert!(
                    pair[1].value > pair[0].value,
                    "{} must improve",
                    pair[1].name
                );
                assert!(
                    pair[1].cost >= pair[0].cost,
                    "{} must not get cheaper",
                    pair[1].name
                );
            }
        }
    }

    #[test]
    fn keys_are_unique_and_save_safe() {
        let mut seen = HashSet::new();
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

    /// The two examples the feature was asked for, exactly as asked.
    #[test]
    fn the_requested_examples_hold() {
        let (p, t, wand) = find_recipe("lesser-magic-wand").unwrap();
        assert_eq!((PROFESSIONS[p].name, t), ("Enchanting", 0));
        assert_eq!((wand.cost, wand.value), (20, 5.0));
        assert_eq!(PROFESSIONS[p].stat.format(wand.value), "+5 Luck");
        let (_, t, greater) = find_recipe("greater-magic-wand").unwrap();
        assert_eq!((t, greater.cost, greater.value), (1, 20, 10.0));

        let (p, t, rough) = find_recipe("rough-dynamite").unwrap();
        assert_eq!((PROFESSIONS[p].name, t), ("Engineering", 0));
        assert_eq!(rough.cost, 20);
        assert_eq!(
            PROFESSIONS[p].stat.format(rough.value),
            "+10% Area of effect"
        );
        let (_, t, coarse) = find_recipe("coarse-dynamite").unwrap();
        assert_eq!((t, coarse.cost), (1, 20));
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
