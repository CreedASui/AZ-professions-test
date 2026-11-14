//! Professions: the part of a run that outlives it.
//!
//! Each run's achievements ([`survivors_professions::achievements`]) earn achievement points for
//! the account; the menu's Professions tab spends them on the trainer's brackets and on recipes,
//! and each profession's best recipe adds its bonus to every run after ([`super::Run::bonuses`]).
//! The rules are the `survivors-professions` crate's; this module keeps the account on disk
//! (`benilla-config/survivors-account.txt`, `localStorage` on the page), counts runs into it, and
//! draws the menu's tab bar, the Professions page and the results screen's line.

use bevy::prelude::*;
use survivors_professions::achievements::Category;
use survivors_professions::{
    Account, Achievement, Bonuses, Bracket, CraftError, RunRecord, UnlockError, PROFESSIONS,
};

use super::cards::{COMMON, EPIC, RARE, UNCOMMON};
use super::hud::{text_bundle, HudFont, SpellIcon, DIM_GREY, GOLD_TONE};
use super::{data, Phase, Run, Selection};
use crate::glue::GlueClicks;

const GOLD: Color = GOLD_TONE;
const DIM: Color = DIM_GREY;
const EDGE: Color = Color::srgb(0.55, 0.45, 0.25);
const RED: Color = Color::srgb(1.0, 0.35, 0.3);
const CELL: Color = Color::srgba(0.12, 0.08, 0.05, 0.95);
const CELL_HOT: Color = Color::srgba(0.3, 0.16, 0.06, 0.98);
const CELL_KNOWN: Color = Color::srgba(0.06, 0.14, 0.06, 0.95);
const CELL_LOCKED: Color = Color::srgba(0.05, 0.05, 0.05, 0.9);

/// Inside the panel's border and padding: the frame is 1000 wide, inside a 4:3 screen's 1024.
const INNER_W: f32 = 960.0;
const COLUMN_GAP: f32 = 12.0;

/// The account file beside the saved pick.
fn account_path() -> Option<std::path::PathBuf> {
    crate::local_state::home().map(|h| h.join("survivors-account.txt"))
}

/// The account and the achievements this build of the game has.
#[derive(Resource, Default)]
pub(super) struct Profile {
    pub account: Account,
    pub catalog: Vec<Achievement>,
}

impl Profile {
    /// What the account's professions add to a run.
    pub fn bonuses(&self) -> Bonuses {
        self.account.bonuses()
    }

    pub fn points(&self) -> u32 {
        self.account.points_available(&self.catalog)
    }

    fn save(&self) {
        if let Some(path) = account_path() {
            if let Err(e) = crate::local_state::write_atomic(&path, &self.account.to_text()) {
                warn!("survivors: the account could not be saved: {e}");
            }
        }
    }
}

/// Every achievement, made from the game's own classes and zones.
fn catalog() -> Vec<Achievement> {
    let classes: Vec<(u8, &str)> = data::CLASSES.iter().map(|c| (c.id, c.name)).collect();
    let zones: Vec<&str> = data::MAPS.iter().map(|m| m.name).collect();
    survivors_professions::catalog(&classes, &zones)
}

/// Read the account at boot; a missing file is a new account.
pub(super) fn load_profile(mut profile: ResMut<Profile>) {
    profile.catalog = catalog();
    profile.account = account_path()
        .and_then(|p| crate::local_state::read_to_string(&p).ok())
        .map(|t| Account::from_text(&t))
        .unwrap_or_default();
}

/// The achievements the last run earned, as indexes into [`Profile::catalog`].
#[derive(Resource, Default)]
pub(super) struct RunReport {
    earned: Vec<usize>,
}

