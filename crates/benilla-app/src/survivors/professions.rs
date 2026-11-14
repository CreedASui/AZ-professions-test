//! Professions: the part of a run that outlives it.
//!
//! Each run's achievements ([`survivors_professions::achievements`]) earn achievement points for
//! the account; the menu's Professions tab spends them on the trainer's brackets and on recipes,
//! and each profession's best recipe adds its bonus to every run after ([`super::Run::bonuses`]).
//! The rules are the `survivors-professions` crate's; this module keeps the account on disk
//! (`benilla-config/survivors-account.txt`, `localStorage` on the page), counts runs into it, and
//! draws the menu's tab bar, the Professions page and the results screen's line.

use bevy::prelude::*;
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

/// The page is laid out at this width, where no item name wraps, and [`fit_panel`] scales it down
/// whole to fit a narrower or shorter screen (4:3 is 1024 wide). Laying it out narrower instead
/// wraps the names, and wrapped text in the grid mis-measures its height.
const FRAME_W: f32 = 1240.0;
/// Room kept between the scaled page and the screen's edges.
const FIT_MARGIN: f32 = 10.0;
const COLUMN_GAP: f32 = 12.0;

/// `n` equal columns that may shrink below their content, so a narrow screen wraps text instead of
/// overflowing.
fn grid(n: usize, row_gap: f32) -> Node {
    Node {
        display: Display::Grid,
        grid_template_columns: RepeatedGridTrack::flex(n as u16, 1.0),
        column_gap: Val::Px(COLUMN_GAP),
        row_gap: Val::Px(row_gap),
        ..default()
    }
}

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

/// Where the tab bar sits, and where the Professions page starts under it.
const TABS_TOP: f32 = 12.0;
const PAGE_TOP: f32 = 58.0;

