# Professions: progress that carries over between runs

Time spent grinding should count for something beyond a single run. Professions turn that time
into small permanent bonuses for the whole account:

1. **Runs earn achievements.** Survive 5 minutes, kill 1,500 enemies in one run, win in Duskwood,
   kill 10,000 enemies in total, and so on. Each achievement gives **achievement points** once
   per account.
2. **Points unlock brackets and buy recipes.** The menu has a **Professions** tab next to
   **Hero** (the pick screen), at the top. It groups everything about professions in one place:
   the trainer's brackets, six professions of recipes, and the achievements that pay for them, by
   category. The tab's label shows the points waiting to be spent, e.g. *Professions (35)*.
   **Esc** goes back to Hero.
3. **Brackets gate the recipes, as WoW Classic's trainer ranks do.** Every account is an
   **Apprentice**; **Journeyman** crafting costs **50** points, **Expert** **100** and **Artisan**
   **150**, each after the one before. A bracket is for the whole account: unlocking Journeyman
   opens the Journeyman recipes of every profession.
4. **A recipe costs 20 points** and needs the recipe before it and its bracket.
5. **The best recipe in each profession boosts every run.** A Greater Magic Wand *replaces* the
   Lesser Magic Wand: bonuses within a profession don't stack, the best one counts. Bonuses from
   different professions do add up.

```
Brackets     Apprentice (free) -> Journeyman (50 pts) -> Expert (100 pts) -> Artisan (150 pts)
Enchanting   Lesser Magic Wand (20 pts) +5 Luck   ->  Greater Magic Wand (20 pts) +10 Luck   -> ...
Engineering  Rough Dynamite    (20 pts) +10% Area ->  Coarse Dynamite (Journeyman) +20% Area -> ...
```

## The professions

Each recipe sits in the bracket where WoW Classic's trainers teach it, after the
[WoW-Professions Classic guides](https://www.wow-professions.com/classic) (Apprentice up to skill
75, Journeyman 150, Expert 225, Artisan 300). That's why Coarse Dynamite (skill 75) already needs
Journeyman while Greater Magic Wand (skill 70) doesn't. The skill level is beside each recipe in
`crates/survivors-professions/src/professions.rs`.

| Profession | Raises | Apprentice | Journeyman | Expert | Artisan |
|---|---|---|---|---|---|
| Alchemy | Maximum health | Minor Healing Potion +5%, Lesser Healing Potion +10% | Healing Potion +20% | Greater Healing Potion +30%, Superior Healing Potion +40% | Major Healing Potion +50% |
| Blacksmithing | All damage | Rough Sharpening Stone +5%, Coarse Sharpening Stone +10% | Heavy Sharpening Stone +15% | Solid Sharpening Stone +20% | Dense Sharpening Stone +25% |
| Enchanting | Luck | Lesser Magic Wand +5, Greater Magic Wand +10 | Runed Silver Rod +15 | Lesser Mystic Wand +20, Greater Mystic Wand +25 | Runed Arcanite Rod +30 |
| Engineering | Area of effect | Rough Dynamite +10% | Coarse Dynamite +20%, Heavy Dynamite +30% | Solid Dynamite +40% | Dense Dynamite +50% |
| Leatherworking | Armor (damage blocked per hit) | Light Armor Kit +0.5 | Medium Armor Kit +1 | Heavy Armor Kit +1.5, Thick Armor Kit +2 | Rugged Armor Kit +2.5 |
| Tailoring | Pickup range | Linen Bag +20% | Woolen Bag +40% | Small Silk Pack +60% | Mageweave Bag +80%, Runecloth Bag +100% |

Within a profession, recipes are crafted left to right; the bonus shown is the profession's total
once that recipe is its best.

**Luck** is new in this build: it is the percent chance that a level-up deals a **fourth card**
to choose from (press **4** or click it). Only Enchanting raises it.