/// Count the run into the account: at the results screen, or on the way back to the menu for a
/// run left from the pause menu. Once per run; a hands-off test run (`WOW_SURVIVORS_AUTO`) leaves
/// the account alone.
pub(super) fn record_run(
    mut run: ResMut<Run>,
    selection: Res<Selection>,
    mut profile: ResMut<Profile>,
    mut report: ResMut<RunReport>,
    auto: Option<Res<super::auto::Auto>>,
) {
    // Read before writing, so a frame with nothing to count marks nothing changed.
    if run.recorded || run.elapsed <= 0.0 {
        return;
    }
    count_run(
        &mut run,
        &selection,
        &mut profile,
        &mut report,
        auto.is_some(),
    );
}

/// Quit game from the pause menu closes the app without passing the menu: count the run first.
pub(super) fn record_on_exit(
    mut exits: MessageReader<AppExit>,
    mut run: ResMut<Run>,
    selection: Res<Selection>,
    mut profile: ResMut<Profile>,
    mut report: ResMut<RunReport>,
    auto: Option<Res<super::auto::Auto>>,
) {
    if exits.read().count() == 0 || run.recorded || run.elapsed <= 0.0 {
        return;
    }
    count_run(
        &mut run,
        &selection,
        &mut profile,
        &mut report,
        auto.is_some(),
    );
}

fn count_run(
    run: &mut Run,
    selection: &Selection,
    profile: &mut Profile,
    report: &mut RunReport,
    auto: bool,
) {
    run.recorded = true;
    report.earned.clear();
    if auto {
        return;
    }
    let record = RunRecord {
        class_id: run.class.id,
        zone: selection.stage,
        seconds: run.elapsed,
        level: run.level,
        kills: run.kills,
        bosses_slain: run.bosses_slain,
        won: run.won,
    };
    report.earned = profile.account.record_run(&record, &profile.catalog);
    profile.save();
}

/// The results screen's line: what the run earned and what is left to spend.
#[derive(Component)]
pub(super) struct OverAchievements;

pub(super) fn refresh_results_line(
    report: Res<RunReport>,
    profile: Res<Profile>,
    mut texts: Query<&mut Text, With<OverAchievements>>,
) {
    if !report.is_changed() && !profile.is_changed() {
        return;
    }
    let line = results_line(&report.earned, &profile);
    for mut t in &mut texts {
        if t.0 != line {
            t.0.clone_from(&line);
        }
    }
}

fn results_line(earned: &[usize], p: &Profile) -> String {
    let points = p.points();
    let spend = if points > 0 {
        format!("{points} achievement points to spend in Professions.")
    } else {
        String::new()
    };
    if earned.is_empty() {
        return format!("No new achievements this run. {spend}")
            .trim_end()
            .to_string();
    }
    const SHOWN: usize = 4;
    let gained: u32 = earned.iter().map(|&i| p.catalog[i].points).sum();
    let names: Vec<&str> = earned
        .iter()
        .take(SHOWN)
        .map(|&i| p.catalog[i].name.as_str())
        .collect();
    let mut list = names.join(", ");
    if earned.len() > SHOWN {
        list.push_str(&format!(" and {} more", earned.len() - SHOWN));
    }
    let head = if earned.len() == 1 {
        "New achievement"
    } else {
        "New achievements"
    };
    format!("{head} (+{gained} points): {list}\n{spend}")
        .trim_end()
        .to_string()
}

// --- The menu's tabs and the Professions page. ---

/// The menu's top-level tabs, in a bar above both pages. Another page (equipment, talents) is one
/// more variant here and one more entry in [`MENU_TABS`].
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub(super) enum MenuTab {
    /// The pick screen: race, class, zone and Enter World.
    #[default]
    Hero,
    Professions,
}

const MENU_TABS: [MenuTab; 2] = [MenuTab::Hero, MenuTab::Professions];

/// The Professions page's own tabs: the recipes, then each achievement category.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub(super) enum Tab {
    #[default]
    Recipes,
    Achievements(Category),
}

const ACHIEVEMENT_TABS: [Category; 4] = [
    Category::Survival,
    Category::Veteran,
    Category::Classes,
    Category::Zones,
];

/// Where the tab bar sits, and where the Professions page starts under it.
const TABS_TOP: f32 = 12.0;
const PAGE_TOP: f32 = 58.0;