/// The menu tab that is up, and the answer to the last craft.
#[derive(Resource, Default)]
pub(super) struct Panel {
    menu: MenuTab,
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
                UiTransform::IDENTITY,
                Node {
                    width: Val::Px(FRAME_W),
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

/// Scale the page down whole when the screen is narrower or shorter than it, keeping its top just
/// under the tab bar. The transform scales the clicks' hit areas with the picture.
pub(super) fn fit_panel(
    roots: Query<&ComputedNode, With<PanelRoot>>,
    mut frames: Query<(&ComputedNode, &mut UiTransform), With<PanelFrame>>,
) {
    let (Ok(root), Ok((frame, mut tf))) = (roots.single(), frames.single_mut()) else {
        return;
    };
    // Physical pixels in, logical units out: the transform's translation is logical.
    let to_logical = root.inverse_scale_factor();
    let screen = root.size() * to_logical;
    let size = frame.size() * to_logical;
    if size.x <= 0.0 || size.y <= 0.0 || screen.x <= 0.0 {
        return;
    }
    let s = ((screen.x - 2.0 * FIT_MARGIN) / size.x)
        .min((screen.y - PAGE_TOP - FIT_MARGIN) / size.y)
        .min(1.0);
    // Scaling is about the centre: lift the page by what its top loses, so it stays under the tabs.
    let lift = Val2::px(0.0, -(1.0 - s) * size.y / 2.0);
    if (tf.scale.x - s).abs() > 1e-3 || tf.translation != lift {
        tf.scale = Vec2::splat(s);
        tf.translation = lift;
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
        recipes(ui, f, &profile);
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

/// The trainer's brackets across the top, then one grid: a column per profession, its header in
/// the first row and then a row per bracket, so each bracket's items line up across the
/// professions whatever their names wrap to.
fn recipes(ui: &mut ChildSpawnerCommands, f: &Handle<Font>, profile: &Profile) {
    brackets(ui, f, profile);
    let rows = PROFESSIONS
        .iter()
        .map(|p| p.recipes.len())
        .max()
        .unwrap_or(0);
    ui.spawn(grid(PROFESSIONS.len(), 4.0)).with_children(|g| {
        for prof in PROFESSIONS {
            profession_header(g, f, profile, prof);
        }
        for t in 0..rows {
            for (p, prof) in PROFESSIONS.iter().enumerate() {
                if t < prof.recipes.len() {
                    recipe_cell(g, f, profile, p, t);
                } else {
                    // Keeps the next profession in its own column.
                    g.spawn(Node::default());
                }
            }
        }
    });
}

/// A profession's icon and name, what it upgrades, and what it gives every run now.
fn profession_header(
    g: &mut ChildSpawnerCommands,
    f: &Handle<Font>,
    profile: &Profile,
    prof: &'static survivors_professions::Profession,
) {
    let tier = profile.account.tier(prof);
    g.spawn(Node {
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(4.0),
        padding: UiRect::bottom(Val::Px(2.0)),
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
    });
}

/// The item at `tier` of the profession at index `p`: known, craftable, or what it waits for.
fn recipe_cell(
    g: &mut ChildSpawnerCommands,
    f: &Handle<Font>,
    profile: &Profile,
    p: usize,
    t: usize,
) {
    let account = &profile.account;
    let prof = &PROFESSIONS[p];
    let recipe = &prof.recipes[t];
    let tier = account.tier(prof);
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
        // The item it needs is the one right above it; naming it would wrap the longest names.
        (
            CELL_LOCKED,
            Color::srgb(0.25, 0.25, 0.25),
            "Requires the item above".to_string(),
            DIM,
        )
    } else if profile.points() >= recipe.cost {
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
    let mut cell = g.spawn((
        Node {
            min_height: Val::Px(64.0),
            flex_direction: FlexDirection::Column,
            padding: UiRect::axes(Val::Px(7.0), Val::Px(6.0)),
            row_gap: Val::Px(2.0),
            border: UiRect::all(Val::Px(1.0)),
            border_radius: BorderRadius::all(Val::Px(5.0)),
            ..default()
        },
        BackgroundColor(base),
        BorderColor::all(edge),
    ));
    // Only the next recipe in the chain, in an unlocked bracket, takes a click; a known or locked
    // one is just shown.
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
        c.spawn(text_bundle(f, recipe.name, 13.0, name_color));
        c.spawn(text_bundle(
            f,
            prof.stat.format(recipe.value),
            12.0,
            if shown { Color::WHITE } else { DIM },
        ));
        c.spawn(text_bundle(f, status, 11.0, status_color));
    });
}

/// The four trainer brackets, each for every profession: Apprentice from the start, the rest
/// bought in order with achievement points.
fn brackets(ui: &mut ChildSpawnerCommands, f: &Handle<Font>, profile: &Profile) {
    let account = &profile.account;
    ui.spawn(grid(Bracket::ALL.len(), 0.0))
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
            max_width: Val::Percent(85.0),
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
    use survivors_professions::achievements::Category;

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

/// A picture of the Professions tab without the game: no WoW data, so a stand-in font (DejaVu
/// Serif for Friz Quadrata), dark squares where the spell icons go, and no world behind the page.
/// It needs a GPU adapter (Mesa's lavapipe is enough) and writes two PNGs: the tab at 16:9, and
/// again at 4:3, the narrowest screen the menu lays out for:
///
/// ```text
/// PROFESSIONS_PREVIEW=<dir> cargo test -p benilla-app --lib render_professions_tab -- --ignored
/// ```
#[cfg(test)]
mod preview {
    use super::*;
    use bevy::camera::RenderTarget;
    use bevy::render::render_resource::{Extent3d, TextureFormat};
    use bevy::render::view::screenshot::{save_to_disk, Screenshot};
    use survivors_professions::Bracket;

    /// The 768-unit canvas the menu lays out on, 16:9 first, drawn at twice its size.
    const W: f32 = 1366.0;
    const W_4X3: f32 = 1024.0;
    const H: f32 = 768.0;
    const SCALE: f32 = 2.0;
    const FONT: &str = "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf";

    #[derive(Resource)]
    struct Target(Handle<Image>);

    /// An account a few hours in: two runs' achievements, Journeyman unlocked, five items crafted.
    fn sample_profile() -> Profile {
        let mut p = Profile {
            account: Account::default(),
            catalog: catalog(),
        };
        let zone = |name: &str| data::MAPS.iter().position(|m| m.name == name).unwrap();
        let runs = [
            RunRecord {
                class_id: 8,
                zone: zone("Elwynn Forest"),
                seconds: 900.0,
                level: 24,
                kills: 1700,
                bosses_slain: 4,
                won: true,
            },
            RunRecord {
                class_id: 1,
                zone: zone("Durotar"),
                seconds: 420.0,
                level: 15,
                kills: 800,
                bosses_slain: 2,
                won: false,
            },
        ];
        for run in &runs {
            p.account.record_run(run, &p.catalog);
        }
        p.account.unlock(Bracket::Journeyman, &p.catalog).unwrap();
        // Lesser and Greater Magic Wand, Rough and Coarse Dynamite, Minor Healing Potion.
        for (prof, tier) in [(2, 0), (2, 1), (3, 0), (3, 1), (0, 0)] {
            p.account.craft(prof, tier, &p.catalog).unwrap();
        }
        p
    }

    fn camera(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
        let image = Image::new_target_texture(
            (W * SCALE) as u32,
            (H * SCALE) as u32,
            TextureFormat::Rgba8UnormSrgb,
            None,
        );
        let handle = images.add(image);
        commands.spawn((
            Camera2d,
            Camera {
                // Where the dimmed world would be.
                clear_color: ClearColorConfig::Custom(Color::srgb(0.12, 0.11, 0.1)),
                ..default()
            },
            RenderTarget::Image(handle.clone().into()),
            // With no window there is no camera the UI would pick by itself.
            IsDefaultUiCamera,
        ));
        commands.insert_resource(Target(handle));
    }

    #[test]
    #[ignore = "renders on a GPU adapter and writes PNGs"]
    fn render_professions_tab() {
        let dir = std::path::PathBuf::from(
            std::env::var("PROFESSIONS_PREVIEW").unwrap_or_else(|_| "professions-preview".into()),
        );
        std::fs::create_dir_all(&dir).unwrap();
        let mut app = App::new();
        app.add_plugins(
            DefaultPlugins
                .build()
                .disable::<bevy::winit::WinitPlugin>()
                .set(bevy::window::WindowPlugin {
                    primary_window: None,
                    exit_condition: bevy::window::ExitCondition::DontExit,
                    ..default()
                }),
        )
        .init_state::<Phase>()
        .insert_resource(UiScale(SCALE))
        .insert_resource(sample_profile())
        .insert_resource(Panel {
            menu: MenuTab::Professions,
            message: Some((
                "Crafted Coarse Dynamite: +20% Area of effect on every run from now on.".into(),
                UNCOMMON,
            )),
        })
        .add_systems(Startup, (setup_panel, camera))
        .add_systems(
            Update,
            (build_tabs, build_panel, show_panel, fit_panel).chain(),
        );
        app.finish();
        app.cleanup();
        let font = Font::try_from_bytes(std::fs::read(FONT).unwrap()).unwrap();
        let font = app.world_mut().resource_mut::<Assets<Font>>().add(font);
        app.insert_resource(HudFont(font));
        // Startup makes the target the shots resize.
        app.update();

        let shots = [
            (W, "professions-recipes.png"),
            (W_4X3, "professions-recipes-4x3.png"),
        ];
        for (width, name) in shots {
            let target = app.world().resource::<Target>().0.clone();
            if let Some(image) = app
                .world_mut()
                .resource_mut::<Assets<Image>>()
                .get_mut(&target)
            {
                image.resize(Extent3d {
                    width: (width * SCALE) as u32,
                    height: (H * SCALE) as u32,
                    depth_or_array_layers: 1,
                });
            }
            for _ in 0..20 {
                app.update();
            }
            let path = dir.join(name);
            let _ = std::fs::remove_file(&path);
            app.world_mut()
                .spawn(Screenshot::image(target))
                .observe(save_to_disk(path.clone()));
            let mut frames = 0;
            while !path.exists() {
                app.update();
                frames += 1;
                assert!(frames < 600, "no picture of {name}");
            }
            // The observer writes the file inside the frame; one more frame for good measure.
            app.update();
            println!("wrote {}", path.display());
        }
    }
}