How the numbers were picked: the examples from the original idea are kept exactly (the wands'
+5/+10 Luck and the dynamites' +10%/+20% Area at 20 points each; Journeyman 50, Expert 100, with
Artisan continuing the step at 150) and continued in steps of the same size. For the other
stats, the top recipe is worth about two and a half levels of the matching in-run passive
(Blessing of Might gives +10% damage a level, so the top sharpening stone gives +25%). Everything
is in one table and easy to rebalance: see [Tuning](#tuning).

## The achievements

71 achievements worth **940 points**: exactly what the three brackets (300) and all 32 recipes
(640) cost. A player who does everything can unlock everything, and everyone else chooses what to
learn first: one profession to the top, or the first steps of all six.

| Category | What | Achievements | Points |
|---|---|---|---|
| Survival | One great run: survive 3/5/10 minutes, win, reach level 10/20/30, kill 500/1,500/3,000 enemies, kill 1/3/5 bosses | 13 | 205 |
| Veteran | Every run added up: enemies and bosses killed, runs played, hours survived, runs won | 17 | 405 |
| Classes | Win a run as each class | 9 | 90 |
| Zones | Survive 5 minutes in each zone (5 points), win in each zone (10 points) | 32 | 240 |

The class and zone achievements are made from the game's own class and zone lists, so a new zone
brings its two achievements with it.

## Rules worth knowing

- **Bonuses are fixed when a run starts.** Crafting changes the next run, not the current one.
- **Every run counts:** a death, a win, a run left from the pause menu, and a run ended with Quit
  game. Closing the browser tab mid-run doesn't count that run.
- **A run shorter than a minute doesn't count toward "runs played"**, so starting and quitting
  over and over earns nothing. Its kills and time still add up.
- **Boss achievements count kills, not encounters.** (The results screen's Bosses tile still
  counts bosses met, as before.)
- **Points are never stored.** Points earned are the sum of achievements earned; points spent are
  the sum of brackets unlocked and recipes crafted. A rebalanced cost applies to old saves, and the file can't drift out
  of step with itself.
- **The dev auto-runner (`WOW_SURVIVORS_AUTO`) plays without the account**: no bonuses, nothing
  recorded.

## Where it's saved

`benilla-config/survivors-account.txt` in a native build, or the page's `localStorage` in the
browser, beside the saved hero pick. It's plain text, one fact per line:

```
azeroth-survivors-account 1
lifetime runs 12
lifetime kills 3456
lifetime bosses 20
lifetime seconds 4567.5
lifetime wins 2
achievement survive-5m
achievement win-in-elwynn-forest
bracket journeyman
recipe lesser-magic-wand
recipe greater-magic-wand
```

A line the game doesn't understand is skipped, never fatal, so a damaged or newer file keeps
everything it can.

## Where the code is

| Path | What |
|---|---|
| `crates/survivors-professions/` | The rules, with no dependencies: brackets, professions and recipes (`professions.rs`), achievements (`achievements.rs`), the account and its save format (`account.rs`). 24 unit tests. |
| `crates/benilla-app/src/survivors/professions.rs` | The game side: loading and saving the account, counting runs, the menu's tab bar and the Professions page, the results screen's line. A new menu page (equipment, talents) is one more `MenuTab` variant. |
| `crates/benilla-app/src/survivors/mod.rs` | `Run::bonuses` and `Stats::luck`; `Run::stats()` adds the account's bonuses. |
| `crates/benilla-app/src/survivors/hud.rs` | `start_run` takes the bonuses; luck deals the fourth card. |

## Tuning

- **Recipe brackets and bonuses:** `PROFESSIONS` in `crates/survivors-professions/src/professions.rs`;
  the recipes' cost is `RECIPE_COST`, the brackets' `Bracket::cost`.
- **Achievements and their points:** `GENERAL`, `CLASS_WIN_POINTS`, `ZONE_WIN_POINTS` and
  `ZONE_SURVIVE_POINTS` in `crates/survivors-professions/src/achievements.rs`.
- Save keys (`lesser-magic-wand`, `survive-5m`, …) are the save format: rename a recipe's `name`
  freely, but never its `key`.
- `cargo test -p survivors-professions` checks that every chain only improves and climbs the
  brackets in order, that every bracket has recipes, that every key is unique and save-safe, and
  that the achievements give enough points to unlock everything.

## Bringing it to the official game

This repository is an open-source re-creation of Azeroth Survivors, not Aethus's own code, and
the official game at [wow.aethus.pl](https://wow.aethus.pl/) doesn't publish its source, so this
feature can't simply be merged into it. It's built to be easy to adopt, though:

- **The rules are a standalone crate** (`survivors-professions`) in the same language as the
  official client (Rust), with no dependencies. It knows nothing about this repository's engine.
- To wire it into another build, the game needs to:
  1. build the catalog with `survivors_professions::catalog(classes, zones)`;
  2. at the end of every run, call `Account::record_run` with a `RunRecord` (class, zone, seconds,
     level, kills, bosses killed, won) and save `Account::to_text()`;
  3. at the start of a run, add `Account::bonuses()` to the hero's stats. The official game
     already has a Luck stat, so `Bonuses::luck` can feed it directly instead of the fourth-card
     rule used here;
  4. draw a page (here, a menu tab) that lists the brackets and `PROFESSIONS` and calls
     `Account::unlock` and `Account::craft`.
- If the official game already has its own achievement points, steps 1 and 2 can be dropped and
  `Account::points_available` replaced by the game's own count.