/// The menu tab that is up, the Professions page's tab, and the answer to the last craft.
#[derive(Resource, Default)]
pub(super) struct Panel {
    menu: MenuTab,
    tab: Tab,
    message: Option<(String, Color)>,
}

impl Panel {
    /// Whether the Professions page is up in place of the pick screen.
    pub fn open(&self) -> bool {
        self.menu == MenuTab::Professions
    }
}

/// What a click on the tab bar or the Professions page does.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum PanelAction {
    Menu(MenuTab),
    Tab(Tab),
    /// Buy a trainer bracket for every profession.
    Unlock(Bracket),
    /// `(profession, tier)`.
    Craft(usize, usize),
}

/// The menu's tab bar.
#[derive(Component)]
pub(super) struct MenuTabsRoot;

/// The Professions page.
#[derive(Component)]
pub(super) struct PanelRoot;

/// The page's bordered frame, whose children are rebuilt on every change.
#[derive(Component)]
pub(super) struct PanelFrame;

/// A cell that lights under the cursor.
#[derive(Component)]
pub(super) struct Hover {
    base: Color,
    hot: Color,
}

/// A bracket's quality colour, which its recipes wear too.
fn bracket_color(b: Bracket) -> Color {
    match b {
        Bracket::Apprentice => COMMON,
        Bracket::Journeyman => UNCOMMON,
        Bracket::Expert => RARE,
        Bracket::Artisan => EPIC,
    }
}

/// The tab bar, and the Professions page under it, both hidden until the menu is up.
pub(super) fn setup_panel(mut commands: Commands) {
    // Spans the screen's width only to centre the tabs: it passes clicks through.
    commands.spawn((
        MenuTabsRoot,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(TABS_TOP),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            column_gap: Val::Px(6.0),
            display: Display::None,
            ..default()
        },
        GlobalZIndex(440),
        Visibility::Hidden,
    ));
    // Above the world, under the tab bar; the pick screen is hidden while it is up.
    commands
        .spawn((
            PanelRoot,
            Interaction::default(),
            bevy::ui::FocusPolicy::Block,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::top(Val::Px(PAGE_TOP)),
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            GlobalZIndex(430),
            Visibility::Hidden,
        ))
        .with_children(|root| {
            root.spawn((
                PanelFrame,
                Node {
                    width: Val::Px(INNER_W + 32.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(14.0)),
                    row_gap: Val::Px(8.0),
                    border: UiRect::all(Val::Px(2.0)),
                    border_radius: BorderRadius::all(Val::Px(8.0)),
                    ..default()
                },
                BackgroundGradient::from(LinearGradient::to_bottom(vec![
                    ColorStop::percent(Color::srgba(0.16, 0.11, 0.05, 0.97), 0.0),
                    ColorStop::percent(Color::srgba(0.04, 0.03, 0.02, 0.98), 100.0),
                ])),
                BorderColor::all(EDGE),
            ));
        });
}

