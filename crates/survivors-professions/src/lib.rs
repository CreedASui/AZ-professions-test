//! Professions for Azeroth Survivors: time spent playing carries over between runs.
//!
//! Every run can earn **achievements** ([`achievements`]), each worth achievement points once per
//! account. Points buy **recipes** in six **professions** ([`professions`]). Each profession is a
//! chain of five recipes: a recipe needs the one before it, and the best recipe crafted in a
//! profession gives its bonus to every run the account starts ([`Account::bonuses`]).
//!
//! ```text
//! Enchanting   Lesser Magic Wand (20 pts, +5 Luck) -> Greater Magic Wand (20 pts, +10 Luck) -> ...
//! Engineering  Rough Dynamite (20 pts, +10% Area)  -> Coarse Dynamite (20 pts, +20% Area)    -> ...
//! ```
//!
//! The crate holds only the rules and the save format; the game wires the bonuses into its stats
//! and draws the screens.

pub mod account;
pub mod achievements;
pub mod professions;

pub use account::{Account, CraftError};
pub use achievements::{catalog, Achievement, Category, Condition, Lifetime, RunRecord};
pub use professions::{BonusStat, Bonuses, Profession, Recipe, PROFESSIONS};

/// `10`, `0.5`, `12.5`: a number as the screens print it, with no trailing `.0`.
pub fn num(v: f32) -> String {
    let r = (v * 10.0).round() / 10.0;
    if (r - r.round()).abs() < 1e-4 {
        format!("{}", r.round() as i64)
    } else {
        format!("{r:.1}")
    }
}

#[cfg(test)]
mod tests {
    use super::num;

    #[test]
    fn numbers_print_without_trailing_zeroes() {
        assert_eq!(num(10.0), "10");
        assert_eq!(num(0.1 * 100.0), "10");
        assert_eq!(num(0.5), "0.5");
        assert_eq!(num(2.5), "2.5");
        assert_eq!(num(0.0), "0");
    }
}