/// Switch tabs and craft; Esc goes back to the Hero tab, and so does leaving the menu.
pub(super) fn panel_clicks(
    clicks: Res<GlueClicks>,
    actions: Query<(Entity, &PanelAction)>,
    phase: Res<State<Phase>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut panel: ResMut<Panel>,
    mut profile: ResMut<Profile>,
) {
    if *phase.get() != Phase::Menu {
        if panel.menu != MenuTab::Hero {
            panel.menu = MenuTab::Hero;
        }
        return;
    }
    if panel.open() && keys.just_pressed(KeyCode::Escape) {
        panel.menu = MenuTab::Hero;
        return;
    }
    let Some(action) = actions
        .iter()
        .find(|(e, _)| clicks.hit(*e))
        .map(|(_, a)| *a)
    else {
        return;
    };
    match action {
        PanelAction::Menu(t) => {
            if panel.menu != t {
                panel.menu = t;
                panel.message = None;
            }
        }
        PanelAction::Tab(t) => {
            panel.tab = t;
            panel.message = None;
        }
        PanelAction::Craft(p, t) => {
            let prof = &PROFESSIONS[p];
            let name = prof.recipes[t].name;
            let profile = &mut *profile;
            panel.message = match profile.account.craft(p, t, &profile.catalog) {
                Ok(r) => {
                    profile.save();
                    Some((
                        format!(
                            "Crafted {}: {} on every run from now on.",
                            r.name,
                            prof.stat.format(r.value)
                        ),
                        UNCOMMON,
                    ))
                }
                Err(CraftError::NeedsPrevious(prev)) => {
                    Some((format!("Craft {} before {name}.", prev.name), RED))
                }
                Err(CraftError::NeedsBracket(b)) => Some((
                    format!("{name} needs {} crafting: unlock it first.", b.name()),
                    RED,
                )),
                Err(CraftError::NotEnoughPoints { need, have }) => Some((
                    format!("{name} costs {need} achievement points; you have {have}."),
                    RED,
                )),
                Err(CraftError::AlreadyKnown | CraftError::Unknown) => None,
            };
        }
        PanelAction::Unlock(b) => {
            let profile = &mut *profile;
            panel.message = match profile.account.unlock(b, &profile.catalog) {
                Ok(()) => {
                    profile.save();
                    Some((
                        format!(
                            "{} crafting unlocked: its recipes can now be crafted in every \
                             profession.",
                            b.name()
                        ),
                        UNCOMMON,
                    ))
                }
                Err(UnlockError::NeedsPrevious(prev)) => {
                    Some((format!("Unlock {} crafting first.", prev.name()), RED))
                }
                Err(UnlockError::NotEnoughPoints { need, have }) => Some((
                    format!(
                        "{} crafting costs {need} achievement points; you have {have}.",
                        b.name()
                    ),
                    RED,
                )),
                Err(UnlockError::AlreadyUnlocked) => None,
            };
        }
    }
}

/// The tab bar is up on the menu; the Professions page when its tab is picked. The pick screen's
/// own root hides itself meanwhile ([`super::hud::show_panels`]).
#[allow(clippy::type_complexity)]
pub(super) fn show_panel(
    phase: Res<State<Phase>>,
    panel: Res<Panel>,
    mut roots: ParamSet<(
        Query<(&mut Visibility, &mut Node), With<MenuTabsRoot>>,
        Query<(&mut Visibility, &mut Node), With<PanelRoot>>,
    )>,
) {
    let menu = *phase.get() == Phase::Menu;
    for root in &mut roots.p0() {
        show(root, menu);
    }
    for root in &mut roots.p1() {
        show(root, menu && panel.open());
    }
}

/// A hidden page leaves the layout too: a merely invisible button still takes clicks.
fn show((mut v, mut node): (Mut<Visibility>, Mut<Node>), on: bool) {
    let want = if on {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    if *v != want {
        *v = want;
    }
    let display = if on { Display::Flex } else { Display::None };
    if node.display != display {
        node.display = display;
    }
}

pub(super) fn hover_cells(
    mut cells: Query<(&Interaction, &Hover, &mut BackgroundColor), Changed<Interaction>>,
) {
    for (i, hover, mut bg) in &mut cells {
        bg.0 = if *i == Interaction::None {
            hover.base
        } else {
            hover.hot
        };
    }
}

/// The tab bar's buttons, rebuilt when the tab or the points waiting to be spent change: the
/// Professions tab shows how many there are.
pub(super) fn build_tabs(
    mut commands: Commands,
    panel: Res<Panel>,
    profile: Res<Profile>,
    font: Option<Res<HudFont>>,
    bars: Query<(Entity, Option<&Children>), With<MenuTabsRoot>>,
) {
    let (Some(font), Ok((bar, built))) = (font, bars.single()) else {
        return;
    };
    if built.is_some() && !panel.is_changed() && !profile.is_changed() {
        return;
    }
    commands.entity(bar).despawn_related::<Children>();
    commands.entity(bar).with_children(|row| {
        for t in MENU_TABS {
            let label = match t {
                MenuTab::Hero => "Hero".to_string(),
                MenuTab::Professions => match profile.points() {
                    0 => "Professions".to_string(),
                    n => format!("Professions ({n})"),
                },
            };
            tab_button(
                row,
                &font.0,
                label,
                PanelAction::Menu(t),
                panel.menu == t,
                18.0,
                150.0,
            );
        }
    });
}

/// Rebuild the open panel whenever it or the account changes.
pub(super) fn build_panel(
    mut commands: Commands,
    panel: Res<Panel>,
    profile: Res<Profile>,
    font: Option<Res<HudFont>>,
    frames: Query<Entity, With<PanelFrame>>,
) {
    if !panel.open() || (!panel.is_changed() && !profile.is_changed()) {
        return;
    }
    let (Some(font), Ok(frame)) = (font, frames.single()) else {
        return;
    };
    let f = &font.0;
    commands.entity(frame).despawn_related::<Children>();
    commands.entity(frame).with_children(|ui| {
        header(ui, f, &profile);
        tabs(ui, f, &panel, &profile);
        match panel.tab {
            Tab::Recipes => recipes(ui, f, &profile),
            Tab::Achievements(c) => achievements(ui, f, &profile, c),
        }
        footer(ui, f, &panel, &profile);
    });
}

fn header(ui: &mut ChildSpawnerCommands, f: &Handle<Font>, profile: &Profile) {
    let earned = profile.account.points_earned(&profile.catalog);
    let spent = profile.account.points_spent();
    ui.spawn(Node {
        justify_content: JustifyContent::SpaceBetween,
        align_items: AlignItems::End,
        ..default()
    })
    .with_children(|row| {
        row.spawn(text_bundle(f, "Professions", 30.0, GOLD));
        row.spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::End,
            ..default()
        })
        .with_children(|c| {
            c.spawn(text_bundle(
                f,
                format!("{} achievement points to spend", profile.points()),
                18.0,
                GOLD,
            ));
            c.spawn(text_bundle(
                f,
                format!("{earned} earned, {spent} spent"),
                12.0,
                DIM,
            ));
        });
    });
    ui.spawn(text_bundle(
        f,
        "Runs earn achievements, each worth points once. Each profession's best recipe adds its \
         bonus to every run on this account.",
        13.0,
        DIM,
    ));
}

fn tab_button(
    ui: &mut ChildSpawnerCommands,
    f: &Handle<Font>,
    label: String,
    action: PanelAction,
    selected: bool,
    size: f32,
    min_w: f32,
) {
    let (base, edge, color) = if selected {
        (CELL_HOT, GOLD, GOLD)
    } else {
        (CELL, EDGE, Color::srgb(0.85, 0.85, 0.85))
    };
    ui.spawn((
        action,
        Button,
        Hover {
            base,
            hot: CELL_HOT,
        },
        Node {
            min_width: Val::Px(min_w),
            justify_content: JustifyContent::Center,
            padding: UiRect::axes(Val::Px(14.0), Val::Px(6.0)),
            border: UiRect::all(Val::Px(1.0)),
            border_radius: BorderRadius::all(Val::Px(4.0)),
            ..default()
        },
        BackgroundColor(base),
        BorderColor::all(edge),
    ))
    .with_children(|b| {
        b.spawn(text_bundle(f, label, size, color));
    });
}

fn tabs(ui: &mut ChildSpawnerCommands, f: &Handle<Font>, panel: &Panel, profile: &Profile) {
    ui.spawn(Node {
        column_gap: Val::Px(8.0),
        ..default()
    })
    .with_children(|row| {
        tab_button(
            row,
            f,
            "Recipes".into(),
            PanelAction::Tab(Tab::Recipes),
            panel.tab == Tab::Recipes,
            15.0,
            0.0,
        );
        for c in ACHIEVEMENT_TABS {
            let all: Vec<&Achievement> =
                profile.catalog.iter().filter(|a| a.category == c).collect();
            let done = all
                .iter()
                .filter(|a| profile.account.has_earned(&a.key))
                .count();
            tab_button(
                row,
                f,
                format!("{} {done}/{}", c.label(), all.len()),
                PanelAction::Tab(Tab::Achievements(c)),
                panel.tab == Tab::Achievements(c),
                15.0,
                0.0,
            );
        }
    });
}

/// The trainer's brackets across the top, then six columns, one per profession, each its
/// recipes top to bottom.
fn recipes(ui: &mut ChildSpawnerCommands, f: &Handle<Font>, profile: &Profile) {
    brackets(ui, f, profile);
    let n = PROFESSIONS.len() as f32;
    let col_w = ((INNER_W - COLUMN_GAP * (n - 1.0)) / n).floor();
    let points = profile.points();
    let account = &profile.account;
    ui.spawn(Node {
        column_gap: Val::Px(COLUMN_GAP),
        ..default()
    })
    .with_children(|row| {
        for (p, prof) in PROFESSIONS.iter().enumerate() {
            let tier = account.tier(prof);
            row.spawn(Node {
                width: Val::Px(col_w),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            })
            .with_children(|col| {
                col.spawn(Node {
                    column_gap: Val::Px(8.0),
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|h| {
                    h.spawn((
                        Node {
                            width: Val::Px(28.0),
                            height: Val::Px(28.0),
                            ..default()
                        },
                        ImageNode {
                            color: Color::srgba(0.0, 0.0, 0.0, 0.6),
                            ..default()
                        },
                        SpellIcon(prof.icon_spell),
                    ));
                    h.spawn(text_bundle(f, prof.name, 15.0, GOLD));
                });
                col.spawn(text_bundle(f, prof.stat.effect(), 10.0, DIM));
                let now = match tier {
                    Some(t) => prof.stat.format(prof.recipes[t].value),
                    None => "Nothing crafted yet".into(),
                };
                col.spawn(text_bundle(
                    f,
                    now,
                    12.0,
                    if tier.is_some() { UNCOMMON } else { DIM },
                ));
                for (t, recipe) in prof.recipes.iter().enumerate() {
                    let known = account.knows(recipe.key);
                    let next = tier.map_or(t == 0, |k| t == k + 1);
                    let bracket_open = account.has_bracket(recipe.bracket);
                    // The bracket is the bigger gate, so it is the one named first.
                    let (base, edge, status, status_color) = if known {
                        (
                            CELL_KNOWN,
                            bracket_color(recipe.bracket),
                            "Known".to_string(),
                            UNCOMMON,
                        )
                    } else if !bracket_open {
                        (
                            CELL_LOCKED,
                            Color::srgb(0.25, 0.25, 0.25),
                            format!("Requires {}", recipe.bracket.name()),
                            DIM,
                        )
                    } else if !next {
                        (
                            CELL_LOCKED,
                            Color::srgb(0.25, 0.25, 0.25),
                            format!("Requires {}", prof.recipes[t - 1].name),
                            DIM,
                        )
                    } else if points >= recipe.cost {
                        (CELL, GOLD, format!("Craft: {} points", recipe.cost), GOLD)
                    } else {
                        (CELL, EDGE, format!("Needs {} points", recipe.cost), RED)
                    };
                    let open = !known && bracket_open && next;
                    let shown = known || open;
                    let name_color = if shown {
                        bracket_color(recipe.bracket)
                    } else {
                        Color::srgb(0.45, 0.45, 0.45)
                    };
                    let mut cell = col.spawn((
                        Node {
                            width: Val::Px(col_w),
                            min_height: Val::Px(54.0),
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::axes(Val::Px(6.0), Val::Px(4.0)),
                            row_gap: Val::Px(1.0),
                            border: UiRect::all(Val::Px(1.0)),
                            border_radius: BorderRadius::all(Val::Px(5.0)),
                            ..default()
                        },
                        BackgroundColor(base),
                        BorderColor::all(edge),
                    ));
                    // Only the next recipe in the chain, in an unlocked bracket, takes a click; a
                    // known or locked one is just shown.
                    if open {
                        cell.insert((
                            PanelAction::Craft(p, t),
                            Button,
                            Hover {
                                base,
                                hot: CELL_HOT,
                            },
                        ));
                    }
                    cell.with_children(|c| {
                        c.spawn(text_bundle(f, recipe.name, 12.0, name_color));
                        c.spawn(text_bundle(
                            f,
                            prof.stat.format(recipe.value),
                            11.0,
                            if shown { Color::WHITE } else { DIM },
                        ));
                        c.spawn(text_bundle(f, status, 10.0, status_color));
                    });
                }
            });
        }
    });
}

/// The four trainer brackets, each for every profession: Apprentice from the start, the rest
/// bought in order with achievement points.
fn brackets(ui: &mut ChildSpawnerCommands, f: &Handle<Font>, profile: &Profile) {
    let n = Bracket::ALL.len() as f32;
    let box_w = ((INNER_W - COLUMN_GAP * (n - 1.0)) / n).floor();
    let account = &profile.account;
    ui.spawn(Node {
        column_gap: Val::Px(COLUMN_GAP),
        ..default()
    })
    .with_children(|row| {
        for b in Bracket::ALL {
            let color = bracket_color(b);
            let (base, edge, status, status_color, clickable) = if account.has_bracket(b) {
                (CELL_KNOWN, color, "Unlocked".to_string(), UNCOMMON, false)
            } else {
                match account.check_unlock(b, &profile.catalog) {
                    Ok(()) => (
                        CELL,
                        GOLD,
                        format!("Unlock: {} points", b.cost()),
                        GOLD,
                        true,
                    ),
                    Err(UnlockError::NeedsPrevious(prev)) => (
                        CELL_LOCKED,
                        Color::srgb(0.25, 0.25, 0.25),
                        format!("Requires {}, then {} points", prev.name(), b.cost()),
                        DIM,
                        false,
                    ),
                    Err(UnlockError::NotEnoughPoints { need, .. }) => {
                        (CELL, EDGE, format!("Needs {need} points"), RED, false)
                    }
                    Err(UnlockError::AlreadyUnlocked) => {
                        (CELL_KNOWN, color, "Unlocked".to_string(), UNCOMMON, false)
                    }
                }
            };
            let mut cell = row.spawn((
                Node {
                    width: Val::Px(box_w),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::axes(Val::Px(10.0), Val::Px(5.0)),
                    row_gap: Val::Px(1.0),
                    border: UiRect::all(Val::Px(1.0)),
                    border_radius: BorderRadius::all(Val::Px(5.0)),
                    ..default()
                },
                BackgroundColor(base),
                BorderColor::all(edge),
            ));
            if clickable {
                cell.insert((
                    PanelAction::Unlock(b),
                    Button,
                    Hover {
                        base,
                        hot: CELL_HOT,
                    },
                ));
            }
            cell.with_children(|c| {
                c.spawn(Node {
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::End,
                    ..default()
                })
                .with_children(|top| {
                    top.spawn(text_bundle(
                        f,
                        format!("{} crafting", b.name()),
                        14.0,
                        color,
                    ));
                    top.spawn(text_bundle(
                        f,
                        format!("skill {}", b.skill_cap()),
                        10.0,
                        DIM,
                    ));
                });
                c.spawn(text_bundle(f, status, 11.0, status_color));
            });
        }
    });
}

/// One category's achievements, four to a row.
fn achievements(
    ui: &mut ChildSpawnerCommands,
    f: &Handle<Font>,
    profile: &Profile,
    category: Category,
) {
    const PER_ROW: f32 = 4.0;
    let cell_w = ((INNER_W - COLUMN_GAP * (PER_ROW - 1.0)) / PER_ROW).floor();
    ui.spawn(Node {
        flex_wrap: FlexWrap::Wrap,
        column_gap: Val::Px(COLUMN_GAP),
        row_gap: Val::Px(6.0),
        ..default()
    })
    .with_children(|grid| {
        for a in profile.catalog.iter().filter(|a| a.category == category) {
            let done = profile.account.has_earned(&a.key);
            let (bg, edge, name_color) = if done {
                (CELL_KNOWN, UNCOMMON, GOLD)
            } else {
                (
                    CELL_LOCKED,
                    Color::srgb(0.25, 0.25, 0.25),
                    Color::srgb(0.75, 0.75, 0.75),
                )
            };
            grid.spawn((
                Node {
                    width: Val::Px(cell_w),
                    min_height: Val::Px(46.0),
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(5.0)),
                    row_gap: Val::Px(2.0),
                    border: UiRect::all(Val::Px(1.0)),
                    border_radius: BorderRadius::all(Val::Px(5.0)),
                    ..default()
                },
                BackgroundColor(bg),
                BorderColor::all(edge),
            ))
            .with_children(|c| {
                c.spawn(Node {
                    justify_content: JustifyContent::SpaceBetween,
                    column_gap: Val::Px(6.0),
                    ..default()
                })
                .with_children(|top| {
                    top.spawn(text_bundle(f, a.name.clone(), 13.0, name_color));
                    top.spawn(text_bundle(
                        f,
                        a.points.to_string(),
                        13.0,
                        if done { GOLD } else { DIM },
                    ));
                });
                c.spawn(text_bundle(f, a.description.clone(), 11.0, DIM));
            });
        }
    });
}

/// The bonuses every run gets now, the last craft's answer, and the way back.
fn footer(ui: &mut ChildSpawnerCommands, f: &Handle<Font>, panel: &Panel, profile: &Profile) {
    let bonuses = profile.bonuses();
    ui.spawn(Node {
        justify_content: JustifyContent::SpaceBetween,
        align_items: AlignItems::Center,
        margin: UiRect::top(Val::Px(4.0)),
        ..default()
    })
    .with_children(|row| {
        row.spawn(Node {
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(3.0),
            max_width: Val::Px(INNER_W - 160.0),
            ..default()
        })
        .with_children(|c| {
            if bonuses.is_empty() {
                c.spawn(text_bundle(
                    f,
                    "No bonuses yet: craft a recipe and every run after gets its bonus.",
                    14.0,
                    DIM,
                ));
            } else {
                c.spawn(text_bundle(
                    f,
                    format!("Every run: {}", bonuses.lines().join(", ")),
                    14.0,
                    UNCOMMON,
                ));
            }
            if let Some((msg, color)) = &panel.message {
                c.spawn(text_bundle(f, msg.clone(), 13.0, *color));
            }
        });
        row.spawn(text_bundle(f, "Esc: back to Hero", 12.0, DIM));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> Profile {
        Profile {
            account: Account::default(),
            catalog: catalog(),
        }
    }

    /// The crate's point budget assumes the game's 9 classes and 16 zones.
    #[test]
    fn the_catalog_has_every_class_and_zone() {
        let cat = catalog();
        let count = |c: Category| cat.iter().filter(|a| a.category == c).count();
        assert_eq!(count(Category::Classes), data::CLASSES.len());
        assert_eq!(count(Category::Zones), 2 * data::MAPS.len());
        let earnable: u32 = cat.iter().map(|a| a.points).sum();
        let costs: u32 = PROFESSIONS
            .iter()
            .flat_map(|p| p.recipes)
            .map(|r| r.cost)
            .sum();
        assert!(earnable >= costs, "{earnable} points cannot buy {costs}");
    }

    #[test]
    fn the_results_line_names_a_few_and_counts_the_rest() {
        let p = profile();
        assert_eq!(results_line(&[], &p), "No new achievements this run.");
        let line = results_line(&[0, 1, 2, 3, 4, 5], &p);
        assert!(line.starts_with("New achievements (+"), "{line}");
        assert!(line.contains(" and 2 more"), "{line}");
        assert_eq!(
            results_line(&[0], &p),
            "New achievement (+10 points): Holding the Line"
        );
    }
}
