#![cfg_attr(all(windows, not(feature = "dev")), windows_subsystem = "windows")]

use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use eframe::egui;
use egui_extras::{Column, TableBuilder};
#[cfg(feature = "dev")]
use egui_extras::{Size, StripBuilder};
use serde::{Deserialize, Serialize};

mod currency;
mod localization;

use currency::{
    format_display_amount, format_internal_amount, format_internal_for_command,
    parse_display_amount, parse_display_to_internal,
};
use localization::Localization;

const BRIDGE_ADDR: &str = "127.0.0.1:28452";
#[cfg(feature = "dev")]
const REQUIRED_BRIDGE_VERSION: &str = "0.2.49";
#[cfg(not(feature = "dev"))]
const REQUIRED_BRIDGE_VERSION: &str = "0.2.49";
#[cfg(feature = "dev")]
const BRIDGE_PROTOCOL_VERSION: u32 = 9;
#[cfg(not(feature = "dev"))]
const BRIDGE_PROTOCOL_VERSION: u32 = 9;
const MINIMUM_SAFE_BRIDGE_PROTOCOL: u32 = 1;
const MAXIMUM_SAFE_BRIDGE_PROTOCOL: u32 = BRIDGE_PROTOCOL_VERSION;
const SUPPORTED_TFM2_VERSION: &str = "0.5.4";
const GITHUB_RELEASES_URL: &str = "https://github.com/jal-io/tfm2-editor/releases/latest";
const STEAM_WORKSHOP_URL: &str =
    "https://steamcommunity.com/sharedfiles/filedetails/?id=3775240765";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompatibilitySeverity {
    Warning,
    NotSupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompatibilityAction {
    BridgeUpdate,
    EditorUpdate,
    VerifyInstallation,
    GameVersionMismatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompatibilityReason {
    VersionMismatch,
    ProtocolMismatch,
    UnverifiedLegacyBridge,
    GameTargetMismatch,
    KnownUnsupportedCombination,
}

#[derive(Debug, Clone, Copy)]
enum UnsupportedRequirement {
    Bridge(&'static str),
}

#[derive(Debug, Clone, Copy)]
struct UnsupportedBridgeRule {
    minimum_bridge_version: Option<&'static str>,
    maximum_bridge_version: Option<&'static str>,
    requirement: UnsupportedRequirement,
}

#[cfg(feature = "dev")]
const KNOWN_UNSUPPORTED_BRIDGE_RULES: &[UnsupportedBridgeRule] = &[UnsupportedBridgeRule {
    minimum_bridge_version: None,
    maximum_bridge_version: Some("0.2.48"),
    requirement: UnsupportedRequirement::Bridge(REQUIRED_BRIDGE_VERSION),
}];

#[cfg(not(feature = "dev"))]
const KNOWN_UNSUPPORTED_BRIDGE_RULES: &[UnsupportedBridgeRule] = &[UnsupportedBridgeRule {
    minimum_bridge_version: None,
    maximum_bridge_version: Some("0.2.48"),
    requirement: UnsupportedRequirement::Bridge(REQUIRED_BRIDGE_VERSION),
}];

#[derive(Debug, Clone)]
struct CompatibilityIssue {
    severity: CompatibilitySeverity,
    action: CompatibilityAction,
    reason: CompatibilityReason,
    installed_bridge_version: String,
    bridge_tfm2_target: Option<String>,
    required_bridge_version: Option<String>,
    required_editor_version: Option<String>,
}
#[cfg(feature = "dev")]
const APP_VERSION: &str = "0.5.10";
#[cfg(not(feature = "dev"))]
const APP_VERSION: &str = "0.4.2";

#[cfg(feature = "dev")]
fn display_version() -> String {
    format!("v{APP_VERSION}-dev")
}

#[cfg(not(feature = "dev"))]
fn display_version() -> String {
    format!("v{APP_VERSION}")
}

#[cfg(feature = "dev")]
fn window_title() -> String {
    format!("TFM2 Editor v{APP_VERSION}-dev")
}

#[cfg(not(feature = "dev"))]
fn window_title() -> String {
    format!("TFM2 Editor v{APP_VERSION}")
}

fn compare_semver(left: &str, right: &str) -> Option<Ordering> {
    fn parse(value: &str) -> Option<(u32, u32, u32)> {
        let value = value.trim().trim_start_matches('v');
        let mut parts = value.split('.');
        let major = parts.next()?.parse::<u32>().ok()?;
        let minor = parts.next()?.parse::<u32>().ok()?;
        let patch = parts.next()?.parse::<u32>().ok()?;
        if parts.next().is_some() {
            return None;
        }
        Some((major, minor, patch))
    }

    Some(parse(left)?.cmp(&parse(right)?))
}

fn unsupported_bridge_rule_for(bridge_version: &str) -> Option<UnsupportedBridgeRule> {
    KNOWN_UNSUPPORTED_BRIDGE_RULES
        .iter()
        .copied()
        .find(|rule| {
            let minimum_matches = rule
                .minimum_bridge_version
                .map(|minimum| {
                    matches!(
                        compare_semver(bridge_version, minimum),
                        Some(Ordering::Equal | Ordering::Greater)
                    )
                })
                .unwrap_or(true);
            let maximum_matches = rule
                .maximum_bridge_version
                .map(|maximum| {
                    matches!(
                        compare_semver(bridge_version, maximum),
                        Some(Ordering::Equal | Ordering::Less)
                    )
                })
                .unwrap_or(true);
            minimum_matches && maximum_matches
        })
}

fn compatibility_action_for_version(bridge_version: &str) -> CompatibilityAction {
    match compare_semver(bridge_version, REQUIRED_BRIDGE_VERSION) {
        Some(Ordering::Less) => CompatibilityAction::BridgeUpdate,
        Some(Ordering::Greater) => CompatibilityAction::EditorUpdate,
        _ => CompatibilityAction::VerifyInstallation,
    }
}

fn open_url(url: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", url])
            .spawn()
            .map_err(|error| format!("Could not open web browser: {error}"))?;
        return Ok(());
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|error| format!("Could not open web browser: {error}"))?;
        return Ok(());
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|error| format!("Could not open web browser: {error}"))?;
        return Ok(());
    }

    #[allow(unreachable_code)]
    Err("Opening a web browser is not supported on this platform".to_string())
}

#[cfg(feature = "dev")]
fn player_editor_intro_key() -> &'static str {
    "player_editor.intro.dev"
}

#[cfg(not(feature = "dev"))]
fn player_editor_intro_key() -> &'static str {
    "player_editor.intro"
}

fn render_editor_safety_recommendation(ui: &mut egui::Ui, localization: &Localization) {
    #[cfg(feature = "dev")]
    ui.weak(localization.tr("editor.recommended_save"));

    #[cfg(not(feature = "dev"))]
    ui.horizontal_wrapped(|ui| {
        ui.label(
            egui::RichText::new(localization.tr("editor.recommended_save_prefix"))
                .strong()
                .color(egui::Color32::from_rgb(235, 196, 0)),
        );
        ui.label(localization.tr("editor.recommended_save_text"));
    });

    ui.add_space(8.0);
}

#[cfg(feature = "dev")]
fn salary_info_key() -> Option<&'static str> {
    Some("player_editor.contract.finance_info.dev")
}

#[cfg(not(feature = "dev"))]
fn salary_info_key() -> Option<&'static str> {
    None
}

#[cfg(feature = "dev")]
fn potential_info_key() -> &'static str {
    "player_editor.potential.info.dev"
}

#[cfg(not(feature = "dev"))]
fn potential_info_key() -> &'static str {
    "player_editor.potential.info"
}

#[cfg(feature = "dev")]
fn transfer_runtime_key() -> &'static str {
    "recruitment.runtime_toggle.dev"
}

#[cfg(not(feature = "dev"))]
fn transfer_runtime_key() -> &'static str {
    "recruitment.runtime_toggle"
}

#[cfg(feature = "dev")]
fn recruitment_player_management_key() -> &'static str {
    "recruitment.player_management.info.dev"
}

#[cfg(not(feature = "dev"))]
fn recruitment_player_management_key() -> &'static str {
    "recruitment.player_management.info"
}

#[cfg(feature = "dev")]
fn search_intro_key() -> &'static str {
    "search.intro.dev"
}

#[cfg(not(feature = "dev"))]
fn search_intro_key() -> &'static str {
    "search.intro"
}

#[cfg(feature = "dev")]
fn advanced_search_info_key() -> &'static str {
    "search.advanced.info.dev"
}

#[cfg(not(feature = "dev"))]
fn advanced_search_info_key() -> &'static str {
    "search.advanced.info"
}

#[cfg(feature = "dev")]
fn champion_mastery_help_key() -> &'static str {
    "champion_mastery.help.dev"
}

#[cfg(not(feature = "dev"))]
fn champion_mastery_help_key() -> &'static str {
    "champion_mastery.help"
}

#[cfg(feature = "dev")]
const CHAMPION_MASTERY_CARD_INNER_WIDTH: f32 = 172.0;
#[cfg(feature = "dev")]
const CHAMPION_MASTERY_CARD_INNER_HEIGHT: f32 = 38.0;
#[cfg(feature = "dev")]
const CHAMPION_MASTERY_CARD_NAME_WIDTH: f32 = 82.0;
#[cfg(feature = "dev")]
const CHAMPION_MASTERY_CARD_NAME_HEIGHT: f32 = 34.0;
#[cfg(feature = "dev")]
const CHAMPION_MASTERY_CARD_VALUE_WIDTH: f32 = 64.0;
#[cfg(feature = "dev")]
const CHAMPION_MASTERY_CARD_OUTER_WIDTH: f32 = 184.0;
#[cfg(feature = "dev")]
const CHAMPION_MASTERY_CARD_HORIZONTAL_GAP: f32 = 8.0;
#[cfg(feature = "dev")]
const CHAMPION_MASTERY_CARD_VERTICAL_GAP: f32 = 8.0;
#[cfg(feature = "dev")]
const CHAMPION_MASTERY_SCROLLBAR_RESERVE: f32 = 18.0;
#[cfg(feature = "dev")]
const CHAMPION_MASTERY_NAME_LINE_LIMIT: usize = 11;

#[cfg(feature = "dev")]
fn champion_mastery_card_display_name(name: &str) -> String {
    let name = name.trim();
    let chars = name.chars().collect::<Vec<_>>();

    if chars.len() <= CHAMPION_MASTERY_NAME_LINE_LIMIT {
        return name.to_string();
    }

    let first_break = (1..=CHAMPION_MASTERY_NAME_LINE_LIMIT)
        .rev()
        .find(|&index| chars[index - 1].is_whitespace())
        .unwrap_or(CHAMPION_MASTERY_NAME_LINE_LIMIT);

    let first_line = chars[..first_break]
        .iter()
        .collect::<String>()
        .trim()
        .to_string();

    let mut remainder_start = first_break;
    while remainder_start < chars.len() && chars[remainder_start].is_whitespace() {
        remainder_start += 1;
    }

    let remainder = &chars[remainder_start..];
    if remainder.len() <= CHAMPION_MASTERY_NAME_LINE_LIMIT {
        return format!("{first_line}\n{}", remainder.iter().collect::<String>());
    }

    let visible_chars = CHAMPION_MASTERY_NAME_LINE_LIMIT.saturating_sub(1);
    let second_line = remainder[..visible_chars].iter().collect::<String>();
    format!("{first_line}\n{second_line}…")
}

#[cfg(feature = "dev")]
fn champion_mastery_columns_for_width(available_width: f32) -> usize {
    let usable_width = (available_width - CHAMPION_MASTERY_SCROLLBAR_RESERVE)
        .max(CHAMPION_MASTERY_CARD_OUTER_WIDTH);
    ((usable_width + CHAMPION_MASTERY_CARD_HORIZONTAL_GAP)
        / (CHAMPION_MASTERY_CARD_OUTER_WIDTH + CHAMPION_MASTERY_CARD_HORIZONTAL_GAP))
        .floor()
        .max(1.0) as usize
}

#[cfg(feature = "dev")]
fn search_rating_info_key() -> &'static str {
    "search.players.rating_info.dev"
}

#[cfg(not(feature = "dev"))]
fn search_rating_info_key() -> &'static str {
    "search.players.rating_info"
}

#[cfg(feature = "dev")]
fn search_player_table_help_key() -> &'static str {
    "search.players.table_help.dev"
}

#[cfg(not(feature = "dev"))]
fn search_player_table_help_key() -> &'static str {
    "search.players.table_help"
}

#[cfg(feature = "dev")]
fn search_staff_table_help_key() -> &'static str {
    "search.staff.table_help.dev"
}

#[cfg(not(feature = "dev"))]
fn search_staff_table_help_key() -> &'static str {
    "search.staff.table_help"
}

#[cfg(feature = "dev")]
fn economy_info_key() -> &'static str {
    "economy.info.dev"
}

#[cfg(not(feature = "dev"))]
fn economy_info_key() -> &'static str {
    "economy.info"
}

#[cfg(feature = "dev")]
fn economy_apply_key() -> &'static str {
    "economy.apply.dev"
}

#[cfg(not(feature = "dev"))]
fn economy_apply_key() -> &'static str {
    "economy.apply"
}

#[cfg(feature = "dev")]
fn move_player_tooltip_key() -> &'static str {
    "recruitment.player_management.move_tooltip.dev"
}

#[cfg(not(feature = "dev"))]
fn move_player_tooltip_key() -> &'static str {
    "recruitment.player_management.move_tooltip"
}

#[cfg(feature = "dev")]
fn transfer_success_tooltip_key() -> &'static str {
    "recruitment.transfer_success_tooltip.dev"
}

#[cfg(not(feature = "dev"))]
fn transfer_success_tooltip_key() -> &'static str {
    "recruitment.transfer_success_tooltip"
}

#[cfg(feature = "dev")]
fn instant_retry_tooltip_key() -> &'static str {
    "recruitment.instant_retry_tooltip.dev"
}

#[cfg(not(feature = "dev"))]
fn instant_retry_tooltip_key() -> &'static str {
    "recruitment.instant_retry_tooltip"
}

#[cfg(feature = "dev")]
fn champion_inactive_info_key() -> &'static str {
    "champion_mastery.inactive_info.dev"
}

#[cfg(not(feature = "dev"))]
fn champion_inactive_info_key() -> &'static str {
    "champion_mastery.inactive_info"
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppTab {
    Economy,
    PlayerEditor,
    StaffEditor,
    #[cfg(feature = "dev")]
    Team,
    Recruitment,
    Search,
}

impl AppTab {
    #[cfg(feature = "dev")]
    const ALL: [Self; 6] = [
        Self::Search,
        Self::PlayerEditor,
        Self::StaffEditor,
        Self::Team,
        Self::Recruitment,
        Self::Economy,
    ];

    #[cfg(not(feature = "dev"))]
    const ALL: [Self; 5] = [
        Self::Search,
        Self::PlayerEditor,
        Self::StaffEditor,
        Self::Recruitment,
        Self::Economy,
    ];

    fn label_key(self) -> &'static str {
        match self {
            Self::Economy => "tabs.economy",
            Self::PlayerEditor => "tabs.player_editor",
            Self::StaffEditor => "tabs.staff_editor",
            #[cfg(feature = "dev")]
            Self::Team => "tabs.team",
            Self::Recruitment => "tabs.recruitment",
            Self::Search => "tabs.search",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecruitmentManagementTab {
    Players,
    Staff,
}

impl RecruitmentManagementTab {
    const ALL: [Self; 2] = [Self::Players, Self::Staff];

    fn label_key(self) -> &'static str {
        match self {
            Self::Players => "recruitment.tabs.player_management",
            Self::Staff => "recruitment.tabs.staff_management",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchTab {
    Players,
    Staff,
    Lists,
    Teams,
    #[cfg(feature = "dev")]
    History,
}

impl SearchTab {
    #[cfg(feature = "dev")]
    const ALL: [Self; 5] = [
        Self::Players,
        Self::Staff,
        Self::Lists,
        Self::Teams,
        Self::History,
    ];

    #[cfg(not(feature = "dev"))]
    const ALL: [Self; 4] = [Self::Players, Self::Staff, Self::Lists, Self::Teams];

    fn label_key(self) -> &'static str {
        match self {
            Self::Players => "search.tabs.players",
            Self::Staff => "search.tabs.staff",
            Self::Lists => "search.tabs.lists",
            Self::Teams => "search.tabs.teams",
            #[cfg(feature = "dev")]
            Self::History => "search.tabs.history",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlayerSortColumn {
    Name,
    Id,
    Age,
    Team,
    Position,
    ActualRating,
    PotentialRating,
    ActualPotential,
    Salary,
    ContractEnd,
    LastHitting,
    SkillshotDodging,
    SkillshotAccuracy,
    InputSpeed,
    Positioning,
    Judgment,
    Mental,
    Focus,
    Calls,
    Roaming,
    Aggression,
    Ego,
}

impl PlayerSortColumn {
    fn label_key(self) -> &'static str {
        match self {
            Self::Name => "common.name",
            Self::Id => "common.id",
            Self::Age => "common.age",
            Self::Team => "common.team",
            Self::Position => "search.players.position",
            Self::ActualRating => "search.columns.actual_rating",
            Self::PotentialRating => "search.columns.potential_rating",
            Self::ActualPotential => "search.players.actual_potential",
            Self::Salary => "search.columns.salary",
            Self::ContractEnd => "contract.end",
            Self::LastHitting => "attributes.last_hitting",
            Self::SkillshotDodging => "attributes.skillshot_dodging",
            Self::SkillshotAccuracy => "attributes.skillshot_accuracy",
            Self::InputSpeed => "attributes.input_speed",
            Self::Positioning => "attributes.positioning",
            Self::Judgment => "attributes.judgment",
            Self::Mental => "attributes.mental",
            Self::Focus => "attributes.focus",
            Self::Calls => "attributes.calls",
            Self::Roaming => "attributes.roaming",
            Self::Aggression => "attributes.aggression",
            Self::Ego => "attributes.ego",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StaffSortColumn {
    Name,
    Id,
    Age,
    Team,
    Role,
    Salary,
    ContractEnd,
    BanPick,
    Strategy,
    Negotiation,
    JudgeAbility,
    JudgePotential,
    Feedback,
    PowerAnalysis,
    ControlCoaching,
    JudgmentCoaching,
    MentalCoaching,
    Communication,
}

impl StaffSortColumn {
    fn label_key(self) -> &'static str {
        match self {
            Self::Name => "common.name",
            Self::Id => "common.id",
            Self::Age => "common.age",
            Self::Team => "common.team",
            Self::Role => "common.role",
            Self::Salary => "search.columns.salary",
            Self::ContractEnd => "contract.end",
            Self::BanPick => "staff.attributes.ban_pick",
            Self::Strategy => "staff.attributes.strategy",
            Self::Negotiation => "staff.attributes.negotiation",
            Self::JudgeAbility => "staff.attributes.ability_analysis",
            Self::JudgePotential => "staff.attributes.potential_analysis",
            Self::Feedback => "staff.attributes.feedback",
            Self::PowerAnalysis => "staff.attributes.power_analysis",
            Self::ControlCoaching => "staff.attributes.control_coaching",
            Self::JudgmentCoaching => "staff.attributes.judgment_coaching",
            Self::MentalCoaching => "staff.attributes.mental_coaching",
            Self::Communication => "staff_editor.communication.heading",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TeamSortColumn {
    Name,
    Id,
    League,
    Manager,
    PlayerTeam,
    RosterSize,
    StaffCount,
    RosterRating,
    MerchandiseFacilityGrade,
    StadiumGrade,
    TrainingFacilityGrade,
    Money,
    RecruitmentBudget,
    SalaryBudget,
}

impl TeamSortColumn {
    fn label_key(self) -> &'static str {
        match self {
            Self::Name => "common.name",
            Self::Id => "common.id",
            Self::League => "search.teams.league",
            Self::Manager => "search.teams.manager",
            Self::PlayerTeam => "search.teams.player_team",
            Self::RosterSize => "search.teams.players",
            Self::StaffCount => "search.teams.staff",
            Self::RosterRating => "search.teams.roster_rating",
            Self::MerchandiseFacilityGrade => "search.teams.merchandise_facility_grade",
            Self::StadiumGrade => "search.teams.stadium_grade",
            Self::TrainingFacilityGrade => "search.teams.training_facility_grade",
            Self::Money => "economy.money",
            Self::RecruitmentBudget => "economy.transfer_budget",
            Self::SalaryBudget => "economy.salary_budget",
        }
    }
}

#[derive(Default, Clone)]
struct EconomyForm {
    money: String,
    transfer_budget: String,
    salary_budget: String,
}


#[derive(Debug, Clone)]
struct AdvancedRangeFilter {
    key: &'static str,
    label: &'static str,
    enabled: bool,
    min: String,
    max: String,
    unit: &'static str,
}

impl AdvancedRangeFilter {
    fn new(key: &'static str, label: &'static str, unit: &'static str) -> Self {
        Self {
            key,
            label,
            enabled: false,
            min: String::new(),
            max: String::new(),
            unit,
        }
    }
}

#[derive(Debug, Clone)]
struct AdvancedPlayerSearch {
    position_enabled: bool,
    position: String,
    region_enabled: bool,
    region: String,
    free_agents_only: bool,
    ranges: Vec<AdvancedRangeFilter>,
}

impl Default for AdvancedPlayerSearch {
    fn default() -> Self {
        Self {
            position_enabled: false,
            position: "No Condition".to_string(),
            region_enabled: false,
            region: "No Condition".to_string(),
            free_agents_only: false,
            ranges: vec![
                AdvancedRangeFilter::new("age", "common.age", ""),
                AdvancedRangeFilter::new("salary", "search.columns.salary", ""),
                AdvancedRangeFilter::new("transfer_fee", "contract.transfer_fee", ""),
                AdvancedRangeFilter::new("actual_rating", "search.columns.actual_rating", ""),
                AdvancedRangeFilter::new(
                    "actual_potential",
                    "search.players.actual_potential",
                    "",
                ),
                AdvancedRangeFilter::new("last_hit", "attributes.last_hitting", ""),
                AdvancedRangeFilter::new("skill_avoid", "attributes.skillshot_dodging", ""),
                AdvancedRangeFilter::new("skill_hit", "attributes.skillshot_accuracy", ""),
                AdvancedRangeFilter::new("control_speed", "attributes.input_speed", ""),
                AdvancedRangeFilter::new("positioning", "attributes.positioning", ""),
                AdvancedRangeFilter::new("judgement", "attributes.judgment", ""),
                AdvancedRangeFilter::new("mental", "attributes.mental", ""),
                AdvancedRangeFilter::new("concentration", "attributes.focus", ""),
                AdvancedRangeFilter::new("order", "attributes.calls", ""),
                AdvancedRangeFilter::new("roaming", "attributes.roaming", ""),
                AdvancedRangeFilter::new("aggressive", "attributes.aggression", ""),
                AdvancedRangeFilter::new("ego", "attributes.ego", ""),
            ],
        }
    }
}

impl AdvancedPlayerSearch {
    fn active_condition_count(&self) -> usize {
        usize::from(self.position_enabled && self.position != "No Condition")
            + usize::from(self.region_enabled && self.region != "No Condition")
            + usize::from(self.free_agents_only)
            + self.ranges.iter().filter(|range| range.enabled).count()
    }

    fn export_text(&self) -> String {
        let mut lines = vec![
            "money_unit_format=display_v1".to_string(),
            format!("position_enabled={}", self.position_enabled),
            format!("position={}", self.position.replace(['\n', '\r'], " ")),
            format!("region_enabled={}", self.region_enabled),
            format!("region={}", self.region.replace(['\n', '\r'], " ")),
            format!("free_agents_only={}", self.free_agents_only),
        ];

        for range in &self.ranges {
            lines.push(format!("range.{}.enabled={}", range.key, range.enabled));
            lines.push(format!("range.{}.min={}", range.key, range.min.replace(['\n', '\r'], " ")));
            lines.push(format!("range.{}.max={}", range.key, range.max.replace(['\n', '\r'], " ")));
        }

        lines.join("\n") + "\n"
    }

    fn import_text(&mut self, text: &str) {
        let uses_display_money = text
            .lines()
            .any(|line| line.trim() == "money_unit_format=display_v1");

        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };

            match key {
                "position_enabled" => self.position_enabled = parse_saved_bool(value),
                "position" => self.position = value.to_string(),
                "region_enabled" => self.region_enabled = parse_saved_bool(value),
                "region" => self.region = value.to_string(),
                "free_agents_only" => self.free_agents_only = parse_saved_bool(value),
                _ => {
                    let Some(rest) = key.strip_prefix("range.") else {
                        continue;
                    };
                    let Some((range_key, field)) = rest.rsplit_once('.') else {
                        continue;
                    };
                    let Some(range) = self.ranges.iter_mut().find(|range| range.key == range_key) else {
                        continue;
                    };
                    match field {
                        "enabled" => range.enabled = parse_saved_bool(value),
                        "min" => range.min = value.to_string(),
                        "max" => range.max = value.to_string(),
                        _ => {}
                    }
                }
            }
        }

        if !uses_display_money {
            migrate_legacy_money_ranges(&mut self.ranges, &["salary", "transfer_fee"]);
        }
    }
}

#[derive(Debug, Clone)]
struct AdvancedStaffSearch {
    role_enabled: bool,
    role: String,
    free_agents_only: bool,
    ranges: Vec<AdvancedRangeFilter>,
}

impl Default for AdvancedStaffSearch {
    fn default() -> Self {
        Self {
            role_enabled: false,
            role: "No Condition".to_string(),
            free_agents_only: false,
            ranges: vec![
                AdvancedRangeFilter::new("age", "common.age", ""),
                AdvancedRangeFilter::new("salary", "search.columns.salary", ""),
                AdvancedRangeFilter::new("banpick", "staff.attributes.ban_pick", ""),
                AdvancedRangeFilter::new("strategy", "staff.attributes.strategy", ""),
                AdvancedRangeFilter::new("negotiation", "staff.attributes.negotiation", ""),
                AdvancedRangeFilter::new("judge_ability", "staff.attributes.ability_analysis", ""),
                AdvancedRangeFilter::new("judge_potential", "staff.attributes.potential_analysis", ""),
                AdvancedRangeFilter::new("feedback", "staff.attributes.feedback", ""),
                AdvancedRangeFilter::new("power_analysis", "staff.attributes.power_analysis", ""),
                AdvancedRangeFilter::new("control_coaching", "staff.attributes.control_coaching", ""),
                AdvancedRangeFilter::new("judgment_coaching", "staff.attributes.judgment_coaching", ""),
                AdvancedRangeFilter::new("mental_coaching", "staff.attributes.mental_coaching", ""),
                AdvancedRangeFilter::new("communication", "staff_editor.communication.heading", ""),
            ],
        }
    }
}

impl AdvancedStaffSearch {
    fn active_condition_count(&self) -> usize {
        usize::from(self.role_enabled && self.role != "No Condition")
            + usize::from(self.free_agents_only)
            + self.ranges.iter().filter(|range| range.enabled).count()
    }

    fn export_text(&self) -> String {
        let mut lines = vec![
            "money_unit_format=display_v1".to_string(),
            format!("role_enabled={}", self.role_enabled),
            format!("role={}", self.role.replace(['\n', '\r'], " ")),
            format!("free_agents_only={}", self.free_agents_only),
        ];

        for range in &self.ranges {
            lines.push(format!("range.{}.enabled={}", range.key, range.enabled));
            lines.push(format!("range.{}.min={}", range.key, range.min.replace(['\n', '\r'], " ")));
            lines.push(format!("range.{}.max={}", range.key, range.max.replace(['\n', '\r'], " ")));
        }

        lines.join("\n") + "\n"
    }

    fn import_text(&mut self, text: &str) {
        let uses_display_money = text
            .lines()
            .any(|line| line.trim() == "money_unit_format=display_v1");

        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };

            match key {
                "role_enabled" => self.role_enabled = parse_saved_bool(value),
                "role" => self.role = value.to_string(),
                "free_agents_only" => self.free_agents_only = parse_saved_bool(value),
                _ => {
                    let Some(rest) = key.strip_prefix("range.") else {
                        continue;
                    };
                    let Some((range_key, field)) = rest.rsplit_once('.') else {
                        continue;
                    };
                    let Some(range) = self.ranges.iter_mut().find(|range| range.key == range_key) else {
                        continue;
                    };
                    match field {
                        "enabled" => range.enabled = parse_saved_bool(value),
                        "min" => range.min = value.to_string(),
                        "max" => range.max = value.to_string(),
                        _ => {}
                    }
                }
            }
        }

        if !uses_display_money {
            migrate_legacy_money_ranges(&mut self.ranges, &["salary"]);
        }
    }
}

fn migrate_legacy_money_ranges(ranges: &mut [AdvancedRangeFilter], money_keys: &[&str]) {
    for range in ranges
        .iter_mut()
        .filter(|range| money_keys.contains(&range.key))
    {
        if !range.min.trim().is_empty() {
            range.min = format_internal_amount(&range.min);
        }
        if !range.max.trim().is_empty() {
            range.max = format_internal_amount(&range.max);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SavedPlayerList {
    format: String,
    version: u32,
    name: String,
    #[serde(default)]
    player_ids: Vec<usize>,
    #[serde(default)]
    staff_ids: Vec<usize>,
}

impl SavedPlayerList {
    fn new(name: String) -> Self {
        Self {
            format: "tfm2-editor-list".to_string(),
            version: 2,
            name,
            player_ids: Vec::new(),
            staff_ids: Vec::new(),
        }
    }

    fn is_supported(&self) -> bool {
        (self.format == "tfm2-editor-player-list" && self.version == 1)
            || (self.format == "tfm2-editor-list" && self.version == 2)
    }

    fn normalize(&mut self) {
        self.format = "tfm2-editor-list".to_string();
        self.version = 2;
        self.player_ids.sort_unstable();
        self.player_ids.dedup();
        self.staff_ids.sort_unstable();
        self.staff_ids.dedup();
    }

    fn total_members(&self) -> usize {
        self.player_ids.len() + self.staff_ids.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ListContentTab {
    Players,
    Staff,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ListNamePopupMode {
    Create,
    Rename,
}

#[derive(Debug, Clone)]
struct PlayerSummary {
    id: usize,
    name: String,
    age: String,
    team: String,
    region: String,
    position: String,
    actual_rating: String,
    _scout_potential_report: String,
    actual_potential: String,
    salary: String,
    transfer_fee: String,
    contract_end: String,
    last_hit: String,
    skill_avoid: String,
    skill_hit: String,
    control_speed: String,
    positioning: String,
    judgement: String,
    mental: String,
    concentration: String,
    order: String,
    roaming: String,
    aggressive: String,
    ego: String,
}

#[derive(Debug, Clone)]
struct StaffSummary {
    id: usize,
    name: String,
    age: String,
    team: String,
    role: String,
    banpick: String,
    strategy: String,
    negotiation: String,
    judge_ability: String,
    judge_potential: String,
    feedback: String,
    power_analysis: String,
    control_coaching: String,
    judgment_coaching: String,
    mental_coaching: String,
    annual_salary: String,
    contract_end: String,
    communication: String,
}

impl StaffSummary {
    fn localized_label(&self, localization: &Localization) -> String {
        format!(
            "{} · {} · {}",
            self.name,
            localized_staff_role(localization, &self.role),
            self.team
        )
    }

    fn matches_search(&self, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }

        self.name.to_lowercase().contains(query)
            || self.team.to_lowercase().contains(query)
            || self.role.to_lowercase().contains(query)
            || display_staff_role(&self.role).to_lowercase().contains(query)
            || self.age.to_lowercase().contains(query)
            || self.id.to_string().contains(query)
    }
}

#[derive(Debug, Default, Clone)]
struct StaffCommunicationEntry {
    region_id: usize,
    value: String,
}

#[derive(Debug, Default, Clone)]
struct StaffStats {
    id: usize,
    name: String,
    age: String,
    role: String,
    team: String,
    banpick: String,
    strategy: String,
    negotiation: String,
    judge_ability: String,
    judge_potential: String,
    feedback: String,
    power_analysis: String,
    control_coaching: String,
    judgment_coaching: String,
    mental_coaching: String,
    annual_salary: String,
    contract_team_id: Option<usize>,
    contract_start_date: String,
    contract_end_date: String,
    communication: Vec<StaffCommunicationEntry>,
}

impl StaffStats {
    fn set_all_max(&mut self) {
        let max = "100".to_string();
        self.banpick = max.clone();
        self.strategy = max.clone();
        self.negotiation = max.clone();
        self.judge_ability = max.clone();
        self.judge_potential = max.clone();
        self.feedback = max.clone();
        self.power_analysis = max.clone();
        self.control_coaching = max.clone();
        self.judgment_coaching = max.clone();
        self.mental_coaching = max;
    }
}

#[cfg(feature = "dev")]
#[derive(Debug, Clone)]
struct TeamConditionEntry {
    player_id: usize,
    player_name: String,
    stamina: String,
    condition: String,
    original_stamina: String,
    original_condition: String,
    write_status: String,
}

#[cfg(feature = "dev")]
impl TeamConditionEntry {
    fn has_changes(&self) -> bool {
        self.stamina.trim() != self.original_stamina.trim()
            || self.condition.trim() != self.original_condition.trim()
    }
}

#[cfg(feature = "dev")]
const TEAM_STRATEGY_KEYS: [&str; 12] = [
    "focused",
    "early_jungle",
    "early_serpen",
    "early_serpen_top",
    "object_buildup",
    "object_battle",
    "morgard_use",
    "tower_press",
    "morgard_defense",
    "object_finish",
    "minion_wave",
    "game_finish",
];

#[cfg(feature = "dev")]
#[derive(Debug, Clone)]
struct TeamMemberReference {
    id: usize,
    name: String,
}

#[cfg(feature = "dev")]
#[derive(Debug, Clone)]
struct TeamLineupEntry {
    slot: String,
    member: Option<TeamMemberReference>,
}

#[cfg(feature = "dev")]
#[derive(Debug, Clone)]
struct TeamStrategyEntry {
    key: String,
    value: String,
}

#[cfg(feature = "dev")]
#[derive(Debug, Clone)]
struct TeamMerchandiseEntry {
    product_type: String,
    athlete_id: usize,
    athlete_name: String,
    stock: String,
    sell_price: String,
    yearly_sales: String,
    yearly_revenue: String,
    total_sales: String,
    total_revenue: String,
    daily_purchase_rate: String,
}

#[cfg(feature = "dev")]
#[derive(Debug, Clone)]
struct TeamChampionSetupEntry {
    champion_id: String,
    tier: String,
    tactic_1: String,
    tactic_2: String,
    tactic_3: String,
}

#[cfg(feature = "dev")]
#[derive(Debug, Clone, Default)]
struct TeamGamingHouseSummary {
    level: String,
    welfare: String,
    owned_furniture_types: usize,
    owned_furniture_total: usize,
    owned_wallpaper_types: usize,
    owned_wallpaper_total: usize,
    owned_wall_types: usize,
    owned_wall_total: usize,
    owned_window_types: usize,
    owned_window_total: usize,
    placed_furniture: usize,
    placed_wallpapers: usize,
    placed_walls: usize,
    placed_windows: usize,
}

#[cfg(feature = "dev")]
#[derive(Debug, Clone)]
struct TeamManagementData {
    team_id: usize,
    lineup: Vec<TeamLineupEntry>,
    watched_players: Vec<TeamMemberReference>,
    no_transfer_players: Vec<TeamMemberReference>,
    release_players: Vec<TeamMemberReference>,
    watched_staff: Vec<TeamMemberReference>,
    release_staff: Vec<TeamMemberReference>,
    pending_installments: usize,
    resale_clauses: usize,
    scout_dispatch: String,
    merchandise_product_count: usize,
    champion_tier_count: usize,
    personal_tactic_count: usize,
    current_strategy: Vec<TeamStrategyEntry>,
    last_strategy: Vec<TeamStrategyEntry>,
    team_color_strategy: Vec<TeamStrategyEntry>,
    merchandise: Vec<TeamMerchandiseEntry>,
    champion_setup: Vec<TeamChampionSetupEntry>,
    gaming_house: TeamGamingHouseSummary,
}

#[cfg(feature = "dev")]
#[derive(Debug, Clone)]
struct TeamMatchSetEntry {
    set_number: usize,
    pattern: String,
    team1_kills: usize,
    team2_kills: usize,
    team1_gold: usize,
    team2_gold: usize,
    mvp_player_id: usize,
    mvp_player_name: String,
    mvp_champion_id: String,
    mvp_kills: usize,
    mvp_deaths: usize,
    mvp_assists: usize,
    was_comeback: bool,
    was_blue_side: bool,
}

#[cfg(feature = "dev")]
#[derive(Debug, Clone)]
struct TeamMatchHistoryEntry {
    date: String,
    match_id: usize,
    opponent_id: usize,
    opponent_name: String,
    is_practice: bool,
    is_win: bool,
    my_score: usize,
    enemy_score: usize,
    article_pattern: String,
    sets: Vec<TeamMatchSetEntry>,
}

#[cfg(feature = "dev")]
#[derive(Debug, Clone)]
struct TeamPreMatchTacticEntry {
    category: String,
    value: String,
}

#[cfg(feature = "dev")]
#[derive(Debug, Clone)]
struct TeamPreMatchChampionEntry {
    champion_id: String,
    position: String,
    wins: usize,
    losses: usize,
}

#[cfg(feature = "dev")]
#[derive(Debug, Clone)]
struct TeamPreMatchInsightEntry {
    section: String,
    label: String,
    details: String,
    source_key: String,
}

#[cfg(feature = "dev")]
#[derive(Debug, Clone)]
struct TeamPreMatchAnalysisEntry {
    date: String,
    match_id: usize,
    opponent_id: usize,
    opponent_name: String,
    analysis_level: String,
    has_match_history: bool,
    star_player_id: usize,
    star_player_name: String,
    tactics: Vec<TeamPreMatchTacticEntry>,
    champion_picks: Vec<TeamPreMatchChampionEntry>,
    insights: Vec<TeamPreMatchInsightEntry>,
}

#[cfg(feature = "dev")]
#[derive(Debug, Clone)]
struct TeamHistoryData {
    team_id: usize,
    matches: Vec<TeamMatchHistoryEntry>,
    analyses: Vec<TeamPreMatchAnalysisEntry>,
    latest_rating: Option<i64>,
    latest_rank: Option<usize>,
    latest_rating_date: String,
}

#[cfg(feature = "dev")]
impl TeamHistoryData {
    fn wins(&self) -> usize {
        self.matches.iter().filter(|entry| entry.is_win).count()
    }

    fn losses(&self) -> usize {
        self.matches.len().saturating_sub(self.wins())
    }

    fn set_wins(&self) -> usize {
        self.matches.iter().map(|entry| entry.my_score).sum()
    }

    fn set_losses(&self) -> usize {
        self.matches.iter().map(|entry| entry.enemy_score).sum()
    }

    fn official_matches(&self) -> usize {
        self.matches.iter().filter(|entry| !entry.is_practice).count()
    }

    fn practice_matches(&self) -> usize {
        self.matches.iter().filter(|entry| entry.is_practice).count()
    }

    fn recent_form(&self) -> String {
        let form = self
            .matches
            .iter()
            .rev()
            .take(5)
            .map(|entry| if entry.is_win { "W" } else { "L" })
            .collect::<Vec<_>>();
        if form.is_empty() {
            "—".to_string()
        } else {
            form.join("-")
        }
    }
}

#[derive(Debug, Clone)]
struct TeamSummary {
    id: usize,
    display_name: String,
    manager_name: String,
    league_id: usize,
    is_player_team: bool,
    roster_size: usize,
    staff_count: usize,
    roster_rating: Option<f64>,
    merchandise_facility_grade: String,
    stadium_grade: String,
    training_facility_grade: String,
    // These expanded Team fields remain in the shared response model so Community and
    // Development can use the same Bridge payload. They are rendered only in Development.
    #[cfg_attr(not(feature = "dev"), allow(dead_code))]
    stadium_name: String,
    #[cfg_attr(not(feature = "dev"), allow(dead_code))]
    stadium_capacity: String,
    #[cfg_attr(not(feature = "dev"), allow(dead_code))]
    total_home_attendance: String,
    #[cfg_attr(not(feature = "dev"), allow(dead_code))]
    home_match_count: String,
    #[cfg_attr(not(feature = "dev"), allow(dead_code))]
    total_entrance_income: f64,
    #[cfg_attr(not(feature = "dev"), allow(dead_code))]
    popularity: String,
    #[cfg_attr(not(feature = "dev"), allow(dead_code))]
    fan_expectation: String,
    #[cfg_attr(not(feature = "dev"), allow(dead_code))]
    fan_satisfaction: String,
    #[cfg_attr(not(feature = "dev"), allow(dead_code))]
    fan_count: String,
    #[cfg_attr(not(feature = "dev"), allow(dead_code))]
    fan_momentum: String,
    #[cfg_attr(not(feature = "dev"), allow(dead_code))]
    gaming_house_level: String,
    #[cfg_attr(not(feature = "dev"), allow(dead_code))]
    welfare: String,
    total_balance: f64,
    transfer_budget: f64,
    salary_budget: f64,
}

impl TeamSummary {
    #[cfg_attr(not(feature = "dev"), allow(dead_code))]
    fn average_home_attendance(&self) -> Option<f64> {
        let attendance = self.total_home_attendance.trim().parse::<f64>().ok()?;
        let matches = self.home_match_count.trim().parse::<f64>().ok()?;
        if matches > 0.0 {
            Some(attendance / matches)
        } else {
            None
        }
    }

    fn localized_label(&self, localization: &Localization) -> String {
        let name = if self.display_name.trim().is_empty() {
            self.localization_fallback_name(localization)
        } else {
            self.display_name.clone()
        };
        let league_id = self.league_id.to_string();
        let league = localization.tr_with(
            "common.league_number",
            &[("id", league_id.as_str())],
        );

        if self.is_player_team {
            format!("{name} · {} · {league}", localization.tr("common.my_team"))
        } else {
            format!("{name} · {league}")
        }
    }

    fn localization_fallback_name(&self, localization: &Localization) -> String {
        let team_id = self.id.to_string();
        localization.tr_with(
            "common.team_number",
            &[("id", team_id.as_str())],
        )
    }

    fn matches_search(&self, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }

        self.display_name.to_lowercase().contains(query)
            || self.manager_name.to_lowercase().contains(query)
            || self.league_id.to_string().contains(query)
            || self.id.to_string().contains(query)
            || (self.is_player_team && "my team".contains(query))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum SquadStatusChoice {
    Core,
    Important,
    #[default]
    General,
    Sub,
    Prospect,
}


impl SquadStatusChoice {
    const ALL: [Self; 5] = [
        Self::Core,
        Self::Important,
        Self::General,
        Self::Sub,
        Self::Prospect,
    ];

    fn internal(self) -> &'static str {
        match self {
            Self::Core => "Core",
            Self::Important => "Important",
            Self::General => "General",
            Self::Sub => "Sub",
            Self::Prospect => "Prospect",
        }
    }


    fn label_key(self) -> &'static str {
        match self {
            Self::Core => "contract.squad.core",
            Self::Important => "contract.squad.important",
            Self::General => "contract.squad.starter",
            Self::Sub => "contract.squad.substitute",
            Self::Prospect => "contract.squad.prospect",
        }
    }

    fn from_internal(value: &str) -> Self {
        match value.trim() {
            "Core" => Self::Core,
            "Important" => Self::Important,
            "Sub" => Self::Sub,
            "Prospect" => Self::Prospect,
            _ => Self::General,
        }
    }
}

#[derive(Debug, Default, Clone)]
struct ContractEditorForm {
    team_id: Option<usize>,
    start_date: String,
    end_date: String,
    annual_salary: String,
    transfer_fee: String,
    squad_status: SquadStatusChoice,
    pog_enabled: bool,
    pog_bonus: String,
    league_enabled: bool,
    league_bonus: String,
    league_rank: String,
    match_enabled: bool,
    match_bonus: String,
    win_enabled: bool,
    win_bonus: String,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum ContractEditorMode {
    #[default]
    EditActive,
    MoveFreeAgent,
}

#[derive(Debug, Default, Clone)]
struct PlayerStats {
    id: usize,
    name: String,
    last_hit: String,
    skill_avoid: String,
    skill_hit: String,
    control_speed: String,
    positioning: String,
    judgement: String,
    mental: String,
    concentration: String,
    order: String,
    roaming: String,
    aggressive: String,
    ego: String,
    top: String,
    jungle: String,
    mid: String,
    bottom: String,
    support: String,
    potential: String,
    annual_salary: String,
    weekly_salary: String,
    contract_team_id: Option<usize>,
    contract_start_date: String,
    contract_end_date: String,
    transfer_fee: String,
    squad_status: String,
    incentive_pog_bonus: String,
    incentive_league_bonus: String,
    incentive_league_rank: String,
    incentive_match_bonus: String,
    incentive_win_bonus: String,
    primary_region: String,
    communication_raw: String,
    communication_xp_raw: String,
}

impl PlayerStats {
    fn set_all_max(&mut self) {
        let max = "100".to_string();
        self.last_hit = max.clone();
        self.skill_avoid = max.clone();
        self.skill_hit = max.clone();
        self.control_speed = max.clone();
        self.positioning = max.clone();
        self.judgement = max.clone();
        self.mental = max.clone();
        self.concentration = max.clone();
        self.order = max.clone();
        self.roaming = max.clone();
        self.aggressive = max.clone();
        self.ego = max;
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PositionChoice {
    Top,
    Jungle,
    Mid,
    Bottom,
    Support,
}

impl PositionChoice {
    const ALL: [Self; 5] = [
        Self::Top,
        Self::Jungle,
        Self::Mid,
        Self::Bottom,
        Self::Support,
    ];


    fn label_key(self) -> &'static str {
        match self {
            Self::Top => "positions.top",
            Self::Jungle => "positions.jungle",
            Self::Mid => "positions.mid",
            Self::Bottom => "positions.bottom",
            Self::Support => "positions.support",
        }
    }

    fn code(self) -> usize {
        match self {
            Self::Top => 0,
            Self::Jungle => 1,
            Self::Mid => 2,
            Self::Bottom => 3,
            Self::Support => 4,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct PlayerPositionSlot {
    position: Option<PositionChoice>,
    proficiency: u16,
}

#[derive(Debug, Clone)]
struct PlayerPositionForm {
    slots: [PlayerPositionSlot; 3],
}

impl PlayerPositionForm {
    fn from_player(player: &PlayerStats) -> Self {
        let raw_values = [
            parse_raw_stat(&player.top),
            parse_raw_stat(&player.jungle),
            parse_raw_stat(&player.mid),
            parse_raw_stat(&player.bottom),
            parse_raw_stat(&player.support),
        ];

        let mut slots = [PlayerPositionSlot::default(); 3];
        let mut slot_index = 0;
        for position in PositionChoice::ALL {
            let proficiency = raw_values[position.code()];
            if proficiency == 0 || slot_index >= slots.len() {
                continue;
            }

            slots[slot_index] = PlayerPositionSlot {
                position: Some(position),
                proficiency,
            };
            slot_index += 1;
        }

        Self { slots }
    }

    fn clear_all(&mut self) {
        self.slots = [PlayerPositionSlot::default(); 3];
    }

    fn values_for_apply(&self) -> [u16; 5] {
        let mut values = [0; 5];
        for slot in self.slots {
            if let Some(position) = slot.position {
                values[position.code()] = slot.proficiency;
            }
        }
        values
    }
}

#[derive(Debug, Clone)]
struct PlayerCommunicationForm {
    primary_region: Option<usize>,
    entries: Vec<(usize, i32)>,
    xp_entries: Vec<(usize, i32)>,
}

impl PlayerCommunicationForm {
    fn from_player(player: &PlayerStats) -> Self {
        Self {
            primary_region: player.primary_region.trim().parse::<usize>().ok(),
            entries: parse_communication_entries(&player.communication_raw),
            xp_entries: parse_communication_entries(&player.communication_xp_raw),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PotentialGrade {
    VeryLow,
    Low,
    Normal,
    High,
    VeryHigh,
}

impl PotentialGrade {
    const ALL: [Self; 5] = [
        Self::VeryLow,
        Self::Low,
        Self::Normal,
        Self::High,
        Self::VeryHigh,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::VeryLow => "Very Low",
            Self::Low => "Low",
            Self::Normal => "Normal",
            Self::High => "High",
            Self::VeryHigh => "Very High",
        }
    }

    fn raw_value(self) -> u16 {
        match self {
            Self::VeryLow => 1,
            Self::Low => 30,
            Self::Normal => 50,
            Self::High => 70,
            Self::VeryHigh => 100,
        }
    }

    fn from_raw(raw: u16) -> Self {
        match raw {
            0..=20 => Self::VeryLow,
            21..=40 => Self::Low,
            41..=60 => Self::Normal,
            61..=80 => Self::High,
            _ => Self::VeryHigh,
        }
    }
}

#[derive(Debug, Clone)]
struct ChampionMasteryEntry {
    id: String,
    display_name: String,
    active: bool,
    selected: bool,
    raw_value: i32,
    #[cfg(feature = "dev")]
    raw_floor: Option<i32>,
    edit_mastery: u16,
}

impl ChampionMasteryEntry {
    fn mastery(&self) -> f32 {
        self.raw_value as f32 / 10.0
    }

    fn mastery_text(&self) -> String {
        let value = self.mastery();
        if (value.fract()).abs() < f32::EPSILON {
            format!("{value:.0}")
        } else {
            format!("{value:.1}")
        }
    }

}

#[derive(Debug, Clone)]
struct PlayerPotentialForm {
    potential: PotentialGrade,
    current_raw: u16,
    edit_raw: u16,
}

impl PlayerPotentialForm {
    fn from_player(player: &PlayerStats) -> Self {
        let potential_raw = player
            .potential
            .trim()
            .parse::<u16>()
            .unwrap_or(50)
            .clamp(1, 100);

        Self {
            potential: PotentialGrade::from_raw(potential_raw),
            current_raw: potential_raw,
            edit_raw: potential_raw,
        }
    }

    fn set_grade(&mut self, grade: PotentialGrade) {
        self.potential = grade;
        self.edit_raw = grade.raw_value();
    }

    fn set_raw(&mut self, raw: u16) {
        self.edit_raw = raw.clamp(1, 100);
        self.potential = PotentialGrade::from_raw(self.edit_raw);
    }
}

fn parse_raw_stat(value: &str) -> u16 {
    value.trim().parse::<u16>().unwrap_or(0)
}

const REGION_FILTER_NAMES: [&str; 6] = [
    "Korea",
    "China",
    "Europe",
    "North America",
    "South America",
    "Japan",
];

const COMMUNICATION_REGIONS: [(usize, &str); 6] = [
    (0, "Korea League"),
    (1, "China League"),
    (2, "Europe League"),
    (3, "North America League"),
    (4, "South America League"),
    (5, "Japan League"),
];

fn communication_region_label_key(region_id: usize) -> Option<&'static str> {
    match region_id {
        0 => Some("regions.korea_league"),
        1 => Some("regions.china_league"),
        2 => Some("regions.europe_league"),
        3 => Some("regions.north_america_league"),
        4 => Some("regions.south_america_league"),
        5 => Some("regions.japan_league"),
        _ => None,
    }
}

fn localized_communication_region_label(
    localization: &Localization,
    region_id: usize,
) -> String {
    let name = communication_region_label_key(region_id)
        .map(|key| localization.tr(key))
        .unwrap_or_else(|| format!("Region {region_id}"));

    #[cfg(feature = "dev")]
    {
        format!("{name} (Region {region_id})")
    }
    #[cfg(not(feature = "dev"))]
    {
        name
    }
}

fn staff_communication_region_label(region_id: usize) -> String {
    COMMUNICATION_REGIONS
        .iter()
        .find(|(id, _)| *id == region_id)
        .map(|(_, name)| {
            #[cfg(feature = "dev")]
            {
                format!("{name} (Region {region_id})")
            }
            #[cfg(not(feature = "dev"))]
            {
                (*name).to_string()
            }
        })
        .unwrap_or_else(|| format!("Region {region_id}"))
}

fn staff_communication_value_for_region(staff: &StaffStats, region_id: usize) -> String {
    staff
        .communication
        .iter()
        .find(|entry| entry.region_id == region_id)
        .map(|entry| entry.value.clone())
        .unwrap_or_else(|| "0".to_string())
}

fn player_communication_region_label(region_id: usize) -> String {
    COMMUNICATION_REGIONS
        .iter()
        .find(|(id, _)| *id == region_id)
        .map(|(_, name)| {
            #[cfg(feature = "dev")]
            {
                format!("{name} (Region {region_id})")
            }
            #[cfg(not(feature = "dev"))]
            {
                (*name).to_string()
            }
        })
        .unwrap_or_else(|| format!("Region {region_id}"))
}

fn player_communication_value_for_region(
    communication: &PlayerCommunicationForm,
    region_id: usize,
) -> String {
    communication
        .entries
        .iter()
        .find(|(stored_region_id, _)| *stored_region_id == region_id)
        .map(|(_, value)| value.to_string())
        .unwrap_or_else(|| "0".to_string())
}

fn first_editable_player_communication_region(primary_region: Option<usize>) -> usize {
    COMMUNICATION_REGIONS
        .iter()
        .map(|(region_id, _)| *region_id)
        .find(|region_id| Some(*region_id) != primary_region)
        .unwrap_or(0)
}

const POSITION_FILTER_NAMES: [&str; 5] = ["Top", "Jungle", "Mid", "Bottom", "Support"];

fn position_label_key(raw: &str) -> Option<&'static str> {
    match raw.trim() {
        "Top" => Some("positions.top"),
        "Jungle" => Some("positions.jungle"),
        "Mid" => Some("positions.mid"),
        "Bottom" => Some("positions.bottom"),
        "Support" => Some("positions.support"),
        _ => None,
    }
}

fn localized_position_name(localization: &Localization, raw: &str) -> String {
    position_label_key(raw)
        .map(|key| localization.tr(key))
        .unwrap_or_else(|| raw.to_string())
}

fn localized_position_summary(localization: &Localization, raw: &str) -> String {
    raw.split('/')
        .map(|value| localized_position_name(localization, value.trim()))
        .collect::<Vec<_>>()
        .join(" / ")
}

fn region_label_key(raw: &str) -> Option<&'static str> {
    match raw.trim() {
        "Korea" => Some("regions.korea"),
        "China" => Some("regions.china"),
        "Europe" => Some("regions.europe"),
        "North America" => Some("regions.north_america"),
        "South America" => Some("regions.south_america"),
        "Japan" => Some("regions.japan"),
        _ => None,
    }
}

fn localized_region_name(localization: &Localization, raw: &str) -> String {
    region_label_key(raw)
        .map(|key| localization.tr(key))
        .unwrap_or_else(|| raw.to_string())
}

fn selected_multi_filter_label_localized<const N: usize>(
    localization: &Localization,
    empty_key: &str,
    labels: &[&str; N],
    selected: &[bool; N],
    localize: fn(&Localization, &str) -> String,
) -> String {
    let active = labels
        .iter()
        .zip(selected.iter())
        .filter_map(|(label, is_selected)| is_selected.then_some(*label))
        .collect::<Vec<_>>();

    match active.as_slice() {
        [] => localization.tr(empty_key),
        [only] => localize(localization, only),
        _ => {
            let count = active.len().to_string();
            localization.tr_with("search.selected_count", &[("count", count.as_str())])
        }
    }
}


fn average_attribute_rating(player: &PlayerSummary) -> Option<f32> {
    let values = [
        &player.last_hit,
        &player.skill_avoid,
        &player.skill_hit,
        &player.control_speed,
        &player.positioning,
        &player.judgement,
        &player.mental,
        &player.concentration,
        &player.order,
        &player.roaming,
        &player.aggressive,
        &player.ego,
    ];

    let parsed = values
        .iter()
        .filter_map(|value| value.trim().parse::<f32>().ok())
        .collect::<Vec<_>>();

    if parsed.len() != 12 {
        return None;
    }

    Some(parsed.iter().sum::<f32>() / parsed.len() as f32)
}

fn effective_actual_rating(player: &PlayerSummary) -> Option<(f32, bool)> {
    if let Ok(value) = player.actual_rating.trim().parse::<f32>() {
        if value.is_finite() {
            return Some((value, false));
        }
    }

    average_attribute_rating(player).map(|value| (value, true))
}

fn render_actual_rating(ui: &mut egui::Ui, player: &PlayerSummary, localization: &Localization) {
    match effective_actual_rating(player) {
        Some((value, false)) => {
            #[cfg(feature = "dev")]
            {
                ui.label(format!("{value:.1}"))
                    .on_hover_text(localization.tr("rating.actual.dev_tooltip"));
            }
            #[cfg(not(feature = "dev"))]
            {
                ui.label(format!("{value:.1}"));
            }
        }
        Some((value, true)) => {
            let response = ui.label(format!("≈{value:.1}"));
            #[cfg(feature = "dev")]
            response.on_hover_text(localization.tr("rating.actual.fallback_dev_tooltip"));
            #[cfg(not(feature = "dev"))]
            response.on_hover_text(localization.tr("rating.actual.fallback_tooltip"));
        }
        None => {
            ui.label("—");
        }
    }
}

fn actual_potential_rating_value(raw: &str) -> Option<f32> {
    let value = raw.trim().replace(',', ".").parse::<f32>().ok()?;
    if !value.is_finite() || value < 0.0 {
        return None;
    }

    let stars = value.clamp(0.0, 100.0) / 20.0;
    Some(((stars * 2.0).round()) / 2.0)
}

fn effective_potential_rating(player: &PlayerSummary) -> Option<f32> {
    actual_potential_rating_value(&player.actual_potential)
}

fn potential_rating_stars(ui: &mut egui::Ui, actual_potential_raw: &str) {
    let Some(rating) = actual_potential_rating_value(actual_potential_raw) else {
        ui.label("—");
        return;
    };

    let source_text = format!(
        "{rating:.1}/5 · Based on hidden Actual Potential ({})",
        actual_potential_raw.trim()
    );

    let star_size = 14.0;
    let spacing = 1.0;
    let width = (star_size + spacing) * 5.0 - spacing;
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, 18.0), egui::Sense::hover());
    let painter = ui.painter();
    let empty_color = ui
        .visuals()
        .widgets
        .noninteractive
        .fg_stroke
        .color
        .linear_multiply(0.35);
    let filled_color = ui.visuals().selection.stroke.color;
    let font = egui::FontId::proportional(star_size);

    for index in 0..5 {
        let x = rect.left() + index as f32 * (star_size + spacing);
        let pos = egui::pos2(x, rect.center().y);

        painter.text(
            pos,
            egui::Align2::LEFT_CENTER,
            "★",
            font.clone(),
            empty_color,
        );

        let fill = (rating - index as f32).clamp(0.0, 1.0);
        if fill > 0.0 {
            let clip = egui::Rect::from_min_max(
                egui::pos2(x, rect.top()),
                egui::pos2(x + star_size * fill, rect.bottom()),
            );
            painter.with_clip_rect(clip).text(
                pos,
                egui::Align2::LEFT_CENTER,
                "★",
                font.clone(),
                filled_color,
            );
        }
    }

    response.on_hover_text(source_text);
}

fn display_staff_role(raw: &str) -> String {
    match raw.trim() {
        "HeadCoach" => "Head Coach".to_string(),
        "TrainingCoach" => "Training Coach".to_string(),
        "Scouter" => "Scouter".to_string(),
        "Analyst" => "Analyst".to_string(),
        "" => "Unknown".to_string(),
        other => other.to_string(),
    }
}

#[cfg(feature = "dev")]
fn summary_belongs_to_team(member_team: &str, team: &TeamSummary) -> bool {
    let member_team = member_team.trim();
    if member_team.is_empty() || member_team.eq_ignore_ascii_case("Free Agent") {
        return false;
    }

    let team_name = team.display_name.trim();
    (!team_name.is_empty() && member_team.eq_ignore_ascii_case(team_name))
        || member_team.eq_ignore_ascii_case(&format!("Team {}", team.id))
}

#[cfg(feature = "dev")]
fn validate_condition_editor_value(value: &str, label: &str) -> Result<(), String> {
    let parsed = value
        .trim()
        .parse::<f64>()
        .map_err(|_| format!("{label} must be a number from 0 to 100"))?;
    if !parsed.is_finite() || !(0.0..=100.0).contains(&parsed) {
        return Err(format!("{label} must be a number from 0 to 100"));
    }
    Ok(())
}

#[cfg(feature = "dev")]
fn parse_team_condition_from_player_probe(raw: &str) -> Result<(String, String), String> {
    let marker = "management: AthleteManagementStat";
    let start = raw
        .find(marker)
        .ok_or_else(|| "AthleteManagementStat block not found".to_string())?;

    let mut stamina = None;
    let mut condition = None;
    let mut depth = 0_i32;
    let mut block_started = false;

    for line in raw[start..].lines() {
        let trimmed = line.trim();

        if let Some(value) = trimmed.strip_prefix("stamina:") {
            stamina = Some(value.trim().trim_end_matches(',').to_string());
        } else if let Some(value) = trimmed.strip_prefix("condition:") {
            condition = Some(value.trim().trim_end_matches(',').to_string());
        }

        for character in line.chars() {
            match character {
                '{' => {
                    depth += 1;
                    block_started = true;
                }
                '}' if block_started => depth -= 1,
                _ => {}
            }
        }

        if block_started && depth <= 0 {
            break;
        }
    }

    match (stamina, condition) {
        (Some(stamina), Some(condition)) => Ok((stamina, condition)),
        _ => Err("Stamina or condition field not found".to_string()),
    }
}

struct ModifierApp {
    active_tab: AppTab,
    localization: Localization,
    search_tab: SearchTab,
    search_preview_filter: String,
    search_age_min: String,
    search_age_max: String,
    search_actual_potential_min: String,
    search_actual_potential_max: String,
    search_team_filter: String,
    search_region_filters: [bool; 6],
    search_position_filters: [bool; 5],
    search_free_agents_only: bool,
    player_sort_column: PlayerSortColumn,
    player_sort_ascending: bool,
    advanced_search_open: bool,
    advanced_player_search: AdvancedPlayerSearch,
    saved_filters: Vec<String>,
    selected_saved_filter: Option<String>,
    saved_filters_width: f32,
    filter_name_popup_open: bool,
    filter_name_draft: String,
    saved_player_lists: Vec<SavedPlayerList>,
    selected_saved_player_list: Option<String>,
    active_player_list_filter: Option<String>,
    active_staff_list_filter: Option<String>,
    selected_search_player_ids: BTreeSet<usize>,
    selected_search_staff_ids: BTreeSet<usize>,
    selected_search_team_ids: BTreeSet<usize>,
    selected_list_player_ids: BTreeSet<usize>,
    selected_list_staff_ids: BTreeSet<usize>,
    pending_new_list_player_ids: Vec<usize>,
    pending_new_list_staff_ids: Vec<usize>,
    player_selection_anchor_id: Option<usize>,
    staff_selection_anchor_id: Option<usize>,
    team_selection_anchor_id: Option<usize>,
    player_shift_drag_start_id: Option<usize>,
    staff_shift_drag_start_id: Option<usize>,
    team_shift_drag_start_id: Option<usize>,
    player_shift_drag_target_selected: Option<bool>,
    staff_shift_drag_target_selected: Option<bool>,
    team_shift_drag_target_selected: Option<bool>,
    player_shift_drag_base_ids: Option<BTreeSet<usize>>,
    staff_shift_drag_base_ids: Option<BTreeSet<usize>>,
    team_shift_drag_base_ids: Option<BTreeSet<usize>>,
    list_content_tab: ListContentTab,
    list_name_popup_open: bool,
    list_delete_confirmation_open: bool,
    list_name_popup_mode: ListNamePopupMode,
    list_name_draft: String,
    staff_database_search: String,
    staff_search_age_min: String,
    staff_search_age_max: String,
    staff_search_team_filter: String,
    staff_search_role_filter: String,
    staff_search_free_agents_only: bool,
    staff_sort_column: StaffSortColumn,
    staff_sort_ascending: bool,
    advanced_staff_search_open: bool,
    advanced_staff_search: AdvancedStaffSearch,
    saved_staff_filters: Vec<String>,
    selected_saved_staff_filter: Option<String>,
    saved_staff_filters_width: f32,
    staff_filter_name_popup_open: bool,
    staff_filter_name_draft: String,
    economy: EconomyForm,
    players: Vec<PlayerSummary>,
    staffs: Vec<StaffSummary>,
    staff_search: String,
    selected_staff_id: Option<usize>,
    selected_staff: Option<StaffStats>,
    staff_communication_region_id: usize,
    staff_communication_value: String,
    staff_contract_window_open: bool,
    staff_contract_form: ContractEditorForm,
    staff_contract_mode: ContractEditorMode,
    #[cfg(feature = "dev")]
    staff_contract_probe_open: bool,
    #[cfg(feature = "dev")]
    staff_contract_probe_raw: String,
    #[cfg(feature = "dev")]
    staff_contract_probe_before: String,
    #[cfg(feature = "dev")]
    staff_contract_probe_after_offer: String,
    #[cfg(feature = "dev")]
    staff_contract_probe_after_accepted: String,
    #[cfg(feature = "dev")]
    staff_contract_probe_comparison: String,
    player_search: String,
    selected_player_id: Option<usize>,
    selected_player: Option<PlayerStats>,
    player_positions: Option<PlayerPositionForm>,
    player_potential: Option<PlayerPotentialForm>,
    player_communication: Option<PlayerCommunicationForm>,
    player_communication_region_id: usize,
    player_communication_value: String,
    player_contract_window_open: bool,
    player_contract_form: ContractEditorForm,
    player_contract_mode: ContractEditorMode,
    #[cfg(feature = "dev")]
    player_contract_probe_open: bool,
    #[cfg(feature = "dev")]
    player_contract_probe_raw: String,
    #[cfg(feature = "dev")]
    player_contract_probe_before: String,
    #[cfg(feature = "dev")]
    player_contract_probe_after_offer: String,
    #[cfg(feature = "dev")]
    player_contract_probe_after_accepted: String,
    #[cfg(feature = "dev")]
    player_contract_probe_comparison: String,
    champion_mastery_open: bool,
    champion_mastery_entries: Vec<ChampionMasteryEntry>,
    champion_mastery_bulk_value: u16,
    transfer_always_success: bool,
    recruitment_instant_retry: bool,
    teams: Vec<TeamSummary>,
    team_database_search: String,
    team_search_league_filter: Option<usize>,
    team_search_player_team_only: bool,
    team_search_roster_min: String,
    team_search_roster_max: String,
    team_search_staff_min: String,
    team_search_staff_max: String,
    team_sort_column: TeamSortColumn,
    team_sort_ascending: bool,
    #[cfg(feature = "dev")]
    team_workspace_search: String,
    #[cfg(feature = "dev")]
    team_workspace_team_id: Option<usize>,
    #[cfg(feature = "dev")]
    team_roster_window_open: bool,
    #[cfg(feature = "dev")]
    team_staff_window_open: bool,
    #[cfg(feature = "dev")]
    team_roster_selected_player_id: Option<usize>,
    #[cfg(feature = "dev")]
    team_staff_selected_staff_id: Option<usize>,
    #[cfg(feature = "dev")]
    team_condition_window_open: bool,
    #[cfg(feature = "dev")]
    team_condition_entries: Vec<TeamConditionEntry>,
    #[cfg(feature = "dev")]
    team_condition_team_id: Option<usize>,
    #[cfg(feature = "dev")]
    team_condition_selected_player_ids: BTreeSet<usize>,
    #[cfg(feature = "dev")]
    team_condition_bulk_stamina: String,
    #[cfg(feature = "dev")]
    team_condition_bulk_condition: String,
    #[cfg(feature = "dev")]
    team_data_probe_window_open: bool,
    #[cfg(feature = "dev")]
    team_data_probe_team_id: Option<usize>,
    #[cfg(feature = "dev")]
    team_data_probe_raw: String,
    #[cfg(feature = "dev")]
    team_management_data: Option<TeamManagementData>,
    #[cfg(feature = "dev")]
    team_management_last_request_team_id: Option<usize>,
    #[cfg(feature = "dev")]
    team_strategy_window_open: bool,
    #[cfg(feature = "dev")]
    team_merchandise_window_open: bool,
    #[cfg(feature = "dev")]
    team_champion_setup_window_open: bool,
    #[cfg(feature = "dev")]
    team_gaming_house_window_open: bool,
    #[cfg(feature = "dev")]
    team_history_data: Option<TeamHistoryData>,
    #[cfg(feature = "dev")]
    team_history_last_request_team_id: Option<usize>,
    #[cfg(feature = "dev")]
    team_match_history_window_open: bool,
    #[cfg(feature = "dev")]
    team_pre_match_analysis_window_open: bool,
    #[cfg(feature = "dev")]
    team_history_summary_window_open: bool,
    recruitment_management_tab: RecruitmentManagementTab,
    recruitment_player_search: String,
    recruitment_player_id: Option<usize>,
    recruitment_team_search: String,
    recruitment_team_id: Option<usize>,
    recruitment_staff_search: String,
    recruitment_staff_id: Option<usize>,
    free_agent_confirmation_player_id: Option<usize>,
    free_agent_confirmation_staff_id: Option<usize>,
    connected: bool,
    status: String,
    player_search_status: Option<String>,
    staff_search_status: Option<String>,
    team_search_status: Option<String>,
    bridge_version: String,
    bridge_protocol: Option<u32>,
    bridge_tfm2_target: Option<String>,
    compatibility_issue: Option<CompatibilityIssue>,
    compatibility_popup_open: bool,
    compatibility_ignored_for_session: bool,
}

impl Default for ModifierApp {
    fn default() -> Self {
        let localization = Localization::load();
        let starting_status = localization.tr("status.starting");
        let mut app = Self {
            active_tab: AppTab::Economy,
            localization,
            search_tab: SearchTab::Players,
            search_preview_filter: String::new(),
            search_age_min: String::new(),
            search_age_max: String::new(),
            search_actual_potential_min: String::new(),
            search_actual_potential_max: String::new(),
            search_team_filter: "Any Team".to_string(),
            search_region_filters: [false; 6],
            search_position_filters: [false; 5],
            search_free_agents_only: false,
            player_sort_column: PlayerSortColumn::Name,
            player_sort_ascending: true,
            advanced_search_open: false,
            advanced_player_search: AdvancedPlayerSearch::default(),
            saved_filters: Vec::new(),
            selected_saved_filter: None,
            saved_filters_width: 175.0,
            filter_name_popup_open: false,
            filter_name_draft: String::new(),
            saved_player_lists: Vec::new(),
            selected_saved_player_list: None,
            active_player_list_filter: None,
            active_staff_list_filter: None,
            selected_search_player_ids: BTreeSet::new(),
            selected_search_staff_ids: BTreeSet::new(),
            selected_search_team_ids: BTreeSet::new(),
            selected_list_player_ids: BTreeSet::new(),
            selected_list_staff_ids: BTreeSet::new(),
            pending_new_list_player_ids: Vec::new(),
            pending_new_list_staff_ids: Vec::new(),
            player_selection_anchor_id: None,
            staff_selection_anchor_id: None,
            team_selection_anchor_id: None,
            player_shift_drag_start_id: None,
            staff_shift_drag_start_id: None,
            team_shift_drag_start_id: None,
            player_shift_drag_target_selected: None,
            staff_shift_drag_target_selected: None,
            team_shift_drag_target_selected: None,
            player_shift_drag_base_ids: None,
            staff_shift_drag_base_ids: None,
            team_shift_drag_base_ids: None,
            list_content_tab: ListContentTab::Players,
            list_name_popup_open: false,
            list_delete_confirmation_open: false,
            list_name_popup_mode: ListNamePopupMode::Create,
            list_name_draft: String::new(),
            staff_database_search: String::new(),
            staff_search_age_min: String::new(),
            staff_search_age_max: String::new(),
            staff_search_team_filter: "Any Team".to_string(),
            staff_search_role_filter: "Any Role".to_string(),
            staff_search_free_agents_only: false,
            staff_sort_column: StaffSortColumn::Name,
            staff_sort_ascending: true,
            advanced_staff_search_open: false,
            advanced_staff_search: AdvancedStaffSearch::default(),
            saved_staff_filters: Vec::new(),
            selected_saved_staff_filter: None,
            saved_staff_filters_width: 175.0,
            staff_filter_name_popup_open: false,
            staff_filter_name_draft: String::new(),
            economy: EconomyForm::default(),
            players: Vec::new(),
            staffs: Vec::new(),
            staff_search: String::new(),
            selected_staff_id: None,
            selected_staff: None,
            staff_communication_region_id: 0,
            staff_communication_value: "0".to_string(),
            staff_contract_window_open: false,
            staff_contract_form: ContractEditorForm::default(),
            staff_contract_mode: ContractEditorMode::EditActive,
            #[cfg(feature = "dev")]
            staff_contract_probe_open: false,
            #[cfg(feature = "dev")]
            staff_contract_probe_raw: String::new(),
            #[cfg(feature = "dev")]
            staff_contract_probe_before: String::new(),
            #[cfg(feature = "dev")]
            staff_contract_probe_after_offer: String::new(),
            #[cfg(feature = "dev")]
            staff_contract_probe_after_accepted: String::new(),
            #[cfg(feature = "dev")]
            staff_contract_probe_comparison: String::new(),
            player_search: String::new(),
            selected_player_id: None,
            selected_player: None,
            player_positions: None,
            player_potential: None,
            player_communication: None,
            player_communication_region_id: 1,
            player_communication_value: "0".to_string(),
            player_contract_window_open: false,
            player_contract_form: ContractEditorForm::default(),
            player_contract_mode: ContractEditorMode::EditActive,
            #[cfg(feature = "dev")]
            player_contract_probe_open: false,
            #[cfg(feature = "dev")]
            player_contract_probe_raw: String::new(),
            #[cfg(feature = "dev")]
            player_contract_probe_before: String::new(),
            #[cfg(feature = "dev")]
            player_contract_probe_after_offer: String::new(),
            #[cfg(feature = "dev")]
            player_contract_probe_after_accepted: String::new(),
            #[cfg(feature = "dev")]
            player_contract_probe_comparison: String::new(),
            champion_mastery_open: false,
            champion_mastery_entries: Vec::new(),
            champion_mastery_bulk_value: 100,
            transfer_always_success: false,
            recruitment_instant_retry: false,
            teams: Vec::new(),
            team_database_search: String::new(),
            team_search_league_filter: None,
            team_search_player_team_only: false,
            team_search_roster_min: String::new(),
            team_search_roster_max: String::new(),
            team_search_staff_min: String::new(),
            team_search_staff_max: String::new(),
            team_sort_column: TeamSortColumn::Name,
            team_sort_ascending: true,
            #[cfg(feature = "dev")]
            team_workspace_search: String::new(),
            #[cfg(feature = "dev")]
            team_workspace_team_id: None,
            #[cfg(feature = "dev")]
            team_roster_window_open: false,
            #[cfg(feature = "dev")]
            team_staff_window_open: false,
            #[cfg(feature = "dev")]
            team_roster_selected_player_id: None,
            #[cfg(feature = "dev")]
            team_staff_selected_staff_id: None,
            #[cfg(feature = "dev")]
            team_condition_window_open: false,
            #[cfg(feature = "dev")]
            team_condition_entries: Vec::new(),
            #[cfg(feature = "dev")]
            team_condition_team_id: None,
            #[cfg(feature = "dev")]
            team_condition_selected_player_ids: BTreeSet::new(),
            #[cfg(feature = "dev")]
            team_condition_bulk_stamina: String::new(),
            #[cfg(feature = "dev")]
            team_condition_bulk_condition: String::new(),
            #[cfg(feature = "dev")]
            team_data_probe_window_open: false,
            #[cfg(feature = "dev")]
            team_data_probe_team_id: None,
            #[cfg(feature = "dev")]
            team_data_probe_raw: String::new(),
            #[cfg(feature = "dev")]
            team_management_data: None,
            #[cfg(feature = "dev")]
            team_management_last_request_team_id: None,
            #[cfg(feature = "dev")]
            team_strategy_window_open: false,
            #[cfg(feature = "dev")]
            team_merchandise_window_open: false,
            #[cfg(feature = "dev")]
            team_champion_setup_window_open: false,
            #[cfg(feature = "dev")]
            team_gaming_house_window_open: false,
            #[cfg(feature = "dev")]
            team_history_data: None,
            #[cfg(feature = "dev")]
            team_history_last_request_team_id: None,
            #[cfg(feature = "dev")]
            team_match_history_window_open: false,
            #[cfg(feature = "dev")]
            team_pre_match_analysis_window_open: false,
            #[cfg(feature = "dev")]
            team_history_summary_window_open: false,
            recruitment_management_tab: RecruitmentManagementTab::Players,
            recruitment_player_search: String::new(),
            recruitment_player_id: None,
            recruitment_team_search: String::new(),
            recruitment_team_id: None,
            recruitment_staff_search: String::new(),
            recruitment_staff_id: None,
            free_agent_confirmation_player_id: None,
            free_agent_confirmation_staff_id: None,
            connected: false,
            status: starting_status,
            player_search_status: None,
            staff_search_status: None,
            team_search_status: None,
            bridge_version: "-".to_string(),
            bridge_protocol: None,
            bridge_tfm2_target: None,
            compatibility_issue: None,
            compatibility_popup_open: false,
            compatibility_ignored_for_session: false,
        };
        app.reload_saved_filters();
        app.reload_saved_staff_filters();
        app.reload_saved_player_lists();
        app.refresh_connection();
        if app.connected {
            app.refresh_economy();
            app.refresh_players();
            app.refresh_staff();
            app.refresh_teams();
            app.refresh_recruitment_settings();
        }
        app
    }
}

#[cfg(feature = "dev")]
fn format_team_member_references(entries: &[TeamMemberReference]) -> String {
    if entries.is_empty() {
        return "—".to_string();
    }
    entries
        .iter()
        .map(|entry| format!("{} [{}]", entry.name, entry.id))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(feature = "dev")]
fn format_team_lineup(entries: &[TeamLineupEntry]) -> String {
    if entries.is_empty() {
        return "—".to_string();
    }
    entries
        .iter()
        .map(|entry| match &entry.member {
            Some(member) => format!("{}: {} [{}]", entry.slot, member.name, member.id),
            None => format!("{}: —", entry.slot),
        })
        .collect::<Vec<_>>()
        .join(" · ")
}

#[cfg(feature = "dev")]
fn team_strategy_value(entries: &[TeamStrategyEntry], key: &str) -> String {
    entries
        .iter()
        .find(|entry| entry.key == key)
        .map(|entry| value_or_dash(&entry.value))
        .unwrap_or_else(|| "—".to_string())
}

#[cfg(feature = "dev")]
fn parse_usize_value(value: &str) -> usize {
    value.trim().parse::<usize>().unwrap_or(0)
}

#[cfg(feature = "dev")]
fn parse_f64_value(value: &str) -> f64 {
    value.trim().parse::<f64>().unwrap_or(0.0)
}

#[cfg(feature = "dev")]
fn render_team_member_table_viewport(
    ui: &mut egui::Ui,
    horizontal_scroll_id: &'static str,
    table_min_width: f32,
    render_table: impl FnOnce(&mut egui::Ui, f32),
) {
    StripBuilder::new(ui)
        .clip(true)
        .size(Size::remainder().at_least(120.0))
        .vertical(|mut strip| {
            strip.cell(|ui| {
                let viewport_height = ui.available_height().max(1.0);
                egui::ScrollArea::horizontal()
                    .id_salt(horizontal_scroll_id)
                    .auto_shrink([false, false])
                    .max_height(viewport_height)
                    .show(ui, |ui| {
                        ui.set_min_width(table_min_width);
                        render_table(ui, viewport_height);
                    });
            });
        });
}

impl ModifierApp {
    fn loaded_dataset_status(dataset: &str, count: usize) -> String {
        format!("{dataset} data loaded: {count}")
    }

    fn update_player_search_status(&mut self) {
        let status = Self::loaded_dataset_status("Player", self.players.len());
        self.player_search_status = Some(status.clone());
        if self.active_tab == AppTab::Search && self.search_tab == SearchTab::Players {
            self.status = status;
        }
    }

    fn update_staff_search_status(&mut self) {
        let status = Self::loaded_dataset_status("Staff", self.staffs.len());
        self.staff_search_status = Some(status.clone());
        if self.active_tab == AppTab::Search && self.search_tab == SearchTab::Staff {
            self.status = status;
        }
    }

    fn update_team_search_status(&mut self) {
        let status = Self::loaded_dataset_status("Team", self.teams.len());
        self.team_search_status = Some(status.clone());
        if self.active_tab == AppTab::Search && self.search_tab == SearchTab::Teams {
            self.status = status;
        }
    }

    fn restore_active_search_status(&mut self) {
        if self.active_tab != AppTab::Search {
            return;
        }

        let search_status = match self.search_tab {
            SearchTab::Players => self.player_search_status.clone(),
            SearchTab::Staff => self.staff_search_status.clone(),
            SearchTab::Teams => self.team_search_status.clone(),
            SearchTab::Lists => Some(format!(
                "Saved Lists loaded: {}",
                self.saved_player_lists.len()
            )),
            #[cfg(feature = "dev")]
            SearchTab::History => Some("History is under development".to_string()),
        };

        if let Some(status) = search_status {
            self.status = status;
        }
    }

    fn render_language_selector(&mut self, ui: &mut egui::Ui) {
        ui.label(self.localization.tr("settings.language"));
        let current_language_name = self.localization.current_language_name().to_string();
        let available_languages = self.localization.available_languages();
        let mut selected_language = None;

        egui::ComboBox::from_id_salt("app_language_selector")
            .selected_text(current_language_name)
            .show_ui(ui, |ui| {
                for (language_code, language_name) in available_languages {
                    let is_selected =
                        self.localization.current_language() == language_code.as_str();
                    if ui.selectable_label(is_selected, &language_name).clicked() {
                        selected_language = Some((language_code, language_name));
                    }
                }
            });

        if let Some((language_code, language_name)) = selected_language {
            match self.localization.select_language(&language_code) {
                Ok(()) => {
                    self.status = self.localization.tr_with(
                        "settings.language_changed",
                        &[("language", language_name.as_str())],
                    );
                }
                Err(error) => {
                    self.status = self.localization.tr_with(
                        "settings.language_error",
                        &[("error", error.as_str())],
                    );
                }
            }
        }

        #[cfg(feature = "dev")]
        {
            if ui
                .small_button(self.localization.tr("localization.reload"))
                .clicked()
            {
                self.localization.reload();
                self.status = self.localization.tr("localization.reloaded");
            }

            let count = self.localization.debug_issue_count().to_string();
            ui.label(self.localization.tr_with(
                "localization.dev_issues",
                &[("count", count.as_str())],
            ))
            .on_hover_text(self.localization.debug_report());
        }
    }

    fn compatibility_issue_for(
        bridge_version: &str,
        bridge_protocol: Option<u32>,
        bridge_tfm2_target: Option<&str>,
    ) -> Option<CompatibilityIssue> {
        let installed_bridge_version = bridge_version.to_string();
        let bridge_tfm2_target_owned = bridge_tfm2_target.map(str::to_string);

        if let Some(rule) = unsupported_bridge_rule_for(bridge_version) {
            let (action, required_bridge_version, required_editor_version) =
                match rule.requirement {
                    UnsupportedRequirement::Bridge(required_bridge_version) => (
                        CompatibilityAction::BridgeUpdate,
                        Some(required_bridge_version.to_string()),
                        None,
                    ),
                };
            return Some(CompatibilityIssue {
                severity: CompatibilitySeverity::NotSupported,
                action,
                reason: CompatibilityReason::KnownUnsupportedCombination,
                installed_bridge_version,
                bridge_tfm2_target: bridge_tfm2_target_owned,
                required_bridge_version,
                required_editor_version,
            });
        }

        let protocol = match bridge_protocol {
            Some(protocol) => protocol,
            None => {
                return Some(CompatibilityIssue {
                    severity: CompatibilitySeverity::Warning,
                    action: compatibility_action_for_version(bridge_version),
                    reason: CompatibilityReason::UnverifiedLegacyBridge,
                    installed_bridge_version,
                    bridge_tfm2_target: bridge_tfm2_target_owned,
                    required_bridge_version: Some(REQUIRED_BRIDGE_VERSION.to_string()),
                    required_editor_version: None,
                });
            }
        };

        let target = match bridge_tfm2_target {
            Some(target) if !target.trim().is_empty() => target,
            _ => {
                return Some(CompatibilityIssue {
                    severity: CompatibilitySeverity::Warning,
                    action: compatibility_action_for_version(bridge_version),
                    reason: CompatibilityReason::UnverifiedLegacyBridge,
                    installed_bridge_version,
                    bridge_tfm2_target: bridge_tfm2_target_owned,
                    required_bridge_version: Some(REQUIRED_BRIDGE_VERSION.to_string()),
                    required_editor_version: None,
                });
            }
        };

        if !(MINIMUM_SAFE_BRIDGE_PROTOCOL..=MAXIMUM_SAFE_BRIDGE_PROTOCOL)
            .contains(&protocol)
        {
            return Some(CompatibilityIssue {
                severity: CompatibilitySeverity::NotSupported,
                action: compatibility_action_for_version(bridge_version),
                reason: CompatibilityReason::ProtocolMismatch,
                installed_bridge_version,
                bridge_tfm2_target: Some(target.to_string()),
                required_bridge_version: Some(REQUIRED_BRIDGE_VERSION.to_string()),
                required_editor_version: None,
            });
        }

        if target != SUPPORTED_TFM2_VERSION {
            return Some(CompatibilityIssue {
                severity: CompatibilitySeverity::Warning,
                action: CompatibilityAction::GameVersionMismatch,
                reason: CompatibilityReason::GameTargetMismatch,
                installed_bridge_version,
                bridge_tfm2_target: Some(target.to_string()),
                required_bridge_version: Some(REQUIRED_BRIDGE_VERSION.to_string()),
                required_editor_version: None,
            });
        }

        if protocol != BRIDGE_PROTOCOL_VERSION {
            return Some(CompatibilityIssue {
                severity: CompatibilitySeverity::Warning,
                action: compatibility_action_for_version(bridge_version),
                reason: CompatibilityReason::ProtocolMismatch,
                installed_bridge_version,
                bridge_tfm2_target: Some(target.to_string()),
                required_bridge_version: Some(REQUIRED_BRIDGE_VERSION.to_string()),
                required_editor_version: None,
            });
        }

        if bridge_version != REQUIRED_BRIDGE_VERSION {
            return Some(CompatibilityIssue {
                severity: CompatibilitySeverity::Warning,
                action: compatibility_action_for_version(bridge_version),
                reason: CompatibilityReason::VersionMismatch,
                installed_bridge_version,
                bridge_tfm2_target: Some(target.to_string()),
                required_bridge_version: Some(REQUIRED_BRIDGE_VERSION.to_string()),
                required_editor_version: None,
            });
        }

        None
    }

    fn update_compatibility_state(&mut self) {
        self.compatibility_issue = Self::compatibility_issue_for(
            &self.bridge_version,
            self.bridge_protocol,
            self.bridge_tfm2_target.as_deref(),
        );

        if let Some(issue) = self.compatibility_issue.as_ref() {
            if issue.severity == CompatibilitySeverity::NotSupported {
                self.connected = false;
                self.status = self.localization.tr("compatibility.status.connection_blocked");
            }
            if !self.compatibility_ignored_for_session {
                self.compatibility_popup_open = true;
            }
        } else {
            self.compatibility_popup_open = false;
            self.compatibility_ignored_for_session = false;
        }
    }

    #[cfg(feature = "dev")]
    fn compatibility_debug_report(&self) -> String {
        let state = match self.compatibility_issue.as_ref() {
            None if self.connected => "OK",
            None => "Unknown",
            Some(issue) if issue.severity == CompatibilitySeverity::NotSupported => {
                "Not Supported"
            }
            Some(_) => "Warning",
        };
        let protocol = self
            .bridge_protocol
            .map(|value| value.to_string())
            .unwrap_or_else(|| "missing".to_string());
        let target = self
            .bridge_tfm2_target
            .clone()
            .unwrap_or_else(|| "missing".to_string());
        let reason = self
            .compatibility_issue
            .as_ref()
            .map(|issue| format!("{:?}", issue.reason))
            .unwrap_or_else(|| "None".to_string());
        let action = self
            .compatibility_issue
            .as_ref()
            .map(|issue| format!("{:?}", issue.action))
            .unwrap_or_else(|| "None".to_string());

        format!(
            "Compatibility: {state}
Reason: {reason}
Action: {action}
Editor: {}
Required Bridge: v{}
Installed Bridge: v{}
Expected protocol: {}
Safe protocol range: {}-{}
Bridge protocol: {}
Supported TFM2: v{}
Bridge TFM2 target: v{}",
            display_version(),
            REQUIRED_BRIDGE_VERSION,
            self.bridge_version,
            BRIDGE_PROTOCOL_VERSION,
            MINIMUM_SAFE_BRIDGE_PROTOCOL,
            MAXIMUM_SAFE_BRIDGE_PROTOCOL,
            protocol,
            SUPPORTED_TFM2_VERSION,
            target,
        )
    }

    fn render_compatibility_popup(&mut self, ctx: &egui::Context) {
        if !self.compatibility_popup_open {
            return;
        }

        let Some(issue) = self.compatibility_issue.clone() else {
            self.compatibility_popup_open = false;
            return;
        };

        let mut continue_requested = false;
        let mut dismiss_requested = false;
        let mut close_editor_requested = false;
        let mut github_requested = false;
        let mut workshop_requested = false;

        let subtitle_key = if issue.severity == CompatibilitySeverity::NotSupported {
            "compatibility.not_supported.title"
        } else {
            match issue.action {
                CompatibilityAction::BridgeUpdate => "compatibility.bridge_update.title",
                CompatibilityAction::EditorUpdate => "compatibility.editor_update.title",
                CompatibilityAction::VerifyInstallation
                | CompatibilityAction::GameVersionMismatch => {
                    "compatibility.warning.title"
                }
            }
        };
        let subtitle = self.localization.tr(subtitle_key);
        let severity_color = if issue.severity == CompatibilitySeverity::NotSupported {
            egui::Color32::from_rgb(220, 70, 70)
        } else {
            egui::Color32::from_rgb(235, 196, 0)
        };
        let popup_frame = egui::Frame::window(&ctx.style())
            .stroke(egui::Stroke::new(2.0_f32, severity_color));

        egui::Window::new("bridge_compatibility_warning_window")
            .id(egui::Id::new("bridge_compatibility_warning"))
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .frame(popup_frame)
            .show(ctx, |ui| {
                ui.set_min_width(540.0);

                ui.label(
                    egui::RichText::new(
                        self.localization.tr("compatibility.warning.heading"),
                    )
                    .size(19.0)
                    .strong()
                    .color(severity_color),
                );
                ui.add_space(3.0);
                ui.label(egui::RichText::new(subtitle.as_str()).size(16.0).strong());
                ui.add_space(10.0);

                let message_key = match (issue.severity, issue.action, issue.reason) {
                    (
                        CompatibilitySeverity::NotSupported,
                        CompatibilityAction::BridgeUpdate,
                        _,
                    ) => "compatibility.not_supported.bridge_message",
                    (
                        CompatibilitySeverity::NotSupported,
                        CompatibilityAction::EditorUpdate,
                        _,
                    ) => "compatibility.not_supported.editor_message",
                    (
                        CompatibilitySeverity::NotSupported,
                        _,
                        CompatibilityReason::ProtocolMismatch,
                    ) => "compatibility.not_supported.protocol_message",
                    (CompatibilitySeverity::NotSupported, _, _) => {
                        "compatibility.not_supported.generic_message"
                    }
                    (
                        CompatibilitySeverity::Warning,
                        CompatibilityAction::BridgeUpdate,
                        _,
                    ) => "compatibility.bridge_update.message",
                    (
                        CompatibilitySeverity::Warning,
                        CompatibilityAction::EditorUpdate,
                        _,
                    ) => "compatibility.editor_update.message",
                    (
                        CompatibilitySeverity::Warning,
                        CompatibilityAction::GameVersionMismatch,
                        _,
                    ) => "compatibility.warning.game_target_message",
                    (
                        CompatibilitySeverity::Warning,
                        CompatibilityAction::VerifyInstallation,
                        CompatibilityReason::ProtocolMismatch,
                    ) => "compatibility.warning.protocol_message",
                    (CompatibilitySeverity::Warning, CompatibilityAction::VerifyInstallation, _) => {
                        "compatibility.warning.unverified_message"
                    }
                };
                ui.label(self.localization.tr(message_key));
                ui.add_space(10.0);

                let editor_version = display_version();
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(
                            self.localization.tr("compatibility.fields.editor_version"),
                        )
                        .strong(),
                    );
                    ui.label(editor_version.as_str());
                });
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(
                            self.localization.tr("compatibility.fields.installed_bridge"),
                        )
                        .strong(),
                    );
                    ui.label(format!("v{}", issue.installed_bridge_version));
                });

                match issue.action {
                    CompatibilityAction::BridgeUpdate | CompatibilityAction::VerifyInstallation => {
                        let required_bridge = issue
                            .required_bridge_version
                            .as_deref()
                            .unwrap_or(REQUIRED_BRIDGE_VERSION);
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(
                                    self.localization.tr("compatibility.fields.required_bridge"),
                                )
                                .strong(),
                            );
                            ui.label(format!("v{required_bridge}"));
                        });
                    }
                    CompatibilityAction::EditorUpdate => {
                        if issue.severity == CompatibilitySeverity::NotSupported {
                            let required_editor = issue
                                .required_editor_version
                                .as_deref()
                                .map(|version| format!("v{version}"))
                                .unwrap_or_else(|| {
                                    self.localization
                                        .tr("compatibility.fields.latest_matching_editor")
                                });
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(
                                        self.localization
                                            .tr("compatibility.fields.required_editor"),
                                    )
                                    .strong(),
                                );
                                ui.label(required_editor);
                            });
                        } else {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(
                                        self.localization
                                            .tr("compatibility.fields.supported_bridge"),
                                    )
                                    .strong(),
                                );
                                let supported_bridge = issue
                                    .required_bridge_version
                                    .as_deref()
                                    .unwrap_or(REQUIRED_BRIDGE_VERSION);
                                ui.label(format!("v{supported_bridge}"));
                            });
                        }
                    }
                    CompatibilityAction::GameVersionMismatch => {
                        let bridge_target = issue
                            .bridge_tfm2_target
                            .as_deref()
                            .unwrap_or("unknown");
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(
                                    self.localization.tr("compatibility.fields.supported_tfm2"),
                                )
                                .strong(),
                            );
                            ui.label(format!("v{SUPPORTED_TFM2_VERSION}"));
                        });
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(
                                    self.localization
                                        .tr("compatibility.fields.bridge_tfm2_target"),
                                )
                                .strong(),
                            );
                            ui.label(format!("v{bridge_target}"));
                        });
                    }
                }

                #[cfg(feature = "dev")]
                {
                    ui.add_space(8.0);
                    ui.separator();
                    ui.monospace(self.compatibility_debug_report());
                    ui.separator();
                }

                ui.add_space(10.0);
                let instruction_key = match (issue.severity, issue.action) {
                    (CompatibilitySeverity::NotSupported, CompatibilityAction::BridgeUpdate) => {
                        "compatibility.not_supported.bridge_instruction"
                    }
                    (CompatibilitySeverity::NotSupported, CompatibilityAction::EditorUpdate) => {
                        "compatibility.not_supported.editor_instruction"
                    }
                    (CompatibilitySeverity::NotSupported, _) => {
                        "compatibility.not_supported.generic_instruction"
                    }
                    (CompatibilitySeverity::Warning, CompatibilityAction::BridgeUpdate) => {
                        "compatibility.bridge_update.instruction"
                    }
                    (CompatibilitySeverity::Warning, CompatibilityAction::EditorUpdate) => {
                        "compatibility.editor_update.instruction"
                    }
                    (CompatibilitySeverity::Warning, _) => {
                        "compatibility.warning.generic_instruction"
                    }
                };
                ui.label(self.localization.tr(instruction_key));
                ui.add_space(12.0);

                ui.horizontal_wrapped(|ui| {
                    if issue.severity == CompatibilitySeverity::Warning
                        && ui
                            .button(self.localization.tr("compatibility.actions.continue_anyway"))
                            .clicked()
                    {
                        continue_requested = true;
                    }
                    if ui
                        .button(self.localization.tr("compatibility.actions.close"))
                        .clicked()
                    {
                        if issue.severity == CompatibilitySeverity::Warning {
                            close_editor_requested = true;
                        } else {
                            dismiss_requested = true;
                        }
                    }

                    match issue.action {
                        CompatibilityAction::BridgeUpdate
                        | CompatibilityAction::VerifyInstallation
                        | CompatibilityAction::GameVersionMismatch => {
                            let recommended_fill = ui.visuals().selection.bg_fill;
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new(
                                            self.localization
                                                .tr("compatibility.actions.steam_workshop"),
                                        )
                                        .strong(),
                                    )
                                    .fill(recommended_fill),
                                )
                                .clicked()
                            {
                                workshop_requested = true;
                            }
                            if ui
                                .button(
                                    self.localization
                                        .tr("compatibility.actions.github_download"),
                                )
                                .clicked()
                            {
                                github_requested = true;
                            }
                        }
                        CompatibilityAction::EditorUpdate => {
                            let recommended_fill = ui.visuals().selection.bg_fill;
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new(
                                            self.localization
                                                .tr("compatibility.actions.download_editor"),
                                        )
                                        .strong(),
                                    )
                                    .fill(recommended_fill),
                                )
                                .clicked()
                            {
                                github_requested = true;
                            }
                        }
                    }
                });
            });

        if continue_requested || dismiss_requested {
            self.compatibility_ignored_for_session = true;
            self.compatibility_popup_open = false;
        }

        if close_editor_requested {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        if workshop_requested {
            if let Err(error) = open_url(STEAM_WORKSHOP_URL) {
                self.status = self.localization.tr_with(
                    "compatibility.workshop_open_error",
                    &[("error", error.as_str())],
                );
            }
        }

        if github_requested {
            if let Err(error) = open_url(GITHUB_RELEASES_URL) {
                self.status = self.localization.tr_with(
                    "compatibility.github_open_error",
                    &[("error", error.as_str())],
                );
            }
        }
    }

    fn request_raw(command: &str) -> Result<String, String> {

        let address: SocketAddr = BRIDGE_ADDR
            .parse()
            .map_err(|_| "Invalid bridge address".to_string())?;

        let mut stream = TcpStream::connect_timeout(&address, Duration::from_millis(600))
            .map_err(|_| "TFM2 Editor Bridge is not responding".to_string())?;

        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .map_err(|e| format!("Could not set read timeout: {e}"))?;
        stream
            .set_write_timeout(Some(Duration::from_secs(3)))
            .map_err(|e| format!("Could not set write timeout: {e}"))?;

        writeln!(stream, "{command}").map_err(|e| format!("Could not send command: {e}"))?;
        stream.flush().map_err(|e| format!("Flush failed: {e}"))?;

        let mut reader = BufReader::new(stream);
        let mut response = String::new();
        reader
            .read_line(&mut response)
            .map_err(|e| format!("Could not read response: {e}"))?;

        if response.is_empty() {
            return Err("Bridge returned an empty response".to_string());
        }

        Ok(response.trim().to_string())
    }

    fn game_request(&self, command: &str) -> Result<String, String> {
        if matches!(
            self.compatibility_issue.as_ref(),
            Some(issue) if issue.severity == CompatibilitySeverity::NotSupported
        ) {
            return Err(self.localization.tr("compatibility.status.command_blocked"));
        }
        if !self.connected {
            return Err(self.localization.tr("connection.not_connected"));
        }
        Self::request_raw(command)
    }

    fn refresh_connection(&mut self) {
        match Self::request_raw("PING") {
            Ok(response) => {
                let parts: Vec<&str> = response.split('|').collect();
                if parts.len() >= 3 && parts[0] == "OK" && parts[1] == "PONG" {
                    self.connected = true;
                    self.bridge_version = parts[2].to_string();
                    self.bridge_protocol = parts.get(3).and_then(|value| value.parse::<u32>().ok());
                    self.bridge_tfm2_target = parts
                        .get(4)
                        .map(|value| value.trim().to_string())
                        .filter(|value| !value.is_empty());
                    self.status = "Connected to TFM2".to_string();
                    self.compatibility_ignored_for_session = false;
                    self.update_compatibility_state();
                } else {
                    self.connected = false;
                    self.bridge_version = "-".to_string();
                    self.bridge_protocol = None;
                    self.bridge_tfm2_target = None;
                    self.compatibility_issue = None;
                    self.compatibility_popup_open = false;
                    self.compatibility_ignored_for_session = false;
                    self.status = format!("Unexpected bridge response: {response}");
                }
            }
            Err(error) => {
                self.connected = false;
                self.bridge_version = "-".to_string();
                self.bridge_protocol = None;
                self.bridge_tfm2_target = None;
                self.compatibility_issue = None;
                self.compatibility_popup_open = false;
                self.compatibility_ignored_for_session = false;
                self.status = error;
            }
        }
    }

    fn parse_economy_response(&mut self, response: &str) -> Result<(), String> {
        let parts: Vec<&str> = response.split('|').collect();
        if parts.first() == Some(&"ERR") {
            return Err(parts.get(1).unwrap_or(&"UNKNOWN_ERROR").to_string());
        }

        if parts.len() != 5 || parts[0] != "OK" || parts[1] != "ECONOMY" {
            return Err(format!("Unexpected response: {response}"));
        }

        self.economy.money = format_internal_amount(parts[2]);
        self.economy.transfer_budget = format_internal_amount(parts[3]);
        self.economy.salary_budget = format_internal_amount(parts[4]);
        Ok(())
    }

    fn refresh_economy(&mut self) {
        match self.game_request("GET_ECONOMY") {
            Ok(response) => match self.parse_economy_response(&response) {
                Ok(()) => {
                    self.connected = true;
                    self.status = "Economy loaded from game".to_string();
                }
                Err(error) => self.status = human_error(&error),
            },
            Err(error) => {
                self.connected = false;
                self.status = error;
            }
        }
    }

    fn apply_economy(&mut self) {
        let parsed = [
            parse_display_to_internal(&self.economy.money),
            parse_display_to_internal(&self.economy.transfer_budget),
            parse_display_to_internal(&self.economy.salary_budget),
        ];

        if parsed.iter().any(Result::is_err) {
            self.status = "All economy fields must contain valid numbers".to_string();
            return;
        }

        let values: Vec<f64> = parsed.into_iter().map(Result::unwrap).collect();
        let command = format!(
            "SET_ECONOMY|{}|{}|{}",
            format_internal_for_command(values[0]),
            format_internal_for_command(values[1]),
            format_internal_for_command(values[2]),
        );

        match self.game_request(&command) {
            Ok(response) => match self.parse_economy_response(&response) {
                Ok(()) => {
                    self.connected = true;
                    self.status = "Economy updated".to_string();
                }
                Err(error) => self.status = human_error(&error),
            },
            Err(error) => {
                self.connected = false;
                self.status = error;
            }
        }
    }

    fn refresh_players(&mut self) {
        match self.game_request("GET_PLAYERS") {
            Ok(response) => match parse_players_response(&response) {
                Ok(players) => {
                    self.connected = true;
                    self.players = players;

                    let keep_recruitment_selection = self
                        .recruitment_player_id
                        .is_some_and(|id| self.players.iter().any(|player| player.id == id));
                    if !keep_recruitment_selection {
                        self.recruitment_player_id = self.players.first().map(|player| player.id);
                    }

                    let keep_selection = self
                        .selected_player_id
                        .is_some_and(|id| self.players.iter().any(|player| player.id == id));

                    if !keep_selection {
                        self.selected_player_id = self.players.first().map(|player| player.id);
                    }

                    self.selected_player = None;
                    self.player_positions = None;
                    self.player_potential = None;
                    self.player_communication = None;
                    if self.selected_player_id.is_some() {
                        self.refresh_selected_player();
                    } else {
                        self.status = "No players found in the active career".to_string();
                    }
                    self.update_player_search_status();
                }
                Err(error) => {
                    let status = human_error(&error);
                    self.player_search_status = Some(status.clone());
                    self.status = status;
                }
            },
            Err(error) => {
                self.connected = false;
                self.player_search_status = Some(error.clone());
                self.status = error;
            }
        }
    }

    fn refresh_staff(&mut self) {
        match self.game_request("GET_STAFFS") {
            Ok(response) => match parse_staffs_response(&response) {
                Ok(staffs) => {
                    self.connected = true;
                    self.staffs = staffs;

                    let keep_recruitment_selection = self
                        .recruitment_staff_id
                        .is_some_and(|id| self.staffs.iter().any(|staff| staff.id == id));
                    if !keep_recruitment_selection {
                        self.recruitment_staff_id = self.staffs.first().map(|staff| staff.id);
                    }

                    let keep_selection = self
                        .selected_staff_id
                        .is_some_and(|id| self.staffs.iter().any(|staff| staff.id == id));
                    if !keep_selection {
                        self.selected_staff_id = self.staffs.first().map(|staff| staff.id);
                    }

                    self.selected_staff = None;
                    if self.selected_staff_id.is_some() {
                        self.refresh_selected_staff();
                    } else {
                        self.status = "No staff found in the active career".to_string();
                    }
                    self.update_staff_search_status();
                }
                Err(error) => {
                    let status = human_error(&error);
                    self.staff_search_status = Some(status.clone());
                    self.status = status;
                }
            },
            Err(error) => {
                self.connected = false;
                self.staff_search_status = Some(error.clone());
                self.status = error;
            }
        }
    }

    fn open_player_in_editor(&mut self, athlete_id: usize) {
        self.selected_player_id = Some(athlete_id);
        self.active_tab = AppTab::PlayerEditor;
        self.refresh_selected_player();
    }

    fn open_staff_in_editor(&mut self, staff_id: usize) {
        self.selected_staff_id = Some(staff_id);
        self.active_tab = AppTab::StaffEditor;
        self.refresh_selected_staff();
    }

    #[cfg(feature = "dev")]
    fn open_team_workspace(&mut self, team_id: usize) {
        if self.teams.iter().any(|team| team.id == team_id) {
            let selection_changed = self.team_workspace_team_id != Some(team_id);
            if selection_changed {
                self.team_roster_selected_player_id = None;
                self.team_staff_selected_staff_id = None;
                self.team_condition_window_open = false;
                self.team_condition_entries.clear();
                self.team_condition_team_id = None;
                self.team_condition_selected_player_ids.clear();
                self.team_data_probe_window_open = false;
                self.team_data_probe_team_id = None;
                self.team_data_probe_raw.clear();
                self.team_management_data = None;
                self.team_management_last_request_team_id = None;
                self.team_history_data = None;
                self.team_history_last_request_team_id = None;
            }
            self.team_workspace_team_id = Some(team_id);
            self.active_tab = AppTab::Team;
            if let Some(team) = self.teams.iter().find(|team| team.id == team_id) {
                self.status = format!("Team data loaded: {}", team.display_name);
            }
            if selection_changed
                || self.team_management_last_request_team_id != Some(team_id)
            {
                self.refresh_team_management_data();
            }
        }
    }

    #[cfg(feature = "dev")]
    fn refresh_team_management_data(&mut self) {
        let Some(team) = self
            .team_workspace_team_id
            .and_then(|team_id| self.teams.iter().find(|team| team.id == team_id))
            .cloned()
        else {
            self.status = "Select a team first".to_string();
            return;
        };

        self.team_management_last_request_team_id = Some(team.id);
        let command = format!("GET_TEAM_MANAGEMENT|{}", team.id);
        match self
            .game_request(&command)
            .and_then(|response| parse_team_management_response(&response))
        {
            Ok(data) => {
                self.team_management_data = Some(data);
                self.status = format!("Team management data loaded: {}", team.display_name);
            }
            Err(error) => {
                self.team_management_data = None;
                self.status = human_error(&error);
            }
        }
    }

    #[cfg(feature = "dev")]
    fn refresh_team_history_data(&mut self) {
        let Some(team) = self
            .team_workspace_team_id
            .and_then(|team_id| self.teams.iter().find(|team| team.id == team_id))
            .cloned()
        else {
            self.status = "Select a team first".to_string();
            return;
        };

        self.team_history_last_request_team_id = Some(team.id);
        let command = format!("GET_TEAM_PROBE|{}", team.id);
        match self.game_request(&command).and_then(|response| {
            let parts = response.split('|').collect::<Vec<_>>();
            if parts.len() != 3 || parts[0] != "OK" || parts[1] != "TEAM_PROBE" {
                return Err(response);
            }
            let raw = hex_decode(parts[2])?;
            parse_team_history_probe(&raw, team.id, &self.teams, &self.players)
        }) {
            Ok(data) => {
                self.team_history_data = Some(data);
                self.status = format!("Team match history loaded: {}", team.display_name);
            }
            Err(error) => {
                self.team_history_data = None;
                self.status = human_error(&error);
            }
        }
    }

    #[cfg(feature = "dev")]
    fn refresh_team_data_probe(&mut self) {
        let Some(team) = self
            .team_workspace_team_id
            .and_then(|team_id| self.teams.iter().find(|team| team.id == team_id))
            .cloned()
        else {
            self.status = "Select a team first".to_string();
            return;
        };

        let command = format!("GET_TEAM_PROBE|{}", team.id);
        match self.game_request(&command).and_then(|response| {
            let parts = response.split('|').collect::<Vec<_>>();
            if parts.len() != 3 || parts[0] != "OK" || parts[1] != "TEAM_PROBE" {
                return Err(response);
            }
            hex_decode(parts[2])
        }) {
            Ok(raw) => {
                self.team_data_probe_raw = raw;
                self.team_data_probe_team_id = Some(team.id);
                self.team_data_probe_window_open = true;
                self.status = format!("Team data probe loaded: {}", team.display_name);
            }
            Err(error) => {
                self.status = human_error(&error);
            }
        }
    }

    #[cfg(feature = "dev")]
    fn refresh_team_condition_probe(&mut self) {
        let Some(team) = self
            .team_workspace_team_id
            .and_then(|team_id| self.teams.iter().find(|team| team.id == team_id))
            .cloned()
        else {
            self.status = "Select a team first".to_string();
            return;
        };

        let roster = self
            .players
            .iter()
            .filter(|player| summary_belongs_to_team(&player.team, &team))
            .map(|player| (player.id, player.name.clone()))
            .collect::<Vec<_>>();

        let mut entries = Vec::with_capacity(roster.len());
        for (player_id, player_name) in roster {
            let command = format!("GET_PLAYER_CONTRACT_PROBE|{player_id}");
            let result = self.game_request(&command).and_then(|response| {
                let parts = response.split('|').collect::<Vec<_>>();
                if parts.first() == Some(&"ERR") {
                    return Err(human_error(parts.get(1).copied().unwrap_or("UNKNOWN_ERROR")));
                }
                if parts.len() != 3
                    || parts[0] != "OK"
                    || parts[1] != "PLAYER_CONTRACT_PROBE"
                {
                    return Err(format!("Unexpected probe response: {response}"));
                }
                let raw = hex_decode(parts[2])?;
                parse_team_condition_from_player_probe(&raw)
            });

            match result {
                Ok((stamina, condition)) => entries.push(TeamConditionEntry {
                    player_id,
                    player_name,
                    original_stamina: stamina.clone(),
                    original_condition: condition.clone(),
                    stamina,
                    condition,
                    write_status: self.localization.tr("team_condition.ready"),
                }),
                Err(error) => entries.push(TeamConditionEntry {
                    player_id,
                    player_name,
                    stamina: String::new(),
                    condition: String::new(),
                    original_stamina: String::new(),
                    original_condition: String::new(),
                    write_status: error,
                }),
            }
        }

        self.team_condition_entries = entries;
        self.team_condition_team_id = Some(team.id);
        self.team_condition_selected_player_ids.clear();
        self.team_condition_window_open = true;
        self.status = format!(
            "Team condition data loaded: {} player(s)",
            self.team_condition_entries.len()
        );
    }

    #[cfg(feature = "dev")]
    fn apply_team_condition_changes(&mut self) {
        let changed_indices = self
            .team_condition_entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| entry.has_changes().then_some(index))
            .collect::<Vec<_>>();

        if changed_indices.is_empty() {
            self.status = self.localization.tr("team_condition.no_changes");
            return;
        }

        let mut applied = 0_usize;
        let mut failed = 0_usize;

        for index in changed_indices {
            let (player_id, stamina, condition) = {
                let entry = &self.team_condition_entries[index];
                (
                    entry.player_id,
                    entry.stamina.trim().to_string(),
                    entry.condition.trim().to_string(),
                )
            };

            let validation = validate_condition_editor_value(&stamina, "Stamina")
                .and_then(|_| validate_condition_editor_value(&condition, "Condition"));
            if let Err(error) = validation {
                self.team_condition_entries[index].write_status = error;
                failed += 1;
                continue;
            }

            let command = format!("SET_PLAYER_CONDITION|{player_id}|{stamina}|{condition}");
            match self.game_request(&command) {
                Ok(response) => {
                    let parts = response.split('|').collect::<Vec<_>>();
                    if parts.first() == Some(&"ERR") {
                        self.team_condition_entries[index].write_status =
                            human_error(parts.get(1).copied().unwrap_or("UNKNOWN_ERROR"));
                        failed += 1;
                    } else if parts.len() == 5
                        && parts[0] == "OK"
                        && parts[1] == "PLAYER_CONDITION"
                        && parts[2].parse::<usize>().ok() == Some(player_id)
                    {
                        let actual_stamina = parts[3].to_string();
                        let actual_condition = parts[4].to_string();
                        let entry = &mut self.team_condition_entries[index];
                        entry.stamina = actual_stamina.clone();
                        entry.condition = actual_condition.clone();
                        entry.original_stamina = actual_stamina;
                        entry.original_condition = actual_condition;
                        entry.write_status = self.localization.tr("team_condition.applied");
                        applied += 1;
                    } else {
                        self.team_condition_entries[index].write_status =
                            format!("Unexpected response: {response}");
                        failed += 1;
                    }
                }
                Err(error) => {
                    self.team_condition_entries[index].write_status = error;
                    failed += 1;
                }
            }
        }

        let applied_text = applied.to_string();
        let failed_text = failed.to_string();
        self.status = self.localization.tr_with(
            "team_condition.apply_summary",
            &[("applied", applied_text.as_str()), ("failed", failed_text.as_str())],
        );
    }

    fn refresh_selected_staff(&mut self) {
        let Some(id) = self.selected_staff_id else {
            self.selected_staff = None;
            return;
        };

        match self.game_request(&format!("GET_STAFF|{id}")) {
            Ok(response) => match parse_staff_response(&response) {
                Ok(staff) => {
                    self.connected = true;
                    self.status = format!("Staff data loaded: {}", staff.name);

                    let selected_region = if staff
                        .communication
                        .iter()
                        .any(|entry| entry.region_id == self.staff_communication_region_id)
                    {
                        self.staff_communication_region_id
                    } else {
                        staff
                            .communication
                            .first()
                            .map(|entry| entry.region_id)
                            .unwrap_or(0)
                    };
                    self.staff_communication_region_id = selected_region;
                    self.staff_communication_value =
                        staff_communication_value_for_region(&staff, selected_region);
                    self.selected_staff = Some(staff);
                }
                Err(error) => self.status = human_error(&error),
            },
            Err(error) => {
                self.connected = false;
                self.status = error;
            }
        }
    }

    fn apply_staff_name(&mut self) {
        let Some(staff) = self.selected_staff.as_ref() else {
            self.status = "Select a staff member first".to_string();
            return;
        };

        let name = match validate_editor_name(&staff.name) {
            Ok(name) => name,
            Err(error) => {
                self.status = error;
                return;
            }
        };
        let staff_id = staff.id;
        let command = format!("SET_STAFF_NAME|{staff_id}|{}", hex_encode(&name));

        match self.game_request(&command) {
            Ok(response) if response == "OK|STAFF_NAME" => {
                if let Some(staff) = self.selected_staff.as_mut() {
                    staff.name = name.clone();
                }
                if let Some(summary) = self.staffs.iter_mut().find(|staff| staff.id == staff_id) {
                    summary.name = name.clone();
                }
                self.connected = true;
                self.update_staff_search_status();
                self.status = format!("Staff name updated: {name}");
            }
            Ok(response) => {
                if let Some(error) = response.strip_prefix("ERR|") {
                    self.status = human_error(error);
                } else {
                    self.status = format!("Unexpected staff name response: {response}");
                }
            }
            Err(error) => {
                self.connected = false;
                self.status = error;
            }
        }
    }

    fn apply_staff_attributes(&mut self) {
        let Some(staff) = self.selected_staff.as_ref() else {
            self.status = "Select a staff member first".to_string();
            return;
        };

        let staff_id = staff.id;
        let staff_name = staff.name.clone();
        let values = match collect_staff_stat_values(staff) {
            Ok(values) => values,
            Err(error) => {
                self.status = error;
                return;
            }
        };

        let command = format!(
            "SET_STAFF_STATS|{}|{}",
            staff_id,
            values.join("|")
        );

        match self.game_request(&command) {
            Ok(response) if response == "OK|STAFF_STATS" => {
                self.connected = true;
                self.refresh_selected_staff();
                self.status = format!("Staff attributes updated: {staff_name}");
            }
            Ok(response) => {
                if let Some(error) = response.strip_prefix("ERR|") {
                    self.status = human_error(error);
                } else {
                    self.status = format!("Unexpected staff update response: {response}");
                }
            }
            Err(error) => {
                self.connected = false;
                self.status = error;
            }
        }
    }


    fn apply_staff_salary(&mut self) {
        let Some(staff) = self.selected_staff.as_ref() else {
            self.status = "Select a staff member first".to_string();
            return;
        };

        if staff.annual_salary.trim().is_empty() {
            self.status = "Free-agent staff do not have an active salary to edit".to_string();
            return;
        }

        let Ok(annual_salary) = parse_display_to_internal(&staff.annual_salary) else {
            self.status = "Salary must contain a valid number".to_string();
            return;
        };
        if annual_salary < 0.0 {
            self.status = "Salary cannot be negative".to_string();
            return;
        }

        let staff_id = staff.id;
        let staff_name = staff.name.clone();
        let command = format!(
            "SET_STAFF_SALARY|{staff_id}|{}",
            format_internal_for_command(annual_salary),
        );

        match self.game_request(&command) {
            Ok(response) if response == "OK|STAFF_SALARY" => {
                self.connected = true;
                self.refresh_selected_staff();
                self.status = format!("Staff salary updated: {staff_name}");
            }
            Ok(response) => {
                if let Some(error) = response.strip_prefix("ERR|") {
                    self.status = human_error(error);
                } else {
                    self.status = format!("Unexpected staff salary response: {response}");
                }
            }
            Err(error) => {
                self.connected = false;
                self.status = error;
            }
        }
    }

    fn fill_free_agent_staff_contract_defaults(&mut self, team_id: usize) -> bool {
        match self.game_request(&format!("GET_CONTRACT_DEFAULTS|STAFF|{team_id}")) {
            Ok(response) => match parse_contract_defaults_response(&response) {
                Ok((start_date, end_date, annual_salary)) => {
                    self.staff_contract_form = ContractEditorForm {
                        team_id: Some(team_id),
                        start_date,
                        end_date,
                        annual_salary,
                        transfer_fee: "$0".to_string(),
                        league_rank: "1".to_string(),
                        pog_bonus: "$0".to_string(),
                        league_bonus: "$0".to_string(),
                        match_bonus: "$0".to_string(),
                        win_bonus: "$0".to_string(),
                        ..ContractEditorForm::default()
                    };
                    true
                }
                Err(error) => {
                    self.status = human_error(&error);
                    false
                }
            },
            Err(error) => {
                self.connected = false;
                self.status = error;
                false
            }
        }
    }

    fn reset_staff_contract_form(&mut self) {
        if self.staff_contract_mode == ContractEditorMode::MoveFreeAgent {
            let Some(team_id) = self.staff_contract_form.team_id.or(self.recruitment_team_id) else {
                self.status = "Select a destination team first".to_string();
                return;
            };
            self.fill_free_agent_staff_contract_defaults(team_id);
            return;
        }

        let Some(staff) = self.selected_staff.as_ref() else {
            self.status = "Select a staff member first".to_string();
            return;
        };

        let fallback_team = self
            .teams
            .iter()
            .find(|team| team.is_player_team)
            .or_else(|| self.teams.first())
            .map(|team| team.id);

        self.staff_contract_form = ContractEditorForm {
            team_id: staff.contract_team_id.or(fallback_team),
            start_date: display_contract_date(&staff.contract_start_date),
            end_date: display_contract_date(&staff.contract_end_date),
            annual_salary: if staff.annual_salary.trim().is_empty() {
                "$0".to_string()
            } else {
                staff.annual_salary.clone()
            },
            transfer_fee: "$0".to_string(),
            ..ContractEditorForm::default()
        };
    }

    fn open_staff_contract_editor(&mut self) {
        if self.selected_staff.is_none() {
            self.status = "Select a staff member first".to_string();
            return;
        }
        self.staff_contract_mode = ContractEditorMode::EditActive;
        self.reset_staff_contract_form();
        self.staff_contract_window_open = true;
    }

    fn apply_staff_contract(&mut self) {
        let Some(staff) = self.selected_staff.as_ref() else {
            self.status = "Select a staff member first".to_string();
            return;
        };
        let Some(team_id) = self.staff_contract_form.team_id else {
            self.status = "Select a destination team for the contract".to_string();
            return;
        };
        let start_date = self.staff_contract_form.start_date.trim().to_string();
        let end_date = self.staff_contract_form.end_date.trim().to_string();
        if !is_iso_date_shape(&start_date) || !is_iso_date_shape(&end_date) {
            self.status = "Contract dates must use YYYY-MM-DD".to_string();
            return;
        }
        if end_date < start_date {
            self.status = "Contract end date cannot be before the start date".to_string();
            return;
        }
        let Ok(annual_salary) = parse_display_to_internal(&self.staff_contract_form.annual_salary) else {
            self.status = "Salary must contain a valid number".to_string();
            return;
        };
        if annual_salary < 0.0 {
            self.status = "Salary cannot be negative".to_string();
            return;
        }

        let staff_id = staff.id;
        let staff_name = staff.name.clone();
        let previous_team_id = staff.contract_team_id;
        let editor_mode = self.staff_contract_mode;
        let command = format!(
            "SET_STAFF_CONTRACT|{staff_id}|{team_id}|{start_date}|{end_date}|{}",
            format_internal_for_command(annual_salary),
        );
        match self.game_request(&command) {
            Ok(response) => match parse_staff_response(&response) {
                Ok(updated) => {
                    self.connected = true;
                    self.selected_staff = Some(updated);
                    self.staff_contract_window_open = false;
                    self.staff_contract_mode = ContractEditorMode::EditActive;
                    self.refresh_staff();
                    self.status = if editor_mode == ContractEditorMode::MoveFreeAgent {
                        let team_name = self
                            .teams
                            .iter()
                            .find(|team| team.id == team_id)
                            .map(|team| team.display_name.clone())
                            .filter(|name| !name.trim().is_empty())
                            .unwrap_or_else(|| format!("Team {team_id}"));
                        format!("Applied contract and moved {staff_name} to {team_name}.")
                    } else {
                        match previous_team_id {
                            None => format!("Free-agent staff signed with active contract: {staff_name}"),
                            Some(previous) if previous != team_id => format!("Staff moved and active contract applied: {staff_name}"),
                            Some(_) => format!("Active staff contract updated: {staff_name}"),
                        }
                    };
                }
                Err(error) => self.status = human_error(&error),
            },
            Err(error) => {
                self.connected = false;
                self.status = error;
            }
        }
    }

    #[cfg(feature = "dev")]
    fn load_staff_contract_probe(&mut self) {
        let Some(staff) = self.selected_staff.as_ref() else {
            self.status = "Select a staff member first".to_string();
            return;
        };
        let staff_id = staff.id;
        let staff_name = staff.name.clone();
        let command = format!("GET_STAFF_CONTRACT_PROBE|{staff_id}");
        match self.game_request(&command) {
            Ok(response) => {
                let parts: Vec<&str> = response.split('|').collect();
                if parts.first() == Some(&"ERR") {
                    self.status = human_error(parts.get(1).copied().unwrap_or("UNKNOWN_ERROR"));
                    return;
                }
                if parts.len() != 3 || parts[0] != "OK" || parts[1] != "STAFF_CONTRACT_PROBE" {
                    self.status = format!("Unexpected contract probe response: {response}");
                    return;
                }
                match hex_decode(parts[2]) {
                    Ok(raw) => {
                        self.staff_contract_probe_raw = raw;
                        self.staff_contract_probe_open = true;
                        self.connected = true;
                        self.status = format!("Staff contract state captured: {staff_name}");
                    }
                    Err(error) => self.status = error,
                }
            }
            Err(error) => {
                self.connected = false;
                self.status = error;
            }
        }
    }

    #[cfg(feature = "dev")]
    fn load_player_contract_probe(&mut self) {
        let Some(player) = self.selected_player.as_ref() else {
            self.status = "Select a player first".to_string();
            return;
        };
        let athlete_id = player.id;
        let player_name = player.name.clone();
        let command = format!("GET_PLAYER_CONTRACT_PROBE|{athlete_id}");
        match self.game_request(&command) {
            Ok(response) => {
                let parts: Vec<&str> = response.split('|').collect();
                if parts.first() == Some(&"ERR") {
                    self.status = human_error(parts.get(1).copied().unwrap_or("UNKNOWN_ERROR"));
                    return;
                }
                if parts.len() != 3 || parts[0] != "OK" || parts[1] != "PLAYER_CONTRACT_PROBE" {
                    self.status = format!("Unexpected contract probe response: {response}");
                    return;
                }
                match hex_decode(parts[2]) {
                    Ok(raw) => {
                        self.player_contract_probe_raw = raw;
                        self.player_contract_probe_open = true;
                        self.connected = true;
                        self.status = format!("Player contract state captured: {player_name}");
                    }
                    Err(error) => self.status = error,
                }
            }
            Err(error) => {
                self.connected = false;
                self.status = error;
            }
        }
    }

    #[cfg(feature = "dev")]
    fn export_player_contract_probe(&mut self) {
        let Some(player) = self.selected_player.as_ref() else {
            self.status = "Select a player first".to_string();
            return;
        };
        let entity_label = format!("Player {} (ID {})", player.name, player.id);
        let file_name = format!(
            "player_contract_probe_{}_{}.txt",
            player.id,
            sanitize_probe_file_component(&player.name)
        );
        let Some(path) = rfd::FileDialog::new()
            .set_title("Export Player Contract Probe")
            .set_file_name(&file_name)
            .add_filter("Text File", &["txt"])
            .save_file()
        else {
            return;
        };
        let text = contract_probe_export_text(
            &entity_label,
            &self.player_contract_probe_before,
            &self.player_contract_probe_after_offer,
            &self.player_contract_probe_after_accepted,
            &self.player_contract_probe_raw,
        );
        match fs::write(&path, text) {
            Ok(()) => self.status = format!("Exported player contract probe to {}", path.display()),
            Err(error) => self.status = format!("Could not export player contract probe: {error}"),
        }
    }

    #[cfg(feature = "dev")]
    fn export_staff_contract_probe(&mut self) {
        let Some(staff) = self.selected_staff.as_ref() else {
            self.status = "Select a staff member first".to_string();
            return;
        };
        let entity_label = format!("Staff {} (ID {})", staff.name, staff.id);
        let file_name = format!(
            "staff_contract_probe_{}_{}.txt",
            staff.id,
            sanitize_probe_file_component(&staff.name)
        );
        let Some(path) = rfd::FileDialog::new()
            .set_title("Export Staff Contract Probe")
            .set_file_name(&file_name)
            .add_filter("Text File", &["txt"])
            .save_file()
        else {
            return;
        };
        let text = contract_probe_export_text(
            &entity_label,
            &self.staff_contract_probe_before,
            &self.staff_contract_probe_after_offer,
            &self.staff_contract_probe_after_accepted,
            &self.staff_contract_probe_raw,
        );
        match fs::write(&path, text) {
            Ok(()) => self.status = format!("Exported staff contract probe to {}", path.display()),
            Err(error) => self.status = format!("Could not export staff contract probe: {error}"),
        }
    }

    fn apply_staff_communication(&mut self) {
        let Some(staff) = self.selected_staff.as_ref() else {
            self.status = "Select a staff member first".to_string();
            return;
        };

        let region_id = self.staff_communication_region_id;
        let value = match normalize_staff_communication_value(
            region_id,
            &self.staff_communication_value,
        ) {
            Ok(value) => value,
            Err(error) => {
                self.status = error;
                return;
            }
        };

        let staff_id = staff.id;
        let staff_name = staff.name.clone();
        let command = format!(
            "SET_STAFF_COMMUNICATION|{staff_id}|{region_id}={value}"
        );

        match self.game_request(&command) {
            Ok(response) if response == "OK|STAFF_COMMUNICATION" => {
                self.connected = true;
                self.refresh_selected_staff();
                self.status = format!(
                    "Staff Communication updated: {staff_name} — {}",
                    staff_communication_region_label(region_id)
                );
            }
            Ok(response) => {
                if let Some(error) = response.strip_prefix("ERR|") {
                    self.status = human_error(error);
                } else {
                    self.status = format!("Unexpected Staff Communication response: {response}");
                }
            }
            Err(error) => {
                self.connected = false;
                self.status = error;
            }
        }
    }

    fn refresh_selected_player(&mut self) {
        let Some(id) = self.selected_player_id else {
            self.selected_player = None;
            self.player_positions = None;
            self.player_potential = None;
            self.player_communication = None;
            return;
        };

        match self.game_request(&format!("GET_PLAYER|{id}")) {
            Ok(response) => match parse_player_response(&response) {
                Ok(player) => {
                    self.connected = true;
                    self.status = format!("Player data loaded: {}", player.name);
                    self.player_positions = Some(PlayerPositionForm::from_player(&player));
                    self.player_potential = Some(PlayerPotentialForm::from_player(&player));
                    let communication = PlayerCommunicationForm::from_player(&player);
                    {
                        let primary_region = communication.primary_region;
                        let selected_region_is_valid = Some(self.player_communication_region_id)
                            != primary_region
                            && COMMUNICATION_REGIONS
                                .iter()
                                .any(|(region_id, _)| *region_id == self.player_communication_region_id);
                        if !selected_region_is_valid {
                            self.player_communication_region_id = communication
                                .entries
                                .iter()
                                .map(|(region_id, _)| *region_id)
                                .find(|region_id| Some(*region_id) != primary_region)
                                .unwrap_or_else(|| {
                                    first_editable_player_communication_region(primary_region)
                                });
                        }
                        self.player_communication_value = player_communication_value_for_region(
                            &communication,
                            self.player_communication_region_id,
                        );
                    }
                    self.player_communication = Some(communication);
                    self.selected_player = Some(player);
                }
                Err(error) => {
                    self.selected_player = None;
                    self.player_positions = None;
                    self.player_potential = None;
                    self.player_communication = None;
                    self.status = human_error(&error);
                }
            },
            Err(error) => {
                self.connected = false;
                self.selected_player = None;
                self.player_positions = None;
                self.player_potential = None;
                self.player_communication = None;
                self.status = error;
            }
        }
    }

    fn load_champion_mastery(&mut self) {
        let Some(athlete_id) = self.selected_player_id else {
            self.status = "Select a player first".to_string();
            return;
        };

        match self.game_request(&format!("GET_CHAMPION_MASTERY_PROBE|{athlete_id}")) {
            Ok(response) => {
                let parts = response.split('|').collect::<Vec<_>>();
                if parts.first() == Some(&"ERR") {
                    self.status =
                        human_error(parts.get(1).copied().unwrap_or("UNKNOWN_ERROR"));
                    return;
                }

                if parts.len() != 4
                    || parts[0] != "OK"
                    || parts[1] != "MASTERY_PROBE"
                {
                    self.status =
                        format!("Unexpected Champion Mastery response: {response}");
                    return;
                }

                let mastery_state = match hex_decode(parts[2]) {
                    Ok(value) => value,
                    Err(error) => {
                        self.status = error;
                        return;
                    }
                };

                let available_pool = match hex_decode(parts[3]) {
                    Ok(value) => value,
                    Err(error) => {
                        self.status = error;
                        return;
                    }
                };

                let pool = parse_available_champions(&available_pool);
                let proficiency = parse_champion_proficiency(&mastery_state);

                if pool.is_empty() {
                    self.status =
                        "Champion Mastery: active champion pool could not be parsed"
                            .to_string();
                    return;
                }

                let active_ids = pool;
                let mut entries = Vec::new();

                // Active save champions first, preserving the save's order.
                for id in &active_ids {
                    let (raw_value, _raw_floor) = proficiency
                        .iter()
                        .find(|(champion_id, _, _)| champion_id == id)
                        .map(|(_, value, floor)| (*value, *floor))
                        .unwrap_or((0, None));

                    entries.push(ChampionMasteryEntry {
                        display_name: champion_display_name(id),
                        id: id.clone(),
                        active: true,
                        selected: false,
                        raw_value,
                        #[cfg(feature = "dev")]
                        raw_floor: _raw_floor,
                        edit_mastery: ((raw_value as f32 / 10.0).round() as i32)
                            .clamp(0, 100) as u16,
                    });
                }

                // All remaining proficiency entries stay editable even if the
                // champion has not yet entered this save through an in-game patch.
                let mut inactive = proficiency
                    .iter()
                    .filter(|(id, _, _)| !active_ids.iter().any(|active| active == id))
                    .map(|(id, raw_value, _raw_floor)| ChampionMasteryEntry {
                        display_name: champion_display_name(id),
                        id: id.clone(),
                        active: false,
                        selected: false,
                        raw_value: *raw_value,
                        #[cfg(feature = "dev")]
                        raw_floor: *_raw_floor,
                        edit_mastery: ((*raw_value as f32 / 10.0).round() as i32)
                            .clamp(0, 100) as u16,
                    })
                    .collect::<Vec<_>>();

                inactive.sort_by_key(|entry| entry.display_name.to_lowercase());
                entries.extend(inactive);

                self.champion_mastery_entries = entries;
                self.champion_mastery_open = true;

                let active_count = self
                    .champion_mastery_entries
                    .iter()
                    .filter(|entry| entry.active)
                    .count();
                let inactive_count = self.champion_mastery_entries.len() - active_count;

                self.status = format!(
                    "Champion Mastery loaded: {active_count} active, {inactive_count} inactive"
                );
            }
            Err(error) => {
                self.connected = false;
                self.status = error;
            }
        }
    }

    fn apply_selected_champion_mastery(&mut self) {
        let Some(athlete_id) = self.selected_player_id else {
            self.status = "Select a player first".to_string();
            return;
        };

        let selected = self
            .champion_mastery_entries
            .iter()
            .filter(|entry| entry.selected)
            .map(|entry| format!("{}:{}", entry.id, entry.edit_mastery))
            .collect::<Vec<_>>();

        if selected.is_empty() {
            self.status = "Champion Mastery: no champions selected".to_string();
            return;
        }

        let command = format!(
            "SET_CHAMPION_MASTERY|{}|{}",
            athlete_id,
            selected.join(";")
        );

        match self.game_request(&command) {
            Ok(response) => {
                let parts = response.split('|').collect::<Vec<_>>();
                if parts.first() == Some(&"ERR") {
                    self.status =
                        human_error(parts.get(1).copied().unwrap_or("UNKNOWN_ERROR"));
                    return;
                }

                if parts.len() < 3
                    || parts[0] != "OK"
                    || parts[1] != "CHAMPION_MASTERY"
                {
                    self.status =
                        format!("Unexpected Champion Mastery response: {response}");
                    return;
                }

                let count = parts.get(2).copied().unwrap_or("0");
                self.status =
                    format!("Champion Mastery applied to {count} champion(s)");
                self.load_champion_mastery();
            }
            Err(error) => {
                self.connected = false;
                self.status = error;
            }
        }
    }

    fn render_champion_mastery_card(
        ui: &mut egui::Ui,
        champion: &mut ChampionMasteryEntry,
        active: bool,
    ) {
        let fill = if active {
            if ui.visuals().dark_mode {
                egui::Color32::from_gray(48)
            } else {
                egui::Color32::from_gray(220)
            }
        } else {
            ui.visuals().window_fill()
        };

        let stroke = if active {
            if ui.visuals().dark_mode {
                egui::Stroke::new(1.0_f32, egui::Color32::from_gray(82))
            } else {
                egui::Stroke::new(1.0_f32, egui::Color32::from_gray(170))
            }
        } else {
            ui.visuals().widgets.noninteractive.bg_stroke
        };

        #[cfg(feature = "dev")]
        let card_name = champion_mastery_card_display_name(&champion.display_name);

        let response = egui::Frame::group(ui.style())
            .fill(fill)
            .stroke(stroke)
            .show(ui, |ui| {
                #[cfg(feature = "dev")]
                {
                    ui.set_min_width(CHAMPION_MASTERY_CARD_INNER_WIDTH);
                    ui.set_max_width(CHAMPION_MASTERY_CARD_INNER_WIDTH);
                    ui.set_min_height(CHAMPION_MASTERY_CARD_INNER_HEIGHT);

                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        ui.checkbox(&mut champion.selected, "");
                        ui.add_sized(
                            [
                                CHAMPION_MASTERY_CARD_NAME_WIDTH,
                                CHAMPION_MASTERY_CARD_NAME_HEIGHT,
                            ],
                            egui::Label::new(egui::RichText::new(&card_name).strong())
                                .wrap(),
                        )
                        .on_hover_text(champion.display_name.clone());

                        let old = champion.edit_mastery;
                        let changed = ui
                            .add_sized(
                                [CHAMPION_MASTERY_CARD_VALUE_WIDTH, 24.0],
                                egui::DragValue::new(&mut champion.edit_mastery)
                                    .range(0..=100)
                                    .speed(1.0)
                                    .suffix(" / 100"),
                            )
                            .changed();

                        if changed && champion.edit_mastery != old {
                            champion.selected = true;
                        }
                    });
                }

                #[cfg(not(feature = "dev"))]
                {
                    // Preserve the validated Community v0.4.1 card geometry and behavior.
                    ui.set_min_width(125.0);
                    ui.set_max_width(145.0);
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut champion.selected, "");
                        ui.label(egui::RichText::new(&champion.display_name).strong());
                    });

                    let old = champion.edit_mastery;
                    let changed = ui
                        .add(
                            egui::DragValue::new(&mut champion.edit_mastery)
                                .range(0..=100)
                                .speed(1.0)
                                .suffix(" / 100"),
                        )
                        .changed();

                    if changed && champion.edit_mastery != old {
                        champion.selected = true;
                    }
                }
            })
            .response;

        let state = if active {
            "Active in this save"
        } else {
            "Inactive in this save"
        };

        #[cfg(feature = "dev")]
        {
            let floor_text = champion
                .raw_floor
                .map(|value| value.to_string())
                .unwrap_or_else(|| "—".to_string());

            response.on_hover_text(format!(
                "{}\n{state}\nID: {}\nCurrent mastery: {}\nPending mastery: {}\nRaw value: {}\nRaw floor: {}",
                champion.display_name,
                champion.id,
                champion.mastery_text(),
                champion.edit_mastery,
                champion.raw_value,
                floor_text,
            ));
        }

        #[cfg(not(feature = "dev"))]
        response.on_hover_text(format!(
            "{state}\nCurrent Mastery: {}\nPending Mastery: {}",
            champion.mastery_text(),
            champion.edit_mastery,
        ));
    }

    fn render_champion_mastery_contents(
        &mut self,
        ui: &mut egui::Ui,
    ) -> (bool, bool) {
        let mut refresh_requested = false;
        let mut apply_requested = false;

        let active_count = self
            .champion_mastery_entries
            .iter()
            .filter(|entry| entry.active)
            .count();
        let inactive_count = self.champion_mastery_entries.len() - active_count;
        let selected_count = self
            .champion_mastery_entries
            .iter()
            .filter(|entry| entry.selected)
            .count();

        ui.horizontal_wrapped(|ui| {
            ui.label(format!(
                "{active_count} active · {inactive_count} inactive · {selected_count} selected"
            ));
            ui.separator();

            if ui
                .button(self.localization.tr("champion_mastery.check_active"))
                .clicked()
            {
                for entry in &mut self.champion_mastery_entries {
                    entry.selected = entry.active;
                }
            }

            if ui
                .button(self.localization.tr("champion_mastery.check_inactive"))
                .clicked()
            {
                for entry in &mut self.champion_mastery_entries {
                    entry.selected = !entry.active;
                }
            }

            if ui
                .button(self.localization.tr("champion_mastery.check_all"))
                .clicked()
            {
                for entry in &mut self.champion_mastery_entries {
                    entry.selected = true;
                }
            }

            if ui
                .button(self.localization.tr("champion_mastery.clear_checks"))
                .clicked()
            {
                for entry in &mut self.champion_mastery_entries {
                    entry.selected = false;
                }
            }

            if ui.button(self.localization.tr("common.refresh")).clicked() {
                refresh_requested = true;
            }
        });

        ui.add_space(6.0);

        ui.horizontal_wrapped(|ui| {
            ui.label(self.localization.tr("champion_mastery.bulk"));
            ui.add(
                egui::Slider::new(&mut self.champion_mastery_bulk_value, 0..=100)
                    .show_value(true),
            );

            if ui
                .add_enabled(
                    selected_count > 0,
                    egui::Button::new(
                        self.localization.tr("champion_mastery.set_checked"),
                    ),
                )
                .clicked()
            {
                for entry in &mut self.champion_mastery_entries {
                    if entry.selected {
                        entry.edit_mastery = self.champion_mastery_bulk_value;
                    }
                }
            }

            ui.separator();

            if ui
                .add_enabled(
                    self.connected && selected_count > 0,
                    egui::Button::new(
                        self.localization.tr("champion_mastery.apply_selected"),
                    ),
                )
                .clicked()
            {
                apply_requested = true;
            }
        });

        ui.add_space(6.0);
        ui.weak(self.localization.tr(champion_mastery_help_key()));
        ui.add_space(8.0);
        ui.separator();

        // This is the embedded Champion Mastery window's own content width.
        // The scrollbar reserve keeps the final column fully inside the clip rect.
        let local_width = ui.available_width().max(180.0);

        #[cfg(feature = "dev")]
        let cards_per_row = champion_mastery_columns_for_width(local_width);

        #[cfg(not(feature = "dev"))]
        let cards_per_row = (local_width / 165.0_f32).floor().max(1.0) as usize;

        let mastery_scroll = egui::ScrollArea::vertical()
            .id_salt("champion_mastery_grid_scroll")
            .auto_shrink([false, false]);

        #[cfg(feature = "dev")]
        let mastery_scroll = mastery_scroll.max_height(ui.available_height().max(120.0));

        mastery_scroll.show(ui, |ui| {
                ui.heading(self.localization.tr("champion_mastery.active_heading"));
                ui.label(self.localization.tr("champion_mastery.active_info"));
                ui.add_space(4.0);

                let active_indices = self
                    .champion_mastery_entries
                    .iter()
                    .enumerate()
                    .filter_map(|(index, entry)| entry.active.then_some(index))
                    .collect::<Vec<_>>();

                #[cfg(feature = "dev")]
                egui::Grid::new("champion_mastery_active_fixed_grid")
                    .num_columns(cards_per_row)
                    .spacing([
                        CHAMPION_MASTERY_CARD_HORIZONTAL_GAP,
                        CHAMPION_MASTERY_CARD_VERTICAL_GAP,
                    ])
                    .show(ui, |ui| {
                        for (cell, &index) in active_indices.iter().enumerate() {
                            let champion = &mut self.champion_mastery_entries[index];
                            Self::render_champion_mastery_card(ui, champion, true);
                            if (cell + 1) % cards_per_row == 0 {
                                ui.end_row();
                            }
                        }
                        if !active_indices.is_empty()
                            && active_indices.len() % cards_per_row != 0
                        {
                            ui.end_row();
                        }
                    });

                #[cfg(not(feature = "dev"))]
                for row in active_indices.chunks(cards_per_row) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 10.0;
                        for &index in row {
                            let champion = &mut self.champion_mastery_entries[index];
                            Self::render_champion_mastery_card(ui, champion, true);
                        }
                    });
                    ui.add_space(10.0);
                }

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(8.0);

                ui.heading(self.localization.tr("champion_mastery.inactive_heading"));
                ui.label(self.localization.tr(champion_inactive_info_key()));
                ui.add_space(4.0);

                let inactive_indices = self
                    .champion_mastery_entries
                    .iter()
                    .enumerate()
                    .filter_map(|(index, entry)| (!entry.active).then_some(index))
                    .collect::<Vec<_>>();

                #[cfg(feature = "dev")]
                egui::Grid::new("champion_mastery_inactive_fixed_grid")
                    .num_columns(cards_per_row)
                    .spacing([
                        CHAMPION_MASTERY_CARD_HORIZONTAL_GAP,
                        CHAMPION_MASTERY_CARD_VERTICAL_GAP,
                    ])
                    .show(ui, |ui| {
                        for (cell, &index) in inactive_indices.iter().enumerate() {
                            let champion = &mut self.champion_mastery_entries[index];
                            Self::render_champion_mastery_card(ui, champion, false);
                            if (cell + 1) % cards_per_row == 0 {
                                ui.end_row();
                            }
                        }
                        if !inactive_indices.is_empty()
                            && inactive_indices.len() % cards_per_row != 0
                        {
                            ui.end_row();
                        }
                    });

                #[cfg(not(feature = "dev"))]
                for row in inactive_indices.chunks(cards_per_row) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 10.0;
                        for &index in row {
                            let champion = &mut self.champion_mastery_entries[index];
                            Self::render_champion_mastery_card(ui, champion, false);
                        }
                    });
                    ui.add_space(10.0);
                }
            });

        (refresh_requested, apply_requested)
    }

    fn render_champion_mastery_window(&mut self, ctx: &egui::Context) {
        if !self.champion_mastery_open {
            return;
        }

        let mut open = self.champion_mastery_open;
        let mut refresh_requested = false;
        let mut apply_requested = false;

        let player_name = self
            .selected_player
            .as_ref()
            .map(|player| player.name.clone())
            .unwrap_or_else(|| self.localization.tr("champion_mastery.selected_player"));

        let title = self.localization.tr_with(
            "champion_mastery.window_title",
            &[("player", player_name.as_str())],
        );

        let mastery_window = egui::Window::new(title)
            .id(egui::Id::new("champion_mastery_grid_v030"))
            .open(&mut open)
            .resizable(true)
            .default_size(egui::vec2(1080.0, 720.0));

        #[cfg(feature = "dev")]
        let mastery_window = mastery_window.min_width(380.0).min_height(320.0);

        mastery_window.show(ctx, |ui| {
            let (refresh, apply) = self.render_champion_mastery_contents(ui);
            refresh_requested |= refresh;
            apply_requested |= apply;
        });

        self.champion_mastery_open = open;

        if apply_requested {
            self.apply_selected_champion_mastery();
        } else if refresh_requested {
            self.load_champion_mastery();
        }
    }

    fn render_player_contract_window(&mut self, ctx: &egui::Context) {
        if !self.player_contract_window_open {
            return;
        }

        let mut open = self.player_contract_window_open;
        let mut apply_requested = false;
        let mut reset_requested = false;
        let mut cancel_requested = false;
        let target = self
            .selected_player
            .as_ref()
            .map(|player| format!("{} · ID {}", player.name, player.id))
            .unwrap_or_else(|| self.localization.tr("contract.no_player_selected"));
        let teams = self.teams.clone();

        egui::Window::new(self.localization.tr("contract.player.window_title"))
            .id(egui::Id::new("edit_player_contract_v039"))
            .open(&mut open)
            .resizable(true)
            .collapsible(false)
            .default_width(590.0)
            .show(ctx, |ui| {
                ui.strong(target);
                let active_team_id = self.selected_player.as_ref().and_then(|player| player.contract_team_id);
                if self.player_contract_mode == ContractEditorMode::MoveFreeAgent {
                    ui.weak(self.localization.tr("contract.mode.player.free_agent_move"));
                } else {
                    match (active_team_id, self.player_contract_form.team_id) {
                        (None, Some(_)) => ui.weak(self.localization.tr("contract.mode.player.free_agent_create")),
                        (Some(current), Some(selected)) if current != selected => ui.weak(self.localization.tr("contract.mode.player.move_existing")),
                        (Some(_), _) => ui.weak(self.localization.tr("contract.mode.player.edit_active")),
                        _ => ui.weak(self.localization.tr("contract.mode.player.select_team")),
                    };
                }
                ui.add_space(8.0);

                let selected_team = self
                    .player_contract_form
                    .team_id
                    .and_then(|id| teams.iter().find(|team| team.id == id))
                    .map(|team| team.localized_label(&self.localization))
                    .unwrap_or_else(|| self.localization.tr("common.select_team"));

                egui::Grid::new("edit_player_contract_main_grid")
                    .num_columns(2)
                    .spacing([22.0, 8.0])
                    .show(ui, |ui| {
                        ui.label(self.localization.tr("common.team"));
                        egui::ComboBox::from_id_salt("edit_player_contract_team")
                            .selected_text(selected_team)
                            .width(350.0)
                            .height(300.0)
                            .show_ui(ui, |ui| {
                                for team in &teams {
                                    ui.selectable_value(
                                        &mut self.player_contract_form.team_id,
                                        Some(team.id),
                                        team.localized_label(&self.localization),
                                    );
                                }
                            });
                        ui.end_row();

                        ui.label(self.localization.tr("contract.start"));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.player_contract_form.start_date)
                                .desired_width(180.0)
                                .hint_text("YYYY-MM-DD"),
                        );
                        ui.end_row();

                        ui.label(self.localization.tr("contract.end"));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.player_contract_form.end_date)
                                .desired_width(180.0)
                                .hint_text("YYYY-MM-DD"),
                        );
                        ui.end_row();

                        ui.label(self.localization.tr("contract.annual_salary"));
                        money_text_edit_with_preview(
                            ui,
                            &self.localization,
                            &mut self.player_contract_form.annual_salary,
                            180.0,
                            true,
                        );
                        ui.end_row();

                        ui.label(self.localization.tr("contract.transfer_fee"));
                        money_text_edit_with_preview(
                            ui,
                            &self.localization,
                            &mut self.player_contract_form.transfer_fee,
                            180.0,
                            true,
                        );
                        ui.end_row();

                        ui.label(self.localization.tr("contract.squad_status"));
                        egui::ComboBox::from_id_salt("edit_player_contract_squad_status")
                            .selected_text(self.localization.tr(self.player_contract_form.squad_status.label_key()))
                            .width(220.0)
                            .show_ui(ui, |ui| {
                                for status in SquadStatusChoice::ALL {
                                    ui.selectable_value(
                                        &mut self.player_contract_form.squad_status,
                                        status,
                                        self.localization.tr(status.label_key()),
                                    );
                                }
                            });
                        ui.end_row();
                    });

                ui.add_space(10.0);
                ui.separator();
                ui.strong(self.localization.tr("contract.active_bonuses"));
                ui.add_space(5.0);

                egui::Grid::new("edit_player_contract_bonus_grid")
                    .num_columns(3)
                    .spacing([14.0, 7.0])
                    .show(ui, |ui| {
                        ui.checkbox(&mut self.player_contract_form.pog_enabled, self.localization.tr("contract.pog_bonus"));
                        let pog_enabled = self.player_contract_form.pog_enabled;
                        money_text_edit_with_preview(
                            ui,
                            &self.localization,
                            &mut self.player_contract_form.pog_bonus,
                            150.0,
                            pog_enabled,
                        );
                        ui.weak(self.localization.tr("contract.amount"));
                        ui.end_row();

                        ui.checkbox(&mut self.player_contract_form.league_enabled, self.localization.tr("contract.league_rank_bonus"));
                        let league_enabled = self.player_contract_form.league_enabled;
                        money_text_edit_with_preview(
                            ui,
                            &self.localization,
                            &mut self.player_contract_form.league_bonus,
                            150.0,
                            league_enabled,
                        );
                        ui.horizontal(|ui| {
                            ui.weak(self.localization.tr("contract.rank"));
                            ui.add_enabled_ui(self.player_contract_form.league_enabled, |ui| {
                                egui::ComboBox::from_id_salt("edit_player_contract_league_rank")
                                    .selected_text(format!("Rank {}", self.player_contract_form.league_rank))
                                    .width(100.0)
                                    .show_ui(ui, |ui| {
                                        for rank in 1..=10 {
                                            ui.selectable_value(
                                                &mut self.player_contract_form.league_rank,
                                                rank.to_string(),
                                                format!("Rank {rank}"),
                                            );
                                        }
                                    });
                            });
                        });
                        ui.end_row();

                        ui.checkbox(&mut self.player_contract_form.match_enabled, self.localization.tr("contract.match_appearance_bonus"));
                        let match_enabled = self.player_contract_form.match_enabled;
                        money_text_edit_with_preview(
                            ui,
                            &self.localization,
                            &mut self.player_contract_form.match_bonus,
                            150.0,
                            match_enabled,
                        );
                        ui.weak(self.localization.tr("contract.amount"));
                        ui.end_row();

                        ui.checkbox(&mut self.player_contract_form.win_enabled, self.localization.tr("contract.match_win_bonus"));
                        let win_enabled = self.player_contract_form.win_enabled;
                        money_text_edit_with_preview(
                            ui,
                            &self.localization,
                            &mut self.player_contract_form.win_bonus,
                            150.0,
                            win_enabled,
                        );
                        ui.weak(self.localization.tr("contract.amount"));
                        ui.end_row();
                    });

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    let apply_label = if self.player_contract_mode == ContractEditorMode::MoveFreeAgent {
                        self.localization.tr("contract.apply_move_player")
                    } else {
                        self.localization.tr("contract.apply")
                    };
                    if ui
                        .add_enabled(self.connected, egui::Button::new(apply_label))
                        .clicked()
                    {
                        apply_requested = true;
                    }
                    let reset_help = if self.player_contract_mode == ContractEditorMode::MoveFreeAgent {
                        self.localization.tr("contract.reset_free_agent_help")
                    } else {
                        self.localization.tr("contract.reset_live_help")
                    };
                    if ui.button(self.localization.tr("common.reset")).on_hover_text(reset_help).clicked() {
                        reset_requested = true;
                    }
                    #[cfg(feature = "dev")]
                    if ui
                        .add_enabled(self.connected, egui::Button::new("Capture Contract Flow"))
                        .clicked()
                    {
                        self.load_player_contract_probe();
                    }
                    if ui.button(self.localization.tr("common.cancel")).clicked() {
                        cancel_requested = true;
                    }
                });
            });

        if cancel_requested {
            open = false;
        }
        if !open {
            self.player_contract_mode = ContractEditorMode::EditActive;
        }
        self.player_contract_window_open = open;
        if reset_requested {
            self.reset_player_contract_form();
        }
        if apply_requested {
            self.apply_player_contract();
        }
    }

    fn render_staff_contract_window(&mut self, ctx: &egui::Context) {
        if !self.staff_contract_window_open {
            return;
        }

        let mut open = self.staff_contract_window_open;
        let mut apply_requested = false;
        let mut reset_requested = false;
        let mut cancel_requested = false;
        let target = self
            .selected_staff
            .as_ref()
            .map(|staff| {
                format!(
                    "{} · {} · ID {}",
                    staff.name,
                    localized_staff_role(&self.localization, &staff.role),
                    staff.id,
                )
            })
            .unwrap_or_else(|| self.localization.tr("contract.no_staff_selected"));
        let teams = self.teams.clone();

        egui::Window::new(self.localization.tr("contract.staff.window_title"))
            .id(egui::Id::new("edit_staff_contract_v039"))
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .default_width(520.0)
            .show(ctx, |ui| {
                ui.strong(target);
                let active_team_id = self.selected_staff.as_ref().and_then(|staff| staff.contract_team_id);
                if self.staff_contract_mode == ContractEditorMode::MoveFreeAgent {
                    ui.weak(self.localization.tr("contract.mode.staff.free_agent_move"));
                } else {
                    match (active_team_id, self.staff_contract_form.team_id) {
                        (None, Some(_)) => ui.weak(self.localization.tr("contract.mode.staff.free_agent_create")),
                        (Some(current), Some(selected)) if current != selected => ui.weak(self.localization.tr("contract.mode.staff.move_existing")),
                        (Some(_), _) => ui.weak(self.localization.tr("contract.mode.staff.edit_active")),
                        _ => ui.weak(self.localization.tr("contract.mode.staff.select_team")),
                    };
                }
                ui.add_space(8.0);

                let selected_team = self
                    .staff_contract_form
                    .team_id
                    .and_then(|id| teams.iter().find(|team| team.id == id))
                    .map(|team| team.localized_label(&self.localization))
                    .unwrap_or_else(|| self.localization.tr("common.select_team"));

                egui::Grid::new("staff_contract_builder_grid")
                    .num_columns(2)
                    .spacing([22.0, 8.0])
                    .show(ui, |ui| {
                        ui.label(self.localization.tr("common.team"));
                        egui::ComboBox::from_id_salt("staff_contract_team")
                            .selected_text(selected_team)
                            .width(330.0)
                            .height(300.0)
                            .show_ui(ui, |ui| {
                                for team in &teams {
                                    ui.selectable_value(
                                        &mut self.staff_contract_form.team_id,
                                        Some(team.id),
                                        team.localized_label(&self.localization),
                                    );
                                }
                            });
                        ui.end_row();

                        ui.label(self.localization.tr("contract.start_date"));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.staff_contract_form.start_date)
                                .desired_width(180.0)
                                .hint_text("YYYY-MM-DD"),
                        );
                        ui.end_row();

                        ui.label(self.localization.tr("contract.end_date"));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.staff_contract_form.end_date)
                                .desired_width(180.0)
                                .hint_text("YYYY-MM-DD"),
                        );
                        ui.end_row();

                        ui.label(self.localization.tr("contract.annual_salary"));
                        money_text_edit_with_preview(
                            ui,
                            &self.localization,
                            &mut self.staff_contract_form.annual_salary,
                            180.0,
                            true,
                        );
                        ui.end_row();
                    });

                ui.add_space(8.0);
                ui.weak(self.localization.tr("contract.staff.direct_info"));
                ui.horizontal(|ui| {
                    let apply_label = if self.staff_contract_mode == ContractEditorMode::MoveFreeAgent {
                        self.localization.tr("contract.apply_move_staff")
                    } else {
                        self.localization.tr("contract.apply")
                    };
                    if ui
                        .add_enabled(self.connected, egui::Button::new(apply_label))
                        .clicked()
                    {
                        apply_requested = true;
                    }
                    let reset_help = if self.staff_contract_mode == ContractEditorMode::MoveFreeAgent {
                        self.localization.tr("contract.reset_free_agent_help")
                    } else {
                        self.localization.tr("contract.reset_live_help")
                    };
                    if ui.button(self.localization.tr("common.reset")).on_hover_text(reset_help).clicked() {
                        reset_requested = true;
                    }
                    #[cfg(feature = "dev")]
                    if ui
                        .add_enabled(self.connected, egui::Button::new("Capture Contract Flow"))
                        .clicked()
                    {
                        self.load_staff_contract_probe();
                    }
                    if ui.button(self.localization.tr("common.cancel")).clicked() {
                        cancel_requested = true;
                    }
                });
            });

        if cancel_requested {
            open = false;
        }
        if !open {
            self.staff_contract_mode = ContractEditorMode::EditActive;
        }
        self.staff_contract_window_open = open;
        if reset_requested {
            self.reset_staff_contract_form();
        }
        if apply_requested {
            self.apply_staff_contract();
        }
    }


    #[cfg(feature = "dev")]
    fn render_player_contract_probe_window(&mut self, ctx: &egui::Context) {
        if !self.player_contract_probe_open {
            return;
        }
        let mut open = self.player_contract_probe_open;
        let mut refresh_requested = false;
        egui::Window::new("Player Contract Flow Probe")
            .id(egui::Id::new("player_contract_flow_probe_v038d"))
            .open(&mut open)
            .resizable(true)
            .default_size([1000.0, 780.0])
            .show(ctx, |ui| {
                ui.strong("Read-only contract discovery with saved phases and automatic comparison");
                ui.weak("Capture the current state, save it as BEFORE, repeat after Make Offer, and repeat after acceptance. The editor stores all three phases and produces a compact line-by-line diff, so you do not need to copy or compare the full raw dump manually.");

                ui.horizontal(|ui| {
                    if ui.button("Refresh Current State").clicked() {
                        refresh_requested = true;
                    }
                    if ui
                        .add_enabled(
                            !self.player_contract_probe_raw.is_empty(),
                            egui::Button::new("Save Current as BEFORE"),
                        )
                        .clicked()
                    {
                        self.player_contract_probe_before = self.player_contract_probe_raw.clone();
                        self.status = "Saved player contract snapshot: BEFORE".to_string();
                    }
                    if ui
                        .add_enabled(
                            !self.player_contract_probe_raw.is_empty(),
                            egui::Button::new("Save Current as AFTER OFFER"),
                        )
                        .clicked()
                    {
                        self.player_contract_probe_after_offer =
                            self.player_contract_probe_raw.clone();
                        self.status = "Saved player contract snapshot: AFTER OFFER".to_string();
                    }
                    if ui
                        .add_enabled(
                            !self.player_contract_probe_raw.is_empty(),
                            egui::Button::new("Save Current as AFTER ACCEPTED"),
                        )
                        .clicked()
                    {
                        self.player_contract_probe_after_accepted =
                            self.player_contract_probe_raw.clone();
                        self.status = "Saved player contract snapshot: AFTER ACCEPTED".to_string();
                    }
                });

                ui.horizontal(|ui| {
                    ui.label(probe_snapshot_summary(
                        "BEFORE",
                        &self.player_contract_probe_before,
                    ));
                    ui.separator();
                    ui.label(probe_snapshot_summary(
                        "AFTER OFFER",
                        &self.player_contract_probe_after_offer,
                    ));
                    ui.separator();
                    ui.label(probe_snapshot_summary(
                        "AFTER ACCEPTED",
                        &self.player_contract_probe_after_accepted,
                    ));
                });

                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            !self.player_contract_probe_before.is_empty()
                                && !self.player_contract_probe_after_offer.is_empty(),
                            egui::Button::new("Compare BEFORE → AFTER OFFER"),
                        )
                        .clicked()
                    {
                        self.player_contract_probe_comparison = contract_probe_diff(
                            "BEFORE",
                            &self.player_contract_probe_before,
                            "AFTER OFFER",
                            &self.player_contract_probe_after_offer,
                        );
                    }
                    if ui
                        .add_enabled(
                            !self.player_contract_probe_after_offer.is_empty()
                                && !self.player_contract_probe_after_accepted.is_empty(),
                            egui::Button::new("Compare AFTER OFFER → AFTER ACCEPTED"),
                        )
                        .clicked()
                    {
                        self.player_contract_probe_comparison = contract_probe_diff(
                            "AFTER OFFER",
                            &self.player_contract_probe_after_offer,
                            "AFTER ACCEPTED",
                            &self.player_contract_probe_after_accepted,
                        );
                    }
                    if ui.button("Export All Captures").clicked() {
                        self.export_player_contract_probe();
                    }
                    if ui.button(self.localization.tr("player_editor.positions.clear_all")).clicked() {
                        self.player_contract_probe_raw.clear();
                        self.player_contract_probe_before.clear();
                        self.player_contract_probe_after_offer.clear();
                        self.player_contract_probe_after_accepted.clear();
                        self.player_contract_probe_comparison.clear();
                    }
                });

                ui.separator();
                if !self.player_contract_probe_comparison.is_empty() {
                    ui.strong("Automatic comparison");
                    ui.add(
                        egui::TextEdit::multiline(
                            &mut self.player_contract_probe_comparison,
                        )
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .desired_rows(22)
                        .interactive(false),
                    );
                    ui.separator();
                }

                egui::CollapsingHeader::new("Current raw capture")
                    .default_open(self.player_contract_probe_comparison.is_empty())
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.player_contract_probe_raw)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(f32::INFINITY)
                                .desired_rows(28)
                                .interactive(false),
                        );
                    });
            });
        self.player_contract_probe_open = open;
        if refresh_requested {
            self.load_player_contract_probe();
        }
    }

    #[cfg(feature = "dev")]
    fn render_staff_contract_probe_window(&mut self, ctx: &egui::Context) {
        if !self.staff_contract_probe_open {
            return;
        }
        let mut open = self.staff_contract_probe_open;
        let mut refresh_requested = false;
        egui::Window::new("Staff Contract Flow Probe")
            .id(egui::Id::new("staff_contract_flow_probe_v038d"))
            .open(&mut open)
            .resizable(true)
            .default_size([1000.0, 780.0])
            .show(ctx, |ui| {
                ui.strong("Read-only staff contract discovery with saved phases and automatic comparison");
                ui.weak("Capture the current state, save it as BEFORE, repeat after Make Offer, and repeat after acceptance. The editor stores all three phases and produces a compact line-by-line diff.");

                ui.horizontal(|ui| {
                    if ui.button("Refresh Current State").clicked() {
                        refresh_requested = true;
                    }
                    if ui
                        .add_enabled(
                            !self.staff_contract_probe_raw.is_empty(),
                            egui::Button::new("Save Current as BEFORE"),
                        )
                        .clicked()
                    {
                        self.staff_contract_probe_before = self.staff_contract_probe_raw.clone();
                        self.status = "Saved staff contract snapshot: BEFORE".to_string();
                    }
                    if ui
                        .add_enabled(
                            !self.staff_contract_probe_raw.is_empty(),
                            egui::Button::new("Save Current as AFTER OFFER"),
                        )
                        .clicked()
                    {
                        self.staff_contract_probe_after_offer =
                            self.staff_contract_probe_raw.clone();
                        self.status = "Saved staff contract snapshot: AFTER OFFER".to_string();
                    }
                    if ui
                        .add_enabled(
                            !self.staff_contract_probe_raw.is_empty(),
                            egui::Button::new("Save Current as AFTER ACCEPTED"),
                        )
                        .clicked()
                    {
                        self.staff_contract_probe_after_accepted =
                            self.staff_contract_probe_raw.clone();
                        self.status = "Saved staff contract snapshot: AFTER ACCEPTED".to_string();
                    }
                });

                ui.horizontal(|ui| {
                    ui.label(probe_snapshot_summary(
                        "BEFORE",
                        &self.staff_contract_probe_before,
                    ));
                    ui.separator();
                    ui.label(probe_snapshot_summary(
                        "AFTER OFFER",
                        &self.staff_contract_probe_after_offer,
                    ));
                    ui.separator();
                    ui.label(probe_snapshot_summary(
                        "AFTER ACCEPTED",
                        &self.staff_contract_probe_after_accepted,
                    ));
                });

                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            !self.staff_contract_probe_before.is_empty()
                                && !self.staff_contract_probe_after_offer.is_empty(),
                            egui::Button::new("Compare BEFORE → AFTER OFFER"),
                        )
                        .clicked()
                    {
                        self.staff_contract_probe_comparison = contract_probe_diff(
                            "BEFORE",
                            &self.staff_contract_probe_before,
                            "AFTER OFFER",
                            &self.staff_contract_probe_after_offer,
                        );
                    }
                    if ui
                        .add_enabled(
                            !self.staff_contract_probe_after_offer.is_empty()
                                && !self.staff_contract_probe_after_accepted.is_empty(),
                            egui::Button::new("Compare AFTER OFFER → AFTER ACCEPTED"),
                        )
                        .clicked()
                    {
                        self.staff_contract_probe_comparison = contract_probe_diff(
                            "AFTER OFFER",
                            &self.staff_contract_probe_after_offer,
                            "AFTER ACCEPTED",
                            &self.staff_contract_probe_after_accepted,
                        );
                    }
                    if ui.button("Export All Captures").clicked() {
                        self.export_staff_contract_probe();
                    }
                    if ui.button(self.localization.tr("player_editor.positions.clear_all")).clicked() {
                        self.staff_contract_probe_raw.clear();
                        self.staff_contract_probe_before.clear();
                        self.staff_contract_probe_after_offer.clear();
                        self.staff_contract_probe_after_accepted.clear();
                        self.staff_contract_probe_comparison.clear();
                    }
                });

                ui.separator();
                if !self.staff_contract_probe_comparison.is_empty() {
                    ui.strong("Automatic comparison");
                    ui.add(
                        egui::TextEdit::multiline(
                            &mut self.staff_contract_probe_comparison,
                        )
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .desired_rows(22)
                        .interactive(false),
                    );
                    ui.separator();
                }

                egui::CollapsingHeader::new("Current raw capture")
                    .default_open(self.staff_contract_probe_comparison.is_empty())
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.staff_contract_probe_raw)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(f32::INFINITY)
                                .desired_rows(28)
                                .interactive(false),
                        );
                    });
            });
        self.staff_contract_probe_open = open;
        if refresh_requested {
            self.load_staff_contract_probe();
        }
    }

    fn apply_player_name(&mut self) {
        let Some(player) = self.selected_player.as_ref() else {
            self.status = "Select a player first".to_string();
            return;
        };

        let name = match validate_editor_name(&player.name) {
            Ok(name) => name,
            Err(error) => {
                self.status = error;
                return;
            }
        };
        let athlete_id = player.id;
        let command = format!("SET_PLAYER_NAME|{athlete_id}|{}", hex_encode(&name));

        match self.game_request(&command) {
            Ok(response) if response == "OK|PLAYER_NAME" => {
                if let Some(player) = self.selected_player.as_mut() {
                    player.name = name.clone();
                }
                if let Some(summary) = self.players.iter_mut().find(|player| player.id == athlete_id) {
                    summary.name = name.clone();
                }
                self.connected = true;
                self.update_player_search_status();
                self.status = format!("Player name updated: {name}");
            }
            Ok(response) => {
                if let Some(error) = response.strip_prefix("ERR|") {
                    self.status = human_error(error);
                } else {
                    self.status = format!("Unexpected player name response: {response}");
                }
            }
            Err(error) => {
                self.connected = false;
                self.status = error;
            }
        }
    }

    fn apply_selected_player(&mut self) {
        let Some(player) = self.selected_player.as_ref() else {
            self.status = "Select a player first".to_string();
            return;
        };

        let values = match collect_player_stat_values(player) {
            Ok(values) => values,
            Err(error) => {
                self.status = error;
                return;
            }
        };

        let command = format!(
            "SET_PLAYER_STATS|{}|{}",
            player.id,
            values.join("|")
        );

        match self.game_request(&command) {
            Ok(response) => match parse_player_response(&response) {
                Ok(updated) => {
                    self.connected = true;
                    self.status = format!("Attributes updated: {}", updated.name);
                    self.player_positions = Some(PlayerPositionForm::from_player(&updated));
                    self.player_potential = Some(PlayerPotentialForm::from_player(&updated));
                    self.player_communication = Some(PlayerCommunicationForm::from_player(&updated));
                    self.selected_player = Some(updated);
                }
                Err(error) => self.status = human_error(&error),
            },
            Err(error) => {
                self.connected = false;
                self.status = error;
            }
        }
    }

    fn apply_player_positions(&mut self) {
        let Some(player) = self.selected_player.as_ref() else {
            self.status = "Select a player first".to_string();
            return;
        };
        let Some(positions) = self.player_positions.as_ref() else {
            self.status = "Positions are not loaded".to_string();
            return;
        };

        let values = positions.values_for_apply();
        if values.into_iter().filter(|value| *value > 0).count() > 3 {
            self.status = "TFM2 supports at most three active positions".to_string();
            return;
        }

        let command = format!(
            "SET_PLAYER_POSITIONS|{}|{}|{}|{}|{}|{}",
            player.id, values[0], values[1], values[2], values[3], values[4]
        );

        match self.game_request(&command) {
            Ok(response) => match parse_player_response(&response) {
                Ok(updated) => {
                    self.connected = true;
                    self.player_positions = Some(PlayerPositionForm::from_player(&updated));
                    self.player_potential = Some(PlayerPotentialForm::from_player(&updated));
                    self.player_communication = Some(PlayerCommunicationForm::from_player(&updated));
                    self.status = format!("Positions updated: {}", updated.name);
                    self.selected_player = Some(updated);
                }
                Err(error) => self.status = human_error(&error),
            },
            Err(error) => {
                self.connected = false;
                self.status = error;
            }
        }
    }

    fn apply_player_potential(&mut self) {
        let Some(player) = self.selected_player.as_ref() else {
            self.status = "Select a player first".to_string();
            return;
        };
        let Some(potential) = self.player_potential.as_ref() else {
            self.status = "Potential is not loaded".to_string();
            return;
        };

        let command = format!(
            "SET_PLAYER_POTENTIAL|{}|{}",
            player.id,
            potential.edit_raw
        );

        match self.game_request(&command) {
            Ok(response) => match parse_player_response(&response) {
                Ok(updated) => {
                    self.connected = true;
                    self.player_positions = Some(PlayerPositionForm::from_player(&updated));
                    self.player_potential = Some(PlayerPotentialForm::from_player(&updated));
                    self.player_communication = Some(PlayerCommunicationForm::from_player(&updated));
                    self.status = format!("Potential updated: {}", updated.name);
                    self.selected_player = Some(updated);
                }
                Err(error) => self.status = human_error(&error),
            },
            Err(error) => {
                self.connected = false;
                self.status = error;
            }
        }
    }

    fn apply_player_salary(&mut self) {
        let Some(player) = self.selected_player.as_ref() else {
            self.status = "Select a player first".to_string();
            return;
        };

        if player.annual_salary.trim().is_empty() {
            self.status = "Free agents do not have an active salary to edit".to_string();
            return;
        }

        let Ok(annual_salary) = parse_display_to_internal(&player.annual_salary) else {
            self.status = "Salary must contain a valid number".to_string();
            return;
        };
        if annual_salary < 0.0 {
            self.status = "Salary cannot be negative".to_string();
            return;
        }

        let command = format!(
            "SET_PLAYER_SALARY|{}|{}",
            player.id,
            format_internal_for_command(annual_salary),
        );
        match self.game_request(&command) {
            Ok(response) => match parse_player_response(&response) {
                Ok(updated) => {
                    self.connected = true;
                    self.player_positions = Some(PlayerPositionForm::from_player(&updated));
                    self.player_potential = Some(PlayerPotentialForm::from_player(&updated));
                    self.player_communication = Some(PlayerCommunicationForm::from_player(&updated));
                    self.status = format!("Salary updated: {}", updated.name);
                    self.selected_player = Some(updated);
                }
                Err(error) => self.status = human_error(&error),
            },
            Err(error) => {
                self.connected = false;
                self.status = error;
            }
        }
    }

    fn fill_free_agent_player_contract_defaults(&mut self, team_id: usize) -> bool {
        match self.game_request(&format!("GET_CONTRACT_DEFAULTS|PLAYER|{team_id}")) {
            Ok(response) => match parse_contract_defaults_response(&response) {
                Ok((start_date, end_date, annual_salary)) => {
                    self.player_contract_form = ContractEditorForm {
                        team_id: Some(team_id),
                        start_date,
                        end_date,
                        annual_salary,
                        transfer_fee: "$0".to_string(),
                        squad_status: SquadStatusChoice::General,
                        pog_enabled: false,
                        pog_bonus: "$0".to_string(),
                        league_enabled: false,
                        league_bonus: "$0".to_string(),
                        league_rank: "1".to_string(),
                        match_enabled: false,
                        match_bonus: "$0".to_string(),
                        win_enabled: false,
                        win_bonus: "$0".to_string(),
                    };
                    true
                }
                Err(error) => {
                    self.status = human_error(&error);
                    false
                }
            },
            Err(error) => {
                self.connected = false;
                self.status = error;
                false
            }
        }
    }

    fn reset_player_contract_form(&mut self) {
        if self.player_contract_mode == ContractEditorMode::MoveFreeAgent {
            let Some(team_id) = self.player_contract_form.team_id.or(self.recruitment_team_id) else {
                self.status = "Select a destination team first".to_string();
                return;
            };
            self.fill_free_agent_player_contract_defaults(team_id);
            return;
        }

        let Some(player) = self.selected_player.as_ref() else {
            self.status = "Select a player first".to_string();
            return;
        };

        let fallback_team = self
            .teams
            .iter()
            .find(|team| team.is_player_team)
            .or_else(|| self.teams.first())
            .map(|team| team.id);

        self.player_contract_form = ContractEditorForm {
            team_id: player.contract_team_id.or(fallback_team),
            start_date: display_contract_date(&player.contract_start_date),
            end_date: display_contract_date(&player.contract_end_date),
            annual_salary: if player.annual_salary.trim().is_empty() {
                "$0".to_string()
            } else {
                player.annual_salary.clone()
            },
            transfer_fee: if player.transfer_fee.trim().is_empty() {
                "$0".to_string()
            } else {
                player.transfer_fee.clone()
            },
            squad_status: SquadStatusChoice::from_internal(&player.squad_status),
            pog_enabled: !player.incentive_pog_bonus.trim().is_empty(),
            pog_bonus: value_or_zero(&player.incentive_pog_bonus),
            league_enabled: !player.incentive_league_bonus.trim().is_empty(),
            league_bonus: value_or_zero(&player.incentive_league_bonus),
            league_rank: if player.incentive_league_rank.trim().is_empty() {
                "1".to_string()
            } else {
                player.incentive_league_rank.clone()
            },
            match_enabled: !player.incentive_match_bonus.trim().is_empty(),
            match_bonus: value_or_zero(&player.incentive_match_bonus),
            win_enabled: !player.incentive_win_bonus.trim().is_empty(),
            win_bonus: value_or_zero(&player.incentive_win_bonus),
        };
    }

    fn open_player_contract_editor(&mut self) {
        if self.selected_player.is_none() {
            self.status = "Select a player first".to_string();
            return;
        }
        self.player_contract_mode = ContractEditorMode::EditActive;
        self.reset_player_contract_form();
        self.player_contract_window_open = true;
    }

    fn apply_player_contract(&mut self) {
        let Some(player) = self.selected_player.as_ref() else {
            self.status = "Select a player first".to_string();
            return;
        };
        let Some(team_id) = self.player_contract_form.team_id else {
            self.status = "Select a destination team for the contract".to_string();
            return;
        };
        let start_date = self.player_contract_form.start_date.trim().to_string();
        let end_date = self.player_contract_form.end_date.trim().to_string();
        if !is_iso_date_shape(&start_date) || !is_iso_date_shape(&end_date) {
            self.status = "Contract dates must use YYYY-MM-DD".to_string();
            return;
        }
        if end_date < start_date {
            self.status = "Contract end date cannot be before the start date".to_string();
            return;
        }
        let Ok(annual_salary) = parse_display_to_internal(&self.player_contract_form.annual_salary) else {
            self.status = "Salary must contain a valid number".to_string();
            return;
        };
        let Ok(transfer_fee) = parse_display_to_internal(&self.player_contract_form.transfer_fee) else {
            self.status = "Transfer fee must contain a valid number".to_string();
            return;
        };
        if annual_salary < 0.0 || transfer_fee < 0.0 {
            self.status = "Salary and transfer fee cannot be negative".to_string();
            return;
        }

        let bonus_value = |enabled: bool, raw: &str, label: &str| -> Result<f64, String> {
            if !enabled {
                return Ok(0.0);
            }
            let value = parse_display_to_internal(raw)
                .map_err(|_| format!("{label} must contain a valid number"))?;
            if value < 0.0 {
                return Err(format!("{label} cannot be negative"));
            }
            Ok(value)
        };
        let pog_bonus = match bonus_value(self.player_contract_form.pog_enabled, &self.player_contract_form.pog_bonus, &self.localization.tr("contract.pog_bonus")) {
            Ok(value) => value,
            Err(error) => { self.status = error; return; }
        };
        let league_bonus = match bonus_value(self.player_contract_form.league_enabled, &self.player_contract_form.league_bonus, &self.localization.tr("contract.league_rank_bonus")) {
            Ok(value) => value,
            Err(error) => { self.status = error; return; }
        };
        let match_bonus = match bonus_value(self.player_contract_form.match_enabled, &self.player_contract_form.match_bonus, &self.localization.tr("contract.match_appearance_bonus")) {
            Ok(value) => value,
            Err(error) => { self.status = error; return; }
        };
        let win_bonus = match bonus_value(self.player_contract_form.win_enabled, &self.player_contract_form.win_bonus, &self.localization.tr("contract.match_win_bonus")) {
            Ok(value) => value,
            Err(error) => { self.status = error; return; }
        };
        let league_rank = if self.player_contract_form.league_enabled {
            match self.player_contract_form.league_rank.trim().parse::<usize>() {
                Ok(rank) if (1..=10).contains(&rank) => rank,
                _ => {
                    self.status = "League Rank must be a whole number between 1 and 10".to_string();
                    return;
                }
            }
        } else {
            1
        };

        let athlete_id = player.id;
        let player_name = player.name.clone();
        let previous_team_id = player.contract_team_id;
        let editor_mode = self.player_contract_mode;
        let command = format!(
            "SET_PLAYER_CONTRACT|{athlete_id}|{team_id}|{start_date}|{end_date}|{}|{}|{}|{}|{}|{}|{}|{league_rank}|{}|{}|{}|{}",
            format_internal_for_command(annual_salary),
            format_internal_for_command(transfer_fee),
            self.player_contract_form.squad_status.internal(),
            bool_digit(self.player_contract_form.pog_enabled),
            format_internal_for_command(pog_bonus),
            bool_digit(self.player_contract_form.league_enabled),
            format_internal_for_command(league_bonus),
            bool_digit(self.player_contract_form.match_enabled),
            format_internal_for_command(match_bonus),
            bool_digit(self.player_contract_form.win_enabled),
            format_internal_for_command(win_bonus),
        );
        match self.game_request(&command) {
            Ok(response) => match parse_player_response(&response) {
                Ok(updated) => {
                    self.connected = true;
                    self.selected_player = Some(updated);
                    self.player_contract_window_open = false;
                    self.player_contract_mode = ContractEditorMode::EditActive;
                    self.refresh_players();
                    self.status = if editor_mode == ContractEditorMode::MoveFreeAgent {
                        let team_name = self
                            .teams
                            .iter()
                            .find(|team| team.id == team_id)
                            .map(|team| team.display_name.clone())
                            .filter(|name| !name.trim().is_empty())
                            .unwrap_or_else(|| format!("Team {team_id}"));
                        format!("Applied contract and moved {player_name} to {team_name}.")
                    } else {
                        match previous_team_id {
                            None => format!("Free-agent player signed with active contract: {player_name}"),
                            Some(previous) if previous != team_id => format!("Player moved and active contract applied: {player_name}"),
                            Some(_) => format!("Active player contract updated: {player_name}"),
                        }
                    };
                }
                Err(error) => self.status = human_error(&error),
            },
            Err(error) => {
                self.connected = false;
                self.status = error;
            }
        }
    }

    fn apply_player_communication(&mut self) {
        let Some(player) = self.selected_player.as_ref() else {
            self.status = "Select a player first".to_string();
            return;
        };
        let Some(communication) = self.player_communication.as_ref() else {
            self.status = "Player Communication is not loaded".to_string();
            return;
        };

        let region_id = self.player_communication_region_id;
        if communication.primary_region == Some(region_id) {
            self.status = "The native region is stored separately and cannot be edited as learned Communication".to_string();
            return;
        }

        let value = match normalize_player_communication_value(region_id, &self.player_communication_value) {
            Ok(value) => value,
            Err(error) => {
                self.status = error;
                return;
            }
        };

        let command = format!("SET_PLAYER_COMMUNICATION|{}|{}|{}", player.id, region_id, value);
        match self.game_request(&command) {
            Ok(response) => match parse_player_response(&response) {
                Ok(updated) => {
                    self.connected = true;
                    self.player_positions = Some(PlayerPositionForm::from_player(&updated));
                    self.player_potential = Some(PlayerPotentialForm::from_player(&updated));
                    let updated_communication = PlayerCommunicationForm::from_player(&updated);
                    self.player_communication_value = player_communication_value_for_region(
                        &updated_communication,
                        region_id,
                    );
                    self.player_communication = Some(updated_communication);
                    self.status = format!(
                        "Player Communication updated: {} — {}",
                        updated.name,
                        player_communication_region_label(region_id)
                    );
                    self.selected_player = Some(updated);
                }
                Err(error) => self.status = human_error(&error),
            },
            Err(error) => {
                self.connected = false;
                self.status = error;
            }
        }
    }



    fn render_economy_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.localization.tr("economy.heading"));
        ui.label(self.localization.tr(economy_info_key()));
        ui.add_space(8.0);

        egui::Grid::new("economy_grid")
            .num_columns(2)
            .spacing([16.0, 8.0])
            .show(ui, |ui| {
                ui.label(self.localization.tr("economy.money"));
                money_text_edit_with_preview(ui, &self.localization, &mut self.economy.money, 180.0, true);
                ui.end_row();

                ui.label(self.localization.tr("economy.transfer_budget"));
                money_text_edit_with_preview(ui, &self.localization, &mut self.economy.transfer_budget, 180.0, true);
                ui.end_row();

                ui.label(self.localization.tr("economy.salary_budget"));
                money_text_edit_with_preview(ui, &self.localization, &mut self.economy.salary_budget, 180.0, true);
                ui.end_row();
            });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(self.connected, egui::Button::new(self.localization.tr("common.refresh")))
                .clicked()
            {
                self.refresh_economy();
            }

            if ui
                .add_enabled(
                    self.connected,
                    egui::Button::new(self.localization.tr(economy_apply_key())),
                )
                .clicked()
            {
                self.apply_economy();
            }

            if ui
                .add_enabled(self.connected, egui::Button::new(self.localization.tr("economy.set_all_1_2t")))
                .clicked()
            {
                let value = "$1B".to_string();
                self.economy.money = value.clone();
                self.economy.transfer_budget = value.clone();
                self.economy.salary_budget = value;
                self.apply_economy();
            }
        });
    }

    fn render_player_editor_tab(&mut self, ui: &mut egui::Ui) {
        const ATTRIBUTES_COLUMN_WIDTH: f32 = 360.0;
        const DETAILS_COLUMN_WIDTH: f32 = 400.0;
        const COLUMN_GAP: f32 = 18.0;

        ui.heading(self.localization.tr("player_editor.heading"));
        ui.label(self.localization.tr(player_editor_intro_key()));
        render_editor_safety_recommendation(ui, &self.localization);

        ui.horizontal(|ui| {
            ui.label(self.localization.tr("common.search"));
            ui.add(
                egui::TextEdit::singleline(&mut self.player_search)
                    .hint_text(self.localization.tr("editor.type_player_name"))
                    .desired_width(250.0),
            );

            if ui
                .add_enabled(
                    !self.player_search.is_empty(),
                    egui::Button::new(self.localization.tr("common.clear")),
                )
                .clicked()
            {
                self.player_search.clear();
            }
        });

        let search = self.player_search.trim().to_lowercase();
        let match_count = self
            .players
            .iter()
            .filter(|player| search.is_empty() || player.name.to_lowercase().contains(&search))
            .count();

        let selected_text = self
            .selected_player_id
            .and_then(|id| self.players.iter().find(|player| player.id == id))
            .map(|player| player.name.clone())
            .unwrap_or_else(|| self.localization.tr("editor.select_player"));
        let before = self.selected_player_id;
        let player_editor_left = ui.cursor().left();
        let available_content_width = ui.available_width();
        let mut details_column_x = None;

        // Keep every Player-row control in one natural horizontal row. The
        // Refresh Players response supplies the shared absolute x-start used
        // by the Positions/Potential column below.
        ui.horizontal(|ui| {
            ui.label(self.localization.tr("common.player"));
            ui.add_enabled_ui(self.connected && !self.players.is_empty(), |ui| {
                egui::ComboBox::from_id_salt("player_select")
                    .selected_text(selected_text)
                    .width(250.0)
                    .show_ui(ui, |ui| {
                        let mut shown = 0usize;
                        for player in &self.players {
                            if !search.is_empty()
                                && !player.name.to_lowercase().contains(&search)
                            {
                                continue;
                            }

                            shown += 1;
                            ui.selectable_value(
                                &mut self.selected_player_id,
                                Some(player.id),
                                &player.name,
                            );
                        }

                        if shown == 0 {
                            ui.label(self.localization.tr("editor.no_matching_players"));
                        }
                    });
            });
            ui.label(format!("{match_count} / {}", self.players.len()));

            let refresh_players_response = ui.add_enabled(
                self.connected,
                egui::Button::new(self.localization.tr("editor.refresh_players")),
            );
            details_column_x = Some(refresh_players_response.rect.left());
            if refresh_players_response.clicked() {
                self.refresh_players();
                self.refresh_staff();
            }

            if ui
                .add_enabled(
                    self.connected && self.selected_player_id.is_some(),
                    egui::Button::new(self.localization.tr("editor.refresh_selected")),
                )
                .clicked()
            {
                self.refresh_selected_player();
            }
        });

        let details_column_x =
            details_column_x.unwrap_or(player_editor_left + ATTRIBUTES_COLUMN_WIDTH + COLUMN_GAP);
        let two_column_required_width =
            (details_column_x - player_editor_left) + DETAILS_COLUMN_WIDTH;
        let use_two_column_layout = available_content_width >= two_column_required_width;

        if self.selected_player_id != before {
            self.refresh_selected_player();
        }

        ui.add_space(8.0);
        let mut apply_player_name_clicked = false;
        let mut apply_player_clicked = false;
        let mut max_all_clicked = false;
        let mut apply_positions_clicked = false;
        let mut apply_potential_clicked = false;
        let mut apply_salary_clicked = false;
        let mut open_contract_clicked = false;

        if self.selected_player.is_some() {
            let (player_age, player_team, player_position) = self
                .selected_player_id
                .and_then(|id| self.players.iter().find(|player| player.id == id))
                .map(|player| {
                    (
                        player.age.clone(),
                        player.team.clone(),
                        localized_position_summary(&self.localization, &player.position),
                    )
                })
                .unwrap_or_else(|| ("-".to_string(), "-".to_string(), "-".to_string()));

            if let Some(player) = self.selected_player.as_mut() {
                egui::Grid::new("player_identity_grid")
                    .num_columns(4)
                    .spacing([20.0, 5.0])
                    .show(ui, |ui| {
                        ui.label(self.localization.tr("common.name"));
                        ui.add(
                            egui::TextEdit::singleline(&mut player.name)
                                .desired_width(240.0)
                                .char_limit(100),
                        );
                        if ui
                            .add_enabled(
                                self.connected,
                                egui::Button::new(self.localization.tr("editor.apply_name")),
                            )
                            .clicked()
                        {
                            apply_player_name_clicked = true;
                        }
                        ui.label(format!(
                            "{}: {}",
                            self.localization.tr("common.id"),
                            player.id
                        ));
                        ui.end_row();

                        ui.label(self.localization.tr("search.players.position"));
                        ui.strong(player_position);
                        ui.label(self.localization.tr("common.age"));
                        ui.strong(player_age);
                        ui.end_row();

                        ui.label(self.localization.tr("common.team"));
                        ui.strong(player_team);
                        ui.end_row();
                    });
            }
            ui.add_space(12.0);

            if use_two_column_layout {
                let column_item_spacing = ui.spacing().item_spacing;
                ui.with_layout(
                    egui::Layout::left_to_right(egui::Align::Min),
                    |ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.allocate_ui_with_layout(
                            egui::vec2(ATTRIBUTES_COLUMN_WIDTH, 0.0),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                ui.spacing_mut().item_spacing = column_item_spacing;
                                ui.set_width(ATTRIBUTES_COLUMN_WIDTH);
                                self.render_player_attributes_layout_test(
                                    ui,
                                    &mut apply_player_clicked,
                                    &mut max_all_clicked,
                                );
                            },
                        );

                        let current_x = ui.cursor().left();
                        if details_column_x > current_x {
                            ui.add_space(details_column_x - current_x);
                        }

                        ui.allocate_ui_with_layout(
                            egui::vec2(DETAILS_COLUMN_WIDTH, 0.0),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                ui.spacing_mut().item_spacing = column_item_spacing;
                                ui.set_width(DETAILS_COLUMN_WIDTH);
                                self.render_player_positions_layout_test(
                                    ui,
                                    &mut apply_positions_clicked,
                                );
                                ui.add_space(8.0);
                                self.render_player_potential_layout_test(
                                    ui,
                                    &mut apply_potential_clicked,
                                );
                            },
                        );
                    },
                );
            } else {
                self.render_player_attributes_layout_test(
                    ui,
                    &mut apply_player_clicked,
                    &mut max_all_clicked,
                );
                ui.add_space(10.0);
                ui.separator();
                self.render_player_positions_layout_test(ui, &mut apply_positions_clicked);
                ui.add_space(10.0);
                ui.separator();
                self.render_player_potential_layout_test(ui, &mut apply_potential_clicked);
            }

            ui.add_space(12.0);
            ui.separator();
            self.render_player_contract_layout_test(
                ui,
                &mut apply_salary_clicked,
                &mut open_contract_clicked,
            );
            self.render_communication_section(ui);
        } else {
            ui.label(self.localization.tr("player_editor.no_data"));
        }

        if apply_player_name_clicked {
            self.apply_player_name();
        }
        if apply_positions_clicked {
            self.apply_player_positions();
        }
        if apply_potential_clicked {
            self.apply_player_potential();
        }
        if apply_salary_clicked {
            self.apply_player_salary();
        }
        if open_contract_clicked {
            self.open_player_contract_editor();
        }

        if max_all_clicked {
            if let Some(player) = self.selected_player.as_mut() {
                player.set_all_max();
                self.status =
                    "All attribute fields set to 100. Click Apply Attributes to save.".to_string();
            }
        }

        if apply_player_clicked {
            self.apply_selected_player();
        }
    }

    fn render_player_attributes_layout_test(
        &mut self,
        ui: &mut egui::Ui,
        apply_player_clicked: &mut bool,
        max_all_clicked: &mut bool,
    ) {
        ui.heading(self.localization.tr("staff_editor.attributes.heading"));
        ui.label(self.localization.tr("player_editor.attributes.range"));
        ui.add_space(4.0);

        if let Some(player) = self.selected_player.as_mut() {
            egui::Grid::new("player_stats_grid")
                .num_columns(4)
                .spacing([18.0, 7.0])
                .striped(true)
                .show(ui, |ui| {
                    stat_edit_cell(
                        ui,
                        &self.localization.tr("attributes.last_hitting"),
                        &mut player.last_hit,
                    );
                    stat_edit_cell(
                        ui,
                        &self.localization.tr("attributes.skillshot_dodging"),
                        &mut player.skill_avoid,
                    );
                    ui.end_row();

                    stat_edit_cell(
                        ui,
                        &self.localization.tr("attributes.skillshot_accuracy"),
                        &mut player.skill_hit,
                    );
                    stat_edit_cell(
                        ui,
                        &self.localization.tr("attributes.input_speed"),
                        &mut player.control_speed,
                    );
                    ui.end_row();

                    stat_edit_cell(
                        ui,
                        &self.localization.tr("attributes.positioning"),
                        &mut player.positioning,
                    );
                    stat_edit_cell(
                        ui,
                        &self.localization.tr("attributes.judgment"),
                        &mut player.judgement,
                    );
                    ui.end_row();

                    stat_edit_cell(
                        ui,
                        &self.localization.tr("attributes.mental"),
                        &mut player.mental,
                    );
                    stat_edit_cell(
                        ui,
                        &self.localization.tr("attributes.focus"),
                        &mut player.concentration,
                    );
                    ui.end_row();

                    stat_edit_cell(
                        ui,
                        &self.localization.tr("attributes.calls"),
                        &mut player.order,
                    );
                    stat_edit_cell(
                        ui,
                        &self.localization.tr("attributes.roaming"),
                        &mut player.roaming,
                    );
                    ui.end_row();

                    stat_edit_cell(
                        ui,
                        &self.localization.tr("attributes.aggression"),
                        &mut player.aggressive,
                    );
                    stat_edit_cell(
                        ui,
                        &self.localization.tr("attributes.ego"),
                        &mut player.ego,
                    );
                    ui.end_row();
                });
        }

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    self.connected,
                    egui::Button::new(self.localization.tr("editor.apply_attributes")),
                )
                .clicked()
            {
                *apply_player_clicked = true;
            }

            if ui
                .add_enabled(
                    self.connected,
                    egui::Button::new(self.localization.tr("editor.max_all")),
                )
                .clicked()
            {
                *max_all_clicked = true;
            }

            if ui
                .add_enabled(
                    self.connected && self.selected_player_id.is_some(),
                    egui::Button::new(
                        self.localization
                            .tr("player_editor.champion_mastery.open"),
                    ),
                )
                .clicked()
            {
                self.load_champion_mastery();
            }
        });
    }

    fn render_player_positions_layout_test(
        &mut self,
        ui: &mut egui::Ui,
        apply_positions_clicked: &mut bool,
    ) {
        ui.heading(self.localization.tr("player_editor.positions.heading"));
        ui.label(self.localization.tr("player_editor.positions.info"));

        let mut clear_all_positions_clicked = false;
        if let Some(positions) = self.player_positions.as_mut() {
            let slot_labels = [
                self.localization.tr("positions.primary"),
                self.localization.tr("positions.secondary"),
                self.localization.tr("positions.tertiary"),
            ];
            let selected_positions = positions.slots.map(|slot| slot.position);

            egui::Grid::new("player_positions_grid_v030")
                .num_columns(3)
                .spacing([18.0, 8.0])
                .show(ui, |ui| {
                    for (slot_index, slot_label) in slot_labels.into_iter().enumerate() {
                        ui.label(slot_label);

                        let slot = &mut positions.slots[slot_index];
                        let previous_position = slot.position;
                        egui::ComboBox::from_id_salt(format!("position_slot_{slot_index}"))
                            .selected_text(
                                slot.position
                                    .map(|position| {
                                        self.localization.tr(position.label_key())
                                    })
                                    .unwrap_or_else(|| self.localization.tr("common.none")),
                            )
                            .width(130.0)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut slot.position,
                                    None,
                                    self.localization.tr("common.none"),
                                );
                                for position in PositionChoice::ALL {
                                    let used_elsewhere = selected_positions
                                        .iter()
                                        .enumerate()
                                        .any(|(other_index, selected)| {
                                            other_index != slot_index
                                                && *selected == Some(position)
                                        });
                                    ui.add_enabled_ui(!used_elsewhere, |ui| {
                                        ui.selectable_value(
                                            &mut slot.position,
                                            Some(position),
                                            self.localization.tr(position.label_key()),
                                        );
                                    });
                                }
                            });

                        if slot.position != previous_position {
                            if slot.position.is_none() {
                                slot.proficiency = 0;
                            } else if slot.proficiency == 0 {
                                slot.proficiency = 100;
                            }
                        }

                        if slot.position.is_some() {
                            position_star_level_combo(
                                ui,
                                format!("position_slot_level_{slot_index}"),
                                &mut slot.proficiency,
                            );
                        } else {
                            ui.add_enabled(
                                false,
                                egui::Button::new("—").min_size(egui::vec2(150.0, 0.0)),
                            );
                        }
                        ui.end_row();
                    }
                });

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        self.connected,
                        egui::Button::new(
                            self.localization.tr("player_editor.positions.apply"),
                        ),
                    )
                    .clicked()
                {
                    *apply_positions_clicked = true;
                }
                if ui
                    .add_enabled(
                        self.connected,
                        egui::Button::new(
                            self.localization.tr("player_editor.positions.clear_all"),
                        ),
                    )
                    .clicked()
                {
                    clear_all_positions_clicked = true;
                }
            });
        }

        if clear_all_positions_clicked {
            if let Some(positions) = self.player_positions.as_mut() {
                positions.clear_all();
                self.status =
                    "All positions set to None. Click Apply Positions to save.".to_string();
            }
        }
    }

    fn render_player_potential_layout_test(
        &mut self,
        ui: &mut egui::Ui,
        apply_potential_clicked: &mut bool,
    ) {
        ui.heading(self.localization.tr("player_editor.potential.heading"));
        ui.label(self.localization.tr(potential_info_key()));
        if let Some(potential) = self.player_potential.as_mut() {
            egui::Grid::new("player_potential_grid")
                .num_columns(2)
                .spacing([18.0, 8.0])
                .show(ui, |ui| {
                    ui.label(self.localization.tr("player_editor.potential.grade"));

                    let mut selected_grade = potential.potential;
                    egui::ComboBox::from_id_salt("potential_grade")
                        .selected_text(selected_grade.label())
                        .width(170.0)
                        .show_ui(ui, |ui| {
                            for grade in PotentialGrade::ALL {
                                ui.selectable_value(
                                    &mut selected_grade,
                                    grade,
                                    format!("{} ({})", grade.label(), grade.raw_value()),
                                );
                            }
                        });

                    if selected_grade != potential.potential {
                        potential.set_grade(selected_grade);
                    }
                    ui.end_row();

                    ui.label(self.localization.tr("player_editor.potential.actual"));
                    let mut raw_value = potential.edit_raw;
                    let response = ui.add(
                        egui::DragValue::new(&mut raw_value)
                            .range(1..=100)
                            .speed(1.0),
                    );
                    if response.changed() {
                        potential.set_raw(raw_value);
                    }
                    ui.end_row();

                    ui.label(
                        self.localization
                            .tr("player_editor.potential.current_value"),
                    );
                    if potential.current_raw == potential.edit_raw {
                        ui.label(potential.current_raw.to_string());
                    } else {
                        ui.label(format!(
                            "{}  →  {}",
                            potential.current_raw, potential.edit_raw
                        ));
                    }
                    ui.end_row();
                });

            ui.add_space(4.0);
            ui.weak("Grade presets: Very Low 1 · Low 30 · Normal 50 · High 70 · Very High 100");

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        self.connected,
                        egui::Button::new(self.localization.tr("player_editor.potential.apply")),
                    )
                    .clicked()
                {
                    *apply_potential_clicked = true;
                }
            });
        }
    }

    fn render_player_contract_layout_test(
        &mut self,
        ui: &mut egui::Ui,
        apply_salary_clicked: &mut bool,
        open_contract_clicked: &mut bool,
    ) {
        ui.heading(self.localization.tr("player_editor.contract.heading"));
        if let Some(key) = salary_info_key() {
            ui.label(self.localization.tr(key));
        }

        if let Some(player) = self.selected_player.as_mut() {
            if player.contract_team_id.is_none() {
                ui.label(self.localization.tr("contract.free_agent_no_active"));
            } else {
                egui::Grid::new("player_contract_finance_grid")
                    .num_columns(2)
                    .spacing([18.0, 8.0])
                    .show(ui, |ui| {
                        ui.label(self.localization.tr("common.team"));
                        let team_label = player
                            .contract_team_id
                            .and_then(|id| self.teams.iter().find(|team| team.id == id))
                            .map(|team| team.display_name.clone())
                            .filter(|name| !name.trim().is_empty())
                            .or_else(|| player.contract_team_id.map(|id| format!("Team {id}")))
                            .unwrap_or_else(|| "—".to_string());
                        ui.strong(team_label);
                        ui.end_row();

                        ui.label(self.localization.tr("contract.start"));
                        ui.strong(display_contract_date(&player.contract_start_date));
                        ui.end_row();

                        ui.label(self.localization.tr("contract.end"));
                        ui.strong(display_contract_date(&player.contract_end_date));
                        ui.end_row();

                        ui.label(self.localization.tr("contract.annual_salary"));
                        money_text_edit_with_preview(
                            ui,
                            &self.localization,
                            &mut player.annual_salary,
                            180.0,
                            true,
                        );
                        ui.end_row();

                        ui.label(self.localization.tr("contract.weekly_salary"));
                        ui.strong(pretty_or_dash(&player.weekly_salary));
                        ui.end_row();

                        ui.label(self.localization.tr("contract.transfer_fee"));
                        ui.strong(pretty_or_dash(&player.transfer_fee));
                        ui.end_row();

                        ui.label(self.localization.tr("contract.squad_status"));
                        let status = SquadStatusChoice::from_internal(&player.squad_status);
                        ui.strong(self.localization.tr(status.label_key()));
                        ui.end_row();

                        ui.label(self.localization.tr("contract.pog_bonus"));
                        ui.strong(contract_bonus_display(&player.incentive_pog_bonus));
                        ui.end_row();

                        ui.label(self.localization.tr("contract.league_rank_bonus"));
                        ui.strong(if player.incentive_league_bonus.trim().is_empty() {
                            "Disabled".to_string()
                        } else {
                            format!(
                                "{} · Rank {}",
                                pretty_number(&player.incentive_league_bonus),
                                pretty_or_dash(&player.incentive_league_rank)
                            )
                        });
                        ui.end_row();

                        ui.label(self.localization.tr("contract.match_appearance_bonus"));
                        ui.strong(contract_bonus_display(&player.incentive_match_bonus));
                        ui.end_row();

                        ui.label(self.localization.tr("contract.match_win_bonus"));
                        ui.strong(contract_bonus_display(&player.incentive_win_bonus));
                        ui.end_row();
                    });
            }

            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_enabled(
                        self.connected && !player.annual_salary.trim().is_empty(),
                        egui::Button::new(self.localization.tr("contract.apply_salary")),
                    )
                    .clicked()
                {
                    *apply_salary_clicked = true;
                }
                if ui
                    .add_enabled(
                        self.connected,
                        egui::Button::new(self.localization.tr("contract.edit")),
                    )
                    .on_hover_text(self.localization.tr("player_editor.contract.edit_help"))
                    .clicked()
                {
                    *open_contract_clicked = true;
                }
            });
            ui.weak(self.localization.tr("player_editor.contract.edit_scope"));
        }
    }


    fn render_staff_editor_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.localization.tr("staff_editor.heading"));
        ui.label(self.localization.tr("staff_editor.intro"));
        render_editor_safety_recommendation(ui, &self.localization);

        let mut refresh_staff_clicked = false;
        let mut selection_changed = false;
        let mut apply_staff_name_clicked = false;
        let mut apply_staff_clicked = false;
        let mut max_all_clicked = false;
        let mut apply_salary_clicked = false;
        let mut open_contract_clicked = false;
        let mut apply_communication_clicked = false;
        let mut max_communication_clicked = false;

        ui.horizontal(|ui| {
            ui.label(self.localization.tr("common.search"));
            ui.add(
                egui::TextEdit::singleline(&mut self.staff_search)
                    .hint_text(self.localization.tr("editor.type_staff_name"))
                    .desired_width(250.0),
            );

            if ui
                .add_enabled(!self.staff_search.is_empty(), egui::Button::new(self.localization.tr("common.clear")))
                .clicked()
            {
                self.staff_search.clear();
            }
        });

        let query = self.staff_search.trim().to_lowercase();
        let filtered_staff = self
            .staffs
            .iter()
            .filter(|staff| staff.matches_search(&query))
            .map(|staff| (staff.id, staff.localized_label(&self.localization)))
            .collect::<Vec<_>>();
        let match_count = filtered_staff.len();

        ui.horizontal(|ui| {
            ui.label(self.localization.tr("common.staff"));
            let selected_label = self
                .selected_staff_id
                .and_then(|id| self.staffs.iter().find(|staff| staff.id == id))
                .map(|staff| staff.localized_label(&self.localization))
                .unwrap_or_else(|| self.localization.tr("editor.select_staff"));

            ui.add_enabled_ui(self.connected && !self.staffs.is_empty(), |ui| {
                egui::ComboBox::from_id_salt("staff_editor_select")
                    .selected_text(selected_label)
                    .width(420.0)
                    .height(320.0)
                    .show_ui(ui, |ui| {
                        ui.set_min_width(420.0);

                        if filtered_staff.is_empty() {
                            ui.label(self.localization.tr("editor.no_matching_staff"));
                        } else {
                            for (id, label) in &filtered_staff {
                                if ui
                                    .selectable_value(&mut self.selected_staff_id, Some(*id), label)
                                    .changed()
                                {
                                    selection_changed = true;
                                }
                            }
                        }
                    });
            });

            ui.label(format!("{match_count} / {}", self.staffs.len()));

            if ui
                .add_enabled(self.connected, egui::Button::new(self.localization.tr("editor.refresh_staff")))
                .clicked()
            {
                refresh_staff_clicked = true;
            }
        });

        ui.add_space(10.0);
        ui.separator();

        if let Some(staff) = self.selected_staff.as_mut() {
            egui::Grid::new("staff_identity_grid")
                .num_columns(4)
                .spacing([20.0, 5.0])
                .show(ui, |ui| {
                    ui.label(self.localization.tr("common.name"));
                    ui.add(
                        egui::TextEdit::singleline(&mut staff.name)
                            .desired_width(240.0)
                            .char_limit(100),
                    );
                    if ui
                        .add_enabled(
                            self.connected,
                            egui::Button::new(self.localization.tr("editor.apply_name")),
                        )
                        .clicked()
                    {
                        apply_staff_name_clicked = true;
                    }
                    ui.label(format!("{}: {}", self.localization.tr("common.id"), staff.id));
                    ui.end_row();

                    ui.label(self.localization.tr("common.role"));
                    ui.strong(localized_staff_role(&self.localization, &staff.role));
                    ui.label(self.localization.tr("common.age"));
                    ui.strong(&staff.age);
                    ui.end_row();

                    ui.label(self.localization.tr("common.team"));
                    ui.strong(&staff.team);
                    ui.end_row();
                });

            ui.add_space(12.0);
            ui.heading(self.localization.tr("staff_editor.attributes.heading"));
            ui.label(self.localization.tr("staff_editor.attributes.info"));

            egui::Grid::new("staff_attributes_grid")
                .num_columns(4)
                .spacing([20.0, 5.0])
                .show(ui, |ui| {
                    stat_edit_cell(ui, &self.localization.tr("staff.attributes.ban_pick"), &mut staff.banpick);
                    stat_edit_cell(ui, &self.localization.tr("staff.attributes.strategy"), &mut staff.strategy);
                    ui.end_row();

                    stat_edit_cell(ui, &self.localization.tr("staff.attributes.negotiation"), &mut staff.negotiation);
                    stat_edit_cell(ui, &self.localization.tr("staff.attributes.ability_analysis"), &mut staff.judge_ability);
                    ui.end_row();

                    stat_edit_cell(ui, &self.localization.tr("staff.attributes.potential_analysis"), &mut staff.judge_potential);
                    stat_edit_cell(ui, &self.localization.tr("staff.attributes.feedback"), &mut staff.feedback);
                    ui.end_row();

                    stat_edit_cell(ui, &self.localization.tr("staff.attributes.power_analysis"), &mut staff.power_analysis);
                    stat_edit_cell(ui, &self.localization.tr("staff.attributes.control_coaching"), &mut staff.control_coaching);
                    ui.end_row();

                    stat_edit_cell(ui, &self.localization.tr("staff.attributes.judgment_coaching"), &mut staff.judgment_coaching);
                    stat_edit_cell(ui, &self.localization.tr("staff.attributes.mental_coaching"), &mut staff.mental_coaching);
                    ui.end_row();
                });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(self.connected, egui::Button::new(self.localization.tr("editor.apply_attributes")))
                    .clicked()
                {
                    apply_staff_clicked = true;
                }

                if ui
                    .add_enabled(self.connected, egui::Button::new(self.localization.tr("editor.max_all")))
                    .clicked()
                {
                    max_all_clicked = true;
                }
            });

            ui.add_space(12.0);
            ui.separator();
            ui.heading(self.localization.tr("player_editor.contract.heading"));
            ui.label(self.localization.tr("staff_editor.contract.finance_info"));

            if staff.contract_team_id.is_none() {
                ui.label(self.localization.tr("contract.free_agent_no_active"));
            } else {
                egui::Grid::new("staff_contract_grid")
                    .num_columns(2)
                    .spacing([24.0, 7.0])
                    .show(ui, |ui| {
                        ui.label(self.localization.tr("contract.annual_salary"));
                        money_text_edit_with_preview(
                            ui,
                            &self.localization,
                            &mut staff.annual_salary,
                            180.0,
                            true,
                        );
                        ui.end_row();

                        ui.label(self.localization.tr("contract.start"));
                        ui.strong(display_contract_date(&staff.contract_start_date));
                        ui.end_row();

                        ui.label(self.localization.tr("contract.end"));
                        ui.strong(display_contract_date(&staff.contract_end_date));
                        ui.end_row();
                    });
            }

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        self.connected && !staff.annual_salary.trim().is_empty(),
                        egui::Button::new(self.localization.tr("contract.apply_salary")),
                    )
                    .clicked()
                {
                    apply_salary_clicked = true;
                }
                if ui
                    .add_enabled(self.connected, egui::Button::new(self.localization.tr("contract.edit")))
                    .on_hover_text(self.localization.tr("staff_editor.contract.edit_help"))
                    .clicked()
                {
                    open_contract_clicked = true;
                }
            });
            ui.weak(self.localization.tr("staff_editor.contract.edit_scope"));

            ui.add_space(12.0);
            ui.separator();
            ui.heading(
                self.localization
                    .tr("staff_editor.communication.section_heading"),
            );
            ui.label(self.localization.tr("staff_editor.communication.info"));

            let previous_region = self.staff_communication_region_id;
            egui::Grid::new("staff_communication_editor_grid")
                .num_columns(2)
                .spacing([24.0, 7.0])
                .show(ui, |ui| {
                    ui.label(self.localization.tr("common.region"));
                    egui::ComboBox::from_id_salt("staff_communication_region_select")
                        .selected_text(localized_communication_region_label(
                            &self.localization,
                            self.staff_communication_region_id,
                        ))
                        .width(240.0)
                        .show_ui(ui, |ui| {
                            for (region_id, _) in COMMUNICATION_REGIONS {
                                ui.selectable_value(
                                    &mut self.staff_communication_region_id,
                                    region_id,
                                    localized_communication_region_label(
                                        &self.localization,
                                        region_id,
                                    ),
                                );
                            }
                        });
                    ui.end_row();

                    ui.label(self.localization.tr("staff_editor.communication.actual"));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.staff_communication_value)
                            .desired_width(64.0),
                    );
                    ui.end_row();
                });

            if self.staff_communication_region_id != previous_region {
                self.staff_communication_value = staff_communication_value_for_region(
                    staff,
                    self.staff_communication_region_id,
                );
            }

            let selected_region_is_stored = staff
                .communication
                .iter()
                .any(|entry| entry.region_id == self.staff_communication_region_id);
            if !selected_region_is_stored {
                ui.weak(self.localization.tr("staff_editor.communication.not_stored"));
            }

            ui.add_space(6.0);
            ui.strong(
                self.localization
                    .tr("staff_editor.communication.learned_regions"),
            );
            if !staff.communication.is_empty() {
                egui::Grid::new("staff_communication_learned_regions_grid")
                    .num_columns(2)
                    .spacing([24.0, 4.0])
                    .show(ui, |ui| {
                        for entry in &staff.communication {
                            ui.label(localized_communication_region_label(
                                &self.localization,
                                entry.region_id,
                            ));
                            ui.label(format!("{} / 100", entry.value));
                            ui.end_row();
                        }
                    });
            }

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        self.connected,
                        egui::Button::new(
                            self.localization.tr("staff_editor.communication.apply"),
                        ),
                    )
                    .clicked()
                {
                    apply_communication_clicked = true;
                }
                if ui
                    .add_enabled(
                        self.connected,
                        egui::Button::new(
                            self.localization
                                .tr("staff_editor.communication.set_actual_to_100"),
                        ),
                    )
                    .clicked()
                {
                    max_communication_clicked = true;
                }
            });

            #[cfg(feature = "dev")]
            {
                ui.add_space(8.0);
                ui.strong(
                    self.localization
                        .tr("staff_editor.communication.development_details"),
                );
                let selected_region_id_text = self.staff_communication_region_id.to_string();
                ui.weak(self.localization.tr_with(
                    "staff_editor.communication.dev.selected_region_id",
                    &[("region_id", &selected_region_id_text)],
                ));

                let selected_region_stored_text = selected_region_is_stored.to_string();
                ui.weak(self.localization.tr_with(
                    "staff_editor.communication.dev.selected_region_stored",
                    &[("stored", &selected_region_stored_text)],
                ));

                let raw_stored_regions = if staff.communication.is_empty() {
                    self.localization.tr("common.none")
                } else {
                    staff
                        .communication
                        .iter()
                        .map(|entry| format!("{}={}", entry.region_id, entry.value))
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                ui.weak(self.localization.tr_with(
                    "staff_editor.communication.dev.raw_stored_regions",
                    &[("values", &raw_stored_regions)],
                ));
            }

        } else if self.staffs.is_empty() {
            ui.label(self.localization.tr("staff_editor.no_data"));
        } else {
            ui.label(self.localization.tr("staff_editor.select_prompt"));
        }

        if refresh_staff_clicked {
            self.refresh_staff();
        } else if selection_changed {
            self.refresh_selected_staff();
        }

        if max_all_clicked {
            if let Some(staff) = self.selected_staff.as_mut() {
                staff.set_all_max();
                self.status =
                    "All staff attribute fields set to 100. Click Apply Attributes to save."
                        .to_string();
            }
        }

        if max_communication_clicked {
            self.staff_communication_value = "100".to_string();
            self.status = format!(
                "{} set to 100. Click Apply Actual Communication to save.",
                staff_communication_region_label(self.staff_communication_region_id)
            );
        }

        if apply_staff_name_clicked {
            self.apply_staff_name();
        } else if apply_staff_clicked {
            self.apply_staff_attributes();
        } else if apply_salary_clicked {
            self.apply_staff_salary();
        } else if open_contract_clicked {
            self.open_staff_contract_editor();
        } else if apply_communication_clicked {
            self.apply_staff_communication();
        }
    }


    fn render_communication_section(&mut self, ui: &mut egui::Ui) {
        ui.add_space(12.0);
        ui.separator();
        ui.heading(self.localization.tr("player_communication.heading"));

        #[cfg(feature = "dev")]
        ui.label(self.localization.tr("player_communication.info.dev"));
        #[cfg(not(feature = "dev"))]
        ui.label(self.localization.tr("player_communication.info"));

        let mut apply_clicked = false;
        let mut max_clicked = false;

        if let Some(communication) = self.player_communication.as_ref() {
            let previous_region = self.player_communication_region_id;

            #[cfg(feature = "dev")]
            {
                if let Some(primary_region) = communication.primary_region {
                    ui.horizontal(|ui| {
                        ui.label(self.localization.tr("player_communication.native_region"));
                        ui.strong(localized_communication_region_label(
                            &self.localization,
                            primary_region,
                        ));
                    });
                } else {
                    ui.weak(self.localization.tr("player_communication.native_unresolved"));
                }

                egui::Grid::new("player_communication_editor_grid")
                    .num_columns(2)
                    .spacing([16.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(self.localization.tr("common.region"));
                        egui::ComboBox::from_id_salt("player_communication_region_select")
                            .selected_text(localized_communication_region_label(
                                &self.localization,
                                self.player_communication_region_id,
                            ))
                            .width(220.0)
                            .show_ui(ui, |ui| {
                                for (region_id, _) in COMMUNICATION_REGIONS {
                                    if communication.primary_region == Some(region_id) {
                                        continue;
                                    }
                                    ui.selectable_value(
                                        &mut self.player_communication_region_id,
                                        region_id,
                                        localized_communication_region_label(
                                            &self.localization,
                                            region_id,
                                        ),
                                    );
                                }
                            });
                        ui.end_row();

                        ui.label(self.localization.tr("player_communication.actual"));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.player_communication_value)
                                .desired_width(90.0),
                        );
                        ui.end_row();
                    });
            }

            #[cfg(not(feature = "dev"))]
            {
                egui::Grid::new("player_communication_editor_grid")
                    .num_columns(2)
                    .spacing([16.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(self.localization.tr("player_communication.native_region"));
                        if let Some(primary_region) = communication.primary_region {
                            ui.strong(localized_communication_region_label(
                                &self.localization,
                                primary_region,
                            ));
                        } else {
                            ui.weak(self.localization.tr("player_communication.native_unresolved"));
                        }
                        ui.end_row();

                        ui.label(self.localization.tr("common.region"));
                        egui::ComboBox::from_id_salt("player_communication_region_select")
                            .selected_text(localized_communication_region_label(
                                &self.localization,
                                self.player_communication_region_id,
                            ))
                            .width(220.0)
                            .show_ui(ui, |ui| {
                                for (region_id, _) in COMMUNICATION_REGIONS {
                                    if communication.primary_region == Some(region_id) {
                                        continue;
                                    }
                                    ui.selectable_value(
                                        &mut self.player_communication_region_id,
                                        region_id,
                                        localized_communication_region_label(
                                            &self.localization,
                                            region_id,
                                        ),
                                    );
                                }
                            });
                        ui.end_row();

                        ui.label(self.localization.tr("player_communication.actual"));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.player_communication_value)
                                .desired_width(90.0),
                        );
                        ui.end_row();

                        ui.label(self.localization.tr("player_communication.pending_xp"))
                            .on_hover_text(
                                self.localization
                                    .tr("player_communication.pending_xp_tooltip"),
                            );
                        let pending_xp = communication
                            .xp_entries
                            .iter()
                            .find(|(region_id, _)| {
                                *region_id == self.player_communication_region_id
                            })
                            .map(|(_, value)| *value)
                            .unwrap_or(0);
                        ui.weak(format!("{pending_xp} XP"));
                        ui.end_row();
                    });
            }

            if self.player_communication_region_id != previous_region {
                self.player_communication_value = player_communication_value_for_region(
                    communication,
                    self.player_communication_region_id,
                );
            }

            #[cfg(feature = "dev")]
            {
                let pending_xp = communication
                    .xp_entries
                    .iter()
                    .find(|(region_id, _)| *region_id == self.player_communication_region_id)
                    .map(|(_, value)| *value)
                    .unwrap_or(0);
                ui.label(format!(
                    "Pending training XP for selected region: {pending_xp}"
                ));
            }

            let selected_region_exists = communication
                .entries
                .iter()
                .any(|(region_id, _)| *region_id == self.player_communication_region_id);
            if !selected_region_exists {
                ui.weak(self.localization.tr("player_communication.no_actual_value"));
            }

            if !communication.entries.is_empty() {
                ui.add_space(6.0);
                ui.strong(self.localization.tr("player_communication.actual_regions"));

                #[cfg(feature = "dev")]
                for (region_id, value) in &communication.entries {
                    ui.label(format!(
                        "{}: {} / 100",
                        localized_communication_region_label(
                            &self.localization,
                            *region_id,
                        ),
                        value
                    ));
                }

                #[cfg(not(feature = "dev"))]
                egui::Grid::new("player_communication_learned_regions_grid")
                    .num_columns(2)
                    .spacing([20.0, 4.0])
                    .show(ui, |ui| {
                        for (region_id, value) in &communication.entries {
                            ui.label(localized_communication_region_label(
                                &self.localization,
                                *region_id,
                            ));
                            ui.label(format!("{value} / 100"));
                            ui.end_row();
                        }
                    });
            } else {
                ui.label(self.localization.tr("player_communication.no_actual_regions"));
            }

            #[cfg(feature = "dev")]
            if !communication.xp_entries.is_empty() {
                ui.add_space(6.0);
                ui.strong(self.localization.tr("player_communication.pending_xp"));
                for (region_id, value) in &communication.xp_entries {
                    ui.label(format!(
                        "{}: {} XP",
                        localized_communication_region_label(
                            &self.localization,
                            *region_id,
                        ),
                        value
                    ));
                }
            }

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        self.connected,
                        egui::Button::new(
                            self.localization.tr("player_communication.apply"),
                        ),
                    )
                    .clicked()
                {
                    apply_clicked = true;
                }

                #[cfg(feature = "dev")]
                let max_button_text = self.localization.tr("communication.max_selected");
                #[cfg(not(feature = "dev"))]
                let max_button_text = self
                    .localization
                    .tr("player_communication.set_actual_to_100");

                if ui
                    .add_enabled(self.connected, egui::Button::new(max_button_text))
                    .clicked()
                {
                    max_clicked = true;
                }
            });

            #[cfg(feature = "dev")]
            ui.weak(self.localization.tr("player_communication.footer"));
        }

        if max_clicked {
            self.player_communication_value = "100".to_string();
            self.apply_player_communication();
        } else if apply_clicked {
            self.apply_player_communication();
        }
    }

    fn recruitment_player_matches_search(
        player_name: &str,
        player_id: usize,
        query: &str,
    ) -> bool {
        query.is_empty()
            || player_name.to_lowercase().contains(query)
            || player_id.to_string().contains(query)
    }

    fn refresh_recruitment_settings(&mut self) {
        if let Ok(response) = self.game_request("GET_RECRUITMENT_SETTINGS") {
            let parts: Vec<&str> = response.split('|').collect();
            if parts.len() >= 4 && parts[0] == "OK" && parts[1] == "RECRUITMENT" {
                self.transfer_always_success = parts[2] == "1";
                self.recruitment_instant_retry = parts[3] == "1";
            }
        }
    }

    fn refresh_teams(&mut self) {
        match self.game_request("GET_TEAMS") {
            Ok(response) => match parse_teams_response(&response) {
                Ok(teams) => {
                    self.connected = true;
                    self.teams = teams;
                    let keep_selection = self
                        .recruitment_team_id
                        .is_some_and(|id| self.teams.iter().any(|team| team.id == id));
                    if !keep_selection {
                        self.recruitment_team_id = self
                            .teams
                            .iter()
                            .find(|team| team.is_player_team)
                            .or_else(|| self.teams.first())
                            .map(|team| team.id);
                    }
                    #[cfg(feature = "dev")]
                    {
                        let keep_workspace_selection = self
                            .team_workspace_team_id
                            .is_some_and(|id| self.teams.iter().any(|team| team.id == id));
                        if !keep_workspace_selection {
                            self.team_workspace_team_id = self
                                .teams
                                .iter()
                                .find(|team| team.is_player_team)
                                .or_else(|| self.teams.first())
                                .map(|team| team.id);
                        }
                    }
                    self.status = format!("Loaded {} teams", self.teams.len());
                    self.update_team_search_status();
                }
                Err(error) => {
                    let status = human_error(&error);
                    self.team_search_status = Some(status.clone());
                    self.status = status;
                }
            },
            Err(error) => {
                self.connected = false;
                self.team_search_status = Some(error.clone());
                self.status = error;
            }
        }
    }

    fn move_recruitment_player_to_team(&mut self) {
        let Some(athlete_id) = self.recruitment_player_id else {
            self.status = "Select a player first".to_string();
            return;
        };
        let Some(team_id) = self.recruitment_team_id else {
            self.status = "Select a destination team first".to_string();
            return;
        };

        let selected_is_free_agent = self
            .players
            .iter()
            .find(|player| player.id == athlete_id)
            .is_some_and(|player| player.team == "Free Agent");

        if selected_is_free_agent {
            self.selected_player_id = Some(athlete_id);
            self.refresh_selected_player();
            if self.selected_player.is_some() {
                self.player_contract_mode = ContractEditorMode::MoveFreeAgent;
                if self.fill_free_agent_player_contract_defaults(team_id) {
                    self.player_contract_window_open = true;
                    self.status = "Free-agent contract filled automatically. Review it or click Apply Contract & Move Player.".to_string();
                } else {
                    self.player_contract_mode = ContractEditorMode::EditActive;
                }
            }
            return;
        }

        let player_name = self
            .players
            .iter()
            .find(|player| player.id == athlete_id)
            .map(|player| player.name.clone())
            .unwrap_or_else(|| format!("Player {athlete_id}"));

        match self.game_request(&format!("MOVE_PLAYER_TO_TEAM|{athlete_id}|{team_id}")) {
            Ok(response) => {
                if let Some(error) = response.strip_prefix("ERR|") {
                    self.status = human_error(error);
                    return;
                }
                if response == "OK|MOVE_PLAYER" {
                    self.connected = true;
                    let team_name = self
                        .teams
                        .iter()
                        .find(|team| team.id == team_id)
                        .map(|team| team.display_name.clone())
                        .filter(|name| !name.trim().is_empty())
                        .unwrap_or_else(|| format!("Team {team_id}"));
                    self.status = format!("Moved {player_name} to {team_name}.");
                    self.refresh_players();
                    if self.selected_player_id == Some(athlete_id) {
                        self.refresh_selected_player();
                    }
                } else {
                    self.status = format!("Unexpected bridge response: {response}");
                }
            }
            Err(error) => {
                self.connected = false;
                self.status = error;
            }
        }
    }

    fn set_recruitment_player_free_agent(&mut self, athlete_id: usize) {
        let player_name = self
            .players
            .iter()
            .find(|player| player.id == athlete_id)
            .map(|player| player.name.clone())
            .unwrap_or_else(|| format!("Player {athlete_id}"));

        match self.game_request(&format!("SET_PLAYER_FREE_AGENT|{athlete_id}")) {
            Ok(response) if response == "OK|PLAYER_FREE_AGENT" => {
                self.connected = true;
                self.status = format!(
                    "{player_name} is now a Free Agent. Verify roster, Proceed, and save/reload."
                );
                self.refresh_players();
                if self.selected_player_id == Some(athlete_id) {
                    self.refresh_selected_player();
                }
            }
            Ok(response) => {
                if let Some(error) = response.strip_prefix("ERR|") {
                    self.status = human_error(error);
                } else {
                    self.status = format!("Unexpected bridge response: {response}");
                }
            }
            Err(error) => {
                self.connected = false;
                self.status = error;
            }
        }
    }


    fn move_recruitment_staff_to_team(&mut self) {
        let Some(staff_id) = self.recruitment_staff_id else {
            self.status = "Select a staff member first".to_string();
            return;
        };
        let Some(team_id) = self.recruitment_team_id else {
            self.status = "Select a destination team first".to_string();
            return;
        };

        let selected_is_free_agent = self
            .staffs
            .iter()
            .find(|staff| staff.id == staff_id)
            .is_some_and(|staff| staff.team == "Free Agent");

        if selected_is_free_agent {
            self.selected_staff_id = Some(staff_id);
            self.refresh_selected_staff();
            if self.selected_staff.is_some() {
                self.staff_contract_mode = ContractEditorMode::MoveFreeAgent;
                if self.fill_free_agent_staff_contract_defaults(team_id) {
                    self.staff_contract_window_open = true;
                    self.status = "Free-agent contract filled automatically. Review it or click Apply Contract & Move Staff.".to_string();
                } else {
                    self.staff_contract_mode = ContractEditorMode::EditActive;
                }
            }
            return;
        }

        let staff_name = self
            .staffs
            .iter()
            .find(|staff| staff.id == staff_id)
            .map(|staff| staff.name.clone())
            .unwrap_or_else(|| format!("Staff {staff_id}"));

        match self.game_request(&format!("MOVE_STAFF_TO_TEAM|{staff_id}|{team_id}")) {
            Ok(response) => {
                if let Some(error) = response.strip_prefix("ERR|") {
                    self.status = human_error(error);
                    return;
                }
                if response == "OK|MOVE_STAFF" {
                    self.connected = true;
                    let team_name = self
                        .teams
                        .iter()
                        .find(|team| team.id == team_id)
                        .map(|team| team.display_name.clone())
                        .filter(|name| !name.trim().is_empty())
                        .unwrap_or_else(|| format!("Team {team_id}"));
                    self.status = format!("Moved {staff_name} to {team_name}.");
                    self.refresh_staff();
                    if self.selected_staff_id == Some(staff_id) {
                        self.refresh_selected_staff();
                    }
                } else {
                    self.status = format!("Unexpected bridge response: {response}");
                }
            }
            Err(error) => {
                self.connected = false;
                self.status = error;
            }
        }
    }

    fn set_recruitment_staff_free_agent(&mut self, staff_id: usize) {
        let staff_name = self
            .staffs
            .iter()
            .find(|staff| staff.id == staff_id)
            .map(|staff| staff.name.clone())
            .unwrap_or_else(|| format!("Staff {staff_id}"));

        match self.game_request(&format!("SET_STAFF_FREE_AGENT|{staff_id}")) {
            Ok(response) if response == "OK|STAFF_FREE_AGENT" => {
                self.connected = true;
                self.status = format!(
                    "{staff_name} is now a Free Agent. Verify staff assignment, Proceed, and save/reload."
                );
                self.refresh_staff();
                if self.selected_staff_id == Some(staff_id) {
                    self.refresh_selected_staff();
                }
            }
            Ok(response) => {
                if let Some(error) = response.strip_prefix("ERR|") {
                    self.status = human_error(error);
                } else {
                    self.status = format!("Unexpected bridge response: {response}");
                }
            }
            Err(error) => {
                self.connected = false;
                self.status = error;
            }
        }
    }

    fn set_transfer_always_success(&mut self, enabled: bool) {
        let command = format!(
            "SET_TRANSFER_ALWAYS_SUCCESS|{}",
            if enabled { 1 } else { 0 }
        );
        match self.game_request(&command) {
            Ok(response) => {
                let parts: Vec<&str> = response.split('|').collect();
                if parts.first() == Some(&"ERR") {
                    self.status = human_error(parts.get(1).copied().unwrap_or("UNKNOWN_ERROR"));
                    self.transfer_always_success = !enabled;
                } else if parts.len() >= 4 && parts[0] == "OK" && parts[1] == "RECRUITMENT" {
                    self.transfer_always_success = parts[2] == "1";
                    self.recruitment_instant_retry = parts[3] == "1";
                    self.status = if self.transfer_always_success {
                        "Transfer Always Success enabled for your current team".to_string()
                    } else {
                        "Transfer Always Success disabled".to_string()
                    };
                } else {
                    self.status = format!("Unexpected bridge response: {response}");
                    self.transfer_always_success = !enabled;
                }
            }
            Err(error) => {
                self.status = error;
                self.transfer_always_success = !enabled;
            }
        }
    }

    fn set_recruitment_instant_retry(&mut self, enabled: bool) {
        let command = format!(
            "SET_RECRUITMENT_INSTANT_RETRY|{}",
            if enabled { 1 } else { 0 }
        );
        match self.game_request(&command) {
            Ok(response) => {
                let parts: Vec<&str> = response.split('|').collect();
                if parts.first() == Some(&"ERR") {
                    self.status = human_error(parts.get(1).copied().unwrap_or("UNKNOWN_ERROR"));
                    self.recruitment_instant_retry = !enabled;
                } else if parts.len() >= 4 && parts[0] == "OK" && parts[1] == "RECRUITMENT" {
                    self.transfer_always_success = parts[2] == "1";
                    self.recruitment_instant_retry = parts[3] == "1";
                    self.status = if self.recruitment_instant_retry {
                        "Instant recruitment retry enabled for your current team".to_string()
                    } else {
                        "Recruitment retry cooldown restored to normal".to_string()
                    };
                } else {
                    self.status = format!("Unexpected bridge response: {response}");
                    self.recruitment_instant_retry = !enabled;
                }
            }
            Err(error) => {
                self.status = error;
                self.recruitment_instant_retry = !enabled;
            }
        }
    }

    fn render_recruitment_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.localization.tr("recruitment.heading"));
        ui.label(self.localization.tr("recruitment.info"));
        render_editor_safety_recommendation(ui, &self.localization);

        ui.group(|ui| {
            ui.strong(self.localization.tr("recruitment.transfer_negotiation"));
            ui.add_space(6.0);

            let previous = self.transfer_always_success;
            let response = ui.checkbox(
                &mut self.transfer_always_success,
                "Transfer Always Success",
            );
            response.on_hover_text(self.localization.tr(transfer_success_tooltip_key()));

            if self.transfer_always_success != previous {
                self.set_transfer_always_success(self.transfer_always_success);
            }

            ui.label(self.localization.tr(transfer_runtime_key()));
        });

        ui.add_space(10.0);
        ui.group(|ui| {
            ui.strong(self.localization.tr("recruitment.retry"));
            ui.add_space(6.0);

            let previous = self.recruitment_instant_retry;
            let response = ui.checkbox(
                &mut self.recruitment_instant_retry,
                "Instant Retry (No Negotiation Cooldown)",
            );
            response.on_hover_text(self.localization.tr(instant_retry_tooltip_key()));

            if self.recruitment_instant_retry != previous {
                self.set_recruitment_instant_retry(self.recruitment_instant_retry);
            }

            ui.label(if self.recruitment_instant_retry {
                "Mode: Instant. Rejected negotiations should be retryable immediately."
            } else {
                "Mode: Normal. TFM2 controls retry cooldowns."
            });
        });

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            for tab in RecruitmentManagementTab::ALL {
                let label = self.localization.tr(tab.label_key());
                ui.selectable_value(
                    &mut self.recruitment_management_tab,
                    tab,
                    label,
                );
            }
        });
        ui.separator();

        match self.recruitment_management_tab {
            RecruitmentManagementTab::Players => {
                ui.group(|ui| {
                    ui.strong(self.localization.tr("recruitment.player_management.heading"));
                    ui.label(self.localization.tr(recruitment_player_management_key()));
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.label(self.localization.tr("common.search"));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.recruitment_player_search)
                                .desired_width(220.0)
                                .hint_text(self.localization.tr("recruitment.search_player_hint")),
                        );
                        if ui.button(self.localization.tr("common.clear")).clicked() {
                            self.recruitment_player_search.clear();
                        }
                        if ui.button(self.localization.tr("editor.refresh_players")).clicked() {
                            self.refresh_players();
                        }
                    });

                    let query = self.recruitment_player_search.trim().to_lowercase();
                    let filtered_players = self
                        .players
                        .iter()
                        .filter(|player| {
                            Self::recruitment_player_matches_search(
                                &player.name,
                                player.id,
                                &query,
                            )
                        })
                        .collect::<Vec<_>>();

                    let selected_player_text = self
                        .recruitment_player_id
                        .and_then(|id| self.players.iter().find(|player| player.id == id))
                        .map(|player| format!("{} · {} · ID {}", player.name, player.team, player.id))
                        .unwrap_or_else(|| self.localization.tr("editor.select_player"));

                    ui.horizontal(|ui| {
                        ui.label(self.localization.tr("common.player"));
                        egui::ComboBox::from_id_salt("recruitment_player_v039")
                            .selected_text(selected_player_text)
                            .width(390.0)
                            .show_ui(ui, |ui| {
                                for player in &filtered_players {
                                    ui.selectable_value(
                                        &mut self.recruitment_player_id,
                                        Some(player.id),
                                        format!("{} · {} · ID {}", player.name, player.team, player.id),
                                    );
                                }
                            });
                        {
                            let count = filtered_players.len().to_string();
                            ui.label(self.localization.tr_with("common.matches", &[("count", count.as_str())]));
                        }
                    });

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label(self.localization.tr("recruitment.search_team"));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.recruitment_team_search)
                                .desired_width(220.0)
                                .hint_text(self.localization.tr("recruitment.search_team_hint")),
                        );
                        if ui.button(self.localization.tr("common.clear")).clicked() {
                            self.recruitment_team_search.clear();
                        }
                        if ui.button(self.localization.tr("recruitment.refresh_teams")).clicked() {
                            self.refresh_teams();
                        }
                    });

                    let team_query = self.recruitment_team_search.trim().to_lowercase();
                    let filtered_teams = self
                        .teams
                        .iter()
                        .filter(|team| team.matches_search(&team_query))
                        .collect::<Vec<_>>();

                    let selected_team_text = self
                        .recruitment_team_id
                        .and_then(|id| self.teams.iter().find(|team| team.id == id))
                        .map(|team| team.localized_label(&self.localization))
                        .unwrap_or_else(|| self.localization.tr("common.select_team"));

                    ui.horizontal(|ui| {
                        ui.label(self.localization.tr("recruitment.destination_team"));
                        egui::ComboBox::from_id_salt("recruitment_player_team_v039")
                            .selected_text(selected_team_text)
                            .width(420.0)
                            .show_ui(ui, |ui| {
                                for team in &filtered_teams {
                                    ui.selectable_value(
                                        &mut self.recruitment_team_id,
                                        Some(team.id),
                                        team.localized_label(&self.localization),
                                    );
                                }
                            });
                        {
                            let count = filtered_teams.len().to_string();
                            ui.label(self.localization.tr_with("common.matches", &[("count", count.as_str())]));
                        }
                    });

                    if let Some(my_team) = self.teams.iter().find(|team| team.is_player_team) {
                        ui.label(format!("{}: {}", self.localization.tr("common.my_team"), my_team.display_name));
                    }

                    let selected_is_free_agent = self
                        .recruitment_player_id
                        .and_then(|id| self.players.iter().find(|player| player.id == id))
                        .is_some_and(|player| player.team == "Free Agent");
                    let (action_label, action_tooltip) = if selected_is_free_agent {
                        (
                            self.localization.tr("recruitment.player_management.create_contract_move"),
                            self.localization.tr("recruitment.player_management.create_contract_move_tooltip"),
                        )
                    } else {
                        (
                            self.localization.tr("recruitment.player_management.move_contracted"),
                            self.localization.tr(move_player_tooltip_key()),
                        )
                    };
                    let can_move = self.connected
                        && self.recruitment_player_id.is_some()
                        && self.recruitment_team_id.is_some();

                    ui.add_space(8.0);
                    if ui
                        .add_enabled(can_move, egui::Button::new(action_label))
                        .on_hover_text(action_tooltip)
                        .clicked()
                    {
                        self.move_recruitment_player_to_team();
                    }

                    if ui
                        .add_enabled(
                            self.connected
                                && self.recruitment_player_id.is_some()
                                && !selected_is_free_agent,
                            egui::Button::new(self.localization.tr("recruitment.player_management.set_free_agent")),
                        )
                        .on_hover_text(self.localization.tr("recruitment.player_management.set_free_agent_info"))
                        .clicked()
                    {
                        self.free_agent_confirmation_player_id = self.recruitment_player_id;
                    }

                    if let Some(confirm_id) = self.free_agent_confirmation_player_id {
                        let confirm_name = self
                            .players
                            .iter()
                            .find(|player| player.id == confirm_id)
                            .map(|player| player.name.clone())
                            .unwrap_or_else(|| format!("Player {confirm_id}"));

                        ui.add_space(6.0);
                        ui.group(|ui| {
                            ui.strong(format!("Confirm: Set {confirm_name} to Free Agent?"));
                            ui.label(self.localization.tr("recruitment.player_management.set_free_agent_warning"));
                            ui.horizontal(|ui| {
                                if ui.button(self.localization.tr("recruitment.confirm_free_agent")).clicked() {
                                    self.set_recruitment_player_free_agent(confirm_id);
                                    self.free_agent_confirmation_player_id = None;
                                }
                                if ui.button(self.localization.tr("common.cancel")).clicked() {
                                    self.free_agent_confirmation_player_id = None;
                                }
                            });
                        });
                    }
                });
            }
            RecruitmentManagementTab::Staff => {
                ui.group(|ui| {
                    ui.strong(self.localization.tr("recruitment.tabs.staff_management"));
                    ui.label(self.localization.tr("recruitment.staff_management.info"));
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.label(self.localization.tr("common.search"));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.recruitment_staff_search)
                                .desired_width(220.0)
                                .hint_text(self.localization.tr("recruitment.search_staff_hint")),
                        );
                        if ui.button(self.localization.tr("common.clear")).clicked() {
                            self.recruitment_staff_search.clear();
                        }
                        if ui.button(self.localization.tr("editor.refresh_staff")).clicked() {
                            self.refresh_staff();
                        }
                    });

                    let query = self.recruitment_staff_search.trim().to_lowercase();
                    let filtered_staff = self
                        .staffs
                        .iter()
                        .filter(|staff| staff.matches_search(&query))
                        .collect::<Vec<_>>();

                    let selected_staff_text = self
                        .recruitment_staff_id
                        .and_then(|id| self.staffs.iter().find(|staff| staff.id == id))
                        .map(|staff| staff.localized_label(&self.localization))
                        .unwrap_or_else(|| self.localization.tr("editor.select_staff"));

                    ui.horizontal(|ui| {
                        ui.label(self.localization.tr("common.staff"));
                        egui::ComboBox::from_id_salt("recruitment_staff_v039")
                            .selected_text(selected_staff_text)
                            .width(420.0)
                            .show_ui(ui, |ui| {
                                for staff in &filtered_staff {
                                    ui.selectable_value(
                                        &mut self.recruitment_staff_id,
                                        Some(staff.id),
                                        staff.localized_label(&self.localization),
                                    );
                                }
                            });
                        {
                            let count = filtered_staff.len().to_string();
                            ui.label(self.localization.tr_with("common.matches", &[("count", count.as_str())]));
                        }
                    });

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label(self.localization.tr("recruitment.search_team"));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.recruitment_team_search)
                                .desired_width(220.0)
                                .hint_text(self.localization.tr("recruitment.search_team_hint")),
                        );
                        if ui.button(self.localization.tr("common.clear")).clicked() {
                            self.recruitment_team_search.clear();
                        }
                        if ui.button(self.localization.tr("recruitment.refresh_teams")).clicked() {
                            self.refresh_teams();
                        }
                    });

                    let team_query = self.recruitment_team_search.trim().to_lowercase();
                    let filtered_teams = self
                        .teams
                        .iter()
                        .filter(|team| team.matches_search(&team_query))
                        .collect::<Vec<_>>();

                    let selected_team_text = self
                        .recruitment_team_id
                        .and_then(|id| self.teams.iter().find(|team| team.id == id))
                        .map(|team| team.localized_label(&self.localization))
                        .unwrap_or_else(|| self.localization.tr("common.select_team"));

                    ui.horizontal(|ui| {
                        ui.label(self.localization.tr("recruitment.destination_team"));
                        egui::ComboBox::from_id_salt("recruitment_staff_team_v039")
                            .selected_text(selected_team_text)
                            .width(420.0)
                            .show_ui(ui, |ui| {
                                for team in &filtered_teams {
                                    ui.selectable_value(
                                        &mut self.recruitment_team_id,
                                        Some(team.id),
                                        team.localized_label(&self.localization),
                                    );
                                }
                            });
                        {
                            let count = filtered_teams.len().to_string();
                            ui.label(self.localization.tr_with("common.matches", &[("count", count.as_str())]));
                        }
                    });

                    let selected_is_free_agent = self
                        .recruitment_staff_id
                        .and_then(|id| self.staffs.iter().find(|staff| staff.id == id))
                        .is_some_and(|staff| staff.team == "Free Agent");
                    let (action_label, action_tooltip) = if selected_is_free_agent {
                        (
                            self.localization.tr("recruitment.staff_management.create_contract_move"),
                            self.localization.tr("recruitment.staff_management.create_contract_move_tooltip"),
                        )
                    } else {
                        (
                            self.localization.tr("recruitment.staff_management.move_contracted"),
                            self.localization.tr("recruitment.staff_management.move_tooltip"),
                        )
                    };
                    let can_move = self.connected
                        && self.recruitment_staff_id.is_some()
                        && self.recruitment_team_id.is_some();

                    ui.add_space(8.0);
                    if ui
                        .add_enabled(can_move, egui::Button::new(action_label))
                        .on_hover_text(action_tooltip)
                        .clicked()
                    {
                        self.move_recruitment_staff_to_team();
                    }

                    if ui
                        .add_enabled(
                            self.connected
                                && self.recruitment_staff_id.is_some()
                                && !selected_is_free_agent,
                            egui::Button::new(self.localization.tr("recruitment.staff_management.set_free_agent")),
                        )
                        .on_hover_text(self.localization.tr("recruitment.staff_management.set_free_agent_info"))
                        .clicked()
                    {
                        self.free_agent_confirmation_staff_id = self.recruitment_staff_id;
                    }

                    if let Some(confirm_id) = self.free_agent_confirmation_staff_id {
                        let confirm_name = self
                            .staffs
                            .iter()
                            .find(|staff| staff.id == confirm_id)
                            .map(|staff| staff.name.clone())
                            .unwrap_or_else(|| format!("Staff {confirm_id}"));

                        ui.add_space(6.0);
                        ui.group(|ui| {
                            ui.strong(format!("Confirm: Set {confirm_name} to Free Agent?"));
                            ui.label(self.localization.tr("recruitment.staff_management.set_free_agent_warning"));
                            ui.horizontal(|ui| {
                                if ui.button(self.localization.tr("recruitment.confirm_free_agent")).clicked() {
                                    self.set_recruitment_staff_free_agent(confirm_id);
                                    self.free_agent_confirmation_staff_id = None;
                                }
                                if ui.button(self.localization.tr("common.cancel")).clicked() {
                                    self.free_agent_confirmation_staff_id = None;
                                }
                            });
                        });
                    }
                });

            }
        }
    }

    fn filter_library_dir() -> Result<PathBuf, String> {
        let exe = std::env::current_exe()
            .map_err(|e| format!("Could not resolve executable path: {e}"))?;
        let parent = exe
            .parent()
            .ok_or_else(|| "Could not resolve executable folder".to_string())?;
        Ok(parent.join("filters"))
    }

    fn sanitize_filter_name(name: &str) -> String {
        let cleaned = name
            .trim()
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() || matches!(ch, ' ' | '-' | '_' | '.') {
                    ch
                } else {
                    '_'
                }
            })
            .collect::<String>()
            .trim_matches([' ', '.'])
            .to_string();

        if cleaned.is_empty() {
            "Player Filter".to_string()
        } else {
            cleaned
        }
    }

    fn saved_filter_path(name: &str) -> Result<PathBuf, String> {
        Ok(Self::filter_library_dir()?
            .join(format!("{}.tfm2filter", Self::sanitize_filter_name(name))))
    }

    fn reload_saved_filters(&mut self) {
        let Ok(dir) = Self::filter_library_dir() else {
            return;
        };

        if let Err(error) = fs::create_dir_all(&dir) {
            self.status = format!("Could not create filter library: {error}");
            return;
        }

        let Ok(entries) = fs::read_dir(&dir) else {
            return;
        };

        let mut names = entries
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let path = entry.path();
                let is_filter = path
                    .extension()
                    .and_then(|value| value.to_str())
                    .is_some_and(|value| value.eq_ignore_ascii_case("tfm2filter"));
                if !is_filter {
                    return None;
                }

                path.file_stem()
                    .and_then(|value| value.to_str())
                    .map(str::to_string)
            })
            .collect::<Vec<_>>();

        names.sort_by_key(|name| name.to_lowercase());
        names.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
        self.saved_filters = names;

        if let Some(selected) = self.selected_saved_filter.as_ref() {
            if !self
                .saved_filters
                .iter()
                .any(|name| name.eq_ignore_ascii_case(selected))
            {
                self.selected_saved_filter = None;
            }
        }
    }

    fn load_saved_filter(&mut self, name: &str) {
        match Self::saved_filter_path(name) {
            Ok(path) => match fs::read_to_string(&path) {
                Ok(text) => {
                    let mut filter = AdvancedPlayerSearch::default();
                    filter.import_text(&text);
                    self.advanced_player_search = filter;
                    self.selected_saved_filter = Some(name.to_string());
                    self.status = format!("Loaded filter: {name}");
                }
                Err(error) => self.status = format!("Could not load filter: {error}"),
            },
            Err(error) => self.status = error,
        }
    }

    fn save_named_filter(&mut self, name: &str, overwrite: bool) {
        let name = Self::sanitize_filter_name(name);

        match Self::saved_filter_path(&name) {
            Ok(path) => {
                if path.exists() && !overwrite {
                    self.status = format!(
                        "Filter '{name}' already exists. Select it and use Update Filter."
                    );
                    return;
                }

                if let Some(parent) = path.parent() {
                    if let Err(error) = fs::create_dir_all(parent) {
                        self.status = format!("Could not create filter library: {error}");
                        return;
                    }
                }

                match fs::write(&path, self.advanced_player_search.export_text()) {
                    Ok(()) => {
                        self.selected_saved_filter = Some(name.clone());
                        self.reload_saved_filters();
                        self.status = format!("Saved filter: {name}");
                    }
                    Err(error) => self.status = format!("Could not save filter: {error}"),
                }
            }
            Err(error) => self.status = error,
        }
    }

    fn update_selected_filter(&mut self) {
        let Some(name) = self.selected_saved_filter.clone() else {
            self.status = "Select a saved filter to update".to_string();
            return;
        };
        self.save_named_filter(&name, true);
    }

    fn delete_selected_filter(&mut self) {
        let Some(name) = self.selected_saved_filter.clone() else {
            self.status = "Select a saved filter to delete".to_string();
            return;
        };

        match Self::saved_filter_path(&name) {
            Ok(path) => match fs::remove_file(&path) {
                Ok(()) => {
                    self.selected_saved_filter = None;
                    self.reload_saved_filters();
                    self.status = format!("Deleted filter: {name}");
                }
                Err(error) => self.status = format!("Could not delete filter: {error}"),
            },
            Err(error) => self.status = error,
        }
    }

    fn export_advanced_filter(&mut self) {
        let default_name = self
            .selected_saved_filter
            .as_deref()
            .unwrap_or("TFM2 Player Filter");

        let Some(path) = rfd::FileDialog::new()
            .set_title("Export TFM2 Player Filter")
            .set_file_name(format!(
                "{}.tfm2filter",
                Self::sanitize_filter_name(default_name)
            ))
            .add_filter("TFM2 Player Filter", &["tfm2filter"])
            .add_filter("Text File", &["txt"])
            .save_file()
        else {
            return;
        };

        match fs::write(&path, self.advanced_player_search.export_text()) {
            Ok(()) => self.status = format!("Exported player filter to {}", path.display()),
            Err(error) => self.status = format!("Could not export player filter: {error}"),
        }
    }

    fn import_advanced_filter(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Import TFM2 Player Filter")
            .add_filter("TFM2 Player Filter", &["tfm2filter", "txt"])
            .pick_file()
        else {
            return;
        };

        match fs::read_to_string(&path) {
            Ok(text) => {
                let mut filter = AdvancedPlayerSearch::default();
                filter.import_text(&text);
                self.advanced_player_search = filter;

                let imported_name = path
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .map(Self::sanitize_filter_name)
                    .unwrap_or_else(|| "Imported Filter".to_string());

                self.selected_saved_filter = Some(imported_name.clone());
                self.save_named_filter(&imported_name, true);
                self.status = format!(
                    "Imported filter: {}",
                    path.file_name()
                        .and_then(|value| value.to_str())
                        .unwrap_or("filter")
                );
            }
            Err(error) => self.status = format!("Could not import player filter: {error}"),
        }
    }


    fn list_library_dir() -> Result<PathBuf, String> {
        let exe = std::env::current_exe()
            .map_err(|error| format!("Could not resolve executable path: {error}"))?;
        let parent = exe
            .parent()
            .ok_or_else(|| "Could not resolve executable folder".to_string())?;
        Ok(parent.join("lists"))
    }

    fn sanitize_list_name(name: &str) -> String {
        let cleaned = name
            .trim()
            .chars()
            .map(|ch| {
                if ch.is_control() || matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') {
                    '_'
                } else {
                    ch
                }
            })
            .collect::<String>()
            .trim_matches([' ', '.'])
            .to_string();

        let reserved = [
            "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5",
            "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5",
            "LPT6", "LPT7", "LPT8", "LPT9",
        ];
        if cleaned.is_empty() {
            "Saved List".to_string()
        } else if reserved.iter().any(|value| cleaned.eq_ignore_ascii_case(value)) {
            format!("_{cleaned}")
        } else {
            cleaned
        }
    }

    fn saved_player_list_path(name: &str) -> Result<PathBuf, String> {
        Ok(Self::list_library_dir()?
            .join(format!("{}.tfm2list", Self::sanitize_list_name(name))))
    }

    fn reload_saved_player_lists(&mut self) {
        let Ok(dir) = Self::list_library_dir() else {
            return;
        };

        if let Err(error) = fs::create_dir_all(&dir) {
            self.status = format!("Could not create list library: {error}");
            return;
        }

        let Ok(entries) = fs::read_dir(&dir) else {
            return;
        };

        let mut lists = Vec::new();
        let mut invalid_count = 0usize;
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            let is_list = path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("tfm2list"));
            if !is_list {
                continue;
            }

            let Ok(text) = fs::read_to_string(&path) else {
                invalid_count += 1;
                continue;
            };
            let Ok(mut list) = serde_json::from_str::<SavedPlayerList>(&text) else {
                invalid_count += 1;
                continue;
            };
            if !list.is_supported() {
                invalid_count += 1;
                continue;
            }
            if list.name.trim().is_empty() {
                list.name = path
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .unwrap_or("Imported List")
                    .to_string();
            }
            list.name = Self::sanitize_list_name(&list.name);
            list.normalize();
            lists.push(list);
        }

        lists.sort_by_key(|list| list.name.to_lowercase());
        lists.dedup_by(|left, right| left.name.eq_ignore_ascii_case(&right.name));
        self.saved_player_lists = lists;

        if let Some(selected) = self.selected_saved_player_list.as_ref() {
            if !self
                .saved_player_lists
                .iter()
                .any(|list| list.name.eq_ignore_ascii_case(selected))
            {
                self.selected_saved_player_list = None;
            }
        }

        if let Some(active) = self.active_player_list_filter.as_ref() {
            if !self
                .saved_player_lists
                .iter()
                .any(|list| list.name.eq_ignore_ascii_case(active))
            {
                self.active_player_list_filter = None;
            }
        }

        if let Some(active) = self.active_staff_list_filter.as_ref() {
            if !self
                .saved_player_lists
                .iter()
                .any(|list| list.name.eq_ignore_ascii_case(active))
            {
                self.active_staff_list_filter = None;
            }
        }

        if invalid_count > 0 {
            let count = invalid_count.to_string();
            self.status = self.localization.tr_with(
                "lists.status.invalid_files",
                &[("count", count.as_str())],
            );
        }
    }

    fn write_player_list(&mut self, list: &SavedPlayerList, overwrite: bool) -> Result<(), String> {
        let path = Self::saved_player_list_path(&list.name)?;
        if path.exists() && !overwrite {
            return Err(self.localization.tr_with(
                "lists.status.already_exists",
                &[("name", list.name.as_str())],
            ));
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("Could not create list library: {error}"))?;
        }
        let mut normalized = list.clone();
        normalized.normalize();
        let text = serde_json::to_string_pretty(&normalized)
            .map_err(|error| format!("Could not serialize list: {error}"))?;
        fs::write(&path, text)
            .map_err(|error| format!("Could not save list: {error}"))?;
        Ok(())
    }

    fn create_named_player_list(&mut self, name: &str) {
        let name = Self::sanitize_list_name(name);
        let mut list = SavedPlayerList::new(name.clone());
        list.player_ids = self.pending_new_list_player_ids.clone();
        list.staff_ids = self.pending_new_list_staff_ids.clone();
        list.normalize();
        match self.write_player_list(&list, false) {
            Ok(()) => {
                self.pending_new_list_player_ids.clear();
                self.pending_new_list_staff_ids.clear();
                self.selected_saved_player_list = Some(name.clone());
                self.reload_saved_player_lists();
                self.status = self.localization.tr_with(
                    "lists.status.created",
                    &[("name", name.as_str())],
                );
            }
            Err(error) => self.status = error,
        }
    }

    fn rename_selected_player_list(&mut self, new_name: &str) {
        let Some(old_name) = self.selected_saved_player_list.clone() else {
            self.status = self.localization.tr("lists.status.select_to_rename");
            return;
        };
        let new_name = Self::sanitize_list_name(new_name);
        if old_name.eq_ignore_ascii_case(&new_name) {
            return;
        }
        let Some(mut list) = self
            .saved_player_lists
            .iter()
            .find(|list| list.name.eq_ignore_ascii_case(&old_name))
            .cloned()
        else {
            self.status = self.localization.tr("lists.status.not_found");
            return;
        };

        list.name = new_name.clone();
        match self.write_player_list(&list, false) {
            Ok(()) => {
                if let Ok(old_path) = Self::saved_player_list_path(&old_name) {
                    let _ = fs::remove_file(old_path);
                }
                if self
                    .active_player_list_filter
                    .as_ref()
                    .is_some_and(|active| active.eq_ignore_ascii_case(&old_name))
                {
                    self.active_player_list_filter = Some(new_name.clone());
                }
                if self
                    .active_staff_list_filter
                    .as_ref()
                    .is_some_and(|active| active.eq_ignore_ascii_case(&old_name))
                {
                    self.active_staff_list_filter = Some(new_name.clone());
                }
                self.selected_saved_player_list = Some(new_name.clone());
                self.reload_saved_player_lists();
                self.status = self.localization.tr_with(
                    "lists.status.renamed",
                    &[("name", new_name.as_str())],
                );
            }
            Err(error) => self.status = error,
        }
    }

    fn delete_selected_player_list(&mut self) {
        let Some(name) = self.selected_saved_player_list.clone() else {
            self.status = self.localization.tr("lists.status.select_to_delete");
            return;
        };
        match Self::saved_player_list_path(&name) {
            Ok(path) => match fs::remove_file(path) {
                Ok(()) => {
                    if self
                        .active_player_list_filter
                        .as_ref()
                        .is_some_and(|active| active.eq_ignore_ascii_case(&name))
                    {
                        self.active_player_list_filter = None;
                    }
                    if self
                        .active_staff_list_filter
                        .as_ref()
                        .is_some_and(|active| active.eq_ignore_ascii_case(&name))
                    {
                        self.active_staff_list_filter = None;
                    }
                    self.selected_saved_player_list = None;
                    self.reload_saved_player_lists();
                    self.status = self.localization.tr_with(
                        "lists.status.deleted",
                        &[("name", name.as_str())],
                    );
                }
                Err(error) => self.status = format!("Could not delete list: {error}"),
            },
            Err(error) => self.status = error,
        }
    }

    fn add_player_ids_to_list(&mut self, list_name: &str, player_ids: &[usize]) {
        let Some(mut list) = self
            .saved_player_lists
            .iter()
            .find(|list| list.name.eq_ignore_ascii_case(list_name))
            .cloned()
        else {
            self.status = self.localization.tr("lists.status.not_found");
            return;
        };

        let before = list.player_ids.len();
        list.player_ids.extend(player_ids.iter().copied());
        list.normalize();
        let added = list.player_ids.len().saturating_sub(before);

        match self.write_player_list(&list, true) {
            Ok(()) => {
                self.reload_saved_player_lists();
                let count = added.to_string();
                self.status = self.localization.tr_with(
                    "lists.status.added",
                    &[("count", count.as_str()), ("name", list.name.as_str())],
                );
            }
            Err(error) => self.status = error,
        }
    }

    fn add_staff_ids_to_list(&mut self, list_name: &str, staff_ids: &[usize]) {
        let Some(mut list) = self
            .saved_player_lists
            .iter()
            .find(|list| list.name.eq_ignore_ascii_case(list_name))
            .cloned()
        else {
            self.status = self.localization.tr("lists.status.not_found");
            return;
        };

        let before = list.staff_ids.len();
        list.staff_ids.extend(staff_ids.iter().copied());
        list.normalize();
        let added = list.staff_ids.len().saturating_sub(before);

        match self.write_player_list(&list, true) {
            Ok(()) => {
                self.reload_saved_player_lists();
                let count = added.to_string();
                self.status = self.localization.tr_with(
                    "lists.status.added_staff",
                    &[("count", count.as_str()), ("name", list.name.as_str())],
                );
            }
            Err(error) => self.status = error,
        }
    }

    fn remove_staff_ids_from_list(&mut self, list_name: &str, staff_ids: &[usize]) {
        let Some(mut list) = self
            .saved_player_lists
            .iter()
            .find(|list| list.name.eq_ignore_ascii_case(list_name))
            .cloned()
        else {
            self.status = self.localization.tr("lists.status.not_found");
            return;
        };

        let ids = staff_ids.iter().copied().collect::<BTreeSet<_>>();
        let before = list.staff_ids.len();
        list.staff_ids.retain(|id| !ids.contains(id));
        let removed = before.saturating_sub(list.staff_ids.len());

        match self.write_player_list(&list, true) {
            Ok(()) => {
                self.reload_saved_player_lists();
                let count = removed.to_string();
                self.status = self.localization.tr_with(
                    "lists.status.removed_staff",
                    &[("count", count.as_str()), ("name", list.name.as_str())],
                );
            }
            Err(error) => self.status = error,
        }
    }

    fn remove_player_ids_from_list(&mut self, list_name: &str, player_ids: &[usize]) {
        let Some(mut list) = self
            .saved_player_lists
            .iter()
            .find(|list| list.name.eq_ignore_ascii_case(list_name))
            .cloned()
        else {
            self.status = self.localization.tr("lists.status.not_found");
            return;
        };

        let ids = player_ids.iter().copied().collect::<BTreeSet<_>>();
        let before = list.player_ids.len();
        list.player_ids.retain(|id| !ids.contains(id));
        let removed = before.saturating_sub(list.player_ids.len());

        match self.write_player_list(&list, true) {
            Ok(()) => {
                self.reload_saved_player_lists();
                let count = removed.to_string();
                self.status = self.localization.tr_with(
                    "lists.status.removed",
                    &[("count", count.as_str()), ("name", list.name.as_str())],
                );
            }
            Err(error) => self.status = error,
        }
    }

    fn import_player_list(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title(self.localization.tr("lists.import_title"))
            .add_filter("TFM2 List", &["tfm2list"])
            .pick_file()
        else {
            return;
        };

        let result = fs::read_to_string(&path)
            .map_err(|error| format!("Could not read list: {error}"))
            .and_then(|text| {
                serde_json::from_str::<SavedPlayerList>(&text)
                    .map_err(|error| format!("Invalid TFM2 list: {error}"))
            });

        match result {
            Ok(mut list) => {
                if !list.is_supported() {
                    self.status = self.localization.tr("lists.status.unsupported_format");
                    return;
                }
                list.name = Self::sanitize_list_name(&list.name);
                list.normalize();
                let name = list.name.clone();
                match self.write_player_list(&list, false) {
                    Ok(()) => {
                        self.selected_saved_player_list = Some(name.clone());
                        self.reload_saved_player_lists();
                        self.status = self.localization.tr_with(
                            "lists.status.imported",
                            &[("name", name.as_str())],
                        );
                    }
                    Err(error) => self.status = error,
                }
            }
            Err(error) => self.status = error,
        }
    }

    fn export_selected_player_list(&mut self) {
        let Some(list) = self
            .selected_saved_player_list
            .as_ref()
            .and_then(|name| {
                self.saved_player_lists
                    .iter()
                    .find(|list| list.name.eq_ignore_ascii_case(name))
            })
            .cloned()
        else {
            self.status = self.localization.tr("lists.status.select_to_export");
            return;
        };

        let Some(path) = rfd::FileDialog::new()
            .set_title(self.localization.tr("lists.export_title"))
            .set_file_name(format!("{}.tfm2list", Self::sanitize_list_name(&list.name)))
            .add_filter("TFM2 List", &["tfm2list"])
            .save_file()
        else {
            return;
        };

        match serde_json::to_string_pretty(&list)
            .map_err(|error| format!("Could not serialize list: {error}"))
            .and_then(|text| {
                fs::write(&path, text)
                    .map_err(|error| format!("Could not export list: {error}"))
            }) {
            Ok(()) => {
                let path_text = path.display().to_string();
                self.status = self.localization.tr_with(
                    "lists.status.exported",
                    &[("path", path_text.as_str())],
                );
            }
            Err(error) => self.status = error,
        }
    }

    fn open_selected_list_in_player_search(&mut self) {
        let Some(name) = self.selected_saved_player_list.clone() else {
            self.status = self.localization.tr("lists.status.select_to_open");
            return;
        };
        self.active_player_list_filter = Some(name);
        self.search_tab = SearchTab::Players;
        self.restore_active_search_status();
    }

    fn open_selected_list_in_staff_search(&mut self) {
        let Some(name) = self.selected_saved_player_list.clone() else {
            self.status = self.localization.tr("lists.status.select_to_open");
            return;
        };
        self.active_staff_list_filter = Some(name);
        self.search_tab = SearchTab::Staff;
        self.restore_active_search_status();
    }


    fn staff_filter_library_dir() -> Result<PathBuf, String> {
        Ok(Self::filter_library_dir()?.join("staff"))
    }

    fn saved_staff_filter_path(name: &str) -> Result<PathBuf, String> {
        Ok(Self::staff_filter_library_dir()?
            .join(format!("{}.tfm2filter", Self::sanitize_filter_name(name))))
    }

    fn reload_saved_staff_filters(&mut self) {
        let Ok(dir) = Self::staff_filter_library_dir() else {
            return;
        };

        if let Err(error) = fs::create_dir_all(&dir) {
            self.status = format!("Could not create staff filter library: {error}");
            return;
        }

        let Ok(entries) = fs::read_dir(&dir) else {
            return;
        };

        let mut names = entries
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let path = entry.path();
                let is_filter = path
                    .extension()
                    .and_then(|value| value.to_str())
                    .is_some_and(|value| value.eq_ignore_ascii_case("tfm2filter"));
                if !is_filter {
                    return None;
                }

                path.file_stem()
                    .and_then(|value| value.to_str())
                    .map(str::to_string)
            })
            .collect::<Vec<_>>();

        names.sort_by_key(|name| name.to_lowercase());
        names.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
        self.saved_staff_filters = names;

        if let Some(selected) = self.selected_saved_staff_filter.as_ref() {
            if !self
                .saved_staff_filters
                .iter()
                .any(|name| name.eq_ignore_ascii_case(selected))
            {
                self.selected_saved_staff_filter = None;
            }
        }
    }

    fn load_saved_staff_filter(&mut self, name: &str) {
        match Self::saved_staff_filter_path(name) {
            Ok(path) => match fs::read_to_string(&path) {
                Ok(text) => {
                    let mut filter = AdvancedStaffSearch::default();
                    filter.import_text(&text);
                    self.advanced_staff_search = filter;
                    self.selected_saved_staff_filter = Some(name.to_string());
                    self.status = format!("Loaded staff filter: {name}");
                }
                Err(error) => self.status = format!("Could not load staff filter: {error}"),
            },
            Err(error) => self.status = error,
        }
    }

    fn save_named_staff_filter(&mut self, name: &str, overwrite: bool) {
        let name = Self::sanitize_filter_name(name);

        match Self::saved_staff_filter_path(&name) {
            Ok(path) => {
                if path.exists() && !overwrite {
                    self.status = format!(
                        "Staff filter '{name}' already exists. Select it and use Update Filter."
                    );
                    return;
                }

                if let Some(parent) = path.parent() {
                    if let Err(error) = fs::create_dir_all(parent) {
                        self.status = format!("Could not create staff filter library: {error}");
                        return;
                    }
                }

                match fs::write(&path, self.advanced_staff_search.export_text()) {
                    Ok(()) => {
                        self.selected_saved_staff_filter = Some(name.clone());
                        self.reload_saved_staff_filters();
                        self.status = format!("Saved staff filter: {name}");
                    }
                    Err(error) => self.status = format!("Could not save staff filter: {error}"),
                }
            }
            Err(error) => self.status = error,
        }
    }

    fn update_selected_staff_filter(&mut self) {
        let Some(name) = self.selected_saved_staff_filter.clone() else {
            self.status = "Select a saved staff filter to update".to_string();
            return;
        };
        self.save_named_staff_filter(&name, true);
    }

    fn delete_selected_staff_filter(&mut self) {
        let Some(name) = self.selected_saved_staff_filter.clone() else {
            self.status = "Select a saved staff filter to delete".to_string();
            return;
        };

        match Self::saved_staff_filter_path(&name) {
            Ok(path) => match fs::remove_file(&path) {
                Ok(()) => {
                    self.selected_saved_staff_filter = None;
                    self.reload_saved_staff_filters();
                    self.status = format!("Deleted staff filter: {name}");
                }
                Err(error) => self.status = format!("Could not delete staff filter: {error}"),
            },
            Err(error) => self.status = error,
        }
    }

    fn export_advanced_staff_filter(&mut self) {
        let default_name = self
            .selected_saved_staff_filter
            .as_deref()
            .unwrap_or("TFM2 Staff Filter");

        let Some(path) = rfd::FileDialog::new()
            .set_title("Export TFM2 Staff Filter")
            .set_file_name(format!(
                "{}.tfm2filter",
                Self::sanitize_filter_name(default_name)
            ))
            .add_filter("TFM2 Staff Filter", &["tfm2filter"])
            .add_filter("Text File", &["txt"])
            .save_file()
        else {
            return;
        };

        match fs::write(&path, self.advanced_staff_search.export_text()) {
            Ok(()) => self.status = format!("Exported staff filter to {}", path.display()),
            Err(error) => self.status = format!("Could not export staff filter: {error}"),
        }
    }

    fn import_advanced_staff_filter(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Import TFM2 Staff Filter")
            .add_filter("TFM2 Staff Filter", &["tfm2filter", "txt"])
            .pick_file()
        else {
            return;
        };

        match fs::read_to_string(&path) {
            Ok(text) => {
                let mut filter = AdvancedStaffSearch::default();
                filter.import_text(&text);
                self.advanced_staff_search = filter;

                let imported_name = path
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .map(Self::sanitize_filter_name)
                    .unwrap_or_else(|| "Imported Staff Filter".to_string());

                self.selected_saved_staff_filter = Some(imported_name.clone());
                self.save_named_staff_filter(&imported_name, true);
                self.status = format!(
                    "Imported staff filter: {}",
                    path.file_name()
                        .and_then(|value| value.to_str())
                        .unwrap_or("filter")
                );
            }
            Err(error) => self.status = format!("Could not import staff filter: {error}"),
        }
    }


    fn render_search_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.localization.tr("common.search"));
        ui.label(self.localization.tr(search_intro_key()));
        ui.add_space(8.0);

        ui.add_space(8.0);
        let search_tab_before_click = self.search_tab;
        ui.horizontal_wrapped(|ui| {
            for tab in SearchTab::ALL {
                let label = self.localization.tr(tab.label_key());
                ui.selectable_value(&mut self.search_tab, tab, label);
            }
        });
        if search_tab_before_click != self.search_tab {
            self.restore_active_search_status();
        }

        ui.separator();
        ui.add_space(6.0);

        match self.search_tab {
            SearchTab::Players => self.render_player_search_page(ui),
            SearchTab::Staff => self.render_staff_search_page(ui),
            SearchTab::Teams => self.render_team_search_page(ui),
            SearchTab::Lists => self.render_saved_lists_page(ui),
            #[cfg(feature = "dev")]
            SearchTab::History => {
                #[cfg(feature = "dev")]
                {

                ui.columns(2, |columns| {
                    columns[0].group(|ui| {
                        ui.strong("Snapshot Filters");
                        ui.add_space(6.0);
                        ui.add_enabled(false, egui::Button::new("Player"));
                        ui.add_enabled(false, egui::Button::new("Season"));
                        ui.add_enabled(false, egui::Button::new("From Date"));
                        ui.add_enabled(false, egui::Button::new("To Date"));
                    });
                    columns[1].group(|ui| {
                        ui.strong("Tracked Fields");
                        ui.add_space(6.0);
                        ui.add_enabled(false, egui::Button::new("Actual Rating"));
                        ui.add_enabled(false, egui::Button::new("Potential Rating"));
                        ui.add_enabled(false, egui::Button::new("Salary / Contract"));
                        ui.add_enabled(false, egui::Button::new("Team / Position"));
                    });
                });

                ui.add_space(10.0);
                ui.group(|ui| {
                    ui.set_min_width(ui.available_width());
                    ui.strong("History / Stats Over Time");
                    ui.label(
                        "Planned snapshot table: Player · Date · Actual Rating · Potential Rating · Salary · Team · Position. \
                         This keeps Search extensible without redesigning the list when historical tracking is added.",
                    );
                    ui.add_space(6.0);
                    ui.add_enabled(false, egui::Button::new("Historical snapshots not enabled yet"));
                });

                }

                #[cfg(not(feature = "dev"))]
                {
                    ui.heading("History");
                    ui.label("Under development.");
                }
            }
        }
    }


    fn render_saved_lists_page(&mut self, ui: &mut egui::Ui) {
        let has_selected_list = self.selected_saved_player_list.is_some();
        let mut create_requested = false;
        let mut rename_requested = false;
        let mut delete_requested = false;
        let mut import_requested = false;
        let mut export_requested = false;
        let mut open_in_player_search_requested = false;
        let mut open_in_staff_search_requested = false;
        let mut reload_requested = false;
        let mut selected_list_change: Option<String> = None;
        let mut remove_player_ids: Vec<usize> = Vec::new();
        let mut remove_staff_ids: Vec<usize> = Vec::new();
        let mut open_player_id: Option<usize> = None;
        let mut open_staff_id: Option<usize> = None;

        ui.group(|ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                ui.strong(self.localization.tr("lists.heading"));
                ui.separator();
                if ui.button(self.localization.tr("lists.create")).clicked() {
                    create_requested = true;
                }
                if ui
                    .add_enabled(
                        has_selected_list,
                        egui::Button::new(self.localization.tr("lists.rename")),
                    )
                    .clicked()
                {
                    rename_requested = true;
                }
                if ui
                    .add_enabled(
                        has_selected_list,
                        egui::Button::new(self.localization.tr("lists.delete")),
                    )
                    .clicked()
                {
                    delete_requested = true;
                }
                ui.separator();
                if ui.button(self.localization.tr("lists.import")).clicked() {
                    import_requested = true;
                }
                if ui
                    .add_enabled(
                        has_selected_list,
                        egui::Button::new(self.localization.tr("lists.export")),
                    )
                    .clicked()
                {
                    export_requested = true;
                }
                ui.separator();
                if ui
                    .add_enabled(
                        has_selected_list,
                        egui::Button::new(self.localization.tr("lists.open_in_player_search")),
                    )
                    .clicked()
                {
                    open_in_player_search_requested = true;
                }
                if ui
                    .add_enabled(
                        has_selected_list,
                        egui::Button::new(self.localization.tr("lists.open_in_staff_search")),
                    )
                    .clicked()
                {
                    open_in_staff_search_requested = true;
                }
                if ui.button(self.localization.tr("common.refresh")).clicked() {
                    reload_requested = true;
                }
            });
            ui.weak(self.localization.tr("lists.info"));
        });

        ui.add_space(8.0);
        let full_height = ui.available_height().max(240.0);
        let left_width = 235.0_f32.min((ui.available_width() * 0.34).max(180.0));
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(left_width, full_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.group(|ui| {
                        ui.set_min_width(ui.available_width());
                        ui.set_min_height(ui.available_height());
                        ui.strong(self.localization.tr("lists.saved_lists"));
                        ui.separator();
                        egui::ScrollArea::vertical()
                            .id_salt("saved_lists_library")
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                if self.saved_player_lists.is_empty() {
                                    ui.weak(self.localization.tr("lists.no_lists"));
                                }
                                for list in self.saved_player_lists.clone() {
                                    let count = list.total_members().to_string();
                                    let label = self.localization.tr_with(
                                        "lists.list_label",
                                        &[("name", list.name.as_str()), ("count", count.as_str())],
                                    );
                                    let selected = self
                                        .selected_saved_player_list
                                        .as_ref()
                                        .is_some_and(|name| name.eq_ignore_ascii_case(&list.name));
                                    if ui.selectable_label(selected, label).clicked() {
                                        selected_list_change = Some(list.name.clone());
                                    }
                                }
                            });
                    });
                },
            );

            ui.add_space(8.0);
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), full_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.group(|ui| {
                        ui.set_min_width(ui.available_width());
                        ui.set_min_height(ui.available_height());

                        let selected_list = self
                            .selected_saved_player_list
                            .as_ref()
                            .and_then(|name| {
                                self.saved_player_lists
                                    .iter()
                                    .find(|list| list.name.eq_ignore_ascii_case(name))
                            })
                            .cloned();

                        let Some(list) = selected_list else {
                            ui.strong(self.localization.tr("lists.contents"));
                            ui.separator();
                            ui.weak(self.localization.tr("lists.select_list"));
                            return;
                        };

                        let player_count = list.player_ids.len().to_string();
                        let staff_count = list.staff_ids.len().to_string();
                        ui.horizontal_wrapped(|ui| {
                            ui.strong(&list.name);
                            ui.separator();
                            ui.selectable_value(
                                &mut self.list_content_tab,
                                ListContentTab::Players,
                                self.localization.tr_with(
                                    "lists.players_tab",
                                    &[("count", player_count.as_str())],
                                ),
                            );
                            ui.selectable_value(
                                &mut self.list_content_tab,
                                ListContentTab::Staff,
                                self.localization.tr_with(
                                    "lists.staff_tab",
                                    &[("count", staff_count.as_str())],
                                ),
                            );
                        });
                        ui.separator();

                        match self.list_content_tab {
                            ListContentTab::Players => {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(self.localization.tr_with(
                                        "lists.player_count",
                                        &[("count", player_count.as_str())],
                                    ));
                                    if ui
                                        .add_enabled(
                                            !self.selected_list_player_ids.is_empty(),
                                            egui::Button::new(
                                                self.localization.tr("lists.remove_selected"),
                                            ),
                                        )
                                        .clicked()
                                    {
                                        remove_player_ids
                                            .extend(self.selected_list_player_ids.iter().copied());
                                    }
                                    if ui
                                        .add_enabled(
                                            !self.selected_list_player_ids.is_empty(),
                                            egui::Button::new(
                                                self.localization.tr("lists.clear_selection"),
                                            ),
                                        )
                                        .clicked()
                                    {
                                        self.selected_list_player_ids.clear();
                                    }
                                });

                                let table_height = (ui.available_height() - 30.0).max(120.0);
                                let mut selected_member_ids = self.selected_list_player_ids.clone();
                                TableBuilder::new(ui)
                                    .id_salt("saved_player_list_contents")
                                    .striped(true)
                                    .resizable(true)
                                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                                    .sense(egui::Sense::click())
                                    .column(Column::initial(42.0).at_least(36.0).clip(true))
                                    .column(
                                        Column::initial(180.0)
                                            .at_least(90.0)
                                            .clip(true)
                                            .resizable(true),
                                    )
                                    .column(
                                        Column::initial(68.0)
                                            .at_least(48.0)
                                            .clip(true)
                                            .resizable(true),
                                    )
                                    .column(
                                        Column::initial(220.0)
                                            .at_least(120.0)
                                            .clip(true)
                                            .resizable(true),
                                    )
                                    .min_scrolled_height(table_height)
                                    .max_scroll_height(table_height)
                                    .auto_shrink([false, false])
                                    .header(22.0, |mut header| {
                                        header.col(|ui| {
                                            ui.strong(self.localization.tr("lists.select_column"));
                                        });
                                        header.col(|ui| {
                                            ui.strong(self.localization.tr("common.name"));
                                        });
                                        header.col(|ui| {
                                            ui.strong(self.localization.tr("common.id"));
                                        });
                                        header.col(|ui| {
                                            ui.strong(self.localization.tr("common.team"));
                                        });
                                    })
                                    .body(|body| {
                                        body.rows(22.0, list.player_ids.len(), |mut row| {
                                            let player_id = list.player_ids[row.index()];
                                            let player = self
                                                .players
                                                .iter()
                                                .find(|player| player.id == player_id);
                                            row.set_selected(selected_member_ids.contains(&player_id));
                                            row.col(|ui| {
                                                let mut selected =
                                                    selected_member_ids.contains(&player_id);
                                                if ui.checkbox(&mut selected, "").changed() {
                                                    if selected {
                                                        selected_member_ids.insert(player_id);
                                                    } else {
                                                        selected_member_ids.remove(&player_id);
                                                    }
                                                }
                                            });
                                            row.col(|ui| {
                                                if let Some(player) = player {
                                                    ui.label(&player.name);
                                                } else {
                                                    ui.weak(
                                                        self.localization.tr("lists.missing_player"),
                                                    );
                                                }
                                            });
                                            row.col(|ui| {
                                                ui.label(player_id.to_string());
                                            });
                                            row.col(|ui| {
                                                if let Some(player) = player {
                                                    ui.label(value_or_dash(&player.team));
                                                } else {
                                                    ui.weak("—");
                                                }
                                            });

                                            let row_response = row.response();
                                            if row_response.double_clicked() && player.is_some() {
                                                selected_member_ids.insert(player_id);
                                                open_player_id = Some(player_id);
                                            }
                                            row_response.context_menu(|ui| {
                                                if player.is_some()
                                                    && ui
                                                        .button(self.localization.tr(
                                                            "search.open_in_player_editor",
                                                        ))
                                                        .clicked()
                                                {
                                                    open_player_id = Some(player_id);
                                                    ui.close_menu();
                                                }
                                                if ui
                                                    .button(self.localization.tr(
                                                        "lists.remove_from_list",
                                                    ))
                                                    .clicked()
                                                {
                                                    remove_player_ids.push(player_id);
                                                    ui.close_menu();
                                                }
                                            });
                                        });
                                    });
                                self.selected_list_player_ids = selected_member_ids;
                            }
                            ListContentTab::Staff => {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(self.localization.tr_with(
                                        "lists.staff_count",
                                        &[("count", staff_count.as_str())],
                                    ));
                                    if ui
                                        .add_enabled(
                                            !self.selected_list_staff_ids.is_empty(),
                                            egui::Button::new(
                                                self.localization.tr("lists.remove_selected"),
                                            ),
                                        )
                                        .clicked()
                                    {
                                        remove_staff_ids
                                            .extend(self.selected_list_staff_ids.iter().copied());
                                    }
                                    if ui
                                        .add_enabled(
                                            !self.selected_list_staff_ids.is_empty(),
                                            egui::Button::new(
                                                self.localization.tr("lists.clear_selection"),
                                            ),
                                        )
                                        .clicked()
                                    {
                                        self.selected_list_staff_ids.clear();
                                    }
                                });

                                let table_height = (ui.available_height() - 30.0).max(120.0);
                                let mut selected_member_ids = self.selected_list_staff_ids.clone();
                                TableBuilder::new(ui)
                                    .id_salt("saved_staff_list_contents")
                                    .striped(true)
                                    .resizable(true)
                                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                                    .sense(egui::Sense::click())
                                    .column(Column::initial(42.0).at_least(36.0).clip(true))
                                    .column(
                                        Column::initial(180.0)
                                            .at_least(90.0)
                                            .clip(true)
                                            .resizable(true),
                                    )
                                    .column(
                                        Column::initial(68.0)
                                            .at_least(48.0)
                                            .clip(true)
                                            .resizable(true),
                                    )
                                    .column(
                                        Column::initial(220.0)
                                            .at_least(120.0)
                                            .clip(true)
                                            .resizable(true),
                                    )
                                    .column(
                                        Column::initial(150.0)
                                            .at_least(100.0)
                                            .clip(true)
                                            .resizable(true),
                                    )
                                    .min_scrolled_height(table_height)
                                    .max_scroll_height(table_height)
                                    .auto_shrink([false, false])
                                    .header(22.0, |mut header| {
                                        header.col(|ui| {
                                            ui.strong(self.localization.tr("lists.select_column"));
                                        });
                                        header.col(|ui| {
                                            ui.strong(self.localization.tr("common.name"));
                                        });
                                        header.col(|ui| {
                                            ui.strong(self.localization.tr("common.id"));
                                        });
                                        header.col(|ui| {
                                            ui.strong(self.localization.tr("common.team"));
                                        });
                                        header.col(|ui| {
                                            ui.strong(self.localization.tr("common.role"));
                                        });
                                    })
                                    .body(|body| {
                                        body.rows(22.0, list.staff_ids.len(), |mut row| {
                                            let staff_id = list.staff_ids[row.index()];
                                            let staff = self
                                                .staffs
                                                .iter()
                                                .find(|staff| staff.id == staff_id);
                                            row.set_selected(selected_member_ids.contains(&staff_id));
                                            row.col(|ui| {
                                                let mut selected =
                                                    selected_member_ids.contains(&staff_id);
                                                if ui.checkbox(&mut selected, "").changed() {
                                                    if selected {
                                                        selected_member_ids.insert(staff_id);
                                                    } else {
                                                        selected_member_ids.remove(&staff_id);
                                                    }
                                                }
                                            });
                                            row.col(|ui| {
                                                if let Some(staff) = staff {
                                                    ui.label(&staff.name);
                                                } else {
                                                    ui.weak(
                                                        self.localization.tr("lists.missing_staff"),
                                                    );
                                                }
                                            });
                                            row.col(|ui| {
                                                ui.label(staff_id.to_string());
                                            });
                                            row.col(|ui| {
                                                if let Some(staff) = staff {
                                                    ui.label(value_or_dash(&staff.team));
                                                } else {
                                                    ui.weak("—");
                                                }
                                            });
                                            row.col(|ui| {
                                                if let Some(staff) = staff {
                                                    ui.label(localized_staff_role(
                                                        &self.localization,
                                                        &staff.role,
                                                    ));
                                                } else {
                                                    ui.weak("—");
                                                }
                                            });

                                            let row_response = row.response();
                                            if row_response.double_clicked() && staff.is_some() {
                                                selected_member_ids.insert(staff_id);
                                                open_staff_id = Some(staff_id);
                                            }
                                            row_response.context_menu(|ui| {
                                                if staff.is_some()
                                                    && ui
                                                        .button(self.localization.tr(
                                                            "search.open_in_staff_editor",
                                                        ))
                                                        .clicked()
                                                {
                                                    open_staff_id = Some(staff_id);
                                                    ui.close_menu();
                                                }
                                                if ui
                                                    .button(self.localization.tr(
                                                        "lists.remove_from_list",
                                                    ))
                                                    .clicked()
                                                {
                                                    remove_staff_ids.push(staff_id);
                                                    ui.close_menu();
                                                }
                                            });
                                        });
                                    });
                                self.selected_list_staff_ids = selected_member_ids;
                            }
                        }
                    });
                },
            );
        });

        if let Some(name) = selected_list_change {
            self.selected_saved_player_list = Some(name);
            self.selected_list_player_ids.clear();
            self.selected_list_staff_ids.clear();
        }
        if create_requested {
            self.pending_new_list_player_ids.clear();
            self.pending_new_list_staff_ids.clear();
            self.list_name_popup_mode = ListNamePopupMode::Create;
            self.list_name_draft.clear();
            self.list_name_popup_open = true;
        }
        if rename_requested {
            self.list_name_popup_mode = ListNamePopupMode::Rename;
            self.list_name_draft = self.selected_saved_player_list.clone().unwrap_or_default();
            self.list_name_popup_open = true;
        }
        if delete_requested {
            self.list_delete_confirmation_open = true;
        }
        if import_requested {
            self.import_player_list();
        }
        if export_requested {
            self.export_selected_player_list();
        }
        if open_in_player_search_requested {
            self.open_selected_list_in_player_search();
        }
        if open_in_staff_search_requested {
            self.open_selected_list_in_staff_search();
        }
        if reload_requested {
            self.reload_saved_player_lists();
        }
        if !remove_player_ids.is_empty() {
            remove_player_ids.sort_unstable();
            remove_player_ids.dedup();
            if let Some(name) = self.selected_saved_player_list.clone() {
                self.remove_player_ids_from_list(&name, &remove_player_ids);
                for id in remove_player_ids {
                    self.selected_list_player_ids.remove(&id);
                }
            }
        }
        if !remove_staff_ids.is_empty() {
            remove_staff_ids.sort_unstable();
            remove_staff_ids.dedup();
            if let Some(name) = self.selected_saved_player_list.clone() {
                self.remove_staff_ids_from_list(&name, &remove_staff_ids);
                for id in remove_staff_ids {
                    self.selected_list_staff_ids.remove(&id);
                }
            }
        }
        if let Some(player_id) = open_player_id {
            self.open_player_in_editor(player_id);
        }
        if let Some(staff_id) = open_staff_id {
            self.open_staff_in_editor(staff_id);
        }
    }

    #[cfg(feature = "dev")]
    fn render_team_workspace_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.localization.tr("team_workspace.heading"));
        ui.label(self.localization.tr("team_workspace.intro"));
        ui.add_space(8.0);

        let mut refresh_requested = false;
        let mut selection_changed = false;
        let mut condition_probe_requested = false;
        let mut team_data_probe_requested = false;
        let mut team_management_requested = false;
        let mut team_history_requested = false;

        ui.horizontal(|ui| {
            ui.label(self.localization.tr("team_workspace.search"));
            ui.add(
                egui::TextEdit::singleline(&mut self.team_workspace_search)
                    .desired_width(260.0)
                    .hint_text(self.localization.tr("team_workspace.search_hint")),
            );
            if ui.button(self.localization.tr("common.clear")).clicked() {
                self.team_workspace_search.clear();
            }
        });

        let query = self.team_workspace_search.trim().to_lowercase();
        let team_options = self
            .teams
            .iter()
            .filter(|team| query.is_empty() || team.matches_search(&query))
            .map(|team| (team.id, team.localized_label(&self.localization)))
            .collect::<Vec<_>>();

        ui.horizontal(|ui| {
            ui.label(self.localization.tr("common.team"));
            let selected_text = self
                .team_workspace_team_id
                .and_then(|team_id| self.teams.iter().find(|team| team.id == team_id))
                .map(|team| team.localized_label(&self.localization))
                .unwrap_or_else(|| self.localization.tr("common.select_team"));

            egui::ComboBox::from_id_salt("team_workspace_team_selector")
                .selected_text(selected_text)
                .width(320.0)
                .show_ui(ui, |ui| {
                    for (team_id, label) in &team_options {
                        if ui
                            .selectable_value(
                                &mut self.team_workspace_team_id,
                                Some(*team_id),
                                label,
                            )
                            .changed()
                        {
                            selection_changed = true;
                        }
                    }
                });

            let count = self.teams.len().to_string();
            ui.label(self.localization.tr_with(
                "common.total_count",
                &[("count", count.as_str())],
            ));

            if ui
                .add_enabled(
                    self.connected,
                    egui::Button::new(self.localization.tr("team_workspace.refresh_data")),
                )
                .clicked()
            {
                refresh_requested = true;
            }
        });

        ui.add_space(6.0);

        let Some(team) = self
            .team_workspace_team_id
            .and_then(|team_id| self.teams.iter().find(|team| team.id == team_id))
            .cloned()
        else {
            ui.weak(self.localization.tr("team_workspace.no_team"));
            if refresh_requested {
                self.refresh_players();
                self.refresh_staff();
                self.refresh_teams();
            }
            return;
        };

        if selection_changed {
            self.team_roster_selected_player_id = None;
            self.team_staff_selected_staff_id = None;
            self.team_condition_window_open = false;
            self.team_condition_entries.clear();
            self.team_condition_team_id = None;
            self.team_condition_selected_player_ids.clear();
            self.team_data_probe_window_open = false;
            self.team_data_probe_team_id = None;
            self.team_data_probe_raw.clear();
            self.team_management_data = None;
            self.team_management_last_request_team_id = None;
            self.team_history_data = None;
            self.team_history_last_request_team_id = None;
            self.status = format!("Team data loaded: {}", team.display_name);
        }

        let loaded_roster_count = self
            .players
            .iter()
            .filter(|player| summary_belongs_to_team(&player.team, &team))
            .count();
        let loaded_staff_count = self
            .staffs
            .iter()
            .filter(|staff| summary_belongs_to_team(&staff.team, &team))
            .count();
        let management_data = self
            .team_management_data
            .as_ref()
            .filter(|data| data.team_id == team.id)
            .cloned();
        let management_auto_request =
            self.team_management_last_request_team_id != Some(team.id);
        let history_data = self
            .team_history_data
            .as_ref()
            .filter(|data| data.team_id == team.id)
            .cloned();
        let history_auto_request = (self.team_match_history_window_open
            || self.team_pre_match_analysis_window_open
            || self.team_history_summary_window_open)
            && self.team_history_last_request_team_id != Some(team.id);

        ui.horizontal_wrapped(|ui| {
            if ui.button(self.localization.tr("team_workspace.open_roster")).clicked() {
                self.team_roster_window_open = true;
            }
            if ui.button(self.localization.tr("team_workspace.open_staff")).clicked() {
                self.team_staff_window_open = true;
            }
            if ui
                .button(self.localization.tr("team_workspace.open_condition_probe"))
                .clicked()
            {
                condition_probe_requested = true;
            }
            if ui
                .button(self.localization.tr("team_workspace.open_strategy"))
                .clicked()
            {
                self.team_strategy_window_open = true;
                team_management_requested = management_data.is_none();
            }
            if ui
                .button(self.localization.tr("team_workspace.open_merchandise"))
                .clicked()
            {
                self.team_merchandise_window_open = true;
                team_management_requested = management_data.is_none();
            }
            if ui
                .button(self.localization.tr("team_workspace.open_champion_setup"))
                .clicked()
            {
                self.team_champion_setup_window_open = true;
                team_management_requested = management_data.is_none();
            }
            if ui
                .button(self.localization.tr("team_workspace.open_gaming_house"))
                .clicked()
            {
                self.team_gaming_house_window_open = true;
                team_management_requested = management_data.is_none();
            }
            if ui
                .button(self.localization.tr("team_workspace.open_match_history"))
                .clicked()
            {
                self.team_match_history_window_open = true;
                team_history_requested = history_data.is_none();
            }
            if ui
                .button(self.localization.tr("team_workspace.open_pre_match_analysis"))
                .clicked()
            {
                self.team_pre_match_analysis_window_open = true;
                team_history_requested = history_data.is_none();
            }
            if ui
                .button(self.localization.tr("team_workspace.open_history_summary"))
                .clicked()
            {
                self.team_history_summary_window_open = true;
                team_history_requested = history_data.is_none();
            }
            if ui
                .button(self.localization.tr("team_workspace.open_data_probe"))
                .clicked()
            {
                team_data_probe_requested = true;
            }
            ui.separator();
            ui.weak(self.localization.tr("team_workspace.read_only"));
        });

        ui.add_space(10.0);

        {
        let render_overview = |ui: &mut egui::Ui| {
            ui.group(|ui| {
                ui.set_min_width(420.0);
                ui.strong(self.localization.tr("team_workspace.overview"));
                ui.add_space(6.0);
                egui::Grid::new("team_workspace_overview_grid")
                    .num_columns(2)
                    .spacing(egui::vec2(24.0, 6.0))
                    .show(ui, |ui| {
                        ui.label(self.localization.tr("team_workspace.team_name"));
                        ui.label(value_or_dash(&team.display_name));
                        ui.end_row();

                        ui.label(self.localization.tr("team_workspace.team_id"));
                        ui.label(team.id.to_string());
                        ui.end_row();

                        ui.label(self.localization.tr("search.teams.league"));
                        let league_id = team.league_id.to_string();
                        ui.label(self.localization.tr_with(
                            "common.league_number",
                            &[("id", league_id.as_str())],
                        ));
                        ui.end_row();

                        ui.label(self.localization.tr("search.teams.manager"));
                        ui.label(value_or_dash(&team.manager_name));
                        ui.end_row();

                        ui.label(self.localization.tr("search.teams.player_team"));
                        ui.label(if team.is_player_team {
                            self.localization.tr("common.my_team")
                        } else {
                            "—".to_string()
                        });
                        ui.end_row();

                        ui.label(self.localization.tr("team_workspace.roster_size"));
                        ui.label(format!("{loaded_roster_count} / {}", team.roster_size));
                        ui.end_row();

                        ui.label(self.localization.tr("team_workspace.staff_count"));
                        ui.label(format!("{loaded_staff_count} / {}", team.staff_count));
                        ui.end_row();

                        ui.label(self.localization.tr("search.teams.roster_rating"));
                        ui.label(
                            team.roster_rating
                                .map(|value| format!("{value:.1}"))
                                .unwrap_or_else(|| "—".to_string()),
                        );
                        ui.end_row();
                    });
            });
        };

        let render_finance = |ui: &mut egui::Ui| {
            ui.group(|ui| {
                ui.set_min_width(420.0);
                ui.strong(self.localization.tr("team_workspace.finance"));
                ui.add_space(6.0);
                egui::Grid::new("team_workspace_finance_grid")
                    .num_columns(2)
                    .spacing(egui::vec2(24.0, 6.0))
                    .show(ui, |ui| {
                        ui.label(self.localization.tr("economy.money"));
                        ui.label(format_internal_amount(&team.total_balance.to_string()));
                        ui.end_row();

                        ui.label(self.localization.tr("economy.transfer_budget"));
                        ui.label(format_internal_amount(&team.transfer_budget.to_string()));
                        ui.end_row();

                        ui.label(self.localization.tr("economy.salary_budget"));
                        ui.label(format_internal_amount(&team.salary_budget.to_string()));
                        ui.end_row();
                    });
            });
        };

        let render_stadium = |ui: &mut egui::Ui| {
            ui.group(|ui| {
                ui.set_min_width(420.0);
                ui.strong(self.localization.tr("team_workspace.stadium"));
                ui.add_space(6.0);
                egui::Grid::new("team_workspace_stadium_grid")
                    .num_columns(2)
                    .spacing(egui::vec2(24.0, 6.0))
                    .show(ui, |ui| {
                        ui.label(self.localization.tr("team_workspace.stadium_name"));
                        ui.label(value_or_dash(&team.stadium_name));
                        ui.end_row();

                        ui.label(self.localization.tr("search.teams.stadium_grade"));
                        ui.label(display_facility_grade(&team.stadium_grade));
                        ui.end_row();

                        ui.label(self.localization.tr("team_workspace.stadium_capacity"));
                        ui.label(value_or_dash(&team.stadium_capacity));
                        ui.end_row();

                        ui.label(self.localization.tr("team_workspace.average_attendance"));
                        ui.label(
                            team.average_home_attendance()
                                .map(|value| format!("{value:.0}"))
                                .unwrap_or_else(|| "—".to_string()),
                        );
                        ui.end_row();

                        ui.label(self.localization.tr("team_workspace.total_entrance_income"));
                        ui.label(format_internal_amount(
                            &team.total_entrance_income.to_string(),
                        ));
                        ui.end_row();
                    });
            });
        };

        let render_fans = |ui: &mut egui::Ui| {
            ui.group(|ui| {
                ui.set_min_width(420.0);
                ui.strong(self.localization.tr("team_workspace.fans"));
                ui.add_space(6.0);
                egui::Grid::new("team_workspace_fans_grid")
                    .num_columns(2)
                    .spacing(egui::vec2(24.0, 6.0))
                    .show(ui, |ui| {
                        ui.label(self.localization.tr("team_workspace.popularity"));
                        ui.label(value_or_dash(&team.popularity));
                        ui.end_row();

                        ui.label(self.localization.tr("team_workspace.fan_count"));
                        ui.label(value_or_dash(&team.fan_count));
                        ui.end_row();

                        ui.label(self.localization.tr("team_workspace.fan_expectation"));
                        ui.label(value_or_dash(&team.fan_expectation));
                        ui.end_row();

                        ui.label(self.localization.tr("team_workspace.fan_satisfaction"));
                        ui.label(value_or_dash(&team.fan_satisfaction));
                        ui.end_row();

                        ui.label(self.localization.tr("team_workspace.fan_momentum"));
                        ui.label(value_or_dash(&team.fan_momentum));
                        ui.end_row();
                    });
            });
        };

        let render_facilities = |ui: &mut egui::Ui| {
            ui.group(|ui| {
                ui.set_min_width(420.0);
                ui.strong(self.localization.tr("team_workspace.facilities"));
                ui.add_space(6.0);
                egui::Grid::new("team_workspace_facilities_grid")
                    .num_columns(2)
                    .spacing(egui::vec2(24.0, 6.0))
                    .show(ui, |ui| {
                        ui.label(
                            self.localization
                                .tr("search.teams.merchandise_facility_grade"),
                        );
                        ui.label(display_facility_grade(
                            &team.merchandise_facility_grade,
                        ));
                        ui.end_row();

                        ui.label(
                            self.localization
                                .tr("search.teams.training_facility_grade"),
                        );
                        ui.label(display_facility_grade(&team.training_facility_grade));
                        ui.end_row();

                        ui.label(self.localization.tr("team_workspace.gaming_house_level"));
                        ui.label(value_or_dash(&team.gaming_house_level));
                        ui.end_row();

                        ui.label(self.localization.tr("team_workspace.welfare"));
                        ui.label(value_or_dash(&team.welfare));
                        ui.end_row();
                    });
            });
        };

        let render_management = |ui: &mut egui::Ui| {
            ui.group(|ui| {
                ui.set_min_width(420.0);
                ui.strong(self.localization.tr("team_workspace.management"));
                ui.add_space(6.0);
                if let Some(data) = management_data.as_ref() {
                    egui::Grid::new("team_workspace_management_grid")
                        .num_columns(2)
                        .spacing(egui::vec2(24.0, 6.0))
                        .show(ui, |ui| {
                            ui.label(self.localization.tr("team_workspace.last_starting"));
                            ui.add(
                                egui::Label::new(format_team_lineup(&data.lineup)).wrap(),
                            );
                            ui.end_row();

                            ui.label(self.localization.tr("team_workspace.watched_players"));
                            ui.label(data.watched_players.len().to_string()).on_hover_text(
                                format_team_member_references(&data.watched_players),
                            );
                            ui.end_row();

                            ui.label(self.localization.tr("team_workspace.no_transfer_players"));
                            ui.label(data.no_transfer_players.len().to_string()).on_hover_text(
                                format_team_member_references(&data.no_transfer_players),
                            );
                            ui.end_row();

                            ui.label(self.localization.tr("team_workspace.release_players"));
                            ui.label(data.release_players.len().to_string()).on_hover_text(
                                format_team_member_references(&data.release_players),
                            );
                            ui.end_row();

                            ui.label(self.localization.tr("team_workspace.watched_staff"));
                            ui.label(data.watched_staff.len().to_string()).on_hover_text(
                                format_team_member_references(&data.watched_staff),
                            );
                            ui.end_row();

                            ui.label(self.localization.tr("team_workspace.release_staff"));
                            ui.label(data.release_staff.len().to_string()).on_hover_text(
                                format_team_member_references(&data.release_staff),
                            );
                            ui.end_row();

                            ui.label(self.localization.tr("team_workspace.scout_dispatch"));
                            ui.label(value_or_dash(&data.scout_dispatch));
                            ui.end_row();

                            ui.label(self.localization.tr("team_workspace.pending_installments"));
                            ui.label(data.pending_installments.to_string());
                            ui.end_row();

                            ui.label(self.localization.tr("team_workspace.resale_clauses"));
                            ui.label(data.resale_clauses.to_string());
                            ui.end_row();

                            ui.label(self.localization.tr("team_workspace.merchandise_products"));
                            ui.label(data.merchandise_product_count.to_string());
                            ui.end_row();

                            ui.label(self.localization.tr("team_workspace.champion_tiers"));
                            ui.label(data.champion_tier_count.to_string());
                            ui.end_row();

                            ui.label(self.localization.tr("team_workspace.personal_tactics"));
                            ui.label(data.personal_tactic_count.to_string());
                            ui.end_row();
                        });
                } else {
                    ui.weak(self.localization.tr("team_workspace.management_loading"));
                }
            });
        };

        if ui.available_width() >= 900.0 {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                ui.vertical(|ui| {
                    render_overview(ui);
                    ui.add_space(12.0);
                    render_management(ui);
                    ui.add_space(12.0);
                    render_stadium(ui);
                });
                ui.add_space(16.0);
                ui.vertical(|ui| {
                    render_finance(ui);
                    ui.add_space(12.0);
                    render_fans(ui);
                    ui.add_space(12.0);
                    render_facilities(ui);
                });
            });
        } else {
            render_overview(ui);
            ui.add_space(12.0);
            render_management(ui);
            ui.add_space(12.0);
            render_finance(ui);
            ui.add_space(12.0);
            render_stadium(ui);
            ui.add_space(12.0);
            render_fans(ui);
            ui.add_space(12.0);
            render_facilities(ui);
        }
        }

        ui.add_space(10.0);
        ui.weak(self.localization.tr("team_workspace.current_scope"));

        if condition_probe_requested {
            self.refresh_team_condition_probe();
        }
        if team_data_probe_requested {
            self.refresh_team_data_probe();
        }

        if refresh_requested {
            let selected_team_id = self.team_workspace_team_id;
            self.refresh_players();
            self.refresh_staff();
            self.refresh_teams();
            if selected_team_id.is_some_and(|team_id| {
                self.teams.iter().any(|candidate| candidate.id == team_id)
            }) {
                self.team_workspace_team_id = selected_team_id;
            }
            if let Some(selected_team) = self
                .team_workspace_team_id
                .and_then(|team_id| self.teams.iter().find(|candidate| candidate.id == team_id))
            {
                self.status = format!("Team data loaded: {}", selected_team.display_name);
            }
        }

        if refresh_requested
            || selection_changed
            || management_auto_request
            || team_management_requested
        {
            self.refresh_team_management_data();
        }
        if team_history_requested || history_auto_request {
            self.refresh_team_history_data();
        }
    }

    #[cfg(feature = "dev")]
    fn render_team_roster_window(&mut self, ctx: &egui::Context) {
        if !self.team_roster_window_open {
            return;
        }

        let Some(team) = self
            .team_workspace_team_id
            .and_then(|team_id| self.teams.iter().find(|team| team.id == team_id))
            .cloned()
        else {
            self.team_roster_window_open = false;
            self.team_roster_selected_player_id = None;
            return;
        };

        let mut roster = self
            .players
            .iter()
            .filter(|player| summary_belongs_to_team(&player.team, &team))
            .cloned()
            .collect::<Vec<_>>();
        roster.sort_by_key(|player| player.name.to_lowercase());

        let valid_player_ids = roster.iter().map(|player| player.id).collect::<BTreeSet<_>>();
        let mut selected_player_id = self
            .team_roster_selected_player_id
            .filter(|player_id| valid_player_ids.contains(player_id));
        let mut open_player_id = None;
        let mut open = self.team_roster_window_open;
        let title = self.localization.tr_with(
            "team_workspace.roster_window_title",
            &[("team", team.display_name.as_str())],
        );
        let window_id = egui::Id::new("team_workspace_roster_window_v051e");
        let default_window_size = egui::vec2(1080.0, 520.0);

        egui::Window::new(title)
            .id(window_id)
            .open(&mut open)
            .resizable(true)
            .default_size(default_window_size)
            .min_size(egui::vec2(680.0, 320.0))
            .constrain(true)
            .show(ctx, |ui| {
                let loaded = roster.len().to_string();
                let reported = team.roster_size.to_string();
                ui.label(self.localization.tr_with(
                    "team_workspace.members_loaded",
                    &[("loaded", loaded.as_str()), ("reported", reported.as_str())],
                ));
                ui.weak(self.localization.tr("team_workspace.roster_source"));
                ui.separator();

                // The shared viewport owns all remaining window space. Data changes
                // update only the rows inside it and cannot resize the outer window.
                let widths = [190.0, 70.0, 70.0, 105.0, 145.0, 110.0, 115.0, 120.0];
                let table_min_width = widths.iter().copied().sum::<f32>() + 48.0;

                render_team_member_table_viewport(
                    ui,
                    "team_workspace_roster_horizontal_v051e",
                    table_min_width,
                    |ui, table_height| {
                        if roster.is_empty() {
                            ui.weak(self.localization.tr("team_workspace.no_roster_members"));
                            return;
                        }

                        let mut table = TableBuilder::new(ui)
                            .id_salt("team_workspace_roster_table")
                            .striped(true)
                            .resizable(true)
                            .sense(egui::Sense::click())
                            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                            .min_scrolled_height(0.0)
                            .max_scroll_height(table_height)
                            .auto_shrink([false, false]);
                        for width in widths {
                            table = table.column(
                                Column::initial(width)
                                    .at_least(58.0)
                                    .clip(true)
                                    .resizable(true),
                            );
                        }

                        table
                            .header(24.0, |mut header| {
                                header.col(|ui| {
                                    ui.strong(self.localization.tr("common.name"));
                                });
                                header.col(|ui| {
                                    ui.strong(self.localization.tr("common.id"));
                                });
                                header.col(|ui| {
                                    ui.strong(self.localization.tr("common.age"));
                                });
                                header.col(|ui| {
                                    ui.strong(self.localization.tr("search.columns.salary"));
                                });
                                header.col(|ui| {
                                    ui.strong(self.localization.tr("search.players.position"));
                                });
                                header.col(|ui| {
                                    ui.strong(self.localization.tr("search.columns.actual_rating"));
                                });
                                header.col(|ui| {
                                    ui.strong(self.localization.tr("search.players.actual_potential"));
                                });
                                header.col(|ui| {
                                    ui.strong(self.localization.tr("contract.end"));
                                });
                            })
                            .body(|body| {
                                body.rows(24.0, roster.len(), |mut row| {
                                    let player = &roster[row.index()];
                                    row.set_selected(selected_player_id == Some(player.id));
                                    row.col(|ui| {
                                        ui.label(&player.name);
                                    });
                                    row.col(|ui| {
                                        ui.label(player.id.to_string());
                                    });
                                    row.col(|ui| {
                                        ui.label(value_or_dash(&player.age));
                                    });
                                    row.col(|ui| {
                                        ui.label(pretty_or_dash(&player.salary));
                                    });
                                    row.col(|ui| {
                                        ui.label(localized_position_summary(
                                            &self.localization,
                                            &player.position,
                                        ));
                                    });
                                    row.col(|ui| {
                                        render_actual_rating(ui, player, &self.localization);
                                    });
                                    row.col(|ui| {
                                        ui.label(pretty_or_dash(&player.actual_potential));
                                    });
                                    row.col(|ui| {
                                        ui.label(value_or_dash(&display_contract_date(
                                            &player.contract_end,
                                        )));
                                    });

                                    let row_response = row.response();
                                    if row_response.double_clicked() {
                                        selected_player_id = Some(player.id);
                                        open_player_id = Some(player.id);
                                    } else if row_response.clicked() {
                                        selected_player_id = Some(player.id);
                                    }
                                    row_response.context_menu(|ui| {
                                        if ui
                                            .button(
                                                self.localization
                                                    .tr("search.open_in_player_editor"),
                                            )
                                            .clicked()
                                        {
                                            selected_player_id = Some(player.id);
                                            open_player_id = Some(player.id);
                                            ui.close_menu();
                                        }
                                    });
                                });
                            });
                    },
                );
            });

        self.team_roster_selected_player_id = selected_player_id;
        self.team_roster_window_open = open;
        if let Some(player_id) = open_player_id {
            self.team_roster_window_open = false;
            self.open_player_in_editor(player_id);
        }
    }

    #[cfg(feature = "dev")]
    fn render_team_staff_window(&mut self, ctx: &egui::Context) {
        if !self.team_staff_window_open {
            return;
        }

        let Some(team) = self
            .team_workspace_team_id
            .and_then(|team_id| self.teams.iter().find(|team| team.id == team_id))
            .cloned()
        else {
            self.team_staff_window_open = false;
            self.team_staff_selected_staff_id = None;
            return;
        };

        let mut staff_members = self
            .staffs
            .iter()
            .filter(|staff| summary_belongs_to_team(&staff.team, &team))
            .cloned()
            .collect::<Vec<_>>();
        staff_members.sort_by_key(|staff| staff.name.to_lowercase());

        let valid_staff_ids = staff_members
            .iter()
            .map(|staff| staff.id)
            .collect::<BTreeSet<_>>();
        let mut selected_staff_id = self
            .team_staff_selected_staff_id
            .filter(|staff_id| valid_staff_ids.contains(staff_id));
        let mut open_staff_id = None;
        let mut open = self.team_staff_window_open;
        let title = self.localization.tr_with(
            "team_workspace.staff_window_title",
            &[("team", team.display_name.as_str())],
        );
        let window_id = egui::Id::new("team_workspace_staff_window_v051e");
        let default_window_size = egui::vec2(820.0, 480.0);

        egui::Window::new(title)
            .id(window_id)
            .open(&mut open)
            .resizable(true)
            .default_size(default_window_size)
            .min_size(egui::vec2(560.0, 300.0))
            .constrain(true)
            .show(ctx, |ui| {
                let loaded = staff_members.len().to_string();
                let reported = team.staff_count.to_string();
                ui.label(self.localization.tr_with(
                    "team_workspace.members_loaded",
                    &[("loaded", loaded.as_str()), ("reported", reported.as_str())],
                ));
                ui.weak(self.localization.tr("team_workspace.staff_source"));
                ui.separator();

                // Roster and Staff use the same fixed viewport helper, so member count,
                // refreshes, and team changes cannot alter the user-controlled window size.
                let widths = [210.0, 70.0, 70.0, 120.0, 170.0, 130.0];
                let table_min_width = widths.iter().copied().sum::<f32>() + 40.0;

                render_team_member_table_viewport(
                    ui,
                    "team_workspace_staff_horizontal_v051e",
                    table_min_width,
                    |ui, table_height| {
                        if staff_members.is_empty() {
                            ui.weak(self.localization.tr("team_workspace.no_staff_members"));
                            return;
                        }

                        let mut table = TableBuilder::new(ui)
                            .id_salt("team_workspace_staff_table")
                            .striped(true)
                            .resizable(true)
                            .sense(egui::Sense::click())
                            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                            .min_scrolled_height(0.0)
                            .max_scroll_height(table_height)
                            .auto_shrink([false, false]);
                        for width in widths {
                            table = table.column(
                                Column::initial(width)
                                    .at_least(58.0)
                                    .clip(true)
                                    .resizable(true),
                            );
                        }

                        table
                            .header(24.0, |mut header| {
                                header.col(|ui| {
                                    ui.strong(self.localization.tr("common.name"));
                                });
                                header.col(|ui| {
                                    ui.strong(self.localization.tr("common.id"));
                                });
                                header.col(|ui| {
                                    ui.strong(self.localization.tr("common.age"));
                                });
                                header.col(|ui| {
                                    ui.strong(self.localization.tr("search.columns.salary"));
                                });
                                header.col(|ui| {
                                    ui.strong(self.localization.tr("common.role"));
                                });
                                header.col(|ui| {
                                    ui.strong(self.localization.tr("contract.end"));
                                });
                            })
                            .body(|body| {
                                body.rows(24.0, staff_members.len(), |mut row| {
                                    let staff = &staff_members[row.index()];
                                    row.set_selected(selected_staff_id == Some(staff.id));
                                    row.col(|ui| {
                                        ui.label(&staff.name);
                                    });
                                    row.col(|ui| {
                                        ui.label(staff.id.to_string());
                                    });
                                    row.col(|ui| {
                                        ui.label(value_or_dash(&staff.age));
                                    });
                                    row.col(|ui| {
                                        ui.label(pretty_or_dash(&staff.annual_salary));
                                    });
                                    row.col(|ui| {
                                        ui.label(localized_staff_role(
                                            &self.localization,
                                            &staff.role,
                                        ));
                                    });
                                    row.col(|ui| {
                                        ui.label(value_or_dash(&display_contract_date(
                                            &staff.contract_end,
                                        )));
                                    });

                                    let row_response = row.response();
                                    if row_response.double_clicked() {
                                        selected_staff_id = Some(staff.id);
                                        open_staff_id = Some(staff.id);
                                    } else if row_response.clicked() {
                                        selected_staff_id = Some(staff.id);
                                    }
                                    row_response.context_menu(|ui| {
                                        if ui
                                            .button(
                                                self.localization
                                                    .tr("search.open_in_staff_editor"),
                                            )
                                            .clicked()
                                        {
                                            selected_staff_id = Some(staff.id);
                                            open_staff_id = Some(staff.id);
                                            ui.close_menu();
                                        }
                                    });
                                });
                            });
                    },
                );
            });

        self.team_staff_selected_staff_id = selected_staff_id;
        self.team_staff_window_open = open;
        if let Some(staff_id) = open_staff_id {
            self.team_staff_window_open = false;
            self.open_staff_in_editor(staff_id);
        }
    }

    #[cfg(feature = "dev")]
    fn current_team_management_context(&self) -> Option<(TeamSummary, TeamManagementData)> {
        let team = self
            .team_workspace_team_id
            .and_then(|team_id| self.teams.iter().find(|team| team.id == team_id))
            .cloned()?;
        let data = self
            .team_management_data
            .as_ref()
            .filter(|data| data.team_id == team.id)
            .cloned()?;
        Some((team, data))
    }

    #[cfg(feature = "dev")]
    fn current_team_history_context(&self) -> Option<(TeamSummary, Option<TeamHistoryData>)> {
        let team = self
            .team_workspace_team_id
            .and_then(|team_id| self.teams.iter().find(|team| team.id == team_id))
            .cloned()?;
        let data = self
            .team_history_data
            .as_ref()
            .filter(|data| data.team_id == team.id)
            .cloned();
        Some((team, data))
    }

    #[cfg(feature = "dev")]
    fn render_team_match_history_window(&mut self, ctx: &egui::Context) {
        if !self.team_match_history_window_open {
            return;
        }

        let Some((team, data)) = self.current_team_history_context() else {
            self.team_match_history_window_open = false;
            return;
        };

        let mut open = self.team_match_history_window_open;
        let mut refresh_requested = false;
        let title = self.localization.tr_with(
            "team_match_history.window_title",
            &[("team", team.display_name.as_str())],
        );

        egui::Window::new(title)
            .id(egui::Id::new("team_match_history_window_v057"))
            .open(&mut open)
            .resizable(true)
            .default_size(egui::vec2(1080.0, 640.0))
            .min_size(egui::vec2(720.0, 400.0))
            .constrain(true)
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.button(self.localization.tr("common.refresh")).clicked() {
                        refresh_requested = true;
                    }
                    ui.weak(self.localization.tr("team_match_history.help"));
                });
                ui.separator();

                let Some(data) = data.as_ref() else {
                    ui.weak(self.localization.tr("team_history.loading"));
                    return;
                };

                ui.horizontal_wrapped(|ui| {
                    let match_count = data.matches.len().to_string();
                    let match_wins = data.wins().to_string();
                    let match_losses = data.losses().to_string();
                    let set_wins = data.set_wins().to_string();
                    let set_losses = data.set_losses().to_string();
                    ui.label(self.localization.tr_with(
                        "team_match_history.matches_count",
                        &[("count", match_count.as_str())],
                    ));
                    ui.separator();
                    ui.label(self.localization.tr_with(
                        "team_match_history.record_value",
                        &[
                            ("wins", match_wins.as_str()),
                            ("losses", match_losses.as_str()),
                        ],
                    ));
                    ui.separator();
                    ui.label(self.localization.tr_with(
                        "team_match_history.set_record_value",
                        &[
                            ("wins", set_wins.as_str()),
                            ("losses", set_losses.as_str()),
                        ],
                    ));
                });
                ui.add_space(6.0);

                if data.matches.is_empty() {
                    ui.weak(self.localization.tr("team_match_history.empty"));
                    return;
                }

                egui::ScrollArea::vertical()
                    .id_salt("team_match_history_scroll_v057")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for entry in data.matches.iter().rev() {
                            let result = if entry.is_win {
                                self.localization.tr("team_history.win")
                            } else {
                                self.localization.tr("team_history.loss")
                            };
                            let header = format!(
                                "{} · {} {}-{} · {} · {}",
                                entry.date,
                                result,
                                entry.my_score,
                                entry.enemy_score,
                                entry.opponent_name,
                                entry.article_pattern
                            );
                            egui::CollapsingHeader::new(header)
                                .id_salt(("team_match_history_entry_v057", entry.match_id))
                                .default_open(false)
                                .show(ui, |ui| {
                                    egui::Grid::new((
                                        "team_match_history_details_grid_v057",
                                        entry.match_id,
                                    ))
                                    .num_columns(2)
                                    .spacing(egui::vec2(24.0, 6.0))
                                    .show(ui, |ui| {
                                        ui.label(self.localization.tr("team_history.match_id"));
                                        ui.label(entry.match_id.to_string());
                                        ui.end_row();

                                        ui.label(self.localization.tr("team_history.opponent"));
                                        ui.label(format!(
                                            "{} (ID {})",
                                            entry.opponent_name, entry.opponent_id
                                        ));
                                        ui.end_row();

                                        ui.label(self.localization.tr("team_history.match_type"));
                                        ui.label(if entry.is_practice {
                                            self.localization.tr("team_history.practice")
                                        } else {
                                            self.localization.tr("team_history.official")
                                        });
                                        ui.end_row();

                                        ui.label(self.localization.tr("team_history.match_pattern"));
                                        ui.label(value_or_dash(&entry.article_pattern));
                                        ui.end_row();
                                    });

                                    ui.add_space(8.0);
                                    ui.strong(self.localization.tr("team_match_history.set_details"));
                                    egui::Grid::new((
                                        "team_match_history_sets_grid_v057",
                                        entry.match_id,
                                    ))
                                    .striped(true)
                                    .num_columns(8)
                                    .spacing(egui::vec2(16.0, 5.0))
                                    .show(ui, |ui| {
                                        for key in [
                                            "team_match_history.set",
                                            "team_match_history.pattern",
                                            "team_match_history.kills",
                                            "team_match_history.gold",
                                            "team_match_history.mvp",
                                            "team_match_history.champion",
                                            "team_match_history.kda",
                                            "team_match_history.side",
                                        ] {
                                            ui.strong(self.localization.tr(key));
                                        }
                                        ui.end_row();

                                        for set in &entry.sets {
                                            ui.label(set.set_number.to_string());
                                            ui.label(value_or_dash(&set.pattern));
                                            ui.label(format!(
                                                "{}-{}",
                                                set.team1_kills, set.team2_kills
                                            ));
                                            ui.label(format!(
                                                "{}-{}",
                                                set.team1_gold, set.team2_gold
                                            ));
                                            ui.label(format!(
                                                "{} ({})",
                                                set.mvp_player_name, set.mvp_player_id
                                            ));
                                            ui.label(champion_display_name(&set.mvp_champion_id))
                                                .on_hover_text(set.mvp_champion_id.clone());
                                            ui.label(format!(
                                                "{}/{}/{}",
                                                set.mvp_kills,
                                                set.mvp_deaths,
                                                set.mvp_assists
                                            ));
                                            let mut side = if set.was_blue_side {
                                                self.localization.tr("team_match_history.blue")
                                            } else {
                                                self.localization.tr("team_match_history.red")
                                            };
                                            if set.was_comeback {
                                                side.push_str(" · ");
                                                side.push_str(
                                                    &self.localization.tr(
                                                        "team_match_history.comeback",
                                                    ),
                                                );
                                            }
                                            ui.label(side);
                                            ui.end_row();
                                        }
                                    });
                                });
                            ui.add_space(4.0);
                        }
                    });
            });

        self.team_match_history_window_open = open;
        if refresh_requested {
            self.refresh_team_history_data();
        }
    }

    #[cfg(feature = "dev")]
    fn render_team_pre_match_analysis_window(&mut self, ctx: &egui::Context) {
        if !self.team_pre_match_analysis_window_open {
            return;
        }

        let Some((team, data)) = self.current_team_history_context() else {
            self.team_pre_match_analysis_window_open = false;
            return;
        };

        let mut open = self.team_pre_match_analysis_window_open;
        let mut refresh_requested = false;
        let title = self.localization.tr_with(
            "team_pre_match.window_title",
            &[("team", team.display_name.as_str())],
        );

        egui::Window::new(title)
            .id(egui::Id::new("team_pre_match_analysis_window_v057"))
            .open(&mut open)
            .resizable(true)
            .default_size(egui::vec2(1040.0, 660.0))
            .min_size(egui::vec2(700.0, 420.0))
            .constrain(true)
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.button(self.localization.tr("common.refresh")).clicked() {
                        refresh_requested = true;
                    }
                    ui.weak(self.localization.tr("team_pre_match.help"));
                });
                ui.separator();

                let Some(data) = data.as_ref() else {
                    ui.weak(self.localization.tr("team_history.loading"));
                    return;
                };
                if data.analyses.is_empty() {
                    ui.weak(self.localization.tr("team_pre_match.empty"));
                    return;
                }

                egui::ScrollArea::vertical()
                    .id_salt("team_pre_match_analysis_scroll_v057")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for entry in data.analyses.iter().rev() {
                            let header = format!(
                                "{} · {} · Match {}",
                                entry.date, entry.opponent_name, entry.match_id
                            );
                            egui::CollapsingHeader::new(header)
                                .id_salt(("team_pre_match_entry_v057", entry.match_id))
                                .default_open(false)
                                .show(ui, |ui| {
                                    egui::Grid::new((
                                        "team_pre_match_summary_grid_v057",
                                        entry.match_id,
                                    ))
                                    .num_columns(2)
                                    .spacing(egui::vec2(24.0, 6.0))
                                    .show(ui, |ui| {
                                        ui.label(self.localization.tr("team_history.opponent"));
                                        ui.label(format!(
                                            "{} (ID {})",
                                            entry.opponent_name, entry.opponent_id
                                        ));
                                        ui.end_row();

                                        ui.label(self.localization.tr("team_pre_match.analysis_level"));
                                        ui.label(value_or_dash(&entry.analysis_level));
                                        ui.end_row();

                                        ui.label(self.localization.tr("team_pre_match.has_history"));
                                        ui.label(if entry.has_match_history {
                                            self.localization.tr("common.yes")
                                        } else {
                                            self.localization.tr("common.no")
                                        });
                                        ui.end_row();

                                        ui.label(self.localization.tr("team_pre_match.star_player"));
                                        ui.label(format!(
                                            "{} (ID {})",
                                            entry.star_player_name, entry.star_player_id
                                        ));
                                        ui.end_row();
                                    });

                                    ui.add_space(10.0);
                                    ui.strong(self.localization.tr("team_pre_match.tactics"));
                                    if entry.tactics.is_empty() {
                                        ui.weak(self.localization.tr("team_pre_match.no_tactics"));
                                    } else {
                                        egui::Grid::new((
                                            "team_pre_match_tactics_grid_v057",
                                            entry.match_id,
                                        ))
                                        .striped(true)
                                        .num_columns(2)
                                        .spacing(egui::vec2(24.0, 5.0))
                                        .show(ui, |ui| {
                                            ui.strong(self.localization.tr("team_pre_match.category"));
                                            ui.strong(self.localization.tr("team_pre_match.value"));
                                            ui.end_row();
                                            for tactic in &entry.tactics {
                                                ui.label(value_or_dash(&tactic.category));
                                                ui.label(value_or_dash(&tactic.value));
                                                ui.end_row();
                                            }
                                        });
                                    }

                                    ui.add_space(10.0);
                                    ui.strong(self.localization.tr("team_pre_match.champion_picks"));
                                    if entry.champion_picks.is_empty() {
                                        ui.weak(self.localization.tr("team_pre_match.no_champions"));
                                    } else {
                                        egui::Grid::new((
                                            "team_pre_match_champions_grid_v057",
                                            entry.match_id,
                                        ))
                                        .striped(true)
                                        .num_columns(4)
                                        .spacing(egui::vec2(24.0, 5.0))
                                        .show(ui, |ui| {
                                            ui.strong(self.localization.tr("team_pre_match.champion"));
                                            ui.strong(self.localization.tr("team_pre_match.position"));
                                            ui.strong(self.localization.tr("team_pre_match.wins"));
                                            ui.strong(self.localization.tr("team_pre_match.losses"));
                                            ui.end_row();
                                            for champion in &entry.champion_picks {
                                                ui.label(champion_display_name(&champion.champion_id))
                                                    .on_hover_text(champion.champion_id.clone());
                                                ui.label(value_or_dash(&champion.position));
                                                ui.label(champion.wins.to_string());
                                                ui.label(champion.losses.to_string());
                                                ui.end_row();
                                            }
                                        });
                                    }

                                    ui.add_space(10.0);
                                    ui.strong(self.localization.tr("team_pre_match.insights"));
                                    if entry.insights.is_empty() {
                                        ui.weak(self.localization.tr("team_pre_match.no_insights"));
                                    } else {
                                        for insight in &entry.insights {
                                            ui.group(|ui| {
                                                ui.horizontal_wrapped(|ui| {
                                                    ui.strong(&insight.section);
                                                    ui.label("·");
                                                    ui.label(&insight.label)
                                                        .on_hover_text(&insight.source_key);
                                                });
                                                if !insight.details.trim().is_empty() {
                                                    ui.weak(&insight.details);
                                                }
                                            });
                                        }
                                    }
                                });
                            ui.add_space(4.0);
                        }
                    });
            });

        self.team_pre_match_analysis_window_open = open;
        if refresh_requested {
            self.refresh_team_history_data();
        }
    }

    #[cfg(feature = "dev")]
    fn render_team_history_summary_window(&mut self, ctx: &egui::Context) {
        if !self.team_history_summary_window_open {
            return;
        }

        let Some((team, data)) = self.current_team_history_context() else {
            self.team_history_summary_window_open = false;
            return;
        };

        let mut open = self.team_history_summary_window_open;
        let mut refresh_requested = false;
        let title = self.localization.tr_with(
            "team_history_summary.window_title",
            &[("team", team.display_name.as_str())],
        );

        egui::Window::new(title)
            .id(egui::Id::new("team_history_summary_window_v057"))
            .open(&mut open)
            .resizable(true)
            .default_size(egui::vec2(680.0, 500.0))
            .min_size(egui::vec2(500.0, 340.0))
            .constrain(true)
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.button(self.localization.tr("common.refresh")).clicked() {
                        refresh_requested = true;
                    }
                    ui.weak(self.localization.tr("team_history_summary.help"));
                });
                ui.separator();

                let Some(data) = data.as_ref() else {
                    ui.weak(self.localization.tr("team_history.loading"));
                    return;
                };

                egui::ScrollArea::vertical()
                    .id_salt("team_history_summary_scroll_v057")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        egui::Grid::new("team_history_summary_grid_v057")
                            .num_columns(2)
                            .spacing(egui::vec2(30.0, 8.0))
                            .show(ui, |ui| {
                                let rows = [
                                    (
                                        "team_history_summary.registered_matches",
                                        data.matches.len().to_string(),
                                    ),
                                    (
                                        "team_history_summary.official_matches",
                                        data.official_matches().to_string(),
                                    ),
                                    (
                                        "team_history_summary.practice_matches",
                                        data.practice_matches().to_string(),
                                    ),
                                    (
                                        "team_history_summary.match_record",
                                        format!("{}-{}", data.wins(), data.losses()),
                                    ),
                                    (
                                        "team_history_summary.set_record",
                                        format!("{}-{}", data.set_wins(), data.set_losses()),
                                    ),
                                    (
                                        "team_history_summary.recent_form",
                                        data.recent_form(),
                                    ),
                                    (
                                        "team_history_summary.pre_match_analyses",
                                        data.analyses.len().to_string(),
                                    ),
                                    (
                                        "team_history_summary.latest_rating",
                                        data.latest_rating
                                            .map(|value| value.to_string())
                                            .unwrap_or_else(|| "—".to_string()),
                                    ),
                                    (
                                        "team_history_summary.latest_rank",
                                        data.latest_rank
                                            .map(|value| value.to_string())
                                            .unwrap_or_else(|| "—".to_string()),
                                    ),
                                    (
                                        "team_history_summary.rating_date",
                                        value_or_dash(&data.latest_rating_date),
                                    ),
                                ];
                                for (key, value) in rows {
                                    ui.label(self.localization.tr(key));
                                    ui.label(value);
                                    ui.end_row();
                                }
                            });

                        if data.matches.is_empty() {
                            ui.add_space(12.0);
                            ui.weak(self.localization.tr("team_match_history.empty"));
                        }
                    });
            });

        self.team_history_summary_window_open = open;
        if refresh_requested {
            self.refresh_team_history_data();
        }
    }

    #[cfg(feature = "dev")]
    fn render_team_data_probe_window(&mut self, ctx: &egui::Context) {
        if !self.team_data_probe_window_open {
            return;
        }

        let Some(team) = self
            .team_data_probe_team_id
            .filter(|team_id| self.team_workspace_team_id == Some(*team_id))
            .and_then(|team_id| self.teams.iter().find(|team| team.id == team_id))
            .cloned()
        else {
            self.team_data_probe_window_open = false;
            self.team_data_probe_team_id = None;
            self.team_data_probe_raw.clear();
            return;
        };

        let mut open = self.team_data_probe_window_open;
        let mut refresh_requested = false;
        let title = self.localization.tr_with(
            "team_data_probe.window_title",
            &[("team", team.display_name.as_str())],
        );
        let help = self.localization.tr("team_data_probe.help");
        let refresh_label = self.localization.tr("team_data_probe.refresh");
        let empty_label = self.localization.tr("team_data_probe.empty");

        egui::Window::new(title)
            .id(egui::Id::new("team_data_probe_window_v054"))
            .open(&mut open)
            .resizable(true)
            .default_size(egui::vec2(900.0, 600.0))
            .min_size(egui::vec2(620.0, 360.0))
            .constrain(true)
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.button(&refresh_label).clicked() {
                        refresh_requested = true;
                    }
                    ui.weak(&help);
                });
                ui.separator();

                StripBuilder::new(ui)
                    .clip(true)
                    .size(Size::remainder().at_least(220.0))
                    .vertical(|mut strip| {
                        strip.cell(|ui| {
                            let viewport_height = ui.available_height().max(1.0);
                            egui::ScrollArea::both()
                                .id_salt("team_data_probe_scroll_v054")
                                .auto_shrink([false, false])
                                .max_height(viewport_height)
                                .show(ui, |ui| {
                                    ui.set_min_width(860.0);
                                    if self.team_data_probe_raw.trim().is_empty() {
                                        ui.weak(&empty_label);
                                    } else {
                                        ui.add(
                                            egui::TextEdit::multiline(
                                                &mut self.team_data_probe_raw,
                                            )
                                            .code_editor()
                                            .desired_width(860.0)
                                            .desired_rows(32),
                                        );
                                    }
                                });
                        });
                    });
            });

        self.team_data_probe_window_open = open;
        if refresh_requested {
            self.refresh_team_data_probe();
        }
    }

    #[cfg(feature = "dev")]
    fn render_team_strategy_window(&mut self, ctx: &egui::Context) {
        if !self.team_strategy_window_open {
            return;
        }

        let Some((team, data)) = self.current_team_management_context() else {
            self.team_strategy_window_open = false;
            return;
        };

        let mut open = self.team_strategy_window_open;
        let title = self.localization.tr_with(
            "team_strategy.window_title",
            &[("team", team.display_name.as_str())],
        );

        egui::Window::new(title)
            .id(egui::Id::new("team_strategy_window_v056"))
            .open(&mut open)
            .resizable(true)
            .default_size(egui::vec2(900.0, 500.0))
            .min_size(egui::vec2(680.0, 340.0))
            .constrain(true)
            .show(ctx, |ui| {
                ui.weak(self.localization.tr("team_strategy.help"));
                ui.separator();

                let widths = [190.0, 210.0, 210.0, 210.0];
                let table_min_width = widths.iter().copied().sum::<f32>() + 40.0;
                render_team_member_table_viewport(
                    ui,
                    "team_strategy_horizontal_v056",
                    table_min_width,
                    |ui, table_height| {
                        let mut table = TableBuilder::new(ui)
                            .id_salt("team_strategy_table_v056")
                            .striped(true)
                            .resizable(true)
                            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                            .min_scrolled_height(0.0)
                            .max_scroll_height(table_height)
                            .auto_shrink([false, false]);
                        for width in widths {
                            table = table.column(
                                Column::initial(width)
                                    .at_least(110.0)
                                    .clip(true)
                                    .resizable(true),
                            );
                        }

                        table
                            .header(26.0, |mut header| {
                                header.col(|ui| {
                                    ui.strong(self.localization.tr("team_strategy.area"));
                                });
                                header.col(|ui| {
                                    ui.strong(self.localization.tr("team_strategy.current"));
                                });
                                header.col(|ui| {
                                    ui.strong(self.localization.tr("team_strategy.last"));
                                });
                                header.col(|ui| {
                                    ui.strong(self.localization.tr("team_strategy.team_color"));
                                });
                            })
                            .body(|body| {
                                body.rows(26.0, TEAM_STRATEGY_KEYS.len(), |mut row| {
                                    let key = TEAM_STRATEGY_KEYS[row.index()];
                                    let label_key = format!("team_strategy.{key}");
                                    row.col(|ui| {
                                        ui.label(self.localization.tr(&label_key));
                                    });
                                    row.col(|ui| {
                                        ui.label(team_strategy_value(&data.current_strategy, key));
                                    });
                                    row.col(|ui| {
                                        ui.label(team_strategy_value(&data.last_strategy, key));
                                    });
                                    row.col(|ui| {
                                        ui.label(team_strategy_value(
                                            &data.team_color_strategy,
                                            key,
                                        ));
                                    });
                                });
                            });
                    },
                );
            });

        self.team_strategy_window_open = open;
    }

    #[cfg(feature = "dev")]
    fn render_team_merchandise_window(&mut self, ctx: &egui::Context) {
        if !self.team_merchandise_window_open {
            return;
        }

        let Some((team, data)) = self.current_team_management_context() else {
            self.team_merchandise_window_open = false;
            return;
        };

        let total_stock = data
            .merchandise
            .iter()
            .map(|entry| parse_usize_value(&entry.stock))
            .sum::<usize>();
        let yearly_sales = data
            .merchandise
            .iter()
            .map(|entry| parse_usize_value(&entry.yearly_sales))
            .sum::<usize>();
        let yearly_revenue = data
            .merchandise
            .iter()
            .map(|entry| parse_f64_value(&entry.yearly_revenue))
            .sum::<f64>();
        let total_sales = data
            .merchandise
            .iter()
            .map(|entry| parse_usize_value(&entry.total_sales))
            .sum::<usize>();
        let total_revenue = data
            .merchandise
            .iter()
            .map(|entry| parse_f64_value(&entry.total_revenue))
            .sum::<f64>();
        let product_count_text = data.merchandise.len().to_string();
        let total_stock_text = total_stock.to_string();
        let yearly_sales_text = yearly_sales.to_string();
        let yearly_revenue_text = format_internal_amount(&yearly_revenue.to_string());
        let total_sales_text = total_sales.to_string();
        let total_revenue_text = format_internal_amount(&total_revenue.to_string());

        let mut open = self.team_merchandise_window_open;
        let title = self.localization.tr_with(
            "team_merchandise.window_title",
            &[("team", team.display_name.as_str())],
        );

        egui::Window::new(title)
            .id(egui::Id::new("team_merchandise_window_v056"))
            .open(&mut open)
            .resizable(true)
            .default_size(egui::vec2(1120.0, 560.0))
            .min_size(egui::vec2(720.0, 360.0))
            .constrain(true)
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(self.localization.tr_with(
                        "team_merchandise.product_count",
                        &[("count", product_count_text.as_str())],
                    ));
                    ui.separator();
                    ui.label(self.localization.tr_with(
                        "team_merchandise.total_stock",
                        &[("count", total_stock_text.as_str())],
                    ));
                    ui.separator();
                    ui.label(self.localization.tr_with(
                        "team_merchandise.yearly_sales_summary",
                        &[("count", yearly_sales_text.as_str())],
                    ));
                    ui.separator();
                    ui.label(self.localization.tr_with(
                        "team_merchandise.yearly_revenue_summary",
                        &[("amount", yearly_revenue_text.as_str())],
                    ));
                    ui.separator();
                    ui.label(self.localization.tr_with(
                        "team_merchandise.total_sales_summary",
                        &[("count", total_sales_text.as_str())],
                    ));
                    ui.separator();
                    ui.label(self.localization.tr_with(
                        "team_merchandise.total_revenue_summary",
                        &[("amount", total_revenue_text.as_str())],
                    ));
                });
                ui.weak(self.localization.tr("team_merchandise.help"));
                ui.separator();

                let widths = [90.0, 180.0, 70.0, 80.0, 105.0, 105.0, 125.0, 105.0, 125.0, 100.0];
                let table_min_width = widths.iter().copied().sum::<f32>() + 60.0;
                render_team_member_table_viewport(
                    ui,
                    "team_merchandise_horizontal_v056",
                    table_min_width,
                    |ui, table_height| {
                        if data.merchandise.is_empty() {
                            ui.weak(self.localization.tr("team_merchandise.empty"));
                            return;
                        }

                        let mut table = TableBuilder::new(ui)
                            .id_salt("team_merchandise_table_v056")
                            .striped(true)
                            .resizable(true)
                            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                            .min_scrolled_height(0.0)
                            .max_scroll_height(table_height)
                            .auto_shrink([false, false]);
                        for width in widths {
                            table = table.column(
                                Column::initial(width)
                                    .at_least(58.0)
                                    .clip(true)
                                    .resizable(true),
                            );
                        }

                        table
                            .header(26.0, |mut header| {
                                for key in [
                                    "team_merchandise.product_type",
                                    "common.player",
                                    "common.id",
                                    "team_merchandise.stock",
                                    "team_merchandise.sell_price",
                                    "team_merchandise.yearly_sales",
                                    "team_merchandise.yearly_revenue",
                                    "team_merchandise.total_sales",
                                    "team_merchandise.total_revenue",
                                    "team_merchandise.daily_rate",
                                ] {
                                    header.col(|ui| {
                                        ui.strong(self.localization.tr(key));
                                    });
                                }
                            })
                            .body(|body| {
                                body.rows(26.0, data.merchandise.len(), |mut row| {
                                    let entry = &data.merchandise[row.index()];
                                    row.col(|ui| {
                                        ui.label(self.localization.tr_with(
                                            "team_merchandise.type_value",
                                            &[("type", entry.product_type.as_str())],
                                        ));
                                    });
                                    row.col(|ui| {
                                        ui.label(value_or_dash(&entry.athlete_name));
                                    });
                                    row.col(|ui| {
                                        ui.label(entry.athlete_id.to_string());
                                    });
                                    row.col(|ui| {
                                        ui.label(value_or_dash(&entry.stock));
                                    });
                                    row.col(|ui| {
                                        ui.label(format_internal_amount(&entry.sell_price));
                                    });
                                    row.col(|ui| {
                                        ui.label(value_or_dash(&entry.yearly_sales));
                                    });
                                    row.col(|ui| {
                                        ui.label(format_internal_amount(&entry.yearly_revenue));
                                    });
                                    row.col(|ui| {
                                        ui.label(value_or_dash(&entry.total_sales));
                                    });
                                    row.col(|ui| {
                                        ui.label(format_internal_amount(&entry.total_revenue));
                                    });
                                    row.col(|ui| {
                                        ui.label(value_or_dash(&entry.daily_purchase_rate));
                                    });
                                });
                            });
                    },
                );
            });

        self.team_merchandise_window_open = open;
    }

    #[cfg(feature = "dev")]
    fn render_team_champion_setup_window(&mut self, ctx: &egui::Context) {
        if !self.team_champion_setup_window_open {
            return;
        }

        let Some((team, data)) = self.current_team_management_context() else {
            self.team_champion_setup_window_open = false;
            return;
        };

        let mut open = self.team_champion_setup_window_open;
        let title = self.localization.tr_with(
            "team_champion_setup.window_title",
            &[("team", team.display_name.as_str())],
        );

        egui::Window::new(title)
            .id(egui::Id::new("team_champion_setup_window_v056"))
            .open(&mut open)
            .resizable(true)
            .default_size(egui::vec2(900.0, 560.0))
            .min_size(egui::vec2(620.0, 340.0))
            .constrain(true)
            .show(ctx, |ui| {
                ui.weak(self.localization.tr("team_champion_setup.help"));
                ui.separator();

                let widths = [230.0, 90.0, 150.0, 150.0, 150.0];
                let table_min_width = widths.iter().copied().sum::<f32>() + 40.0;
                render_team_member_table_viewport(
                    ui,
                    "team_champion_setup_horizontal_v056",
                    table_min_width,
                    |ui, table_height| {
                        if data.champion_setup.is_empty() {
                            ui.weak(self.localization.tr("team_champion_setup.empty"));
                            return;
                        }

                        let mut table = TableBuilder::new(ui)
                            .id_salt("team_champion_setup_table_v056")
                            .striped(true)
                            .resizable(true)
                            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                            .min_scrolled_height(0.0)
                            .max_scroll_height(table_height)
                            .auto_shrink([false, false]);
                        for width in widths {
                            table = table.column(
                                Column::initial(width)
                                    .at_least(70.0)
                                    .clip(true)
                                    .resizable(true),
                            );
                        }

                        table
                            .header(26.0, |mut header| {
                                for key in [
                                    "team_champion_setup.champion",
                                    "team_champion_setup.tier",
                                    "team_champion_setup.tactic_1",
                                    "team_champion_setup.tactic_2",
                                    "team_champion_setup.tactic_3",
                                ] {
                                    header.col(|ui| {
                                        ui.strong(self.localization.tr(key));
                                    });
                                }
                            })
                            .body(|body| {
                                body.rows(26.0, data.champion_setup.len(), |mut row| {
                                    let entry = &data.champion_setup[row.index()];
                                    row.col(|ui| {
                                        ui.label(champion_display_name(&entry.champion_id))
                                            .on_hover_text(entry.champion_id.clone());
                                    });
                                    row.col(|ui| {
                                        ui.label(value_or_dash(&entry.tier));
                                    });
                                    row.col(|ui| {
                                        ui.label(value_or_dash(&entry.tactic_1));
                                    });
                                    row.col(|ui| {
                                        ui.label(value_or_dash(&entry.tactic_2));
                                    });
                                    row.col(|ui| {
                                        ui.label(value_or_dash(&entry.tactic_3));
                                    });
                                });
                            });
                    },
                );
            });

        self.team_champion_setup_window_open = open;
    }

    #[cfg(feature = "dev")]
    fn render_team_gaming_house_window(&mut self, ctx: &egui::Context) {
        if !self.team_gaming_house_window_open {
            return;
        }

        let Some((team, data)) = self.current_team_management_context() else {
            self.team_gaming_house_window_open = false;
            return;
        };

        let summary = data.gaming_house;
        let mut open = self.team_gaming_house_window_open;
        let title = self.localization.tr_with(
            "team_gaming_house.window_title",
            &[("team", team.display_name.as_str())],
        );

        egui::Window::new(title)
            .id(egui::Id::new("team_gaming_house_window_v056"))
            .open(&mut open)
            .resizable(true)
            .default_size(egui::vec2(660.0, 500.0))
            .min_size(egui::vec2(500.0, 340.0))
            .constrain(true)
            .show(ctx, |ui| {
                ui.weak(self.localization.tr("team_gaming_house.help"));
                ui.separator();
                egui::ScrollArea::vertical()
                    .id_salt("team_gaming_house_scroll_v056")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        egui::Grid::new("team_gaming_house_summary_grid_v056")
                            .num_columns(2)
                            .spacing(egui::vec2(28.0, 8.0))
                            .show(ui, |ui| {
                                let rows = [
                                    ("team_workspace.gaming_house_level", value_or_dash(&summary.level)),
                                    ("team_workspace.welfare", value_or_dash(&summary.welfare)),
                                    ("team_gaming_house.owned_furniture_types", summary.owned_furniture_types.to_string()),
                                    ("team_gaming_house.owned_furniture_total", summary.owned_furniture_total.to_string()),
                                    ("team_gaming_house.owned_wallpaper_types", summary.owned_wallpaper_types.to_string()),
                                    ("team_gaming_house.owned_wallpaper_total", summary.owned_wallpaper_total.to_string()),
                                    ("team_gaming_house.owned_wall_types", summary.owned_wall_types.to_string()),
                                    ("team_gaming_house.owned_wall_total", summary.owned_wall_total.to_string()),
                                    ("team_gaming_house.owned_window_types", summary.owned_window_types.to_string()),
                                    ("team_gaming_house.owned_window_total", summary.owned_window_total.to_string()),
                                    ("team_gaming_house.placed_furniture", summary.placed_furniture.to_string()),
                                    ("team_gaming_house.placed_wallpapers", summary.placed_wallpapers.to_string()),
                                    ("team_gaming_house.placed_walls", summary.placed_walls.to_string()),
                                    ("team_gaming_house.placed_windows", summary.placed_windows.to_string()),
                                ];
                                for (key, value) in rows {
                                    ui.label(self.localization.tr(key));
                                    ui.label(value);
                                    ui.end_row();
                                }
                            });
                    });
            });

        self.team_gaming_house_window_open = open;
    }

    #[cfg(feature = "dev")]
    fn render_team_condition_window(&mut self, ctx: &egui::Context) {
        if !self.team_condition_window_open {
            return;
        }

        let Some(team) = self
            .team_condition_team_id
            .and_then(|team_id| self.teams.iter().find(|team| team.id == team_id))
            .cloned()
        else {
            self.team_condition_window_open = false;
            self.team_condition_entries.clear();
            self.team_condition_team_id = None;
            self.team_condition_selected_player_ids.clear();
            return;
        };

        let mut entries = self.team_condition_entries.clone();
        let mut selected_ids = self.team_condition_selected_player_ids.clone();
        let mut bulk_stamina = self.team_condition_bulk_stamina.clone();
        let mut bulk_condition = self.team_condition_bulk_condition.clone();
        let mut open = self.team_condition_window_open;
        let mut refresh_requested = false;
        let mut apply_requested = false;
        let mut action_status: Option<String> = None;

        let title = self.localization.tr_with(
            "team_condition.window_title",
            &[("team", team.display_name.as_str())],
        );
        let editor_help = self.localization.tr("team_condition.editor_help");
        let selected_label = self.localization.tr("team_condition.selected");
        let changed_label = self.localization.tr("team_condition.changed");
        let select_all_label = self.localization.tr("lists.select_all_visible");
        let clear_selection_label = self.localization.tr("lists.clear_selection");
        let stamina_label = self.localization.tr("team_condition.stamina");
        let condition_label = self.localization.tr("team_condition.condition");
        let set_selected_label = self.localization.tr("team_condition.set_selected");
        let set_selected_max_label = self.localization.tr("team_condition.set_selected_max");
        let set_team_max_label = self.localization.tr("team_condition.set_team_max");
        let apply_label = self.localization.tr("team_condition.apply_changes");
        let refresh_label = self.localization.tr("common.refresh");
        let no_members_label = self.localization.tr("team_condition.no_members");
        let select_label = self.localization.tr("team_condition.select");
        let name_label = self.localization.tr("common.name");
        let id_label = self.localization.tr("common.id");
        let status_label = self.localization.tr("team_condition.status");
        let ready_label = self.localization.tr("team_condition.ready");
        let bulk_hint = self.localization.tr("team_condition.bulk_hint");
        let no_bulk_value_message = self.localization.tr("team_condition.no_bulk_value");

        egui::Window::new(title)
            .id(egui::Id::new("team_condition_editor_window_v053"))
            .open(&mut open)
            .resizable(true)
            .default_size(egui::vec2(920.0, 520.0))
            .min_size(egui::vec2(650.0, 340.0))
            .constrain(true)
            .show(ctx, |ui| {
                ui.weak(editor_help);
                ui.add_space(4.0);

                ui.horizontal_wrapped(|ui| {
                    let selected_count = selected_ids.len();
                    let changed_count = entries.iter().filter(|entry| entry.has_changes()).count();
                    ui.label(format!("{selected_label}: {selected_count}"));
                    ui.label(format!("{changed_label}: {changed_count}"));
                    ui.separator();

                    if ui.button(&select_all_label).clicked() {
                        selected_ids.extend(entries.iter().map(|entry| entry.player_id));
                    }
                    if ui
                        .add_enabled(!selected_ids.is_empty(), egui::Button::new(&clear_selection_label))
                        .clicked()
                    {
                        selected_ids.clear();
                    }
                    ui.separator();

                    ui.label(&stamina_label);
                    ui.add(
                        egui::TextEdit::singleline(&mut bulk_stamina)
                            .desired_width(58.0)
                            .hint_text("0-100"),
                    );
                    ui.label(&condition_label);
                    ui.add(
                        egui::TextEdit::singleline(&mut bulk_condition)
                            .desired_width(58.0)
                            .hint_text("0-100"),
                    );

                    if ui
                        .add_enabled(!selected_ids.is_empty(), egui::Button::new(&set_selected_label))
                        .on_hover_text(&bulk_hint)
                        .clicked()
                    {
                        if bulk_stamina.trim().is_empty() && bulk_condition.trim().is_empty() {
                            action_status = Some(no_bulk_value_message.clone());
                        } else {
                            let validation = if bulk_stamina.trim().is_empty() {
                                Ok(())
                            } else {
                                validate_condition_editor_value(&bulk_stamina, "Stamina")
                            }
                            .and_then(|_| {
                                if bulk_condition.trim().is_empty() {
                                    Ok(())
                                } else {
                                    validate_condition_editor_value(&bulk_condition, "Condition")
                                }
                            });

                            match validation {
                                Ok(()) => {
                                    for entry in &mut entries {
                                        if selected_ids.contains(&entry.player_id) {
                                            if !bulk_stamina.trim().is_empty() {
                                                entry.stamina = bulk_stamina.trim().to_string();
                                            }
                                            if !bulk_condition.trim().is_empty() {
                                                entry.condition = bulk_condition.trim().to_string();
                                            }
                                            entry.write_status = ready_label.clone();
                                        }
                                    }
                                }
                                Err(error) => action_status = Some(error),
                            }
                        }
                    }

                    if ui
                        .add_enabled(
                            !selected_ids.is_empty(),
                            egui::Button::new(&set_selected_max_label),
                        )
                        .clicked()
                    {
                        for entry in &mut entries {
                            if selected_ids.contains(&entry.player_id) {
                                entry.stamina = "100".to_string();
                                entry.condition = "100".to_string();
                                entry.write_status = ready_label.clone();
                            }
                        }
                    }

                    if ui
                        .add_enabled(!entries.is_empty(), egui::Button::new(&set_team_max_label))
                        .clicked()
                    {
                        for entry in &mut entries {
                            entry.stamina = "100".to_string();
                            entry.condition = "100".to_string();
                            entry.write_status = ready_label.clone();
                        }
                    }
                });

                ui.horizontal(|ui| {
                    let changed = entries.iter().any(|entry| entry.has_changes());
                    if ui
                        .add_enabled(changed && self.connected, egui::Button::new(&apply_label))
                        .clicked()
                    {
                        apply_requested = true;
                    }
                    if ui.button(&refresh_label).clicked() {
                        refresh_requested = true;
                    }
                });
                ui.separator();

                let widths = [42.0, 210.0, 70.0, 120.0, 120.0, 210.0];
                let table_min_width = widths.iter().copied().sum::<f32>() + 40.0;

                render_team_member_table_viewport(
                    ui,
                    "team_condition_editor_horizontal_v053",
                    table_min_width,
                    |ui, table_height| {
                        if entries.is_empty() {
                            ui.weak(no_members_label);
                            return;
                        }

                        let mut table = TableBuilder::new(ui)
                            .id_salt("team_condition_editor_table_v053")
                            .striped(true)
                            .resizable(true)
                            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                            .min_scrolled_height(0.0)
                            .max_scroll_height(table_height)
                            .auto_shrink([false, false]);
                        for width in widths {
                            table = table.column(
                                Column::initial(width)
                                    .at_least(42.0)
                                    .clip(true)
                                    .resizable(true),
                            );
                        }

                        table
                            .header(26.0, |mut header| {
                                header.col(|ui| { ui.strong(&select_label); });
                                header.col(|ui| { ui.strong(&name_label); });
                                header.col(|ui| { ui.strong(&id_label); });
                                header.col(|ui| { ui.strong(&stamina_label); });
                                header.col(|ui| { ui.strong(&condition_label); });
                                header.col(|ui| { ui.strong(&status_label); });
                            })
                            .body(|body| {
                                body.rows(30.0, entries.len(), |mut row| {
                                    let index = row.index();
                                    let player_id = entries[index].player_id;
                                    row.set_selected(selected_ids.contains(&player_id));

                                    row.col(|ui| {
                                        let mut selected = selected_ids.contains(&player_id);
                                        if ui.checkbox(&mut selected, "").changed() {
                                            if selected {
                                                selected_ids.insert(player_id);
                                            } else {
                                                selected_ids.remove(&player_id);
                                            }
                                        }
                                    });
                                    row.col(|ui| { ui.label(&entries[index].player_name); });
                                    row.col(|ui| { ui.label(player_id.to_string()); });
                                    row.col(|ui| {
                                        let response = ui.add(
                                            egui::TextEdit::singleline(&mut entries[index].stamina)
                                                .desired_width(76.0),
                                        );
                                        if response.changed() {
                                            entries[index].write_status = ready_label.clone();
                                        }
                                    });
                                    row.col(|ui| {
                                        let response = ui.add(
                                            egui::TextEdit::singleline(&mut entries[index].condition)
                                                .desired_width(76.0),
                                        );
                                        if response.changed() {
                                            entries[index].write_status = ready_label.clone();
                                        }
                                    });
                                    row.col(|ui| { ui.label(&entries[index].write_status); });
                                });
                            });
                    },
                );
            });

        self.team_condition_entries = entries;
        self.team_condition_selected_player_ids = selected_ids;
        self.team_condition_bulk_stamina = bulk_stamina;
        self.team_condition_bulk_condition = bulk_condition;
        self.team_condition_window_open = open;

        if let Some(status) = action_status {
            self.status = status;
        }
        if apply_requested {
            self.apply_team_condition_changes();
        }
        if refresh_requested {
            self.refresh_team_condition_probe();
        }
    }

    fn render_team_search_page(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                ui.strong(self.localization.tr("search.teams.quick_filters"));
                ui.separator();

                ui.label(self.localization.tr("search.teams.name_or_manager"));
                ui.add(
                    egui::TextEdit::singleline(&mut self.team_database_search)
                        .desired_width(190.0)
                        .hint_text(self.localization.tr("search.teams.search_hint")),
                );

                ui.separator();
                ui.label(self.localization.tr("search.teams.league"));
                let selected_league = self
                    .team_search_league_filter
                    .map(|id| {
                        let id = id.to_string();
                        self.localization
                            .tr_with("common.league_number", &[("id", id.as_str())])
                    })
                    .unwrap_or_else(|| self.localization.tr("search.teams.any_league"));
                egui::ComboBox::from_id_salt("search_team_league_filter")
                    .selected_text(selected_league)
                    .width(135.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.team_search_league_filter,
                            None,
                            self.localization.tr("search.teams.any_league"),
                        );
                        let mut league_ids = self
                            .teams
                            .iter()
                            .map(|team| team.league_id)
                            .collect::<Vec<_>>();
                        league_ids.sort_unstable();
                        league_ids.dedup();
                        for league_id in league_ids {
                            let id = league_id.to_string();
                            let label = self.localization.tr_with(
                                "common.league_number",
                                &[("id", id.as_str())],
                            );
                            ui.selectable_value(
                                &mut self.team_search_league_filter,
                                Some(league_id),
                                label,
                            );
                        }
                    });

                ui.separator();
                ui.checkbox(
                    &mut self.team_search_player_team_only,
                    self.localization.tr("search.teams.my_team_only"),
                );
            });

            ui.add_space(5.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(self.localization.tr("search.teams.players"));
                ui.add(
                    egui::TextEdit::singleline(&mut self.team_search_roster_min)
                        .desired_width(58.0)
                        .hint_text(self.localization.tr("common.min")),
                );
                ui.label(self.localization.tr("common.to"));
                ui.add(
                    egui::TextEdit::singleline(&mut self.team_search_roster_max)
                        .desired_width(58.0)
                        .hint_text(self.localization.tr("common.max")),
                );

                ui.separator();
                ui.label(self.localization.tr("search.teams.staff"));
                ui.add(
                    egui::TextEdit::singleline(&mut self.team_search_staff_min)
                        .desired_width(58.0)
                        .hint_text(self.localization.tr("common.min")),
                );
                ui.label(self.localization.tr("common.to"));
                ui.add(
                    egui::TextEdit::singleline(&mut self.team_search_staff_max)
                        .desired_width(58.0)
                        .hint_text(self.localization.tr("common.max")),
                );
            });
        });

        ui.add_space(8.0);

        let query = self.team_database_search.trim().to_lowercase();
        let selected_league = self.team_search_league_filter;
        let player_team_only = self.team_search_player_team_only;
        let roster_min = self.team_search_roster_min.trim().parse::<usize>().ok();
        let roster_max = self.team_search_roster_max.trim().parse::<usize>().ok();
        let staff_min = self.team_search_staff_min.trim().parse::<usize>().ok();
        let staff_max = self.team_search_staff_max.trim().parse::<usize>().ok();

        let mut filtered_teams = self
            .teams
            .iter()
            .filter(|team| {
                if !query.is_empty() && !team.matches_search(&query) {
                    return false;
                }
                if selected_league.is_some_and(|league| team.league_id != league) {
                    return false;
                }
                if player_team_only && !team.is_player_team {
                    return false;
                }
                if roster_min.is_some_and(|minimum| team.roster_size < minimum) {
                    return false;
                }
                if roster_max.is_some_and(|maximum| team.roster_size > maximum) {
                    return false;
                }
                if staff_min.is_some_and(|minimum| team.staff_count < minimum) {
                    return false;
                }
                if staff_max.is_some_and(|maximum| team.staff_count > maximum) {
                    return false;
                }
                true
            })
            .collect::<Vec<_>>();

        let mut sort_column = self.team_sort_column;
        let mut sort_ascending = self.team_sort_ascending;
        filtered_teams.sort_by(|a, b| {
            compare_team_summaries(a, b, sort_column, sort_ascending)
        });
        let filtered_team_ids = filtered_teams
            .iter()
            .map(|team| team.id)
            .collect::<Vec<_>>();

        let mut refresh_requested = false;
        let mut reset_columns_requested = false;
        let mut select_all_visible_requested = false;
        #[cfg(feature = "dev")]
        let mut open_team_id: Option<usize> = None;
        let available_height = ui.available_height().max(180.0);
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), available_height),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.group(|ui| {
                    ui.set_min_width(ui.available_width());
                    ui.set_min_height((ui.available_height() - 2.0).max(160.0));

                    ui.horizontal_wrapped(|ui| {
                        ui.strong(self.localization.tr("search.teams.team_list"));
                        let matches = filtered_teams.len().to_string();
                        ui.label(self.localization.tr_with(
                            "common.matches",
                            &[("count", matches.as_str())],
                        ));
                        ui.separator();
                        if ui.button(self.localization.tr("recruitment.refresh_teams")).clicked() {
                            refresh_requested = true;
                        }
                        if ui.button(self.localization.tr("search.teams.reset_columns")).clicked() {
                            reset_columns_requested = true;
                        }
                        ui.separator();
                        let selected_count = self.selected_search_team_ids.len().to_string();
                        ui.label(self.localization.tr_with(
                            "search.selected_count",
                            &[("count", selected_count.as_str())],
                        ));
                        if ui.button(self.localization.tr("lists.select_all_visible")).clicked() {
                            select_all_visible_requested = true;
                        }
                        if ui
                            .add_enabled(
                                !self.selected_search_team_ids.is_empty(),
                                egui::Button::new(self.localization.tr("lists.clear_selection")),
                            )
                            .clicked()
                        {
                            self.selected_search_team_ids.clear();
                            self.team_selection_anchor_id = None;
                            self.team_shift_drag_start_id = None;
                            self.team_shift_drag_target_selected = None;
                            self.team_shift_drag_base_ids = None;
                        }
                    });
                    ui.add_space(4.0);

                    let table_height = (ui.available_height() - 26.0).max(120.0);
                    let widths = [
                        42.0, 190.0, 64.0, 88.0, 150.0, 84.0, 72.0, 72.0,
                        105.0, 145.0, 120.0, 145.0, 105.0, 132.0, 112.0,
                    ];
                    let table_min_width = widths.iter().copied().sum::<f32>() + 120.0;

                    egui::ScrollArea::horizontal()
                        .id_salt("search_teams_table_horizontal")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.set_min_width(table_min_width);

                            let shift_down = ui.input(|input| input.modifiers.shift);
                            let primary_down = ui.input(|input| input.pointer.primary_down());
                            let primary_released = ui.input(|input| input.pointer.primary_released());

                            let mut table = TableBuilder::new(ui)
                                .id_salt("search_teams_resizable_table")
                                .striped(true)
                                .resizable(true)
                                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                                .sense(egui::Sense::click_and_drag())
                                .min_scrolled_height(table_height)
                                .max_scroll_height(table_height)
                                .auto_shrink([false, false]);

                            for width in widths {
                                table = table.column(
                                    Column::initial(width)
                                        .at_least(48.0)
                                        .clip(true)
                                        .resizable(true),
                                );
                            }

                            if reset_columns_requested {
                                table.reset();
                            }

                            let mut selected_team_ids = self.selected_search_team_ids.clone();
                            let mut team_selection_anchor_id = self.team_selection_anchor_id;
                            let mut team_shift_drag_start_id = self.team_shift_drag_start_id;
                            let mut team_shift_drag_target_selected =
                                self.team_shift_drag_target_selected;
                            let mut team_shift_drag_base_ids =
                                self.team_shift_drag_base_ids.clone();
                            if select_all_visible_requested {
                                selected_team_ids.extend(filtered_team_ids.iter().copied());
                                team_selection_anchor_id = None;
                                team_shift_drag_start_id = None;
                                team_shift_drag_target_selected = None;
                                team_shift_drag_base_ids = None;
                            }

                            table
                                .header(22.0, |mut header| {
                                    header.col(|ui| {
                                        ui.strong(self.localization.tr("lists.select_column"));
                                    });
                                    for column in [
                                        TeamSortColumn::Name,
                                        TeamSortColumn::Id,
                                        TeamSortColumn::League,
                                        TeamSortColumn::Manager,
                                        TeamSortColumn::PlayerTeam,
                                        TeamSortColumn::RosterSize,
                                        TeamSortColumn::StaffCount,
                                        TeamSortColumn::RosterRating,
                                        TeamSortColumn::MerchandiseFacilityGrade,
                                        TeamSortColumn::StadiumGrade,
                                        TeamSortColumn::TrainingFacilityGrade,
                                        TeamSortColumn::Money,
                                        TeamSortColumn::RecruitmentBudget,
                                        TeamSortColumn::SalaryBudget,
                                    ] {
                                        header.col(|ui| {
                                            team_sort_header(
                                                ui,
                                                column,
                                                &mut sort_column,
                                                &mut sort_ascending,
                                                &self.localization,
                                            );
                                        });
                                    }
                                })
                                .body(|body| {
                                    body.rows(21.0, filtered_teams.len(), |mut row| {
                                        let team = filtered_teams[row.index()];
                                        row.set_selected(selected_team_ids.contains(&team.id));
                                        let mut selection_checkbox_clicked = false;
                                        row.col(|ui| {
                                            let mut selected = selected_team_ids.contains(&team.id);
                                            let checkbox_response = ui.checkbox(&mut selected, "");
                                            selection_checkbox_clicked = checkbox_response.clicked();
                                            if checkbox_response.changed() {
                                                if selected {
                                                    selected_team_ids.insert(team.id);
                                                    team_selection_anchor_id = Some(team.id);
                                                } else {
                                                    selected_team_ids.remove(&team.id);
                                                    team_selection_anchor_id = None;
                                                }
                                                team_shift_drag_start_id = None;
                                                team_shift_drag_target_selected = None;
                                                team_shift_drag_base_ids = None;
                                            }
                                        });
                                        row.col(|ui| {
                                            let name = if team.display_name.trim().is_empty() {
                                                team.localization_fallback_name(&self.localization)
                                            } else {
                                                team.display_name.clone()
                                            };
                                            ui.label(name);
                                        });
                                        row.col(|ui| { ui.label(team.id.to_string()); });
                                        row.col(|ui| {
                                            let id = team.league_id.to_string();
                                            ui.label(self.localization.tr_with(
                                                "common.league_number",
                                                &[("id", id.as_str())],
                                            ));
                                        });
                                        row.col(|ui| { ui.label(value_or_dash(&team.manager_name)); });
                                        row.col(|ui| {
                                            ui.label(if team.is_player_team {
                                                self.localization.tr("common.my_team")
                                            } else {
                                                "—".to_string()
                                            });
                                        });
                                        row.col(|ui| { ui.label(team.roster_size.to_string()); });
                                        row.col(|ui| { ui.label(team.staff_count.to_string()); });
                                        row.col(|ui| {
                                            ui.label(
                                                team.roster_rating
                                                    .map(|value| format!("{value:.1}"))
                                                    .unwrap_or_else(|| "—".to_string()),
                                            );
                                        });
                                        row.col(|ui| {
                                            ui.label(display_facility_grade(
                                                &team.merchandise_facility_grade,
                                            ));
                                        });
                                        row.col(|ui| {
                                            ui.label(display_facility_grade(&team.stadium_grade));
                                        });
                                        row.col(|ui| {
                                            ui.label(display_facility_grade(
                                                &team.training_facility_grade,
                                            ));
                                        });
                                        row.col(|ui| {
                                            ui.label(format_internal_amount(
                                                &team.total_balance.to_string(),
                                            ));
                                        });
                                        row.col(|ui| {
                                            ui.label(format_internal_amount(
                                                &team.transfer_budget.to_string(),
                                            ));
                                        });
                                        row.col(|ui| {
                                            ui.label(format_internal_amount(
                                                &team.salary_budget.to_string(),
                                            ));
                                        });

                                        let row_response = row.response();
                                        if shift_down
                                            && row_response.drag_started_by(egui::PointerButton::Primary)
                                            && !selection_checkbox_clicked
                                        {
                                            team_shift_drag_start_id = Some(team.id);
                                            team_shift_drag_target_selected =
                                                Some(!selected_team_ids.contains(&team.id));
                                            team_shift_drag_base_ids = Some(selected_team_ids.clone());
                                            team_selection_anchor_id = None;
                                        }
                                        if (primary_down || primary_released) && row_response.contains_pointer() {
                                            if let (
                                                Some(start_id),
                                                Some(target_selected),
                                                Some(base_ids),
                                            ) = (
                                                team_shift_drag_start_id,
                                                team_shift_drag_target_selected,
                                                team_shift_drag_base_ids.as_ref(),
                                            ) {
                                                selected_team_ids = base_ids.clone();
                                                apply_id_range_selection(
                                                    &filtered_team_ids,
                                                    start_id,
                                                    team.id,
                                                    target_selected,
                                                    &mut selected_team_ids,
                                                );
                                            }
                                        }

                                        let shift_drag_active = team_shift_drag_start_id.is_some();
                                        if row_response.double_clicked() && !selection_checkbox_clicked {
                                            selected_team_ids.insert(team.id);
                                            team_selection_anchor_id = None;
                                            team_shift_drag_start_id = None;
                                            team_shift_drag_target_selected = None;
                                            team_shift_drag_base_ids = None;
                                            #[cfg(feature = "dev")]
                                            {
                                                open_team_id = Some(team.id);
                                            }
                                        } else if row_response.clicked()
                                            && !selection_checkbox_clicked
                                            && !shift_drag_active
                                        {
                                            if shift_down {
                                                let target_selected = !selected_team_ids.contains(&team.id);
                                                let anchor_id =
                                                    team_selection_anchor_id.unwrap_or(team.id);
                                                apply_id_range_selection(
                                                    &filtered_team_ids,
                                                    anchor_id,
                                                    team.id,
                                                    target_selected,
                                                    &mut selected_team_ids,
                                                );
                                                // A completed Shift-click is a one-shot range action.
                                                // Resetting the anchor prevents the next Shift-click from
                                                // accidentally reusing an old selection start.
                                                team_selection_anchor_id = None;
                                            } else if selected_team_ids.contains(&team.id) {
                                                selected_team_ids.remove(&team.id);
                                                team_selection_anchor_id = None;
                                            } else {
                                                selected_team_ids.insert(team.id);
                                                team_selection_anchor_id = Some(team.id);
                                            }
                                        }
                                        #[cfg(feature = "dev")]
                                        row_response.context_menu(|ui| {
                                            if ui
                                                .button(self.localization.tr("team_workspace.open_in_team"))
                                                .clicked()
                                            {
                                                open_team_id = Some(team.id);
                                                ui.close_menu();
                                            }
                                        });
                                    });
                                });
                            if primary_released || !primary_down {
                                if team_shift_drag_start_id.is_some() {
                                    team_selection_anchor_id = None;
                                }
                                team_shift_drag_start_id = None;
                                team_shift_drag_target_selected = None;
                                team_shift_drag_base_ids = None;
                            }
                            self.selected_search_team_ids = selected_team_ids;
                            self.team_selection_anchor_id = team_selection_anchor_id;
                            self.team_shift_drag_start_id = team_shift_drag_start_id;
                            self.team_shift_drag_target_selected =
                                team_shift_drag_target_selected;
                            self.team_shift_drag_base_ids = team_shift_drag_base_ids;
                        });
                });
            },
        );

        self.team_sort_column = sort_column;
        self.team_sort_ascending = sort_ascending;
        if refresh_requested {
            self.refresh_teams();
        }
        #[cfg(feature = "dev")]
        if let Some(team_id) = open_team_id {
            self.open_team_workspace(team_id);
        }
    }

    fn render_staff_search_page(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                ui.strong(self.localization.tr("search.staff.quick_filters"));
                ui.separator();

                let advanced_button = egui::Button::new(
                    egui::RichText::new(self.localization.tr("search.staff.advanced_search"))
                        .strong()
                        .color(ui.visuals().selection.stroke.color),
                )
                .fill(ui.visuals().selection.bg_fill)
                .stroke(ui.visuals().selection.stroke);
                if ui.add(advanced_button).clicked() {
                    self.advanced_staff_search_open = true;
                }

                let advanced_count = self.advanced_staff_search.active_condition_count();
                if advanced_count > 0 {
                    let advanced_count_text = advanced_count.to_string();
                    ui.label(
                        egui::RichText::new(self.localization.tr_with(
                            "search.active_filters",
                            &[("count", advanced_count_text.as_str())],
                        ))
                        .strong()
                        .color(ui.visuals().selection.stroke.color),
                    );
                }

                ui.separator();
                ui.weak(self.localization.tr("search.staff.quick_filters_info"));
            });
            ui.add_space(5.0);

            ui.horizontal_wrapped(|ui| {
                ui.label(self.localization.tr("common.name"));
                ui.add(
                    egui::TextEdit::singleline(&mut self.staff_database_search)
                        .desired_width(190.0)
                        .hint_text(self.localization.tr("search.staff.name_hint")),
                );

                ui.separator();
                ui.label(self.localization.tr("common.team"));
                let selected_team_text = if self.staff_search_team_filter == "Any Team" {
                    self.localization.tr("search.players.any_team")
                } else {
                    self.staff_search_team_filter.clone()
                };
                egui::ComboBox::from_id_salt("search_staff_quick_team")
                    .selected_text(selected_team_text)
                    .width(150.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.staff_search_team_filter,
                            "Any Team".to_string(),
                            self.localization.tr("search.players.any_team"),
                        );
                        for team in &self.teams {
                            let label = if team.display_name.trim().is_empty() {
                                format!("Team {}", team.id)
                            } else {
                                team.display_name.clone()
                            };
                            ui.selectable_value(
                                &mut self.staff_search_team_filter,
                                label.clone(),
                                label,
                            );
                        }
                    });

                ui.label(self.localization.tr("common.role"));
                let selected_role_text = if self.staff_search_role_filter == "Any Role" {
                    self.localization.tr("search.staff.any_role")
                } else {
                    localized_staff_role(&self.localization, &self.staff_search_role_filter)
                };
                egui::ComboBox::from_id_salt("search_staff_quick_role")
                    .selected_text(selected_role_text)
                    .width(155.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.staff_search_role_filter,
                            "Any Role".to_string(),
                            self.localization.tr("search.staff.any_role"),
                        );
                        for role in ["HeadCoach", "TrainingCoach", "Scouter", "Analyst"] {
                            ui.selectable_value(
                                &mut self.staff_search_role_filter,
                                role.to_string(),
                                localized_staff_role(&self.localization, role),
                            );
                        }
                    });
            });

            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(self.localization.tr("common.age"));
                ui.add(
                    egui::TextEdit::singleline(&mut self.staff_search_age_min)
                        .desired_width(58.0)
                        .hint_text(self.localization.tr("common.min")),
                );
                ui.label(self.localization.tr("common.to"));
                ui.add(
                    egui::TextEdit::singleline(&mut self.staff_search_age_max)
                        .desired_width(58.0)
                        .hint_text(self.localization.tr("common.max")),
                );

                ui.separator();
                ui.checkbox(
                    &mut self.staff_search_free_agents_only,
                    self.localization.tr("search.players.free_agents_only"),
                );

                ui.separator();
                ui.label(self.localization.tr("lists.filter_label"));
                let active_list_text = self
                    .active_staff_list_filter
                    .clone()
                    .unwrap_or_else(|| self.localization.tr("lists.all_staff"));
                egui::ComboBox::from_id_salt("staff_search_active_list_filter")
                    .selected_text(active_list_text)
                    .width(170.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.active_staff_list_filter,
                            None,
                            self.localization.tr("lists.all_staff"),
                        );
                        for list in &self.saved_player_lists {
                            ui.selectable_value(
                                &mut self.active_staff_list_filter,
                                Some(list.name.clone()),
                                &list.name,
                            );
                        }
                    });
            });
        });

        ui.add_space(8.0);

        let query = self.staff_database_search.trim().to_lowercase();
        let age_min = self.staff_search_age_min.trim().parse::<f64>().ok();
        let age_max = self.staff_search_age_max.trim().parse::<f64>().ok();
        let selected_team = self.staff_search_team_filter.clone();
        let selected_role = self.staff_search_role_filter.clone();
        let free_agents_only = self.staff_search_free_agents_only;
        let advanced_filter = self.advanced_staff_search.clone();
        let active_list_ids = self
            .active_staff_list_filter
            .as_ref()
            .and_then(|name| {
                self.saved_player_lists
                    .iter()
                    .find(|list| list.name.eq_ignore_ascii_case(name))
            })
            .map(|list| list.staff_ids.iter().copied().collect::<BTreeSet<_>>());

        let mut filtered_staff = self
            .staffs
            .iter()
            .filter(|staff| {
                if active_list_ids
                    .as_ref()
                    .is_some_and(|ids| !ids.contains(&staff.id))
                {
                    return false;
                }

                if !query.is_empty() && !staff.name.to_lowercase().contains(&query) {
                    return false;
                }

                let age = staff.age.parse::<f64>().ok();
                if age_min.is_some_and(|min| age.is_none_or(|value| value < min)) {
                    return false;
                }
                if age_max.is_some_and(|max| age.is_none_or(|value| value > max)) {
                    return false;
                }

                if selected_team != "Any Team" && staff.team != selected_team {
                    return false;
                }
                if selected_role != "Any Role" && staff.role != selected_role {
                    return false;
                }
                if free_agents_only && staff.team != "Free Agent" {
                    return false;
                }

                advanced_staff_filter_matches(staff, &advanced_filter)
            })
            .collect::<Vec<_>>();

        let mut sort_column = self.staff_sort_column;
        let mut sort_ascending = self.staff_sort_ascending;
        filtered_staff.sort_by(|a, b| {
            compare_staff_summaries(a, b, sort_column, sort_ascending)
        });
        let filtered_staff_ids = filtered_staff.iter().map(|staff| staff.id).collect::<Vec<_>>();

        let mut refresh_staff_requested = false;
        let mut reset_columns_requested = false;
        let mut open_staff_id: Option<usize> = None;
        let mut add_to_list_request: Option<(String, Vec<usize>)> = None;
        let mut create_list_from_ids_request: Option<Vec<usize>> = None;
        let mut select_all_visible_requested = false;

        let available_height = ui.available_height().max(180.0);
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), available_height),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.group(|ui| {
                    ui.set_min_width(ui.available_width());
                    ui.set_min_height((ui.available_height() - 2.0).max(160.0));

                    ui.horizontal_wrapped(|ui| {
                        ui.strong(self.localization.tr("search.staff.list_heading"));
                        let result_count = filtered_staff.len().to_string();
                        ui.label(self.localization.tr_with(
                            "search.staff.results",
                            &[("count", result_count.as_str())],
                        ));
                        if ui.button(self.localization.tr("editor.refresh_staff")).clicked() {
                            refresh_staff_requested = true;
                        }
                        if ui.button(self.localization.tr("search.players.reset_columns")).clicked() {
                            reset_columns_requested = true;
                        }
                        ui.separator();
                        let selected_count = self.selected_search_staff_ids.len().to_string();
                        ui.label(self.localization.tr_with(
                            "search.selected_count",
                            &[("count", selected_count.as_str())],
                        ));
                        if ui.button(self.localization.tr("lists.select_all_visible")).clicked() {
                            select_all_visible_requested = true;
                        }
                        if ui
                            .add_enabled(
                                !self.selected_search_staff_ids.is_empty(),
                                egui::Button::new(self.localization.tr("lists.clear_selection")),
                            )
                            .clicked()
                        {
                            self.selected_search_staff_ids.clear();
                            self.staff_selection_anchor_id = None;
                            self.staff_shift_drag_start_id = None;
                            self.staff_shift_drag_target_selected = None;
                            self.staff_shift_drag_base_ids = None;
                        }
                        ui.separator();
                        ui.weak(self.localization.tr(search_staff_table_help_key()));
                    });
                    ui.add_space(4.0);

                    let table_height = (ui.available_height() - 26.0).max(120.0);
                    let viewport_width = ui.available_width();
                    let table_min_width = 2502.0_f32.max(viewport_width);

                    egui::ScrollArea::horizontal()
                        .id_salt("search_staff_table_horizontal")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.set_min_width(table_min_width);

                            let widths = [
                                42.0, 140.0, 58.0, 52.0, 150.0, 125.0, 125.0, 104.0,
                                86.0, 86.0, 96.0, 118.0, 122.0, 88.0, 112.0,
                                118.0, 132.0, 118.0, 108.0,
                            ];

                            let shift_down = ui.input(|input| input.modifiers.shift);
                            let primary_down = ui.input(|input| input.pointer.primary_down());
                            let primary_released = ui.input(|input| input.pointer.primary_released());

                            let mut table = TableBuilder::new(ui)
                                .id_salt("search_staff_resizable_table")
                                .striped(true)
                                .resizable(true)
                                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                                .sense(egui::Sense::click_and_drag())
                                .min_scrolled_height(table_height)
                                .max_scroll_height(table_height)
                                .auto_shrink([false, false]);

                            for width in widths {
                                table = table.column(
                                    Column::initial(width)
                                        .at_least(48.0)
                                        .clip(true)
                                        .resizable(true),
                                );
                            }

                            if reset_columns_requested {
                                table.reset();
                            }

                            let mut selected_staff_ids = self.selected_search_staff_ids.clone();
                            let mut staff_selection_anchor_id = self.staff_selection_anchor_id;
                            let mut staff_shift_drag_start_id = self.staff_shift_drag_start_id;
                            let mut staff_shift_drag_target_selected =
                                self.staff_shift_drag_target_selected;
                            let mut staff_shift_drag_base_ids =
                                self.staff_shift_drag_base_ids.clone();
                            if select_all_visible_requested {
                                selected_staff_ids.extend(filtered_staff_ids.iter().copied());
                                staff_selection_anchor_id = None;
                                staff_shift_drag_start_id = None;
                                staff_shift_drag_target_selected = None;
                                staff_shift_drag_base_ids = None;
                            }
                            let saved_lists_for_menu = self
                                .saved_player_lists
                                .iter()
                                .map(|list| list.name.clone())
                                .collect::<Vec<_>>();

                            table
                                .header(22.0, |mut header| {
                                    header.col(|ui| {
                                        ui.strong(self.localization.tr("lists.select_column"));
                                    });
                                    for column in [
                                        StaffSortColumn::Name,
                                        StaffSortColumn::Id,
                                        StaffSortColumn::Age,
                                        StaffSortColumn::Team,
                                        StaffSortColumn::Role,
                                        StaffSortColumn::Salary,
                                        StaffSortColumn::ContractEnd,
                                        StaffSortColumn::BanPick,
                                        StaffSortColumn::Strategy,
                                        StaffSortColumn::Negotiation,
                                        StaffSortColumn::JudgeAbility,
                                        StaffSortColumn::JudgePotential,
                                        StaffSortColumn::Feedback,
                                        StaffSortColumn::PowerAnalysis,
                                        StaffSortColumn::ControlCoaching,
                                        StaffSortColumn::JudgmentCoaching,
                                        StaffSortColumn::MentalCoaching,
                                        StaffSortColumn::Communication,
                                    ] {
                                        header.col(|ui| {
                                            staff_sort_header(
                                                ui,
                                                column,
                                                &mut sort_column,
                                                &mut sort_ascending,
                                                &self.localization,
                                            );
                                        });
                                    }
                                })
                                .body(|body| {
                                    body.rows(21.0, filtered_staff.len(), |mut row| {
                                        let staff = filtered_staff[row.index()];
                                        row.set_selected(selected_staff_ids.contains(&staff.id));
                                        let mut selection_checkbox_clicked = false;
                                        row.col(|ui| {
                                            let mut selected = selected_staff_ids.contains(&staff.id);
                                            let checkbox_response = ui.checkbox(&mut selected, "");
                                            selection_checkbox_clicked = checkbox_response.clicked();
                                            if checkbox_response.changed() {
                                                if selected {
                                                    selected_staff_ids.insert(staff.id);
                                                    staff_selection_anchor_id = Some(staff.id);
                                                } else {
                                                    selected_staff_ids.remove(&staff.id);
                                                    staff_selection_anchor_id = None;
                                                }
                                                staff_shift_drag_start_id = None;
                                                staff_shift_drag_target_selected = None;
                                                staff_shift_drag_base_ids = None;
                                            }
                                        });
                                        row.col(|ui| { ui.label(&staff.name); });
                                        row.col(|ui| { ui.label(staff.id.to_string()); });
                                        row.col(|ui| { ui.label(value_or_dash(&staff.age)); });
                                        row.col(|ui| { ui.label(value_or_dash(&staff.team)); });
                                        row.col(|ui| { ui.label(localized_staff_role(&self.localization, &staff.role)); });
                                        row.col(|ui| { ui.label(pretty_or_dash(&staff.annual_salary)); });
                                        row.col(|ui| { ui.label(value_or_dash(&display_contract_date(&staff.contract_end))); });
                                        row.col(|ui| { ui.label(pretty_or_dash(&staff.banpick)); });
                                        row.col(|ui| { ui.label(pretty_or_dash(&staff.strategy)); });
                                        row.col(|ui| { ui.label(pretty_or_dash(&staff.negotiation)); });
                                        row.col(|ui| { ui.label(pretty_or_dash(&staff.judge_ability)); });
                                        row.col(|ui| { ui.label(pretty_or_dash(&staff.judge_potential)); });
                                        row.col(|ui| { ui.label(pretty_or_dash(&staff.feedback)); });
                                        row.col(|ui| { ui.label(pretty_or_dash(&staff.power_analysis)); });
                                        row.col(|ui| { ui.label(pretty_or_dash(&staff.control_coaching)); });
                                        row.col(|ui| { ui.label(pretty_or_dash(&staff.judgment_coaching)); });
                                        row.col(|ui| { ui.label(pretty_or_dash(&staff.mental_coaching)); });
                                        row.col(|ui| { ui.label(pretty_or_dash(&staff.communication)); });

                                        let row_response = row.response();
                                        if shift_down
                                            && row_response.drag_started_by(egui::PointerButton::Primary)
                                            && !selection_checkbox_clicked
                                        {
                                            staff_shift_drag_start_id = Some(staff.id);
                                            staff_shift_drag_target_selected =
                                                Some(!selected_staff_ids.contains(&staff.id));
                                            staff_shift_drag_base_ids = Some(selected_staff_ids.clone());
                                            staff_selection_anchor_id = None;
                                        }
                                        if (primary_down || primary_released) && row_response.contains_pointer() {
                                            if let (
                                                Some(start_id),
                                                Some(target_selected),
                                                Some(base_ids),
                                            ) = (
                                                staff_shift_drag_start_id,
                                                staff_shift_drag_target_selected,
                                                staff_shift_drag_base_ids.as_ref(),
                                            ) {
                                                selected_staff_ids = base_ids.clone();
                                                apply_id_range_selection(
                                                    &filtered_staff_ids,
                                                    start_id,
                                                    staff.id,
                                                    target_selected,
                                                    &mut selected_staff_ids,
                                                );
                                            }
                                        }

                                        let shift_drag_active = staff_shift_drag_start_id.is_some();
                                        if row_response.double_clicked() && !selection_checkbox_clicked {
                                            selected_staff_ids.insert(staff.id);
                                            staff_selection_anchor_id = None;
                                            staff_shift_drag_start_id = None;
                                            staff_shift_drag_target_selected = None;
                                            staff_shift_drag_base_ids = None;
                                            open_staff_id = Some(staff.id);
                                        } else if row_response.clicked()
                                            && !selection_checkbox_clicked
                                            && !shift_drag_active
                                        {
                                            if shift_down {
                                                let target_selected = !selected_staff_ids.contains(&staff.id);
                                                let anchor_id =
                                                    staff_selection_anchor_id.unwrap_or(staff.id);
                                                apply_id_range_selection(
                                                    &filtered_staff_ids,
                                                    anchor_id,
                                                    staff.id,
                                                    target_selected,
                                                    &mut selected_staff_ids,
                                                );
                                                // A completed Shift-click is a one-shot range action.
                                                // Resetting the anchor prevents the next Shift-click from
                                                // accidentally reusing an old selection start.
                                                staff_selection_anchor_id = None;
                                            } else if selected_staff_ids.contains(&staff.id) {
                                                selected_staff_ids.remove(&staff.id);
                                                staff_selection_anchor_id = None;
                                            } else {
                                                selected_staff_ids.insert(staff.id);
                                                staff_selection_anchor_id = Some(staff.id);
                                            }
                                        }

                                        row_response.context_menu(|ui| {
                                            if ui
                                                .button(self.localization.tr("search.open_in_staff_editor"))
                                                .clicked()
                                            {
                                                open_staff_id = Some(staff.id);
                                                ui.close_menu();
                                            }

                                            ui.separator();
                                            let ids_to_add = if selected_staff_ids.contains(&staff.id)
                                                && !selected_staff_ids.is_empty()
                                            {
                                                selected_staff_ids.iter().copied().collect::<Vec<_>>()
                                            } else {
                                                vec![staff.id]
                                            };
                                            ui.menu_button(self.localization.tr("lists.add_to_list"), |ui| {
                                                if ui
                                                    .button(self.localization.tr("lists.create_from_selection"))
                                                    .clicked()
                                                {
                                                    selected_staff_ids.extend(ids_to_add.iter().copied());
                                                    create_list_from_ids_request = Some(ids_to_add.clone());
                                                    ui.close_menu();
                                                }
                                                if !saved_lists_for_menu.is_empty() {
                                                    ui.separator();
                                                }
                                                for list_name in &saved_lists_for_menu {
                                                    if ui.button(list_name).clicked() {
                                                        add_to_list_request = Some((
                                                            list_name.clone(),
                                                            ids_to_add.clone(),
                                                        ));
                                                        ui.close_menu();
                                                    }
                                                }
                                            });
                                        });
                                    });
                                });
                            if primary_released || !primary_down {
                                if staff_shift_drag_start_id.is_some() {
                                    staff_selection_anchor_id = None;
                                }
                                staff_shift_drag_start_id = None;
                                staff_shift_drag_target_selected = None;
                                staff_shift_drag_base_ids = None;
                            }
                            self.selected_search_staff_ids = selected_staff_ids;
                            self.staff_selection_anchor_id = staff_selection_anchor_id;
                            self.staff_shift_drag_start_id = staff_shift_drag_start_id;
                            self.staff_shift_drag_target_selected =
                                staff_shift_drag_target_selected;
                            self.staff_shift_drag_base_ids = staff_shift_drag_base_ids;
                        });
                });
            },
        );

        self.staff_sort_column = sort_column;
        self.staff_sort_ascending = sort_ascending;
        if refresh_staff_requested {
            self.refresh_staff();
        }
        if let Some(staff_ids) = create_list_from_ids_request {
            self.pending_new_list_player_ids.clear();
            self.pending_new_list_staff_ids = staff_ids;
            self.list_name_popup_mode = ListNamePopupMode::Create;
            self.list_name_draft.clear();
            self.list_name_popup_open = true;
        }
        if let Some((list_name, staff_ids)) = add_to_list_request {
            self.add_staff_ids_to_list(&list_name, &staff_ids);
        }
        if let Some(staff_id) = open_staff_id {
            self.open_staff_in_editor(staff_id);
        }
    }

    fn render_player_search_page(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                ui.strong(self.localization.tr("search.players.quick_filters"));
                ui.separator();

                let advanced_button = egui::Button::new(
                    egui::RichText::new(self.localization.tr("search.staff.advanced_search"))
                        .strong()
                        .color(ui.visuals().selection.stroke.color),
                )
                .fill(ui.visuals().selection.bg_fill)
                .stroke(ui.visuals().selection.stroke);
                if ui.add(advanced_button).clicked() {
                    self.advanced_search_open = true;
                }

                let advanced_count = self.advanced_player_search.active_condition_count();
                if advanced_count > 0 {
                    let advanced_count_text = advanced_count.to_string();
                    ui.label(
                        egui::RichText::new(self.localization.tr_with(
                            "search.active_filters",
                            &[("count", advanced_count_text.as_str())],
                        ))
                        .strong()
                        .color(ui.visuals().selection.stroke.color),
                    );
                }

                ui.separator();
                ui.weak(self.localization.tr("search.players.quick_filters_info"));
            });
            ui.add_space(5.0);

            ui.horizontal_wrapped(|ui| {
                ui.label(self.localization.tr("common.name"));
                ui.add(
                    egui::TextEdit::singleline(&mut self.search_preview_filter)
                        .desired_width(190.0)
                        .hint_text(self.localization.tr("search.players.name_hint")),
                );

                ui.separator();
                ui.label(self.localization.tr("common.team"));
                let selected_team_text = if self.search_team_filter == "Any Team" {
                    self.localization.tr("search.players.any_team")
                } else {
                    self.search_team_filter.clone()
                };
                egui::ComboBox::from_id_salt("search_quick_team")
                    .selected_text(selected_team_text)
                    .width(150.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.search_team_filter,
                            "Any Team".to_string(),
                            self.localization.tr("search.players.any_team"),
                        );
                        for team in &self.teams {
                            let label = if team.display_name.trim().is_empty() {
                                format!("Team {}", team.id)
                            } else {
                                team.display_name.clone()
                            };
                            ui.selectable_value(&mut self.search_team_filter, label.clone(), label);
                        }
                    });

                ui.label(self.localization.tr("common.region"));
                let region_label = selected_multi_filter_label_localized(
                    &self.localization,
                    "search.players.any_region",
                    &REGION_FILTER_NAMES,
                    &self.search_region_filters,
                    localized_region_name,
                );
                ui.menu_button(region_label, |ui| {
                    if ui.button(self.localization.tr("common.clear")).clicked() {
                        self.search_region_filters = [false; 6];
                    }
                    ui.separator();
                    for (index, label) in REGION_FILTER_NAMES.iter().enumerate() {
                        ui.checkbox(
                            &mut self.search_region_filters[index],
                            localized_region_name(&self.localization, label),
                        );
                    }
                });

                ui.label(self.localization.tr("search.players.position"));
                let position_label = selected_multi_filter_label_localized(
                    &self.localization,
                    "search.players.any_position",
                    &POSITION_FILTER_NAMES,
                    &self.search_position_filters,
                    localized_position_name,
                );
                ui.menu_button(position_label, |ui| {
                    if ui.button(self.localization.tr("common.clear")).clicked() {
                        self.search_position_filters = [false; 5];
                    }
                    ui.separator();
                    for (index, label) in POSITION_FILTER_NAMES.iter().enumerate() {
                        ui.checkbox(
                            &mut self.search_position_filters[index],
                            localized_position_name(&self.localization, label),
                        );
                    }
                });
            });

            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(self.localization.tr("common.age"));
                ui.add(
                    egui::TextEdit::singleline(&mut self.search_age_min)
                        .desired_width(58.0)
                        .hint_text(self.localization.tr("common.min")),
                );
                ui.label(self.localization.tr("common.to"));
                ui.add(
                    egui::TextEdit::singleline(&mut self.search_age_max)
                        .desired_width(58.0)
                        .hint_text(self.localization.tr("common.max")),
                );

                ui.separator();
                ui.label(self.localization.tr("player_editor.potential.actual"));
                ui.add(
                    egui::TextEdit::singleline(&mut self.search_actual_potential_min)
                        .desired_width(58.0)
                        .hint_text(self.localization.tr("common.min")),
                );
                ui.label(self.localization.tr("common.to"));
                ui.add(
                    egui::TextEdit::singleline(&mut self.search_actual_potential_max)
                        .desired_width(58.0)
                        .hint_text(self.localization.tr("common.max")),
                );

                ui.separator();
                ui.checkbox(&mut self.search_free_agents_only, self.localization.tr("search.players.free_agents_only"));

                ui.separator();
                ui.label(self.localization.tr("lists.filter_label"));
                let active_list_text = self
                    .active_player_list_filter
                    .clone()
                    .unwrap_or_else(|| self.localization.tr("lists.all_players"));
                egui::ComboBox::from_id_salt("player_search_active_list_filter")
                    .selected_text(active_list_text)
                    .width(170.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.active_player_list_filter,
                            None,
                            self.localization.tr("lists.all_players"),
                        );
                        for list in &self.saved_player_lists {
                            ui.selectable_value(
                                &mut self.active_player_list_filter,
                                Some(list.name.clone()),
                                &list.name,
                            );
                        }
                    });
            });
        });

        ui.add_space(8.0);

        let mut refresh_players_requested = false;
        let mut reset_columns_requested = false;
        let mut open_player_id: Option<usize> = None;
        let mut add_to_list_request: Option<(String, Vec<usize>)> = None;
        let mut create_list_from_ids_request: Option<Vec<usize>> = None;
        let mut select_all_visible_requested = false;
        let mut sort_column = self.player_sort_column;
        let mut sort_ascending = self.player_sort_ascending;

        let query = self.search_preview_filter.trim().to_lowercase();
        let age_min = self.search_age_min.trim().parse::<f64>().ok();
        let age_max = self.search_age_max.trim().parse::<f64>().ok();
        let potential_min = self.search_actual_potential_min.trim().parse::<f64>().ok();
        let potential_max = self.search_actual_potential_max.trim().parse::<f64>().ok();
        let selected_team = self.search_team_filter.clone();
        let selected_regions = self.search_region_filters;
        let selected_positions = self.search_position_filters;
        let free_agents_only = self.search_free_agents_only;
        let advanced_filter = self.advanced_player_search.clone();
        let active_list_ids = self
            .active_player_list_filter
            .as_ref()
            .and_then(|name| {
                self.saved_player_lists
                    .iter()
                    .find(|list| list.name.eq_ignore_ascii_case(name))
            })
            .map(|list| list.player_ids.iter().copied().collect::<BTreeSet<_>>());

        let mut filtered_players = self
            .players
            .iter()
            .filter(|player| {
                if active_list_ids
                    .as_ref()
                    .is_some_and(|ids| !ids.contains(&player.id))
                {
                    return false;
                }

                if !query.is_empty() && !player.name.to_lowercase().contains(&query) {
                    return false;
                }

                let age = player.age.parse::<f64>().ok();
                if age_min.is_some_and(|min| age.is_none_or(|value| value < min)) {
                    return false;
                }
                if age_max.is_some_and(|max| age.is_none_or(|value| value > max)) {
                    return false;
                }

                let potential = player.actual_potential.parse::<f64>().ok();
                if potential_min.is_some_and(|min| potential.is_none_or(|value| value < min)) {
                    return false;
                }
                if potential_max.is_some_and(|max| potential.is_none_or(|value| value > max)) {
                    return false;
                }

                if selected_team != "Any Team" && player.team != selected_team {
                    return false;
                }

                if free_agents_only && player.team != "Free Agent" {
                    return false;
                }

                if selected_regions.iter().any(|selected| *selected) {
                    let region_matches = REGION_FILTER_NAMES
                        .iter()
                        .enumerate()
                        .any(|(index, label)| selected_regions[index] && player.region == *label);
                    if !region_matches {
                        return false;
                    }
                }

                if selected_positions.iter().any(|selected| *selected) {
                    let position_matches = POSITION_FILTER_NAMES
                        .iter()
                        .enumerate()
                        .any(|(index, label)| {
                            selected_positions[index]
                                && player
                                    .position
                                    .split('/')
                                    .any(|value| value.trim() == *label)
                        });
                    if !position_matches {
                        return false;
                    }
                }

                advanced_player_filter_matches(player, &advanced_filter)
            })
            .cloned()
            .collect::<Vec<_>>();

        filtered_players.sort_by(|a, b| {
            compare_player_summaries(a, b, sort_column, sort_ascending)
        });
        let filtered_player_ids = filtered_players
            .iter()
            .map(|player| player.id)
            .collect::<Vec<_>>();

        let available_height = ui.available_height().max(180.0);
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), available_height),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.group(|ui| {
                    ui.set_min_width(ui.available_width());
                    ui.set_min_height((ui.available_height() - 2.0).max(160.0));

                    ui.horizontal_wrapped(|ui| {
                        ui.strong(self.localization.tr("search.players.list_heading"));
                        let result_count = filtered_players.len().to_string();
                        ui.label(self.localization.tr_with("search.players.results", &[("count", result_count.as_str())]));
                        if ui.button(self.localization.tr("editor.refresh_players")).clicked() {
                            refresh_players_requested = true;
                        }
                        if ui.button(self.localization.tr("search.players.reset_columns")).clicked() {
                            reset_columns_requested = true;
                        }
                        ui.separator();
                        let selected_count = self.selected_search_player_ids.len().to_string();
                        ui.label(self.localization.tr_with(
                            "search.selected_count",
                            &[("count", selected_count.as_str())],
                        ));
                        if ui.button(self.localization.tr("lists.select_all_visible")).clicked() {
                            select_all_visible_requested = true;
                        }
                        if ui
                            .add_enabled(
                                !self.selected_search_player_ids.is_empty(),
                                egui::Button::new(self.localization.tr("lists.clear_selection")),
                            )
                            .clicked()
                        {
                            self.selected_search_player_ids.clear();
                            self.player_selection_anchor_id = None;
                            self.player_shift_drag_start_id = None;
                            self.player_shift_drag_target_selected = None;
                            self.player_shift_drag_base_ids = None;
                        }
                        ui.separator();
                        ui.weak(self.localization.tr(search_player_table_help_key()));
                    });
                    ui.weak(self.localization.tr(search_rating_info_key()));
                    ui.add_space(4.0);

                    let table_height = (ui.available_height() - 26.0).max(120.0);
                    let viewport_width = ui.available_width();
                    let table_min_width = 2905.0_f32.max(viewport_width);

                    egui::ScrollArea::horizontal()
                        .id_salt("search_players_table_horizontal")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.set_min_width(table_min_width);

                            let widths = [
                                42.0,  // Select
                                130.0, // Name
                                58.0,  // ID
                                52.0,  // Age
                                150.0, // Team
                                120.0, // Position
                                108.0, // Actual Rating
                                118.0, // Potential Rating
                                118.0, // Actual Potential
                                128.0, // Salary
                                104.0, // Contract End
                                100.0, // Last Hitting
                                124.0, // Skillshot Dodging
                                126.0, // Skillshot Accuracy
                                100.0, // Input Speed
                                100.0, // Positioning
                                92.0,  // Judgment
                                76.0,  // Mental
                                72.0,  // Focus
                                72.0,  // Calls
                                80.0,  // Roaming
                                92.0,  // Aggression
                                66.0,  // Ego
                                76.0,  // History
                            ];

                            let shift_down = ui.input(|input| input.modifiers.shift);
                            let primary_down = ui.input(|input| input.pointer.primary_down());
                            let primary_released = ui.input(|input| input.pointer.primary_released());

                            let mut table = TableBuilder::new(ui)
                                .id_salt("search_players_resizable_table")
                                .striped(true)
                                .resizable(true)
                                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                                .sense(egui::Sense::click_and_drag())
                                .min_scrolled_height(table_height)
                                .max_scroll_height(table_height)
                                .auto_shrink([false, false]);

                            for width in widths {
                                table = table.column(
                                    Column::initial(width)
                                        .at_least(48.0)
                                        .clip(true)
                                        .resizable(true),
                                );
                            }

                            if reset_columns_requested {
                                table.reset();
                            }

                            let mut selected_player_ids = self.selected_search_player_ids.clone();
                            let mut player_selection_anchor_id = self.player_selection_anchor_id;
                            let mut player_shift_drag_start_id = self.player_shift_drag_start_id;
                            let mut player_shift_drag_target_selected =
                                self.player_shift_drag_target_selected;
                            let mut player_shift_drag_base_ids =
                                self.player_shift_drag_base_ids.clone();
                            if select_all_visible_requested {
                                selected_player_ids.extend(filtered_players.iter().map(|player| player.id));
                                player_selection_anchor_id = None;
                                player_shift_drag_start_id = None;
                                player_shift_drag_target_selected = None;
                                player_shift_drag_base_ids = None;
                            }
                            let saved_lists_for_menu = self
                                .saved_player_lists
                                .iter()
                                .map(|list| list.name.clone())
                                .collect::<Vec<_>>();

                            table
                                .header(22.0, |mut header| {
                                    header.col(|ui| {
                                        ui.strong(self.localization.tr("lists.select_column"));
                                    });
                                    for column in [
                                        PlayerSortColumn::Name,
                                        PlayerSortColumn::Id,
                                        PlayerSortColumn::Age,
                                        PlayerSortColumn::Team,
                                        PlayerSortColumn::Position,
                                        PlayerSortColumn::ActualRating,
                                        PlayerSortColumn::PotentialRating,
                                        PlayerSortColumn::ActualPotential,
                                        PlayerSortColumn::Salary,
                                        PlayerSortColumn::ContractEnd,
                                        PlayerSortColumn::LastHitting,
                                        PlayerSortColumn::SkillshotDodging,
                                        PlayerSortColumn::SkillshotAccuracy,
                                        PlayerSortColumn::InputSpeed,
                                        PlayerSortColumn::Positioning,
                                        PlayerSortColumn::Judgment,
                                        PlayerSortColumn::Mental,
                                        PlayerSortColumn::Focus,
                                        PlayerSortColumn::Calls,
                                        PlayerSortColumn::Roaming,
                                        PlayerSortColumn::Aggression,
                                        PlayerSortColumn::Ego,
                                    ] {
                                        header.col(|ui| {
                                            player_sort_header(
                                                ui,
                                                column,
                                                &mut sort_column,
                                                &mut sort_ascending,
                                                &self.localization,
                                            );
                                        });
                                    }
                                    header.col(|ui| {
                                        ui.strong(self.localization.tr("search.tabs.history"));
                                    });
                                })
                                .body(|body| {
                                    body.rows(21.0, filtered_players.len(), |mut row| {
                                        let player = &filtered_players[row.index()];
                                        row.set_selected(selected_player_ids.contains(&player.id));
                                        let mut selection_checkbox_clicked = false;
                                        row.col(|ui| {
                                            let mut selected = selected_player_ids.contains(&player.id);
                                            let checkbox_response = ui.checkbox(&mut selected, "");
                                            selection_checkbox_clicked = checkbox_response.clicked();
                                            if checkbox_response.changed() {
                                                if selected {
                                                    selected_player_ids.insert(player.id);
                                                    player_selection_anchor_id = Some(player.id);
                                                } else {
                                                    selected_player_ids.remove(&player.id);
                                                    player_selection_anchor_id = None;
                                                }
                                                player_shift_drag_start_id = None;
                                                player_shift_drag_target_selected = None;
                                                player_shift_drag_base_ids = None;
                                            }
                                        });
                                        row.col(|ui| { ui.label(&player.name); });
                                        row.col(|ui| { ui.label(player.id.to_string()); });
                                        row.col(|ui| { ui.label(value_or_dash(&player.age)); });
                                        row.col(|ui| { ui.label(value_or_dash(&player.team)); });
                                        row.col(|ui| {
                                            ui.label(localized_position_summary(&self.localization, &player.position));
                                        });
                                        row.col(|ui| { render_actual_rating(ui, player, &self.localization); });
                                        row.col(|ui| { potential_rating_stars(ui, &player.actual_potential); });
                                        row.col(|ui| { ui.label(pretty_or_dash(&player.actual_potential)); });
                                        row.col(|ui| { ui.label(pretty_or_dash(&player.salary)); });
                                        row.col(|ui| { ui.label(value_or_dash(&display_contract_date(&player.contract_end))); });
                                        row.col(|ui| { ui.label(pretty_or_dash(&player.last_hit)); });
                                        row.col(|ui| { ui.label(pretty_or_dash(&player.skill_avoid)); });
                                        row.col(|ui| { ui.label(pretty_or_dash(&player.skill_hit)); });
                                        row.col(|ui| { ui.label(pretty_or_dash(&player.control_speed)); });
                                        row.col(|ui| { ui.label(pretty_or_dash(&player.positioning)); });
                                        row.col(|ui| { ui.label(pretty_or_dash(&player.judgement)); });
                                        row.col(|ui| { ui.label(pretty_or_dash(&player.mental)); });
                                        row.col(|ui| { ui.label(pretty_or_dash(&player.concentration)); });
                                        row.col(|ui| { ui.label(pretty_or_dash(&player.order)); });
                                        row.col(|ui| { ui.label(pretty_or_dash(&player.roaming)); });
                                        row.col(|ui| { ui.label(pretty_or_dash(&player.aggressive)); });
                                        row.col(|ui| { ui.label(pretty_or_dash(&player.ego)); });
                                        row.col(|ui| {
                                            ui.add_enabled(
                                                false,
                                                egui::Button::new(
                                                    self.localization.tr("search.tabs.history"),
                                                ),
                                            );
                                        });

                                        let row_response = row.response();
                                        if shift_down
                                            && row_response.drag_started_by(egui::PointerButton::Primary)
                                            && !selection_checkbox_clicked
                                        {
                                            player_shift_drag_start_id = Some(player.id);
                                            player_shift_drag_target_selected =
                                                Some(!selected_player_ids.contains(&player.id));
                                            player_shift_drag_base_ids = Some(selected_player_ids.clone());
                                            player_selection_anchor_id = None;
                                        }
                                        if (primary_down || primary_released) && row_response.contains_pointer() {
                                            if let (
                                                Some(start_id),
                                                Some(target_selected),
                                                Some(base_ids),
                                            ) = (
                                                player_shift_drag_start_id,
                                                player_shift_drag_target_selected,
                                                player_shift_drag_base_ids.as_ref(),
                                            ) {
                                                selected_player_ids = base_ids.clone();
                                                apply_id_range_selection(
                                                    &filtered_player_ids,
                                                    start_id,
                                                    player.id,
                                                    target_selected,
                                                    &mut selected_player_ids,
                                                );
                                            }
                                        }

                                        let shift_drag_active = player_shift_drag_start_id.is_some();
                                        if row_response.double_clicked() && !selection_checkbox_clicked {
                                            selected_player_ids.insert(player.id);
                                            player_selection_anchor_id = None;
                                            player_shift_drag_start_id = None;
                                            player_shift_drag_target_selected = None;
                                            player_shift_drag_base_ids = None;
                                            open_player_id = Some(player.id);
                                        } else if row_response.clicked()
                                            && !selection_checkbox_clicked
                                            && !shift_drag_active
                                        {
                                            if shift_down {
                                                let target_selected = !selected_player_ids.contains(&player.id);
                                                let anchor_id =
                                                    player_selection_anchor_id.unwrap_or(player.id);
                                                apply_id_range_selection(
                                                    &filtered_player_ids,
                                                    anchor_id,
                                                    player.id,
                                                    target_selected,
                                                    &mut selected_player_ids,
                                                );
                                                // A completed Shift-click is a one-shot range action.
                                                // Resetting the anchor prevents the next Shift-click from
                                                // accidentally reusing an old selection start.
                                                player_selection_anchor_id = None;
                                            } else if selected_player_ids.contains(&player.id) {
                                                selected_player_ids.remove(&player.id);
                                                player_selection_anchor_id = None;
                                            } else {
                                                selected_player_ids.insert(player.id);
                                                player_selection_anchor_id = Some(player.id);
                                            }
                                        }
                                        row_response.context_menu(|ui| {
                                            if ui
                                                .button(self.localization.tr("search.open_in_player_editor"))
                                                .clicked()
                                            {
                                                open_player_id = Some(player.id);
                                                ui.close_menu();
                                            }

                                            ui.separator();
                                            let ids_to_add = if selected_player_ids.contains(&player.id)
                                                && !selected_player_ids.is_empty()
                                            {
                                                selected_player_ids.iter().copied().collect::<Vec<_>>()
                                            } else {
                                                vec![player.id]
                                            };
                                            ui.menu_button(self.localization.tr("lists.add_to_list"), |ui| {
                                                if ui
                                                    .button(self.localization.tr("lists.create_from_selection"))
                                                    .clicked()
                                                {
                                                    selected_player_ids.extend(ids_to_add.iter().copied());
                                                    create_list_from_ids_request = Some(ids_to_add.clone());
                                                    ui.close_menu();
                                                }
                                                if !saved_lists_for_menu.is_empty() {
                                                    ui.separator();
                                                }
                                                for list_name in &saved_lists_for_menu {
                                                    if ui.button(list_name).clicked() {
                                                        add_to_list_request = Some((
                                                            list_name.clone(),
                                                            ids_to_add.clone(),
                                                        ));
                                                        ui.close_menu();
                                                    }
                                                }
                                            });
                                        });
                                    });
                                });
                            if primary_released || !primary_down {
                                if player_shift_drag_start_id.is_some() {
                                    player_selection_anchor_id = None;
                                }
                                player_shift_drag_start_id = None;
                                player_shift_drag_target_selected = None;
                                player_shift_drag_base_ids = None;
                            }
                            self.selected_search_player_ids = selected_player_ids;
                            self.player_selection_anchor_id = player_selection_anchor_id;
                            self.player_shift_drag_start_id = player_shift_drag_start_id;
                            self.player_shift_drag_target_selected =
                                player_shift_drag_target_selected;
                            self.player_shift_drag_base_ids = player_shift_drag_base_ids;
                        });
                });
            },
        );

        self.player_sort_column = sort_column;
        self.player_sort_ascending = sort_ascending;
        if refresh_players_requested {
            self.refresh_players();
        }
        if let Some(player_ids) = create_list_from_ids_request {
            self.pending_new_list_player_ids = player_ids;
            self.pending_new_list_staff_ids.clear();
            self.list_name_popup_mode = ListNamePopupMode::Create;
            self.list_name_draft.clear();
            self.list_name_popup_open = true;
        }
        if let Some((list_name, player_ids)) = add_to_list_request {
            self.add_player_ids_to_list(&list_name, &player_ids);
        }
        if let Some(athlete_id) = open_player_id {
            self.open_player_in_editor(athlete_id);
        }
    }

    fn render_advanced_staff_search_window(&mut self, ctx: &egui::Context) {
        if !self.advanced_staff_search_open {
            return;
        }

        let mut open = self.advanced_staff_search_open;
        let mut close_after = false;
        let mut reset_filter = false;
        let mut import_filter = false;
        let mut export_filter = false;

        egui::Window::new(self.localization.tr("search.staff.advanced_window_title"))
            .id(egui::Id::new("advanced_staff_search_window_v040i"))
            .open(&mut open)
            .resizable(true)
            .default_size(egui::vec2(920.0, 680.0))
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.button(self.localization.tr("search.advanced.new_filter")).clicked() {
                        reset_filter = true;
                    }
                    if ui.button(self.localization.tr("search.advanced.import_filter")).clicked() {
                        import_filter = true;
                    }
                    if ui.button(self.localization.tr("search.advanced.export_filter")).clicked() {
                        export_filter = true;
                    }
                    ui.separator();
                    ui.weak(self.localization.tr("search.staff.saved_filters_info"));
                });

                ui.separator();
                let footer_height = 34.0;
                let body_height = (ui.available_height() - footer_height).max(0.0);
                let body_width = ui.available_width();
                let mut saved_filter_to_load: Option<String> = None;

                ui.allocate_ui_with_layout(
                    egui::vec2(body_width, body_height),
                    egui::Layout::left_to_right(egui::Align::Min),
                    |body_ui| {
                        let full_height = body_ui.available_height();
                        let available_body_width = body_ui.available_width();
                        let max_left_width = (available_body_width - 90.0).max(90.0);
                        self.saved_staff_filters_width =
                            self.saved_staff_filters_width.clamp(90.0, max_left_width);

                        body_ui.allocate_ui_with_layout(
                            egui::vec2(self.saved_staff_filters_width, full_height),
                            egui::Layout::top_down(egui::Align::Min),
                            |left_ui| {
                                left_ui.set_min_height(full_height);
                                left_ui.strong(self.localization.tr("search.advanced.saved_filters"));
                                left_ui.separator();

                                let list_height = left_ui.available_height();
                                egui::ScrollArea::vertical()
                                    .id_salt("saved_staff_filters_scroll")
                                    .auto_shrink([false, false])
                                    .max_height(list_height)
                                    .show(left_ui, |ui| {
                                        ui.set_min_height(list_height);
                                        if self.saved_staff_filters.is_empty() {
                                            ui.weak(self.localization.tr("search.advanced.no_saved_filters"));
                                        }
                                        for name in self.saved_staff_filters.clone() {
                                            let selected = self
                                                .selected_saved_staff_filter
                                                .as_deref()
                                                .is_some_and(|value| value.eq_ignore_ascii_case(&name));
                                            if ui.selectable_label(selected, &name).clicked() {
                                                saved_filter_to_load = Some(name);
                                            }
                                        }
                                    });
                            },
                        );

                        let (divider_rect, divider_response) = body_ui.allocate_exact_size(
                            egui::vec2(8.0, full_height),
                            egui::Sense::click_and_drag(),
                        );
                        let divider_response =
                            divider_response.on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                        if divider_response.dragged() {
                            let pointer_dx = body_ui.input(|input| input.pointer.delta().x);
                            self.saved_staff_filters_width =
                                (self.saved_staff_filters_width + pointer_dx)
                                    .clamp(90.0, max_left_width);
                        }
                        let divider_color = if divider_response.hovered()
                            || divider_response.dragged()
                        {
                            body_ui.visuals().selection.stroke.color
                        } else {
                            body_ui.visuals().widgets.noninteractive.bg_stroke.color
                        };
                        body_ui.painter().vline(
                            divider_rect.center().x,
                            divider_rect.y_range(),
                            egui::Stroke::new(1.0_f32, divider_color),
                        );

                        let right_width = body_ui.available_width();
                        body_ui.allocate_ui_with_layout(
                            egui::vec2(right_width, full_height),
                            egui::Layout::top_down(egui::Align::Min),
                            |right_ui| {
                                right_ui.set_min_height(full_height);
                                egui::ScrollArea::vertical()
                                    .id_salt("advanced_staff_search_scroll")
                                    .auto_shrink([false, false])
                                    .max_height(full_height)
                                    .show(right_ui, |ui| {
                                        ui.set_min_height(full_height);

                                        ui.horizontal(|ui| {
                                            ui.add_sized(
                                                [20.0, 24.0],
                                                egui::Checkbox::without_text(
                                                    &mut self.advanced_staff_search.role_enabled,
                                                ),
                                            );
                                            ui.add_sized(
                                                [138.0, 24.0],
                                                egui::Label::new(self.localization.tr("common.role")),
                                            );
                                            ui.add_enabled_ui(
                                                self.advanced_staff_search.role_enabled,
                                                |ui| {
                                                    let selected = if self.advanced_staff_search.role
                                                        == "No Condition"
                                                    {
                                                        self.localization.tr("search.no_condition")
                                                    } else {
                                                        localized_staff_role(
                                                            &self.localization,
                                                            &self.advanced_staff_search.role,
                                                        )
                                                    };
                                                    egui::ComboBox::from_id_salt(
                                                        "advanced_staff_role_choice",
                                                    )
                                                    .selected_text(selected)
                                                    .width(198.0)
                                                    .show_ui(ui, |ui| {
                                                        ui.selectable_value(
                                                            &mut self.advanced_staff_search.role,
                                                            "No Condition".to_string(),
                                                            self.localization.tr("search.no_condition"),
                                                        );
                                                        for role in [
                                                            "HeadCoach",
                                                            "TrainingCoach",
                                                            "Scouter",
                                                            "Analyst",
                                                        ] {
                                                            ui.selectable_value(
                                                                &mut self.advanced_staff_search.role,
                                                                role.to_string(),
                                                                localized_staff_role(
                                                                    &self.localization,
                                                                    role,
                                                                ),
                                                            );
                                                        }
                                                    });
                                                },
                                            );
                                        });

                                        for range in &mut self.advanced_staff_search.ranges {
                                            advanced_range_filter_row(ui, range, &self.localization);
                                        }

                                        advanced_boolean_filter_row(
                                            ui,
                                            &self.localization.tr("search.players.free_agents_only"),
                                            &mut self.advanced_staff_search.free_agents_only,
                                        );
                                    });
                            },
                        );
                    },
                );

                if let Some(name) = saved_filter_to_load {
                    self.load_saved_staff_filter(&name);
                }

                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    if ui.button(self.localization.tr("common.reset")).clicked() {
                        reset_filter = true;
                    }
                    if ui.button(self.localization.tr("common.confirm")).clicked() {
                        self.status = format!(
                            "Advanced Staff Search applied: {} active condition(s)",
                            self.advanced_staff_search.active_condition_count()
                        );
                        close_after = true;
                    }

                    ui.separator();
                    if ui.button(self.localization.tr("search.advanced.save_filter")).clicked() {
                        self.staff_filter_name_draft = self
                            .selected_saved_staff_filter
                            .clone()
                            .unwrap_or_default();
                        self.staff_filter_name_popup_open = true;
                    }

                    let update = ui.add_enabled(
                        self.selected_saved_staff_filter.is_some(),
                        egui::Button::new(self.localization.tr("search.advanced.update_filter")),
                    );
                    if update.clicked() {
                        self.update_selected_staff_filter();
                    }

                    let delete = ui.add_enabled(
                        self.selected_saved_staff_filter.is_some(),
                        egui::Button::new(self.localization.tr("search.advanced.delete_filter")),
                    );
                    if delete.clicked() {
                        self.delete_selected_staff_filter();
                    }
                });
            });

        if reset_filter {
            self.advanced_staff_search = AdvancedStaffSearch::default();
            self.selected_saved_staff_filter = None;
            self.status = "Advanced staff filter reset".to_string();
        }
        if import_filter {
            self.import_advanced_staff_filter();
        }
        if export_filter {
            self.export_advanced_staff_filter();
        }
        if close_after {
            open = false;
        }
        self.advanced_staff_search_open = open;
        self.render_staff_filter_name_popup(ctx);
    }

    fn render_staff_filter_name_popup(&mut self, ctx: &egui::Context) {
        if !self.staff_filter_name_popup_open {
            return;
        }

        let mut open = self.staff_filter_name_popup_open;
        let mut save_requested = false;
        let mut cancel_requested = false;

        egui::Window::new(self.localization.tr("search.staff.filter_name_window"))
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .default_width(360.0)
            .show(ctx, |ui| {
                ui.label(self.localization.tr("search.filter_name.label"));
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.staff_filter_name_draft)
                        .desired_width(f32::INFINITY)
                        .hint_text(self.localization.tr("search.staff.filter_name_hint")),
                );
                if response.lost_focus()
                    && ui.input(|input| input.key_pressed(egui::Key::Enter))
                {
                    save_requested = true;
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(self.localization.tr("common.save")).clicked() {
                        save_requested = true;
                    }
                    if ui.button(self.localization.tr("common.cancel")).clicked() {
                        cancel_requested = true;
                    }
                });
            });

        if save_requested {
            let name = self.staff_filter_name_draft.clone();
            self.save_named_staff_filter(&name, false);
            open = false;
        }
        if cancel_requested {
            open = false;
        }
        self.staff_filter_name_popup_open = open;
    }

    fn render_advanced_search_window(&mut self, ctx: &egui::Context) {
        if !self.advanced_search_open {
            return;
        }

        let mut open = self.advanced_search_open;
        let mut close_after = false;
        let mut reset_filter = false;
        let mut import_filter = false;
        let mut export_filter = false;

        egui::Window::new(self.localization.tr("search.advanced.window_title"))
            // New persistent ID intentionally resets the undersized window rect
            // that could be remembered from v0.2.17.
            .id(egui::Id::new("advanced_player_search_window_v0218"))
            .open(&mut open)
            .resizable(true)
            .default_size(egui::vec2(1040.0, 720.0))
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.button(self.localization.tr("search.advanced.new_filter")).clicked() {
                        reset_filter = true;
                    }
                    if ui.button(self.localization.tr("search.advanced.import_filter")).clicked() {
                        import_filter = true;
                    }
                    if ui.button(self.localization.tr("search.advanced.export_filter")).clicked() {
                        export_filter = true;
                    }

                    ui.separator();
                    ui.weak(self.localization.tr("search.advanced.saved_on_left"));
                });

                ui.add_space(4.0);
                ui.weak(self.localization.tr(advanced_search_info_key()));
                ui.separator();

                // Reserve a small fixed footer, then let the filter body consume
                // every remaining vertical pixel in the resizable window.
                let footer_height = 34.0;
                let body_height = (ui.available_height() - footer_height).max(0.0);
                let body_width = ui.available_width();
                let mut saved_filter_to_load: Option<String> = None;

                ui.allocate_ui_with_layout(
                    egui::vec2(body_width, body_height),
                    egui::Layout::left_to_right(egui::Align::Min),
                    |body_ui| {
                        let full_height = body_ui.available_height();

                        // Resizable Saved Filters column. The divider follows
                        // the pointer while dragging and keeps the user's width
                        // for the rest of the editor session.
                        let available_body_width = body_ui.available_width();
                        let max_left_width = (available_body_width - 90.0).max(90.0);
                        self.saved_filters_width =
                            self.saved_filters_width.clamp(90.0, max_left_width);

                        body_ui.allocate_ui_with_layout(
                            egui::vec2(self.saved_filters_width, full_height),
                            egui::Layout::top_down(egui::Align::Min),
                            |left_ui| {
                                left_ui.set_min_height(full_height);
                                left_ui.strong(self.localization.tr("search.advanced.saved_filters"));
                                left_ui.separator();

                                let list_height = left_ui.available_height();
                                egui::ScrollArea::vertical()
                                    .id_salt("saved_player_filters_scroll")
                                    .auto_shrink([false, false])
                                    .max_height(list_height)
                                    .show(left_ui, |ui| {
                                        ui.set_min_height(list_height);

                                        if self.saved_filters.is_empty() {
                                            ui.weak(self.localization.tr("search.advanced.no_saved_filters"));
                                        }

                                        for name in self.saved_filters.clone() {
                                            let selected = self
                                                .selected_saved_filter
                                                .as_deref()
                                                .is_some_and(|value| {
                                                    value.eq_ignore_ascii_case(&name)
                                                });

                                            if ui.selectable_label(selected, &name).clicked() {
                                                saved_filter_to_load = Some(name);
                                            }
                                        }
                                    });
                            },
                        );

                        let (divider_rect, divider_response) = body_ui.allocate_exact_size(
                            egui::vec2(8.0, full_height),
                            egui::Sense::click_and_drag(),
                        );
                        let divider_response =
                            divider_response.on_hover_cursor(egui::CursorIcon::ResizeHorizontal);

                        if divider_response.dragged() {
                            let pointer_dx = body_ui.input(|input| input.pointer.delta().x);
                            self.saved_filters_width =
                                (self.saved_filters_width + pointer_dx).clamp(90.0, max_left_width);
                        }

                        let divider_color = if divider_response.hovered()
                            || divider_response.dragged()
                        {
                            body_ui.visuals().selection.stroke.color
                        } else {
                            body_ui.visuals().widgets.noninteractive.bg_stroke.color
                        };

                        body_ui.painter().vline(
                            divider_rect.center().x,
                            divider_rect.y_range(),
                            egui::Stroke::new(1.0_f32, divider_color),
                        );

                        let right_width = body_ui.available_width();
                        body_ui.allocate_ui_with_layout(
                            egui::vec2(right_width, full_height),
                            egui::Layout::top_down(egui::Align::Min),
                            |right_ui| {
                                right_ui.set_min_height(full_height);

                                egui::ScrollArea::vertical()
                                    .id_salt("advanced_player_search_scroll")
                                    .auto_shrink([false, false])
                                    .max_height(full_height)
                                    .show(right_ui, |ui| {
                                        ui.set_min_height(full_height);

                                        advanced_choice_filter_row(
                                            ui,
                                            &self.localization.tr("search.players.position"),
                                            &mut self.advanced_player_search.position_enabled,
                                            &mut self.advanced_player_search.position,
                                            &[
                                                ("No Condition", "search.no_condition"),
                                                ("Top", "positions.top"),
                                                ("Jungle", "positions.jungle"),
                                                ("Mid", "positions.mid"),
                                                ("Bottom", "positions.bottom"),
                                                ("Support", "positions.support"),
                                            ],
                                            "advanced_position_choice",
                                            &self.localization,
                                        );
                                        advanced_choice_filter_row(
                                            ui,
                                            &self.localization.tr("common.region"),
                                            &mut self.advanced_player_search.region_enabled,
                                            &mut self.advanced_player_search.region,
                                            &[
                                                ("No Condition", "search.no_condition"),
                                                ("Korea", "regions.korea"),
                                                ("China", "regions.china"),
                                                ("Europe", "regions.europe"),
                                                ("North America", "regions.north_america"),
                                                ("South America", "regions.south_america"),
                                                ("Japan", "regions.japan"),
                                            ],
                                            "advanced_region_choice",
                                            &self.localization,
                                        );

                                        for range in &mut self.advanced_player_search.ranges {
                                            advanced_range_filter_row(ui, range, &self.localization);
                                        }

                                        advanced_boolean_filter_row(
                                            ui,
                                            &self.localization.tr("search.players.free_agents_only"),
                                            &mut self.advanced_player_search.free_agents_only,
                                        );
                                    });
                            },
                        );
                    },
                );

                if let Some(name) = saved_filter_to_load {
                    self.load_saved_filter(&name);
                }

                // Fixed footer: this stays at the bottom because the body above
                // consumes the remaining window height.
                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    if ui.button(self.localization.tr("common.reset")).clicked() {
                        reset_filter = true;
                    }
                    if ui.button(self.localization.tr("common.confirm")).clicked() {
                        self.status = format!(
                            "Advanced Search applied: {} active condition(s)",
                            self.advanced_player_search.active_condition_count()
                        );
                        close_after = true;
                    }

                    ui.separator();

                    if ui.button(self.localization.tr("search.advanced.save_filter")).clicked() {
                        self.filter_name_draft =
                            self.selected_saved_filter.clone().unwrap_or_default();
                        self.filter_name_popup_open = true;
                    }

                    let update = ui.add_enabled(
                        self.selected_saved_filter.is_some(),
                        egui::Button::new(self.localization.tr("search.advanced.update_filter")),
                    );
                    if update.clicked() {
                        self.update_selected_filter();
                    }

                    let delete = ui.add_enabled(
                        self.selected_saved_filter.is_some(),
                        egui::Button::new(self.localization.tr("search.advanced.delete_filter")),
                    );
                    if delete.clicked() {
                        self.delete_selected_filter();
                    }
                });
            });

        if reset_filter {
            self.advanced_player_search = AdvancedPlayerSearch::default();
            self.selected_saved_filter = None;
            self.status = "Advanced player filter reset".to_string();
        }
        if import_filter {
            self.import_advanced_filter();
        }
        if export_filter {
            self.export_advanced_filter();
        }
        if close_after {
            open = false;
        }
        self.advanced_search_open = open;

        self.render_filter_name_popup(ctx);
    }

    fn render_filter_name_popup(&mut self, ctx: &egui::Context) {
        if !self.filter_name_popup_open {
            return;
        }

        let mut open = self.filter_name_popup_open;
        let mut save_requested = false;
        let mut cancel_requested = false;

        egui::Window::new(self.localization.tr("search.filter_name.window_title"))
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .default_width(360.0)
            .show(ctx, |ui| {
                ui.label(self.localization.tr("search.filter_name.label"));
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.filter_name_draft)
                        .desired_width(f32::INFINITY)
                        .hint_text(self.localization.tr("search.filter_name.hint")),
                );

                if response.lost_focus()
                    && ui.input(|input| input.key_pressed(egui::Key::Enter))
                {
                    save_requested = true;
                }

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(self.localization.tr("common.save")).clicked() {
                        save_requested = true;
                    }
                    if ui.button(self.localization.tr("common.cancel")).clicked() {
                        cancel_requested = true;
                    }
                });
            });

        if save_requested {
            let name = self.filter_name_draft.trim().to_string();
            if name.is_empty() {
                self.status = "Enter a filter name".to_string();
            } else {
                self.save_named_filter(&name, false);
                self.filter_name_draft.clear();
                open = false;
            }
        }

        if cancel_requested {
            open = false;
        }

        self.filter_name_popup_open = open;
    }



    fn render_list_name_popup(&mut self, ctx: &egui::Context) {
        if !self.list_name_popup_open {
            return;
        }

        let mut open = self.list_name_popup_open;
        let mut save_requested = false;
        let mut cancel_requested = false;
        let title_key = match self.list_name_popup_mode {
            ListNamePopupMode::Create => "lists.create_window_title",
            ListNamePopupMode::Rename => "lists.rename_window_title",
        };

        egui::Window::new(self.localization.tr(title_key))
            .id(egui::Id::new("player_list_name_popup"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(380.0)
            .show(ctx, |ui| {
                ui.label(self.localization.tr("lists.name_label"));
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.list_name_draft)
                        .desired_width(f32::INFINITY)
                        .hint_text(self.localization.tr("lists.name_hint")),
                );
                response.request_focus();
                if response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                    save_requested = true;
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let action_key = match self.list_name_popup_mode {
                        ListNamePopupMode::Create => "lists.create",
                        ListNamePopupMode::Rename => "lists.rename",
                    };
                    if ui.button(self.localization.tr(action_key)).clicked() {
                        save_requested = true;
                    }
                    if ui.button(self.localization.tr("common.cancel")).clicked() {
                        cancel_requested = true;
                    }
                });
            });

        if save_requested {
            let name = self.list_name_draft.trim().to_string();
            if name.is_empty() {
                self.status = self.localization.tr("lists.status.enter_name");
            } else {
                match self.list_name_popup_mode {
                    ListNamePopupMode::Create => self.create_named_player_list(&name),
                    ListNamePopupMode::Rename => self.rename_selected_player_list(&name),
                }
                self.list_name_draft.clear();
                open = false;
            }
        }
        if cancel_requested {
            if self.list_name_popup_mode == ListNamePopupMode::Create {
                self.pending_new_list_player_ids.clear();
            }
            open = false;
        }
        self.list_name_popup_open = open;
    }

    fn render_list_delete_confirmation(&mut self, ctx: &egui::Context) {
        if !self.list_delete_confirmation_open {
            return;
        }

        let mut open = self.list_delete_confirmation_open;
        let mut delete_requested = false;
        let mut cancel_requested = false;
        let name = self.selected_saved_player_list.clone().unwrap_or_default();

        egui::Window::new(self.localization.tr("lists.delete_window_title"))
            .id(egui::Id::new("player_list_delete_confirmation"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(420.0)
            .show(ctx, |ui| {
                ui.label(self.localization.tr_with(
                    "lists.delete_confirmation",
                    &[("name", name.as_str())],
                ));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(self.localization.tr("lists.delete")).clicked() {
                        delete_requested = true;
                    }
                    if ui.button(self.localization.tr("common.cancel")).clicked() {
                        cancel_requested = true;
                    }
                });
            });

        if delete_requested {
            self.delete_selected_player_list();
            self.selected_list_player_ids.clear();
            open = false;
        }
        if cancel_requested {
            open = false;
        }
        self.list_delete_confirmation_open = open;
    }


}

impl eframe::App for ModifierApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("app_header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(self.localization.tr("app.title"));
                ui.weak(self.localization.tr_with("app.made_by", &[("author", "jal-io")]));
                ui.separator();
                ui.label(display_version());

                #[cfg(feature = "dev")]
                {
                    ui.label(
                        egui::RichText::new(self.localization.tr("app.dev_build"))
                            .strong()
                            .color(ui.visuals().selection.stroke.color),
                    );
                }
                ui.separator();

                let bridge_display = if self.bridge_version == "-" {
                    "—".to_string()
                } else {
                    format!("v{}", self.bridge_version)
                };
                let (connection_key, compatibility_key, compatibility_color) =
                    match self.compatibility_issue.as_ref() {
                        Some(issue)
                            if issue.severity == CompatibilitySeverity::NotSupported =>
                        {
                            (
                                "connection.state.disconnected",
                                "compatibility.status.not_supported",
                                egui::Color32::from_rgb(220, 70, 70),
                            )
                        }
                        Some(_) if self.connected => (
                            "connection.state.connected",
                            "compatibility.status.warning",
                            egui::Color32::from_rgb(235, 196, 0),
                        ),
                        None if self.connected => (
                            "connection.state.connected",
                            "compatibility.status.ok",
                            egui::Color32::from_rgb(80, 190, 100),
                        ),
                        _ => (
                            "connection.state.disconnected",
                            "compatibility.status.unknown",
                            egui::Color32::GRAY,
                        ),
                    };
                let connection_text = self.localization.tr(connection_key);
                let compatibility_text = self.localization.tr(compatibility_key);
                let compatibility_prefix = self.localization.tr_with(
                    "connection.compatibility_prefix",
                    &[
                        ("connection", connection_text.as_str()),
                        ("bridge", bridge_display.as_str()),
                    ],
                );
                let compatibility_response = ui
                    .horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.label(compatibility_prefix);
                        ui.label(
                            egui::RichText::new(compatibility_text)
                                .strong()
                                .color(compatibility_color),
                        );
                    })
                    .response;

                #[cfg(feature = "dev")]
                compatibility_response.on_hover_text(self.compatibility_debug_report());
                #[cfg(not(feature = "dev"))]
                let _ = compatibility_response;

                if ui.button(self.localization.tr("connection.reconnect")).clicked() {
                    self.refresh_connection();
                    if self.connected {
                        self.refresh_economy();
                        self.refresh_players();
                        self.refresh_staff();
                        self.refresh_teams();
                        self.refresh_recruitment_settings();
                        self.restore_active_search_status();
                    }
                }

                ui.separator();
                self.render_language_selector(ui);
            });

            ui.add_space(8.0);
            let tab_before_click = self.active_tab;
            ui.scope(|ui| {
                ui.spacing_mut().button_padding = egui::vec2(10.0, 5.0);
                ui.spacing_mut().item_spacing.x = 7.0;
                ui.horizontal(|ui| {
                    for tab in AppTab::ALL {
                        let label = self.localization.tr(tab.label_key());
                        ui.selectable_value(
                            &mut self.active_tab,
                            tab,
                            egui::RichText::new(label).size(15.0).strong(),
                        );
                    }
                });
            });
            if tab_before_click != self.active_tab
                && self.active_tab == AppTab::Search
                && self.connected
            {
                self.refresh_players();
                self.refresh_staff();
                self.restore_active_search_status();
            }
            #[cfg(feature = "dev")]
            if tab_before_click != self.active_tab && self.active_tab == AppTab::Team {
                if let Some(team) = self
                    .team_workspace_team_id
                    .and_then(|team_id| self.teams.iter().find(|team| team.id == team_id))
                {
                    self.status = format!("Team data loaded: {}", team.display_name);
                }
            }
            ui.add_space(4.0);
        });

        egui::TopBottomPanel::bottom("app_status").show(ctx, |ui| {
            ui.separator();
            let status = self.status.clone();
            ui.label(self.localization.tr_with("status.label", &[("status", status.as_str())]));
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.active_tab == AppTab::Search {
                self.render_search_tab(ui);
            } else {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.active_tab {
                        AppTab::Economy => self.render_economy_tab(ui),
                        AppTab::PlayerEditor => self.render_player_editor_tab(ui),
                        AppTab::StaffEditor => self.render_staff_editor_tab(ui),
                        #[cfg(feature = "dev")]
                        AppTab::Team => self.render_team_workspace_tab(ui),
                        AppTab::Recruitment => self.render_recruitment_tab(ui),
                        AppTab::Search => unreachable!(),
                    });
            }
        });

        self.render_advanced_search_window(ctx);
        self.render_advanced_staff_search_window(ctx);

        self.render_champion_mastery_window(ctx);

        #[cfg(feature = "dev")]
        self.render_team_roster_window(ctx);
        #[cfg(feature = "dev")]
        self.render_team_staff_window(ctx);
        #[cfg(feature = "dev")]
        self.render_team_condition_window(ctx);
        #[cfg(feature = "dev")]
        self.render_team_data_probe_window(ctx);
        #[cfg(feature = "dev")]
        self.render_team_strategy_window(ctx);
        #[cfg(feature = "dev")]
        self.render_team_merchandise_window(ctx);
        #[cfg(feature = "dev")]
        self.render_team_champion_setup_window(ctx);
        #[cfg(feature = "dev")]
        self.render_team_gaming_house_window(ctx);
        #[cfg(feature = "dev")]
        self.render_team_match_history_window(ctx);
        #[cfg(feature = "dev")]
        self.render_team_pre_match_analysis_window(ctx);
        #[cfg(feature = "dev")]
        self.render_team_history_summary_window(ctx);

        self.render_player_contract_window(ctx);
        #[cfg(feature = "dev")]
        self.render_player_contract_probe_window(ctx);

        self.render_staff_contract_window(ctx);
        #[cfg(feature = "dev")]
        self.render_staff_contract_probe_window(ctx);

        self.render_list_name_popup(ctx);
        self.render_list_delete_confirmation(ctx);
        self.render_compatibility_popup(ctx);
    }
}



fn player_sort_header(
    ui: &mut egui::Ui,
    column: PlayerSortColumn,
    sort_column: &mut PlayerSortColumn,
    sort_ascending: &mut bool,
    localization: &Localization,
) {
    let is_active = *sort_column == column;
    let arrow = if is_active {
        if *sort_ascending { " ↑" } else { " ↓" }
    } else {
        ""
    };

    if ui
        .button(format!("{}{}", localization.tr(column.label_key()), arrow))
        .clicked()
    {
        if is_active {
            *sort_ascending = !*sort_ascending;
        } else {
            *sort_column = column;
            *sort_ascending = true;
        }
    }
}


fn localized_staff_role(localization: &Localization, raw: &str) -> String {
    match raw.trim() {
        "HeadCoach" => localization.tr("staff.roles.head_coach"),
        "TrainingCoach" => localization.tr("staff.roles.training_coach"),
        "Scouter" => localization.tr("staff.roles.scouter"),
        "Analyst" => localization.tr("staff.roles.analyst"),
        "" => localization.tr("common.unknown"),
        other => other.to_string(),
    }
}

fn staff_sort_header(
    ui: &mut egui::Ui,
    column: StaffSortColumn,
    sort_column: &mut StaffSortColumn,
    sort_ascending: &mut bool,
    localization: &Localization,
) {
    let is_active = *sort_column == column;
    let arrow = if is_active {
        if *sort_ascending { " ↑" } else { " ↓" }
    } else {
        ""
    };

    if ui
        .button(format!("{}{}", localization.tr(column.label_key()), arrow))
        .clicked()
    {
        if is_active {
            *sort_ascending = !*sort_ascending;
        } else {
            *sort_column = column;
            *sort_ascending = true;
        }
    }
}

fn team_sort_header(
    ui: &mut egui::Ui,
    column: TeamSortColumn,
    sort_column: &mut TeamSortColumn,
    sort_ascending: &mut bool,
    localization: &Localization,
) {
    let is_active = *sort_column == column;
    let arrow = if is_active {
        if *sort_ascending { " ↑" } else { " ↓" }
    } else {
        ""
    };

    if ui
        .button(format!("{}{}", localization.tr(column.label_key()), arrow))
        .clicked()
    {
        if is_active {
            *sort_ascending = !*sort_ascending;
        } else {
            *sort_column = column;
            *sort_ascending = true;
        }
    }
}

fn staff_advanced_range_value(staff: &StaffSummary, key: &str) -> Option<f64> {
    let raw = match key {
        "age" => &staff.age,
        "salary" => &staff.annual_salary,
        "banpick" => &staff.banpick,
        "strategy" => &staff.strategy,
        "negotiation" => &staff.negotiation,
        "judge_ability" => &staff.judge_ability,
        "judge_potential" => &staff.judge_potential,
        "feedback" => &staff.feedback,
        "power_analysis" => &staff.power_analysis,
        "control_coaching" => &staff.control_coaching,
        "judgment_coaching" => &staff.judgment_coaching,
        "mental_coaching" => &staff.mental_coaching,
        "communication" => &staff.communication,
        _ => return None,
    };
    parse_filter_number(raw)
}

fn facility_grade_rank(raw: &str) -> i32 {
    match raw
        .trim()
        .trim_start_matches("Grade")
        .trim()
        .to_ascii_uppercase()
        .as_str()
    {
        "S" => 5,
        "A" => 4,
        "B" => 3,
        "C" => 2,
        "D" => 1,
        _ => 0,
    }
}

fn display_facility_grade(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "—".to_string();
    }

    let grade = trimmed.trim_start_matches("Grade").trim();
    if matches!(grade, "S" | "A" | "B" | "C" | "D") {
        format!("Grade {grade}")
    } else {
        trimmed.to_string()
    }
}

fn compare_facility_grade(left: &str, right: &str, ascending: bool) -> Ordering {
    let rank_order = facility_grade_rank(left).cmp(&facility_grade_rank(right));
    let order = if rank_order == Ordering::Equal {
        left.to_lowercase().cmp(&right.to_lowercase())
    } else {
        rank_order
    };
    if ascending { order } else { order.reverse() }
}

fn compare_team_summaries(
    a: &TeamSummary,
    b: &TeamSummary,
    column: TeamSortColumn,
    ascending: bool,
) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    fn finish(order: Ordering, ascending: bool) -> Ordering {
        if ascending { order } else { order.reverse() }
    }

    fn compare_text(a: &str, b: &str, ascending: bool) -> Ordering {
        finish(a.to_lowercase().cmp(&b.to_lowercase()), ascending)
    }

    fn compare_f64(a: f64, b: f64, ascending: bool) -> Ordering {
        finish(a.partial_cmp(&b).unwrap_or(Ordering::Equal), ascending)
    }

    fn compare_optional_f64(a: Option<f64>, b: Option<f64>, ascending: bool) -> Ordering {
        match (a, b) {
            (Some(left), Some(right)) => compare_f64(left, right, ascending),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }
    }

    let order = match column {
        TeamSortColumn::Name => compare_text(&a.display_name, &b.display_name, ascending),
        TeamSortColumn::Id => finish(a.id.cmp(&b.id), ascending),
        TeamSortColumn::League => finish(a.league_id.cmp(&b.league_id), ascending),
        TeamSortColumn::Manager => compare_text(&a.manager_name, &b.manager_name, ascending),
        TeamSortColumn::PlayerTeam => finish(a.is_player_team.cmp(&b.is_player_team), !ascending),
        TeamSortColumn::RosterSize => finish(a.roster_size.cmp(&b.roster_size), ascending),
        TeamSortColumn::StaffCount => finish(a.staff_count.cmp(&b.staff_count), ascending),
        TeamSortColumn::RosterRating => compare_optional_f64(a.roster_rating, b.roster_rating, ascending),
        TeamSortColumn::MerchandiseFacilityGrade => compare_facility_grade(
            &a.merchandise_facility_grade,
            &b.merchandise_facility_grade,
            ascending,
        ),
        TeamSortColumn::StadiumGrade => {
            compare_facility_grade(&a.stadium_grade, &b.stadium_grade, ascending)
        }
        TeamSortColumn::TrainingFacilityGrade => compare_facility_grade(
            &a.training_facility_grade,
            &b.training_facility_grade,
            ascending,
        ),
        TeamSortColumn::Money => compare_f64(a.total_balance, b.total_balance, ascending),
        TeamSortColumn::RecruitmentBudget => {
            compare_f64(a.transfer_budget, b.transfer_budget, ascending)
        }
        TeamSortColumn::SalaryBudget => compare_f64(a.salary_budget, b.salary_budget, ascending),
    };

    if order == Ordering::Equal {
        a.id.cmp(&b.id)
    } else {
        order
    }
}

fn advanced_staff_filter_matches(staff: &StaffSummary, filter: &AdvancedStaffSearch) -> bool {
    if filter.role_enabled
        && filter.role != "No Condition"
        && staff.role != filter.role
    {
        return false;
    }

    if filter.free_agents_only && staff.team != "Free Agent" {
        return false;
    }

    for range in filter.ranges.iter().filter(|range| range.enabled) {
        let min = parse_filter_number(&range.min);
        let max = parse_filter_number(&range.max);
        if min.is_none() && max.is_none() {
            continue;
        }

        let Some(value) = staff_advanced_range_value(staff, range.key) else {
            return false;
        };
        if min.is_some_and(|minimum| value < minimum) {
            return false;
        }
        if max.is_some_and(|maximum| value > maximum) {
            return false;
        }
    }

    true
}

fn compare_staff_summaries(
    a: &StaffSummary,
    b: &StaffSummary,
    column: StaffSortColumn,
    ascending: bool,
) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    fn compare_text(a: &str, b: &str, ascending: bool) -> Ordering {
        let order = a.to_lowercase().cmp(&b.to_lowercase());
        if ascending { order } else { order.reverse() }
    }

    fn compare_number(a: &str, b: &str, ascending: bool) -> Ordering {
        match (parse_filter_number(a), parse_filter_number(b)) {
            (Some(a), Some(b)) => {
                let order = a.partial_cmp(&b).unwrap_or(Ordering::Equal);
                if ascending { order } else { order.reverse() }
            }
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }
    }

    fn compare_optional_text(a: &str, b: &str, ascending: bool) -> Ordering {
        let a_missing = a.trim().is_empty() || matches!(a.trim(), "—" | "-");
        let b_missing = b.trim().is_empty() || matches!(b.trim(), "—" | "-");
        match (a_missing, b_missing) {
            (false, false) => compare_text(a, b, ascending),
            (false, true) => Ordering::Less,
            (true, false) => Ordering::Greater,
            (true, true) => Ordering::Equal,
        }
    }

    let order = match column {
        StaffSortColumn::Name => compare_text(&a.name, &b.name, ascending),
        StaffSortColumn::Id => {
            let order = a.id.cmp(&b.id);
            if ascending { order } else { order.reverse() }
        }
        StaffSortColumn::Age => compare_number(&a.age, &b.age, ascending),
        StaffSortColumn::Team => compare_text(&a.team, &b.team, ascending),
        StaffSortColumn::Role => compare_text(&a.role, &b.role, ascending),
        StaffSortColumn::Salary => compare_number(&a.annual_salary, &b.annual_salary, ascending),
        StaffSortColumn::ContractEnd => compare_optional_text(&a.contract_end, &b.contract_end, ascending),
        StaffSortColumn::BanPick => compare_number(&a.banpick, &b.banpick, ascending),
        StaffSortColumn::Strategy => compare_number(&a.strategy, &b.strategy, ascending),
        StaffSortColumn::Negotiation => compare_number(&a.negotiation, &b.negotiation, ascending),
        StaffSortColumn::JudgeAbility => compare_number(&a.judge_ability, &b.judge_ability, ascending),
        StaffSortColumn::JudgePotential => compare_number(&a.judge_potential, &b.judge_potential, ascending),
        StaffSortColumn::Feedback => compare_number(&a.feedback, &b.feedback, ascending),
        StaffSortColumn::PowerAnalysis => compare_number(&a.power_analysis, &b.power_analysis, ascending),
        StaffSortColumn::ControlCoaching => compare_number(&a.control_coaching, &b.control_coaching, ascending),
        StaffSortColumn::JudgmentCoaching => compare_number(&a.judgment_coaching, &b.judgment_coaching, ascending),
        StaffSortColumn::MentalCoaching => compare_number(&a.mental_coaching, &b.mental_coaching, ascending),
        StaffSortColumn::Communication => compare_number(&a.communication, &b.communication, ascending),
    };

    if order == Ordering::Equal {
        a.id.cmp(&b.id)
    } else {
        order
    }
}


fn parse_filter_number(value: &str) -> Option<f64> {
    let trimmed = value.trim();
    if trimmed.is_empty() || matches!(trimmed, "—" | "-") {
        None
    } else {
        parse_display_amount(trimmed).ok()
    }
}

fn player_advanced_range_value(player: &PlayerSummary, key: &str) -> Option<f64> {
    if key == "actual_rating" {
        return effective_actual_rating(player).map(|(value, _)| value as f64);
    }

    let raw = match key {
        "age" => &player.age,
        "salary" => &player.salary,
        "transfer_fee" => &player.transfer_fee,
        "actual_potential" => &player.actual_potential,
        "last_hit" => &player.last_hit,
        "skill_avoid" => &player.skill_avoid,
        "skill_hit" => &player.skill_hit,
        "control_speed" => &player.control_speed,
        "positioning" => &player.positioning,
        "judgement" => &player.judgement,
        "mental" => &player.mental,
        "concentration" => &player.concentration,
        "order" => &player.order,
        "roaming" => &player.roaming,
        "aggressive" => &player.aggressive,
        "ego" => &player.ego,
        _ => return None,
    };

    parse_filter_number(raw)
}

fn advanced_player_filter_matches(
    player: &PlayerSummary,
    filter: &AdvancedPlayerSearch,
) -> bool {
    if filter.position_enabled && filter.position != "No Condition" {
        let matches = player
            .position
            .split('/')
            .any(|position| position.trim() == filter.position);
        if !matches {
            return false;
        }
    }

    if filter.region_enabled
        && filter.region != "No Condition"
        && player.region != filter.region
    {
        return false;
    }

    if filter.free_agents_only && player.team != "Free Agent" {
        return false;
    }

    for range in filter.ranges.iter().filter(|range| range.enabled) {
        let min = parse_filter_number(&range.min);
        let max = parse_filter_number(&range.max);

        if min.is_none() && max.is_none() {
            continue;
        }

        let Some(value) = player_advanced_range_value(player, range.key) else {
            return false;
        };

        if min.is_some_and(|minimum| value < minimum) {
            return false;
        }
        if max.is_some_and(|maximum| value > maximum) {
            return false;
        }
    }

    true
}

fn compare_player_summaries(
    a: &PlayerSummary,
    b: &PlayerSummary,
    column: PlayerSortColumn,
    ascending: bool,
) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    fn compare_text(a: &str, b: &str, ascending: bool) -> Ordering {
        let order = a.to_lowercase().cmp(&b.to_lowercase());
        if ascending { order } else { order.reverse() }
    }

    fn compare_number(a: &str, b: &str, ascending: bool) -> Ordering {
        match (parse_filter_number(a), parse_filter_number(b)) {
            (Some(a), Some(b)) => {
                let order = a.partial_cmp(&b).unwrap_or(Ordering::Equal);
                if ascending { order } else { order.reverse() }
            }
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }
    }

    fn compare_optional_text(a: &str, b: &str, ascending: bool) -> Ordering {
        let a_missing = a.trim().is_empty() || matches!(a.trim(), "—" | "-");
        let b_missing = b.trim().is_empty() || matches!(b.trim(), "—" | "-");
        match (a_missing, b_missing) {
            (false, false) => compare_text(a, b, ascending),
            (false, true) => Ordering::Less,
            (true, false) => Ordering::Greater,
            (true, true) => Ordering::Equal,
        }
    }

    let order = match column {
        PlayerSortColumn::Name => compare_text(&a.name, &b.name, ascending),
        PlayerSortColumn::Id => {
            let order = a.id.cmp(&b.id);
            if ascending { order } else { order.reverse() }
        }
        PlayerSortColumn::Age => compare_number(&a.age, &b.age, ascending),
        PlayerSortColumn::Team => compare_text(&a.team, &b.team, ascending),
        PlayerSortColumn::Position => compare_text(&a.position, &b.position, ascending),
        PlayerSortColumn::ActualRating => {
            let a_value = effective_actual_rating(a)
                .map(|(value, _)| value.to_string())
                .unwrap_or_default();
            let b_value = effective_actual_rating(b)
                .map(|(value, _)| value.to_string())
                .unwrap_or_default();
            compare_number(&a_value, &b_value, ascending)
        },
        PlayerSortColumn::PotentialRating => {
            let a_value = effective_potential_rating(a)
                .map(|value| value.to_string())
                .unwrap_or_default();
            let b_value = effective_potential_rating(b)
                .map(|value| value.to_string())
                .unwrap_or_default();
            compare_number(&a_value, &b_value, ascending)
        },
        PlayerSortColumn::ActualPotential => compare_number(&a.actual_potential, &b.actual_potential, ascending),
        PlayerSortColumn::Salary => compare_number(&a.salary, &b.salary, ascending),
        PlayerSortColumn::ContractEnd => compare_optional_text(&a.contract_end, &b.contract_end, ascending),
        PlayerSortColumn::LastHitting => compare_number(&a.last_hit, &b.last_hit, ascending),
        PlayerSortColumn::SkillshotDodging => compare_number(&a.skill_avoid, &b.skill_avoid, ascending),
        PlayerSortColumn::SkillshotAccuracy => compare_number(&a.skill_hit, &b.skill_hit, ascending),
        PlayerSortColumn::InputSpeed => compare_number(&a.control_speed, &b.control_speed, ascending),
        PlayerSortColumn::Positioning => compare_number(&a.positioning, &b.positioning, ascending),
        PlayerSortColumn::Judgment => compare_number(&a.judgement, &b.judgement, ascending),
        PlayerSortColumn::Mental => compare_number(&a.mental, &b.mental, ascending),
        PlayerSortColumn::Focus => compare_number(&a.concentration, &b.concentration, ascending),
        PlayerSortColumn::Calls => compare_number(&a.order, &b.order, ascending),
        PlayerSortColumn::Roaming => compare_number(&a.roaming, &b.roaming, ascending),
        PlayerSortColumn::Aggression => compare_number(&a.aggressive, &b.aggressive, ascending),
        PlayerSortColumn::Ego => compare_number(&a.ego, &b.ego, ascending),
    };

    if order == Ordering::Equal {
        a.id.cmp(&b.id)
    } else {
        order
    }
}

fn parse_saved_bool(value: &str) -> bool {
    matches!(value.trim(), "true" | "1" | "yes" | "on")
}

fn advanced_choice_filter_row(
    ui: &mut egui::Ui,
    label: &str,
    enabled: &mut bool,
    value: &mut String,
    choices: &[(&str, &str)],
    id: &'static str,
    localization: &Localization,
) {
    ui.horizontal(|ui| {
        ui.add_sized([20.0, 24.0], egui::Checkbox::without_text(enabled));
        ui.add_sized([138.0, 24.0], egui::Label::new(label));
        ui.add_enabled_ui(*enabled, |ui| {
            let selected_text = choices
                .iter()
                .find(|(raw, _)| *raw == value.as_str())
                .map(|(_, key)| localization.tr(key))
                .unwrap_or_else(|| value.clone());
            egui::ComboBox::from_id_salt(id)
                .selected_text(selected_text)
                .width(198.0)
                .show_ui(ui, |ui| {
                    for (raw, key) in choices {
                        ui.selectable_value(value, (*raw).to_string(), localization.tr(key));
                    }
                });
        });
    });
}

fn advanced_range_filter_row(ui: &mut egui::Ui, filter: &mut AdvancedRangeFilter, localization: &Localization) {
    ui.horizontal(|ui| {
        ui.add_sized([20.0, 24.0], egui::Checkbox::without_text(&mut filter.enabled));
        ui.add_sized([138.0, 24.0], egui::Label::new(localization.tr(filter.label)));
        ui.add_enabled(
            filter.enabled,
            egui::TextEdit::singleline(&mut filter.min)
                .desired_width(82.0)
                .hint_text(localization.tr("common.min")),
        );
        ui.add_sized([14.0, 24.0], egui::Label::new("~"));
        ui.add_enabled(
            filter.enabled,
            egui::TextEdit::singleline(&mut filter.max)
                .desired_width(82.0)
                .hint_text(localization.tr("common.max")),
        );
        if !filter.unit.is_empty() {
            ui.weak(filter.unit);
        }
    });
}

fn advanced_boolean_filter_row(ui: &mut egui::Ui, label: &str, enabled: &mut bool) {
    ui.horizontal(|ui| {
        ui.add_sized([20.0, 24.0], egui::Checkbox::without_text(enabled));
        ui.add_sized([138.0, 24.0], egui::Label::new(label));
    });
}

fn star_level_label(raw: u16) -> String {
    if raw == 0 {
        return "None".to_string();
    }
    let clamped = raw.min(100);
    let full = clamped / 20;
    let half = (clamped % 20) >= 10;
    let stars = if half {
        format!("{full}.5/5")
    } else {
        format!("{full}/5")
    };
    if raw.is_multiple_of(10) {
        stars
    } else {
        format!("{stars} (raw {raw})")
    }
}

fn position_star_level_combo(ui: &mut egui::Ui, id: impl std::hash::Hash, value: &mut u16) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(star_level_label(*value))
        .width(150.0)
        .show_ui(ui, |ui| {
            for raw in (10u16..=100).step_by(10) {
                ui.selectable_value(value, raw, star_level_label(raw));
            }
        });
}

fn money_text_edit_with_preview(
    ui: &mut egui::Ui,
    localization: &Localization,
    value: &mut String,
    width: f32,
    enabled: bool,
) -> egui::Response {
    ui.horizontal(|ui| {
        let response = ui.add_enabled(
            enabled,
            egui::TextEdit::singleline(value).desired_width(width),
        );

        if enabled {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                ui.weak("—");
            } else {
                match parse_display_amount(trimmed) {
                    Ok(amount) => {
                        ui.weak(format!("→ {}", format_display_amount(amount)));
                    }
                    Err(()) => {
                        ui.colored_label(
                            egui::Color32::from_rgb(220, 90, 90),
                            localization.tr("currency.invalid_amount"),
                        );
                    }
                }
            }
        }

        response
    })
    .inner
}

fn stat_edit_cell(ui: &mut egui::Ui, label: &str, value: &mut String) {
    ui.label(label);
    ui.add(
        egui::TextEdit::singleline(value)
            .desired_width(64.0),
    );
}

fn collect_staff_stat_values(staff: &StaffStats) -> Result<Vec<String>, String> {
    [
        ("Ban/Pick", &staff.banpick),
        ("Strategy", &staff.strategy),
        ("Negotiation", &staff.negotiation),
        ("Ability Analysis", &staff.judge_ability),
        ("Potential Analysis", &staff.judge_potential),
        ("Feedback", &staff.feedback),
        ("Power Analysis", &staff.power_analysis),
        ("Control Coaching", &staff.control_coaching),
        ("Judgment Coaching", &staff.judgment_coaching),
        ("Mental Coaching", &staff.mental_coaching),
    ]
    .into_iter()
    .map(|(label, value)| normalize_stat(label, value))
    .collect()
}


fn normalize_staff_communication_value(
    region_id: usize,
    raw: &str,
) -> Result<u16, String> {
    let value = raw
        .trim()
        .parse::<u16>()
        .map_err(|_| {
            format!(
                "{} Communication must be an integer between 0 and 100",
                staff_communication_region_label(region_id)
            )
        })?;

    if value > 100 {
        return Err(format!(
            "{} Communication must be between 0 and 100",
            staff_communication_region_label(region_id)
        ));
    }

    Ok(value)
}

fn normalize_player_communication_value(region_id: usize, raw: &str) -> Result<i32, String> {
    let value = raw.trim().parse::<i32>().map_err(|_| {
        format!(
            "{} Communication must be a whole number between 0 and 100",
            player_communication_region_label(region_id)
        )
    })?;

    if !(0..=100).contains(&value) {
        return Err(format!(
            "{} Communication must be between 0 and 100",
            player_communication_region_label(region_id)
        ));
    }

    Ok(value)
}

fn collect_player_stat_values(player: &PlayerStats) -> Result<Vec<String>, String> {
    [
        ("Last Hitting", &player.last_hit),
        ("Skillshot Dodging", &player.skill_avoid),
        ("Skillshot Accuracy", &player.skill_hit),
        ("Input Speed", &player.control_speed),
        ("Positioning", &player.positioning),
        ("Judgment", &player.judgement),
        ("Mental", &player.mental),
        ("Focus", &player.concentration),
        ("Calls", &player.order),
        ("Roaming", &player.roaming),
        ("Aggression", &player.aggressive),
        ("Ego", &player.ego),
    ]
    .into_iter()
    .map(|(label, value)| normalize_stat(label, value))
    .collect()
}

fn normalize_stat(label: &str, raw: &str) -> Result<String, String> {
    let value = raw
        .trim()
        .parse::<u16>()
        .map_err(|_| format!("{label} must be an integer between 1 and 100"))?;

    if !(1..=100).contains(&value) {
        return Err(format!("{label} must be between 1 and 100"));
    }

    Ok(value.to_string())
}

fn parse_players_response(response: &str) -> Result<Vec<PlayerSummary>, String> {
    if let Some(error) = response.strip_prefix("ERR|") {
        return Err(error.to_string());
    }

    let mut parts = response.splitn(4, '|');
    if parts.next() != Some("OK") || parts.next() != Some("PLAYERS") {
        return Err(format!("Unexpected response: {response}"));
    }

    let expected_count = parts
        .next()
        .ok_or_else(|| "Missing player count in bridge response".to_string())?
        .parse::<usize>()
        .map_err(|_| "Invalid player count in bridge response".to_string())?;

    let payload = parts.next().unwrap_or_default();
    if payload.is_empty() {
        return Ok(Vec::new());
    }

    let mut players = Vec::with_capacity(expected_count);
    for entry in payload.split(';') {
        let fields = entry.split(':').collect::<Vec<_>>();
        if fields.len() != 24 {
            return Err("Invalid player entry from bridge".to_string());
        }

        let id = fields[0]
            .parse::<usize>()
            .map_err(|_| "Invalid player ID from bridge".to_string())?;
        players.push(PlayerSummary {
            id,
            name: hex_decode(fields[1])?,
            age: hex_decode(fields[2])?,
            team: hex_decode(fields[3])?,
            region: hex_decode(fields[4])?,
            position: hex_decode(fields[5])?,
            actual_rating: hex_decode(fields[6])?,
            _scout_potential_report: hex_decode(fields[7])?,
            actual_potential: hex_decode(fields[8])?,
            salary: format_internal_amount(&hex_decode(fields[9])?),
            transfer_fee: format_internal_amount(&hex_decode(fields[10])?),
            contract_end: hex_decode(fields[11])?,
            last_hit: hex_decode(fields[12])?,
            skill_avoid: hex_decode(fields[13])?,
            skill_hit: hex_decode(fields[14])?,
            control_speed: hex_decode(fields[15])?,
            positioning: hex_decode(fields[16])?,
            judgement: hex_decode(fields[17])?,
            mental: hex_decode(fields[18])?,
            concentration: hex_decode(fields[19])?,
            order: hex_decode(fields[20])?,
            roaming: hex_decode(fields[21])?,
            aggressive: hex_decode(fields[22])?,
            ego: hex_decode(fields[23])?,
        });
    }

    if players.len() != expected_count {
        return Err(format!(
            "Bridge reported {expected_count} players but sent {}",
            players.len()
        ));
    }

    Ok(players)
}

fn parse_staffs_response(response: &str) -> Result<Vec<StaffSummary>, String> {
    if let Some(error) = response.strip_prefix("ERR|") {
        return Err(error.to_string());
    }

    let mut parts = response.splitn(4, '|');
    if parts.next() != Some("OK") || parts.next() != Some("STAFFS") {
        return Err(format!("Unexpected response: {response}"));
    }

    let expected_count = parts
        .next()
        .ok_or_else(|| "Missing staff count in bridge response".to_string())?
        .parse::<usize>()
        .map_err(|_| "Invalid staff count in bridge response".to_string())?;

    let payload = parts.next().unwrap_or_default();
    if payload.is_empty() {
        return Ok(Vec::new());
    }

    let mut staffs = Vec::with_capacity(expected_count);
    for entry in payload.split(';') {
        let fields = entry.split(':').collect::<Vec<_>>();
        if fields.len() != 18 {
            return Err("Invalid staff entry from bridge".to_string());
        }
        staffs.push(StaffSummary {
            id: fields[0]
                .parse::<usize>()
                .map_err(|_| "Invalid staff ID from bridge".to_string())?,
            name: hex_decode(fields[1])?,
            age: hex_decode(fields[2])?,
            team: hex_decode(fields[3])?,
            role: hex_decode(fields[4])?,
            banpick: hex_decode(fields[5])?,
            strategy: hex_decode(fields[6])?,
            negotiation: hex_decode(fields[7])?,
            judge_ability: hex_decode(fields[8])?,
            judge_potential: hex_decode(fields[9])?,
            feedback: hex_decode(fields[10])?,
            power_analysis: hex_decode(fields[11])?,
            control_coaching: hex_decode(fields[12])?,
            judgment_coaching: hex_decode(fields[13])?,
            mental_coaching: hex_decode(fields[14])?,
            annual_salary: format_internal_amount(&hex_decode(fields[15])?),
            contract_end: hex_decode(fields[16])?,
            communication: hex_decode(fields[17])?,
        });
    }

    if staffs.len() != expected_count {
        return Err(format!(
            "Bridge reported {expected_count} staff but sent {}",
            staffs.len()
        ));
    }

    Ok(staffs)
}

fn parse_optional_usize(raw: &str) -> Result<Option<usize>, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        Ok(None)
    } else {
        raw.parse::<usize>()
            .map(Some)
            .map_err(|_| "Invalid team ID from bridge".to_string())
    }
}

fn parse_staff_response(response: &str) -> Result<StaffStats, String> {
    if let Some(error) = response.strip_prefix("ERR|") {
        return Err(error.to_string());
    }

    let parts = response.split('|').collect::<Vec<_>>();
    if parts.len() != 22 || parts[0] != "OK" || parts[1] != "STAFF" {
        return Err(format!("Unexpected staff response: {response}"));
    }

    Ok(StaffStats {
        id: parts[2]
            .parse::<usize>()
            .map_err(|_| "Invalid staff ID from bridge".to_string())?,
        name: hex_decode(parts[3])?,
        age: hex_decode(parts[4])?,
        role: hex_decode(parts[5])?,
        team: hex_decode(parts[6])?,
        banpick: pretty_number(parts[7]),
        strategy: pretty_number(parts[8]),
        negotiation: pretty_number(parts[9]),
        judge_ability: pretty_number(parts[10]),
        judge_potential: pretty_number(parts[11]),
        feedback: pretty_number(parts[12]),
        power_analysis: pretty_number(parts[13]),
        control_coaching: pretty_number(parts[14]),
        judgment_coaching: pretty_number(parts[15]),
        mental_coaching: pretty_number(parts[16]),
        annual_salary: format_internal_amount(parts[17]),
        contract_team_id: parse_optional_usize(parts[18])?,
        contract_start_date: hex_decode(parts[19])?,
        contract_end_date: hex_decode(parts[20])?,
        communication: parse_staff_communication_entries(&hex_decode(parts[21])?)?,
    })
}

fn parse_staff_communication_entries(
    raw: &str,
) -> Result<Vec<StaffCommunicationEntry>, String> {
    let mut entries = Vec::new();
    for raw_entry in raw.split(';').filter(|entry| !entry.trim().is_empty()) {
        let (region_id, value) = raw_entry
            .split_once('=')
            .ok_or_else(|| "Invalid Staff Communication entry from bridge".to_string())?;
        entries.push(StaffCommunicationEntry {
            region_id: region_id
                .trim()
                .parse::<usize>()
                .map_err(|_| "Invalid Staff Communication region from bridge".to_string())?,
            value: value.trim().to_string(),
        });
    }
    entries.sort_by_key(|entry| entry.region_id);
    Ok(entries)
}

fn parse_contract_defaults_response(response: &str) -> Result<(String, String, String), String> {
    if let Some(error) = response.strip_prefix("ERR|") {
        return Err(error.to_string());
    }

    let parts = response.split('|').collect::<Vec<_>>();
    if parts.len() != 5 || parts[0] != "OK" || parts[1] != "CONTRACT_DEFAULTS" {
        return Err(format!("Unexpected contract defaults response: {response}"));
    }

    Ok((
        parts[2].to_string(),
        parts[3].to_string(),
        format_internal_amount(parts[4]),
    ))
}

#[cfg(feature = "dev")]
fn extract_debug_blocks(source: &str, marker: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut offset = 0usize;

    while offset < source.len() {
        let Some(relative_start) = source[offset..].find(marker) else {
            break;
        };
        let start = offset + relative_start;
        let Some(relative_open) = source[start..].find('{') else {
            break;
        };
        let open = start + relative_open;
        let mut depth = 0usize;
        let mut in_string = false;
        let mut escaped = false;
        let mut end = None;

        for (relative_index, character) in source[open..].char_indices() {
            if in_string {
                if escaped {
                    escaped = false;
                } else if character == '\\' {
                    escaped = true;
                } else if character == '"' {
                    in_string = false;
                }
                continue;
            }

            match character {
                '"' => in_string = true,
                '{' => depth += 1,
                '}' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        end = Some(open + relative_index + character.len_utf8());
                        break;
                    }
                }
                _ => {}
            }
        }

        let Some(end) = end else {
            break;
        };
        blocks.push(source[start..end].to_string());
        offset = end;
    }

    blocks
}

#[cfg(feature = "dev")]
fn debug_field_expression(source: &str, field: &str) -> Option<String> {
    let prefix = format!("{field}:");
    let mut line_start = 0usize;

    for line in source.split_inclusive('\n') {
        let line_without_newline = line.strip_suffix('\n').unwrap_or(line);
        let trimmed = line_without_newline.trim_start();
        if trimmed.starts_with(&prefix) {
            let indent = line_without_newline.len().saturating_sub(trimmed.len());
            let mut value_start = line_start + indent + prefix.len();
            while value_start < source.len()
                && source.as_bytes()[value_start].is_ascii_whitespace()
            {
                value_start += 1;
            }

            let mut braces = 0usize;
            let mut brackets = 0usize;
            let mut parentheses = 0usize;
            let mut in_string = false;
            let mut escaped = false;
            let mut value_end = source.len();

            for (relative_index, character) in source[value_start..].char_indices() {
                if in_string {
                    if escaped {
                        escaped = false;
                    } else if character == '\\' {
                        escaped = true;
                    } else if character == '"' {
                        in_string = false;
                    }
                    continue;
                }

                match character {
                    '"' => in_string = true,
                    '{' => braces += 1,
                    '}' => braces = braces.saturating_sub(1),
                    '[' => brackets += 1,
                    ']' => brackets = brackets.saturating_sub(1),
                    '(' => parentheses += 1,
                    ')' => parentheses = parentheses.saturating_sub(1),
                    ',' if braces == 0 && brackets == 0 && parentheses == 0 => {
                        value_end = value_start + relative_index;
                        break;
                    }
                    _ => {}
                }
            }

            return Some(
                source[value_start..value_end]
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" "),
            );
        }
        line_start += line.len();
    }

    None
}

#[cfg(feature = "dev")]
fn debug_unquote(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        trimmed[1..trimmed.len() - 1]
            .replace("\\\"", "\"")
            .replace("\\\\", "\\")
    } else {
        trimmed.to_string()
    }
}

#[cfg(feature = "dev")]
fn debug_parse_usize(source: &str, field: &str) -> Option<usize> {
    debug_field_expression(source, field)?.parse::<usize>().ok()
}

#[cfg(feature = "dev")]
fn debug_parse_i64(source: &str, field: &str) -> Option<i64> {
    debug_field_expression(source, field)?.parse::<i64>().ok()
}

#[cfg(feature = "dev")]
fn debug_parse_bool(source: &str, field: &str) -> Option<bool> {
    match debug_field_expression(source, field)?.as_str() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

#[cfg(feature = "dev")]
fn debug_quoted_values(source: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut escaped = false;

    for character in source.chars() {
        if in_string {
            if escaped {
                current.push(character);
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                values.push(current.clone());
                current.clear();
                in_string = false;
            } else {
                current.push(character);
            }
        } else if character == '"' {
            in_string = true;
        }
    }

    values
}

#[cfg(feature = "dev")]
fn debug_bind_pairs(source: &str) -> Vec<(String, String)> {
    let values = debug_quoted_values(source);
    values
        .chunks_exact(2)
        .map(|pair| (pair[0].clone(), pair[1].clone()))
        .collect()
}

#[cfg(feature = "dev")]
fn humanize_debug_value(value: &str) -> String {
    let mut text = debug_unquote(value);
    if let Some(rest) = text.strip_prefix("Some(") {
        text = rest.trim_end_matches(')').trim().to_string();
    }
    if let Some(index) = text.find("tactic_name.") {
        text = text[index + "tactic_name.".len()..].replace('.', " / ");
    } else if let Some(index) = text.find("insight.") {
        text = text[index + "insight.".len()..].replace('.', " / ");
    } else if text.starts_with("#asset/") {
        text = text
            .rsplit(['.', '?'])
            .next()
            .unwrap_or(text.as_str())
            .to_string();
    }

    let mut spaced = String::new();
    let mut previous_lowercase = false;
    for character in text.chars() {
        if character == '_' || character == '.' {
            spaced.push(' ');
            previous_lowercase = false;
        } else {
            if character.is_ascii_uppercase() && previous_lowercase {
                spaced.push(' ');
            }
            spaced.push(character);
            previous_lowercase = character.is_ascii_lowercase() || character.is_ascii_digit();
        }
    }

    spaced
        .split_whitespace()
        .map(|word| {
            let mut characters = word.chars();
            match characters.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), characters.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
        .replace("( ", "(")
        .replace(", )", ")")
        .replace(" )", ")")
}

#[cfg(feature = "dev")]
fn debug_position_name(position: usize) -> String {
    match position {
        0 => "Top",
        1 => "Jungle",
        2 => "Mid",
        3 => "Bottom",
        4 => "Support",
        _ => "Unknown",
    }
    .to_string()
}

#[cfg(feature = "dev")]
fn debug_list_identifiers(expression: &str) -> Vec<String> {
    expression
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(humanize_debug_value)
        .collect()
}

#[cfg(feature = "dev")]
fn team_name_from_lookup(teams: &[TeamSummary], team_id: usize) -> String {
    teams
        .iter()
        .find(|team| team.id == team_id)
        .map(|team| team.display_name.clone())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| format!("Team {team_id}"))
}

#[cfg(feature = "dev")]
fn player_name_from_lookup(players: &[PlayerSummary], player_id: usize) -> String {
    players
        .iter()
        .find(|player| player.id == player_id)
        .map(|player| player.name.clone())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| format!("Player {player_id}"))
}

#[cfg(feature = "dev")]
fn parse_team_history_probe(
    raw: &str,
    team_id: usize,
    teams: &[TeamSummary],
    players: &[PlayerSummary],
) -> Result<TeamHistoryData, String> {
    if !raw.contains("=== SELECTED TEAM RECORD ===") {
        return Err("Unexpected Team probe payload".to_string());
    }

    let mut matches = Vec::new();
    let mut analyses = Vec::new();
    let mut latest_rating = None;
    let mut latest_rank = None;
    let mut latest_rating_date = String::new();
    let news_blocks = extract_debug_blocks(raw, "News {");

    for news in &news_blocks {
        let date = debug_field_expression(news, "date").unwrap_or_default();

        if let Some(report) = extract_debug_blocks(news, "ty: MatchReport {")
            .into_iter()
            .next()
        {
            let match_id = debug_parse_usize(&report, "match_id").unwrap_or_default();
            let opponent_id = debug_parse_usize(&report, "enemy_team_id").unwrap_or_default();
            let is_practice = debug_parse_bool(&report, "is_practice").unwrap_or(false);
            let is_win = debug_parse_bool(&report, "is_win").unwrap_or(false);
            let my_score = debug_parse_usize(&report, "my_team_score").unwrap_or_default();
            let enemy_score = debug_parse_usize(&report, "enemy_team_score").unwrap_or_default();
            let article_pattern = debug_field_expression(&report, "article_pattern")
                .map(|value| humanize_debug_value(&value))
                .unwrap_or_else(|| "—".to_string());
            let set_patterns = debug_field_expression(&report, "set_patterns")
                .map(|value| debug_list_identifiers(&value))
                .unwrap_or_default();
            let mut sets = Vec::new();

            for (index, set_block) in extract_debug_blocks(&report, "MatchSetArticleData {")
                .into_iter()
                .enumerate()
            {
                let mvp_player_id =
                    debug_parse_usize(&set_block, "mvp_athlete_id").unwrap_or_default();
                sets.push(TeamMatchSetEntry {
                    set_number: index + 1,
                    pattern: set_patterns
                        .get(index)
                        .cloned()
                        .unwrap_or_else(|| "—".to_string()),
                    team1_kills: debug_parse_usize(&set_block, "team1_total_kill")
                        .unwrap_or_default(),
                    team2_kills: debug_parse_usize(&set_block, "team2_total_kill")
                        .unwrap_or_default(),
                    team1_gold: debug_parse_usize(&set_block, "team1_total_gold")
                        .unwrap_or_default(),
                    team2_gold: debug_parse_usize(&set_block, "team2_total_gold")
                        .unwrap_or_default(),
                    mvp_player_id,
                    mvp_player_name: player_name_from_lookup(players, mvp_player_id),
                    mvp_champion_id: debug_field_expression(&set_block, "mvp_champion")
                        .map(|value| debug_unquote(&value))
                        .unwrap_or_default(),
                    mvp_kills: debug_parse_usize(&set_block, "mvp_kills").unwrap_or_default(),
                    mvp_deaths: debug_parse_usize(&set_block, "mvp_deaths")
                        .unwrap_or_default(),
                    mvp_assists: debug_parse_usize(&set_block, "mvp_assists")
                        .unwrap_or_default(),
                    was_comeback: debug_parse_bool(&set_block, "was_comeback")
                        .unwrap_or(false),
                    was_blue_side: debug_parse_bool(&set_block, "is_team1_blue")
                        .unwrap_or(false),
                });
            }

            matches.push(TeamMatchHistoryEntry {
                date,
                match_id,
                opponent_id,
                opponent_name: team_name_from_lookup(teams, opponent_id),
                is_practice,
                is_win,
                my_score,
                enemy_score,
                article_pattern,
                sets,
            });
            continue;
        }

        if let Some(analysis) = extract_debug_blocks(news, "ty: PreMatchAnalysis {")
            .into_iter()
            .next()
        {
            let match_id = debug_parse_usize(&analysis, "match_id").unwrap_or_default();
            let opponent_id =
                debug_parse_usize(&analysis, "enemy_team_id").unwrap_or_default();
            let star_player_id =
                debug_parse_usize(&analysis, "star_player_id").unwrap_or_default();
            let tactics = extract_debug_blocks(&analysis, "PreMatchTacticEntry {")
                .into_iter()
                .map(|entry| TeamPreMatchTacticEntry {
                    category: debug_field_expression(&entry, "category")
                        .map(|value| humanize_debug_value(&value))
                        .unwrap_or_default(),
                    value: debug_field_expression(&entry, "value")
                        .map(|value| humanize_debug_value(&value))
                        .unwrap_or_default(),
                })
                .collect::<Vec<_>>();
            let champion_picks = extract_debug_blocks(&analysis, "PreMatchChampionEntry {")
                .into_iter()
                .map(|entry| TeamPreMatchChampionEntry {
                    champion_id: debug_field_expression(&entry, "champion")
                        .map(|value| debug_unquote(&value))
                        .unwrap_or_default(),
                    position: debug_parse_usize(&entry, "position")
                        .map(debug_position_name)
                        .unwrap_or_else(|| "Unknown".to_string()),
                    wins: debug_parse_usize(&entry, "wins").unwrap_or_default(),
                    losses: debug_parse_usize(&entry, "losses").unwrap_or_default(),
                })
                .collect::<Vec<_>>();
            let mut insights = Vec::new();

            for insight in extract_debug_blocks(&analysis, "PreMatchInsight {") {
                let section = debug_field_expression(&insight, "section")
                    .map(|value| humanize_debug_value(&value))
                    .unwrap_or_else(|| "Other".to_string());
                for text in extract_debug_blocks(&insight, "PreMatchInsightText {") {
                    let source_key = debug_field_expression(&text, "i18n_key")
                        .map(|value| debug_unquote(&value))
                        .unwrap_or_default();
                    let label = source_key
                        .rsplit('.')
                        .next()
                        .map(humanize_debug_value)
                        .unwrap_or_else(|| "Insight".to_string());
                    let binds = debug_field_expression(&text, "binds")
                        .map(|value| debug_bind_pairs(&value))
                        .unwrap_or_default();
                    let champions = debug_field_expression(&text, "champion_keys")
                        .map(|value| debug_quoted_values(&value))
                        .unwrap_or_default();
                    let mut detail_parts = binds
                        .into_iter()
                        .map(|(key, value)| {
                            format!(
                                "{}: {}",
                                humanize_debug_value(&key),
                                humanize_debug_value(&value)
                            )
                        })
                        .collect::<Vec<_>>();
                    if !champions.is_empty() {
                        detail_parts.push(format!(
                            "Champions: {}",
                            champions
                                .iter()
                                .map(|champion| champion_display_name(champion))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                    }
                    insights.push(TeamPreMatchInsightEntry {
                        section: section.clone(),
                        label,
                        details: detail_parts.join(" · "),
                        source_key,
                    });
                }
            }

            analyses.push(TeamPreMatchAnalysisEntry {
                date,
                match_id,
                opponent_id,
                opponent_name: team_name_from_lookup(teams, opponent_id),
                analysis_level: debug_field_expression(&analysis, "analysis_level")
                    .unwrap_or_else(|| "—".to_string()),
                has_match_history: debug_parse_bool(&analysis, "has_match_history")
                    .unwrap_or(false),
                star_player_id,
                star_player_name: player_name_from_lookup(players, star_player_id),
                tactics,
                champion_picks,
                insights,
            });
            continue;
        }

        if let Some(report) = extract_debug_blocks(news, "ty: TeamRatingRankingReport {")
            .into_iter()
            .next()
        {
            for (index, entry) in extract_debug_blocks(&report, "TeamRatingRankEntry {")
                .into_iter()
                .enumerate()
            {
                if debug_parse_usize(&entry, "team_id") == Some(team_id) {
                    latest_rating = debug_parse_i64(&entry, "rating");
                    latest_rank = Some(index + 1);
                    latest_rating_date = date.clone();
                    break;
                }
            }
        }

        let rating_subject = format!("rating:team-delta:{team_id}");
        if news.contains(&rating_subject) {
            if let Some(title_bind) = debug_field_expression(news, "title_bind") {
                let pairs = debug_bind_pairs(&title_bind);
                for (key, value) in pairs {
                    match key.as_str() {
                        "Rating" => latest_rating = value.parse::<i64>().ok(),
                        "Rank" => latest_rank = value.parse::<usize>().ok(),
                        _ => {}
                    }
                }
                latest_rating_date = date;
            }
        }
    }

    matches.sort_by(|left, right| left.date.cmp(&right.date));
    analyses.sort_by(|left, right| left.date.cmp(&right.date));

    Ok(TeamHistoryData {
        team_id,
        matches,
        analyses,
        latest_rating,
        latest_rank,
        latest_rating_date,
    })
}

#[cfg(feature = "dev")]
fn parse_team_strategy_section(section: &str) -> Result<Vec<TeamStrategyEntry>, String> {
    let mut entries = Vec::new();
    for line in section.lines().filter(|line| !line.trim().is_empty()) {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 2 {
            return Err("Invalid Team strategy section from bridge".to_string());
        }
        entries.push(TeamStrategyEntry {
            key: fields[0].to_string(),
            value: fields[1].to_string(),
        });
    }
    Ok(entries)
}

#[cfg(feature = "dev")]
fn parse_team_management_response(response: &str) -> Result<TeamManagementData, String> {
    if let Some(error) = response.strip_prefix("ERR|") {
        return Err(error.to_string());
    }

    let parts = response.split('|').collect::<Vec<_>>();
    if parts.len() != 10 || parts[0] != "OK" || parts[1] != "TEAM_MANAGEMENT" {
        return Err(format!("Unexpected Team management response: {response}"));
    }

    let team_id = parts[2]
        .parse::<usize>()
        .map_err(|_| "Invalid Team management team ID".to_string())?;
    let management = hex_decode(parts[3])?;
    let current_strategy_raw = hex_decode(parts[4])?;
    let last_strategy_raw = hex_decode(parts[5])?;
    let team_color_strategy_raw = hex_decode(parts[6])?;
    let merchandise_raw = hex_decode(parts[7])?;
    let champion_setup_raw = hex_decode(parts[8])?;
    let gaming_house_raw = hex_decode(parts[9])?;

    let mut lineup = Vec::new();
    let mut watched_players = Vec::new();
    let mut no_transfer_players = Vec::new();
    let mut release_players = Vec::new();
    let mut watched_staff = Vec::new();
    let mut release_staff = Vec::new();
    let mut pending_installments = 0;
    let mut resale_clauses = 0;
    let mut scout_dispatch = String::new();
    let mut merchandise_product_count = 0;
    let mut champion_tier_count = 0;
    let mut personal_tactic_count = 0;

    for line in management.lines().filter(|line| !line.trim().is_empty()) {
        let fields = line.split('\t').collect::<Vec<_>>();
        match fields.first().copied().unwrap_or_default() {
            "lineup" if fields.len() == 4 => {
                let member = if fields[2].trim().is_empty() {
                    None
                } else {
                    Some(TeamMemberReference {
                        id: fields[2]
                            .parse::<usize>()
                            .map_err(|_| "Invalid lineup member ID".to_string())?,
                        name: fields[3].to_string(),
                    })
                };
                lineup.push(TeamLineupEntry {
                    slot: fields[1].to_string(),
                    member,
                });
            }
            "watched_player" | "no_transfer_player" | "release_player"
                if fields.len() == 3 =>
            {
                let entry = TeamMemberReference {
                    id: fields[1]
                        .parse::<usize>()
                        .map_err(|_| "Invalid Team player reference ID".to_string())?,
                    name: fields[2].to_string(),
                };
                match fields[0] {
                    "watched_player" => watched_players.push(entry),
                    "no_transfer_player" => no_transfer_players.push(entry),
                    _ => release_players.push(entry),
                }
            }
            "watched_staff" | "release_staff" if fields.len() == 3 => {
                let entry = TeamMemberReference {
                    id: fields[1]
                        .parse::<usize>()
                        .map_err(|_| "Invalid Team staff reference ID".to_string())?,
                    name: fields[2].to_string(),
                };
                if fields[0] == "watched_staff" {
                    watched_staff.push(entry);
                } else {
                    release_staff.push(entry);
                }
            }
            "metric" if fields.len() == 3 => match fields[1] {
                "pending_installments" => {
                    pending_installments = fields[2]
                        .parse::<usize>()
                        .map_err(|_| "Invalid pending installment count".to_string())?;
                }
                "resale_clauses" => {
                    resale_clauses = fields[2]
                        .parse::<usize>()
                        .map_err(|_| "Invalid resale clause count".to_string())?;
                }
                "scout_dispatch" => scout_dispatch = fields[2].to_string(),
                "merchandise_products" => {
                    merchandise_product_count = fields[2]
                        .parse::<usize>()
                        .map_err(|_| "Invalid merchandise product count".to_string())?;
                }
                "champion_tiers" => {
                    champion_tier_count = fields[2]
                        .parse::<usize>()
                        .map_err(|_| "Invalid champion tier count".to_string())?;
                }
                "personal_tactics" => {
                    personal_tactic_count = fields[2]
                        .parse::<usize>()
                        .map_err(|_| "Invalid personal tactic count".to_string())?;
                }
                _ => {}
            },
            _ => {}
        }
    }

    let mut merchandise = Vec::new();
    for line in merchandise_raw.lines().filter(|line| !line.trim().is_empty()) {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 10 {
            return Err("Invalid Team merchandise row from bridge".to_string());
        }
        merchandise.push(TeamMerchandiseEntry {
            product_type: fields[0].to_string(),
            athlete_id: fields[1]
                .parse::<usize>()
                .map_err(|_| "Invalid merchandise athlete ID".to_string())?,
            athlete_name: fields[2].to_string(),
            stock: fields[3].to_string(),
            sell_price: fields[4].to_string(),
            yearly_sales: fields[5].to_string(),
            yearly_revenue: fields[6].to_string(),
            total_sales: fields[7].to_string(),
            total_revenue: fields[8].to_string(),
            daily_purchase_rate: fields[9].to_string(),
        });
    }

    let mut champion_setup = Vec::new();
    for line in champion_setup_raw
        .lines()
        .filter(|line| !line.trim().is_empty())
    {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 5 {
            return Err("Invalid Team champion setup row from bridge".to_string());
        }
        champion_setup.push(TeamChampionSetupEntry {
            champion_id: fields[0].to_string(),
            tier: fields[1].to_string(),
            tactic_1: fields[2].to_string(),
            tactic_2: fields[3].to_string(),
            tactic_3: fields[4].to_string(),
        });
    }

    let mut gaming_house = TeamGamingHouseSummary::default();
    for line in gaming_house_raw.lines().filter(|line| !line.trim().is_empty()) {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 2 {
            return Err("Invalid Gaming House summary row from bridge".to_string());
        }
        let parse_count = || {
            fields[1]
                .parse::<usize>()
                .map_err(|_| "Invalid Gaming House count".to_string())
        };
        match fields[0] {
            "level" => gaming_house.level = fields[1].to_string(),
            "welfare" => gaming_house.welfare = fields[1].to_string(),
            "owned_furniture_types" => gaming_house.owned_furniture_types = parse_count()?,
            "owned_furniture_total" => gaming_house.owned_furniture_total = parse_count()?,
            "owned_wallpaper_types" => gaming_house.owned_wallpaper_types = parse_count()?,
            "owned_wallpaper_total" => gaming_house.owned_wallpaper_total = parse_count()?,
            "owned_wall_types" => gaming_house.owned_wall_types = parse_count()?,
            "owned_wall_total" => gaming_house.owned_wall_total = parse_count()?,
            "owned_window_types" => gaming_house.owned_window_types = parse_count()?,
            "owned_window_total" => gaming_house.owned_window_total = parse_count()?,
            "placed_furniture" => gaming_house.placed_furniture = parse_count()?,
            "placed_wallpapers" => gaming_house.placed_wallpapers = parse_count()?,
            "placed_walls" => gaming_house.placed_walls = parse_count()?,
            "placed_windows" => gaming_house.placed_windows = parse_count()?,
            _ => {}
        }
    }

    Ok(TeamManagementData {
        team_id,
        lineup,
        watched_players,
        no_transfer_players,
        release_players,
        watched_staff,
        release_staff,
        pending_installments,
        resale_clauses,
        scout_dispatch,
        merchandise_product_count,
        champion_tier_count,
        personal_tactic_count,
        current_strategy: parse_team_strategy_section(&current_strategy_raw)?,
        last_strategy: parse_team_strategy_section(&last_strategy_raw)?,
        team_color_strategy: parse_team_strategy_section(&team_color_strategy_raw)?,
        merchandise,
        champion_setup,
        gaming_house,
    })
}

fn parse_teams_response(response: &str) -> Result<Vec<TeamSummary>, String> {
    if let Some(error) = response.strip_prefix("ERR|") {
        return Err(error.to_string());
    }

    let mut parts = response.splitn(4, '|');
    if parts.next() != Some("OK") || parts.next() != Some("TEAMS") {
        return Err(format!("Unexpected response: {response}"));
    }

    let expected_count = parts
        .next()
        .ok_or_else(|| "Missing team count in bridge response".to_string())?
        .parse::<usize>()
        .map_err(|_| "Invalid team count in bridge response".to_string())?;

    let payload = parts.next().unwrap_or_default();
    if payload.is_empty() {
        return Ok(Vec::new());
    }

    let mut teams = Vec::with_capacity(expected_count);
    for entry in payload.split(';') {
        let fields = entry.split(':').collect::<Vec<_>>();
        if fields.len() != 14 && fields.len() != 26 {
            return Err("Invalid team entry from bridge".to_string());
        }
        let id = fields[0]
            .parse::<usize>()
            .map_err(|_| "Invalid team ID from bridge".to_string())?;
        let display_name = hex_decode(fields[1])?;
        let manager_name = hex_decode(fields[2])?;
        let league_id = fields[3]
            .parse::<usize>()
            .map_err(|_| "Invalid league ID from bridge".to_string())?;
        let is_player_team = match fields[4] {
            "1" => true,
            "0" => false,
            _ => return Err("Invalid player-team flag from bridge".to_string()),
        };
        let roster_size = fields[5]
            .parse::<usize>()
            .map_err(|_| "Invalid team roster size from bridge".to_string())?;
        let staff_count = fields[6]
            .parse::<usize>()
            .map_err(|_| "Invalid team staff count from bridge".to_string())?;
        let roster_rating = if fields[7].trim().is_empty() {
            None
        } else {
            Some(
                fields[7]
                    .parse::<f64>()
                    .map_err(|_| "Invalid team roster rating from bridge".to_string())?,
            )
        };
        let merchandise_facility_grade = hex_decode(fields[8])?;
        let stadium_grade = hex_decode(fields[9])?;
        let training_facility_grade = hex_decode(fields[10])?;
        let total_balance = fields[11]
            .parse::<f64>()
            .map_err(|_| "Invalid team balance from bridge".to_string())?;
        let transfer_budget = fields[12]
            .parse::<f64>()
            .map_err(|_| "Invalid team recruitment budget from bridge".to_string())?;
        let salary_budget = fields[13]
            .parse::<f64>()
            .map_err(|_| "Invalid team salary budget from bridge".to_string())?;
        let (
            stadium_name,
            stadium_capacity,
            total_home_attendance,
            home_match_count,
            total_entrance_income,
            popularity,
            fan_expectation,
            fan_satisfaction,
            fan_count,
            fan_momentum,
            gaming_house_level,
            welfare,
        ) = if fields.len() == 26 {
            (
                hex_decode(fields[14])?,
                fields[15].to_string(),
                fields[16].to_string(),
                fields[17].to_string(),
                fields[18]
                    .parse::<f64>()
                    .map_err(|_| "Invalid team entrance income from bridge".to_string())?,
                fields[19].to_string(),
                hex_decode(fields[20])?,
                hex_decode(fields[21])?,
                fields[22].to_string(),
                fields[23].to_string(),
                hex_decode(fields[24])?,
                fields[25].to_string(),
            )
        } else {
            (
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                0.0,
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            )
        };
        teams.push(TeamSummary {
            id,
            display_name,
            manager_name,
            league_id,
            is_player_team,
            roster_size,
            staff_count,
            roster_rating,
            merchandise_facility_grade,
            stadium_grade,
            training_facility_grade,
            stadium_name,
            stadium_capacity,
            total_home_attendance,
            home_match_count,
            total_entrance_income,
            popularity,
            fan_expectation,
            fan_satisfaction,
            fan_count,
            fan_momentum,
            gaming_house_level,
            welfare,
            total_balance,
            transfer_budget,
            salary_budget,
        });
    }

    if teams.len() != expected_count {
        return Err(format!(
            "Bridge reported {expected_count} teams but sent {}",
            teams.len()
        ));
    }

    Ok(teams)
}

fn parse_player_response(response: &str) -> Result<PlayerStats, String> {
    if let Some(error) = response.strip_prefix("ERR|") {
        return Err(error.to_string());
    }

    let parts: Vec<&str> = response.split('|').collect();
    if parts.len() != 37 || parts[0] != "OK" || parts[1] != "PLAYER" {
        return Err(format!("Unexpected player response: {response}"));
    }

    Ok(PlayerStats {
        id: parts[2]
            .parse::<usize>()
            .map_err(|_| "Invalid player ID from bridge".to_string())?,
        name: hex_decode(parts[3])?,
        last_hit: pretty_number(parts[4]),
        skill_avoid: pretty_number(parts[5]),
        skill_hit: pretty_number(parts[6]),
        control_speed: pretty_number(parts[7]),
        positioning: pretty_number(parts[8]),
        judgement: pretty_number(parts[9]),
        mental: pretty_number(parts[10]),
        concentration: pretty_number(parts[11]),
        order: pretty_number(parts[12]),
        roaming: pretty_number(parts[13]),
        aggressive: pretty_number(parts[14]),
        ego: pretty_number(parts[15]),
        top: pretty_number(parts[16]),
        jungle: pretty_number(parts[17]),
        mid: pretty_number(parts[18]),
        bottom: pretty_number(parts[19]),
        support: pretty_number(parts[20]),
        potential: pretty_number(parts[21]),
        annual_salary: format_internal_amount(parts[22]),
        weekly_salary: format_internal_amount(parts[23]),
        contract_team_id: parse_optional_usize(parts[24])?,
        contract_start_date: display_contract_date(&hex_decode(parts[25])?),
        contract_end_date: display_contract_date(&hex_decode(parts[26])?),
        transfer_fee: format_internal_amount(parts[27]),
        squad_status: hex_decode(parts[28])?,
        incentive_pog_bonus: format_internal_amount(parts[29]),
        incentive_league_bonus: format_internal_amount(parts[30]),
        incentive_league_rank: pretty_number(parts[31]),
        incentive_match_bonus: format_internal_amount(parts[32]),
        incentive_win_bonus: format_internal_amount(parts[33]),
        primary_region: parts[34].to_string(),
        communication_raw: hex_decode(parts[35])?,
        communication_xp_raw: hex_decode(parts[36])?,
    })
}

fn champion_display_name(id: &str) -> String {
    id.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let mut value = first.to_uppercase().collect::<String>();
                    value.push_str(chars.as_str());
                    value
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_available_champions(raw: &str) -> Vec<String> {
    raw.lines()
        .filter_map(|line| {
            let value = line.trim().trim_end_matches(',').trim();
            if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
                Some(value[1..value.len() - 1].to_string())
            } else {
                None
            }
        })
        .collect()
}

fn parse_champion_proficiency(
    raw: &str,
) -> Vec<(String, i32, Option<i32>)> {
    let mut entries = Vec::new();
    let mut current_id: Option<String> = None;
    let mut current_value: Option<i32> = None;
    let mut current_floor: Option<i32> = None;
    let mut in_proficiency = false;

    for line in raw.lines() {
        let trimmed = line.trim();

        if trimmed == "champion_proficiency:" {
            in_proficiency = true;
            continue;
        }

        if trimmed == "recent_champions:" {
            break;
        }

        if !in_proficiency {
            continue;
        }

        if trimmed.starts_with('"') && trimmed.contains(": ChampionProficiency {") {
            if let (Some(id), Some(value)) =
                (current_id.take(), current_value.take())
            {
                entries.push((id, value, current_floor.take()));
            }

            if let Some(end_quote) = trimmed[1..].find('"') {
                current_id =
                    Some(trimmed[1..1 + end_quote].to_string());
                current_value = None;
                current_floor = None;
            }
            continue;
        }

        if let Some(value) = trimmed.strip_prefix("value:") {
            current_value = value
                .trim()
                .trim_end_matches(',')
                .parse::<i32>()
                .ok();
            continue;
        }

        if let Some(value) = trimmed.strip_prefix("floor:") {
            current_floor = value
                .trim()
                .trim_end_matches(',')
                .parse::<i32>()
                .ok();
        }
    }

    if let (Some(id), Some(value)) = (current_id, current_value) {
        entries.push((id, value, current_floor));
    }

    entries
}

fn parse_communication_entries(raw: &str) -> Vec<(usize, i32)> {
    let mut entries = raw
        .split(',')
        .filter_map(|entry| {
            let (region, value) = entry.split_once(':')?;
            Some((region.parse::<usize>().ok()?, value.parse::<i32>().ok()?))
        })
        .collect::<Vec<_>>();
    entries.sort_by_key(|(region_id, _)| *region_id);
    entries
}


#[cfg(feature = "dev")]
fn probe_snapshot_summary(label: &str, snapshot: &str) -> String {
    if snapshot.is_empty() {
        format!("{label}: not saved")
    } else {
        format!("{label}: saved ({} lines)", snapshot.lines().count())
    }
}

#[cfg(feature = "dev")]
fn sanitize_probe_file_component(value: &str) -> String {
    let cleaned = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    let trimmed = cleaned.trim_matches('_');
    if trimmed.is_empty() {
        "unknown".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(feature = "dev")]
fn contract_probe_diff(left_label: &str, left: &str, right_label: &str, right: &str) -> String {
    if left.is_empty() || right.is_empty() {
        return format!("Save both {left_label} and {right_label} snapshots before comparing.");
    }

    #[derive(Clone, Copy)]
    enum DiffLine<'a> {
        Same(&'a str),
        Removed(&'a str),
        Added(&'a str),
    }

    let left_lines = left.lines().collect::<Vec<_>>();
    let right_lines = right.lines().collect::<Vec<_>>();
    let rows = left_lines.len() + 1;
    let columns = right_lines.len() + 1;
    let mut lcs = vec![0usize; rows.saturating_mul(columns)];

    for left_index in (0..left_lines.len()).rev() {
        for right_index in (0..right_lines.len()).rev() {
            let index = left_index * columns + right_index;
            lcs[index] = if left_lines[left_index] == right_lines[right_index] {
                lcs[(left_index + 1) * columns + right_index + 1] + 1
            } else {
                lcs[(left_index + 1) * columns + right_index]
                    .max(lcs[left_index * columns + right_index + 1])
            };
        }
    }

    let mut diff = Vec::new();
    let mut left_index = 0usize;
    let mut right_index = 0usize;
    while left_index < left_lines.len() && right_index < right_lines.len() {
        if left_lines[left_index] == right_lines[right_index] {
            diff.push(DiffLine::Same(left_lines[left_index]));
            left_index += 1;
            right_index += 1;
        } else if lcs[(left_index + 1) * columns + right_index]
            >= lcs[left_index * columns + right_index + 1]
        {
            diff.push(DiffLine::Removed(left_lines[left_index]));
            left_index += 1;
        } else {
            diff.push(DiffLine::Added(right_lines[right_index]));
            right_index += 1;
        }
    }
    while left_index < left_lines.len() {
        diff.push(DiffLine::Removed(left_lines[left_index]));
        left_index += 1;
    }
    while right_index < right_lines.len() {
        diff.push(DiffLine::Added(right_lines[right_index]));
        right_index += 1;
    }

    let changed = diff
        .iter()
        .filter(|line| !matches!(line, DiffLine::Same(_)))
        .count();
    if changed == 0 {
        return format!("--- {left_label}\n+++ {right_label}\n\nNo differences found.");
    }

    let mut output = format!(
        "--- {left_label}\n+++ {right_label}\n\nChanged lines: {changed}\n\n"
    );
    let mut index = 0usize;
    while index < diff.len() {
        match diff[index] {
            DiffLine::Same(_) => {
                let start = index;
                while index < diff.len() && matches!(diff[index], DiffLine::Same(_)) {
                    index += 1;
                }
                let run_length = index - start;
                if run_length <= 6 {
                    for line in &diff[start..index] {
                        if let DiffLine::Same(text) = line {
                            output.push_str("  ");
                            output.push_str(text);
                            output.push('\n');
                        }
                    }
                } else {
                    for line in &diff[start..start + 2] {
                        if let DiffLine::Same(text) = line {
                            output.push_str("  ");
                            output.push_str(text);
                            output.push('\n');
                        }
                    }
                    output.push_str(&format!(
                        "  ... {} unchanged lines omitted ...\n",
                        run_length - 4
                    ));
                    for line in &diff[index - 2..index] {
                        if let DiffLine::Same(text) = line {
                            output.push_str("  ");
                            output.push_str(text);
                            output.push('\n');
                        }
                    }
                }
            }
            DiffLine::Removed(text) => {
                output.push_str("- ");
                output.push_str(text);
                output.push('\n');
                index += 1;
            }
            DiffLine::Added(text) => {
                output.push_str("+ ");
                output.push_str(text);
                output.push('\n');
                index += 1;
            }
        }
    }
    output
}

#[cfg(feature = "dev")]
fn contract_probe_export_text(
    entity_label: &str,
    before: &str,
    after_offer: &str,
    after_accepted: &str,
    current: &str,
) -> String {
    let before_to_offer = contract_probe_diff("BEFORE", before, "AFTER OFFER", after_offer);
    let offer_to_accepted =
        contract_probe_diff("AFTER OFFER", after_offer, "AFTER ACCEPTED", after_accepted);
    format!(
        "TFM2 Contract Flow Probe\nEntity: {entity_label}\n\n\
========= AUTOMATIC DIFF: BEFORE -> AFTER OFFER =========\n{before_to_offer}\n\n\
========= AUTOMATIC DIFF: AFTER OFFER -> AFTER ACCEPTED =========\n{offer_to_accepted}\n\n\
========= BEFORE =========\n{before}\n\n\
========= AFTER OFFER =========\n{after_offer}\n\n\
========= AFTER ACCEPTED =========\n{after_accepted}\n\n\
========= CURRENT STATE =========\n{current}\n"
    )
}

fn hex_encode(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(value.len() * 2);
    for byte in value.as_bytes() {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn validate_editor_name(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("Name cannot be empty".to_string());
    }
    if trimmed.chars().count() > 100 {
        return Err("Name cannot contain more than 100 characters".to_string());
    }
    if trimmed.chars().any(char::is_control) {
        return Err("Name cannot contain line breaks or control characters".to_string());
    }
    Ok(trimmed.to_string())
}

fn hex_decode(encoded: &str) -> Result<String, String> {
    if !encoded.len().is_multiple_of(2) {
        return Err("Invalid text encoding from bridge".to_string());
    }

    let mut bytes = Vec::with_capacity(encoded.len() / 2);
    let raw = encoded.as_bytes();
    for pair in raw.chunks_exact(2) {
        let high = hex_value(pair[0]).ok_or_else(|| "Invalid text encoding from bridge".to_string())?;
        let low = hex_value(pair[1]).ok_or_else(|| "Invalid text encoding from bridge".to_string())?;
        bytes.push((high << 4) | low);
    }

    String::from_utf8(bytes).map_err(|_| "Bridge returned invalid UTF-8".to_string())
}

fn hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn value_or_dash(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        "—".to_string()
    } else {
        trimmed.to_string()
    }
}

fn pretty_or_dash(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        "—".to_string()
    } else {
        pretty_number(trimmed)
    }
}

fn pretty_number(raw: &str) -> String {
    let Ok(value) = raw.parse::<f64>() else {
        return raw.to_string();
    };

    let mut text = if value.fract().abs() < f64::EPSILON {
        format!("{value:.0}")
    } else {
        let mut text = format!("{value:.2}");
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
        text
    };

    let negative = text.starts_with('-');
    if negative {
        text.remove(0);
    }
    let (integer, fraction) = text.split_once('.').unwrap_or((&text, ""));
    let mut grouped_rev = String::new();
    for (index, ch) in integer.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            grouped_rev.push(' ');
        }
        grouped_rev.push(ch);
    }
    let mut grouped: String = grouped_rev.chars().rev().collect();
    if negative {
        grouped.insert(0, '-');
    }
    if !fraction.is_empty() {
        grouped.push('.');
        grouped.push_str(fraction);
    }
    grouped
}

fn apply_id_range_selection(
    ordered_ids: &[usize],
    start_id: usize,
    end_id: usize,
    target_selected: bool,
    selected_ids: &mut BTreeSet<usize>,
) {
    let Some(start_index) = ordered_ids.iter().position(|id| *id == start_id) else {
        return;
    };
    let Some(end_index) = ordered_ids.iter().position(|id| *id == end_id) else {
        return;
    };

    let range_start = start_index.min(end_index);
    let range_end = start_index.max(end_index);
    for id in &ordered_ids[range_start..=range_end] {
        if target_selected {
            selected_ids.insert(*id);
        } else {
            selected_ids.remove(id);
        }
    }
}

fn contract_bonus_display(raw: &str) -> String {
    if raw.trim().is_empty() {
        "Disabled".to_string()
    } else {
        raw.to_string()
    }
}

fn value_or_zero(raw: &str) -> String {
    let value = raw.trim();
    if value.is_empty() {
        "$0".to_string()
    } else {
        value.to_string()
    }
}

fn bool_digit(value: bool) -> u8 {
    if value { 1 } else { 0 }
}

fn display_contract_date(raw: &str) -> String {
    let raw = raw.trim();
    if raw.len() >= 10 && is_iso_date_shape(&raw[..10]) {
        raw[..10].to_string()
    } else {
        raw.to_string()
    }
}

fn is_iso_date_shape(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes.iter().enumerate().all(|(i, b)| {
            i == 4 || i == 7 || b.is_ascii_digit()
        })
}

fn human_error(error: &str) -> String {
    match error {
        "NOT_IN_GAME" => "Bridge is connected, but a career must be loaded".to_string(),
        "PLAYER_TEAM_NOT_FOUND" => "Could not find the player team in the database".to_string(),
        "PLAYER_NOT_FOUND" => "Could not find the selected player in the active career".to_string(),
        "STAFF_NOT_FOUND" => "Could not find the selected staff member in the active career".to_string(),
        "INVALID_ID" => "Bridge received an invalid ID".to_string(),
        "INVALID_NAME_ENCODING" => "Bridge received an invalid encoded name".to_string(),
        "NAME_EMPTY" => "Name cannot be empty".to_string(),
        "NAME_TOO_LONG" => "Name cannot contain more than 100 characters".to_string(),
        "NAME_CONTROL_CHARACTER" => "Name cannot contain line breaks or control characters".to_string(),
        "INVALID_STAT" => "Bridge received an invalid attribute value".to_string(),
        "STAT_OUT_OF_RANGE" => "Attributes must be integers between 1 and 100".to_string(),
        "INVALID_POSITION" => "Bridge received an invalid position value".to_string(),
        "POSITION_OUT_OF_RANGE" => "Position values must be between 0 and 100".to_string(),
        "TOO_MANY_POSITIONS" => "TFM2 supports at most three active positions".to_string(),
        "INVALID_POTENTIAL" => "Bridge received an invalid potential value".to_string(),
        "POTENTIAL_OUT_OF_RANGE" => "Potential must be between 1 and 100".to_string(),
        "INVALID_SALARY" => "Bridge received an invalid salary value".to_string(),
        "SALARY_OUT_OF_RANGE" => "Salary must be zero or greater".to_string(),
        "SALARY_TYPE_ERROR" => "TFM2 rejected the salary value type".to_string(),
        "INVALID_CONTRACT_DATE" => "TFM2 rejected the contract date. Use YYYY-MM-DD".to_string(),
        "CONTRACT_WRITE_NOT_APPLIED" => "TFM2 returned the original contract instead of the requested values".to_string(),
        "CONTRACT_END_BEFORE_START" => "Contract end date cannot be before the start date".to_string(),
        "INVALID_TRANSFER_FEE" => "Bridge received an invalid transfer fee".to_string(),
        "TRANSFER_FEE_OUT_OF_RANGE" => "Transfer fee must be zero or greater".to_string(),
        "TRANSFER_FEE_TYPE_ERROR" => "TFM2 rejected the transfer fee value type".to_string(),
        "INVALID_SQUAD_STATUS" => "Bridge received an invalid Squad Status".to_string(),
        "INVALID_CONTRACT_BONUS" => "Bridge received an invalid contract bonus".to_string(),
        "CONTRACT_BONUS_OUT_OF_RANGE" => "Contract bonuses must be zero or greater".to_string(),
        "INVALID_LEAGUE_RANK" => "League Rank must be a whole number between 1 and 10".to_string(),
        "STAFF_FREE_AGENT_NEEDS_CONTRACT" => "Free-agent staff must be signed through Edit Contract".to_string(),
        "STAFF_ALREADY_FREE_AGENT" => "The selected staff member is already a free agent".to_string(),
        "TEAM_NOT_FOUND" => "Could not find the selected team in the active career".to_string(),
        "PLAYER_FREE_AGENT" => "The selected player is a free agent and has no active salary".to_string(),
        "PLAYER_ALREADY_FREE_AGENT" => "The selected player is already a free agent".to_string(),
        "STAFF_FREE_AGENT" => "The selected staff member is a free agent and has no active salary".to_string(),
        "NO_COMMUNICATION_REGIONS" => "The selected staff member has no Communication regions to edit".to_string(),
        "DUPLICATE_REGION" => "The Staff Communication data contains a duplicate region".to_string(),
        "COMMUNICATION_TYPE_ERROR" => "TFM2 rejected the Staff Communication value type".to_string(),
        "INVALID_BOOLEAN" => "Bridge received an invalid toggle value".to_string(),
        "INVALID_COMMUNICATION" => "Bridge received an invalid communication value".to_string(),
        "COMMUNICATION_OUT_OF_RANGE" => "Communication must be between 0 and 100".to_string(),
        "INVALID_STAMINA" => "Stamina must be a number from 0 to 100".to_string(),
        "INVALID_CONDITION" => "Condition must be a number from 0 to 100".to_string(),
        "CONDITION_OUT_OF_RANGE" => "Stamina and Condition must be between 0 and 100".to_string(),
        "NO_REGIONS_DETECTED" => "No region IDs could be detected in the current save".to_string(),
        "INVALID_REGION" => "The selected Communication region is not available in the current save".to_string(),
        "SERVER_COMMAND_FAILED" => "Could not send the change to TFM2 management/server state".to_string(),
        "GAME_RESPONSE_TIMEOUT" => "The game did not respond to the bridge command".to_string(),
        other => format!("Bridge error: {other}"),
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod compatibility_tests {
    use super::*;

    #[cfg(feature = "dev")]
    fn test_hex_encode(value: &str) -> String {
        value
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect::<Vec<_>>()
            .join("")
    }

    fn test_player(age: u8, actual_potential: u8) -> PlayerSummary {
        PlayerSummary {
            id: 1,
            name: "Test Player".to_string(),
            age: age.to_string(),
            team: "Free Agent".to_string(),
            region: "Europe".to_string(),
            position: "Mid".to_string(),
            actual_rating: "60".to_string(),
            _scout_potential_report: String::new(),
            actual_potential: actual_potential.to_string(),
            salary: "0".to_string(),
            transfer_fee: "0".to_string(),
            contract_end: String::new(),
            last_hit: "50".to_string(),
            skill_avoid: "50".to_string(),
            skill_hit: "50".to_string(),
            control_speed: "50".to_string(),
            positioning: "50".to_string(),
            judgement: "50".to_string(),
            mental: "50".to_string(),
            concentration: "50".to_string(),
            order: "50".to_string(),
            roaming: "50".to_string(),
            aggressive: "50".to_string(),
            ego: "50".to_string(),
        }
    }

    #[test]
    fn editor_name_validation_trims_and_preserves_unicode() {
        assert_eq!(
            validate_editor_name("  René | 홍길동  ").unwrap(),
            "René | 홍길동"
        );
    }

    #[test]
    fn editor_name_validation_rejects_invalid_values() {
        assert!(validate_editor_name("   ").is_err());
        assert!(validate_editor_name("line\nbreak").is_err());
        assert!(validate_editor_name(&"x".repeat(101)).is_err());
    }

    #[test]
    fn editor_name_hex_payload_round_trips_unicode_and_separator() {
        let name = "René | 홍길동";
        assert_eq!(hex_decode(&hex_encode(name)).unwrap(), name);
    }

    fn range_mut<'a>(
        filter: &'a mut AdvancedPlayerSearch,
        key: &str,
    ) -> &'a mut AdvancedRangeFilter {
        filter
            .ranges
            .iter_mut()
            .find(|range| range.key == key)
            .expect("advanced range must exist")
    }

    #[test]
    fn actual_potential_range_is_positioned_after_actual_rating() {
        let filter = AdvancedPlayerSearch::default();
        let keys = filter
            .ranges
            .iter()
            .map(|range| range.key)
            .collect::<Vec<_>>();
        let rating = keys.iter().position(|key| *key == "actual_rating").unwrap();
        let potential = keys.iter().position(|key| *key == "actual_potential").unwrap();
        let last_hit = keys.iter().position(|key| *key == "last_hit").unwrap();

        assert_eq!(potential, rating + 1);
        assert_eq!(last_hit, potential + 1);
    }

    #[test]
    fn actual_potential_range_supports_min_max_exact_disabled_and_combined_age() {
        let player = test_player(18, 80);
        let mut filter = AdvancedPlayerSearch::default();

        {
            let potential = range_mut(&mut filter, "actual_potential");
            potential.enabled = true;
            potential.min = "80".to_string();
        }
        assert!(advanced_player_filter_matches(&player, &filter));
        range_mut(&mut filter, "actual_potential").min = "81".to_string();
        assert!(!advanced_player_filter_matches(&player, &filter));

        {
            let potential = range_mut(&mut filter, "actual_potential");
            potential.min.clear();
            potential.max = "80".to_string();
        }
        assert!(advanced_player_filter_matches(&player, &filter));
        range_mut(&mut filter, "actual_potential").max = "79".to_string();
        assert!(!advanced_player_filter_matches(&player, &filter));

        {
            let potential = range_mut(&mut filter, "actual_potential");
            potential.min = "80".to_string();
            potential.max = "80".to_string();
        }
        assert!(advanced_player_filter_matches(&player, &filter));

        {
            let age = range_mut(&mut filter, "age");
            age.enabled = true;
            age.max = "18".to_string();
        }
        assert!(advanced_player_filter_matches(&player, &filter));
        range_mut(&mut filter, "age").max = "17".to_string();
        assert!(!advanced_player_filter_matches(&player, &filter));

        {
            let age = range_mut(&mut filter, "age");
            age.enabled = false;
            let potential = range_mut(&mut filter, "actual_potential");
            potential.enabled = false;
            potential.min = "100".to_string();
            potential.max = "100".to_string();
        }
        assert!(advanced_player_filter_matches(&player, &filter));
    }

    #[test]
    fn actual_potential_saved_filter_round_trip_preserves_condition() {
        let mut source = AdvancedPlayerSearch::default();
        {
            let potential = range_mut(&mut source, "actual_potential");
            potential.enabled = true;
            potential.min = "80".to_string();
            potential.max = "95".to_string();
        }

        let exported = source.export_text();
        assert!(exported.contains("range.actual_potential.enabled=true"));
        assert!(exported.contains("range.actual_potential.min=80"));
        assert!(exported.contains("range.actual_potential.max=95"));

        let mut loaded = AdvancedPlayerSearch::default();
        loaded.import_text(&exported);
        let potential = loaded
            .ranges
            .iter()
            .find(|range| range.key == "actual_potential")
            .unwrap();
        assert!(potential.enabled);
        assert_eq!(potential.min, "80");
        assert_eq!(potential.max, "95");
    }

    #[test]
    fn legacy_saved_filter_without_actual_potential_loads_with_empty_disabled_default() {
        let legacy = concat!(
            "money_unit_format=display_v1\n",
            "position_enabled=false\n",
            "position=No Condition\n",
            "region_enabled=false\n",
            "region=No Condition\n",
            "free_agents_only=false\n",
            "range.age.enabled=true\n",
            "range.age.min=\n",
            "range.age.max=18\n",
            "range.actual_rating.enabled=true\n",
            "range.actual_rating.min=50\n",
            "range.actual_rating.max=\n",
        );

        let mut loaded = AdvancedPlayerSearch::default();
        loaded.import_text(legacy);

        let potential = loaded
            .ranges
            .iter()
            .find(|range| range.key == "actual_potential")
            .unwrap();
        assert!(!potential.enabled);
        assert!(potential.min.is_empty());
        assert!(potential.max.is_empty());

        let age = loaded
            .ranges
            .iter()
            .find(|range| range.key == "age")
            .unwrap();
        assert!(age.enabled);
        assert_eq!(age.max, "18");
    }

    #[test]
    fn teams_parser_preserves_legacy_payload_and_reads_expanded_team_information() {
        let legacy = parse_teams_response(
            "OK|TEAMS|1|1:5465616d:4d616e61676572:2:1:5:3:80.5:53:42:41:1000:500:300",
        )
        .unwrap();
        assert_eq!(legacy.len(), 1);
        assert_eq!(legacy[0].display_name, "Team");
        assert!(legacy[0].stadium_name.is_empty());

        let expanded = parse_teams_response(
            "OK|TEAMS|1|1:5465616d:4d616e61676572:2:1:5:3:80.5:53:42:41:1000:500:300:4172656e61:15000:30000:3:250000:2:546f70:4e6f726d616c:12000:1:4c7633:52",
        )
        .unwrap();
        assert_eq!(expanded.len(), 1);
        assert_eq!(expanded[0].stadium_name, "Arena");
        assert_eq!(expanded[0].stadium_capacity, "15000");
        assert_eq!(expanded[0].fan_expectation, "Top");
        assert_eq!(expanded[0].fan_satisfaction, "Normal");
        assert_eq!(expanded[0].gaming_house_level, "Lv3");
        assert_eq!(expanded[0].welfare, "52");
        assert_eq!(expanded[0].average_home_attendance(), Some(10000.0));
    }

    #[cfg(feature = "dev")]
    #[test]
    fn team_workspace_member_matching_uses_loaded_team_name_or_fallback_id() {
        let team = TeamSummary {
            id: 42,
            display_name: "GO Team Astrals".to_string(),
            manager_name: String::new(),
            league_id: 1,
            is_player_team: true,
            roster_size: 6,
            staff_count: 7,
            roster_rating: None,
            merchandise_facility_grade: String::new(),
            stadium_grade: String::new(),
            training_facility_grade: String::new(),
            stadium_name: String::new(),
            stadium_capacity: String::new(),
            total_home_attendance: String::new(),
            home_match_count: String::new(),
            total_entrance_income: 0.0,
            popularity: String::new(),
            fan_expectation: String::new(),
            fan_satisfaction: String::new(),
            fan_count: String::new(),
            fan_momentum: String::new(),
            gaming_house_level: String::new(),
            welfare: String::new(),
            total_balance: 0.0,
            transfer_budget: 0.0,
            salary_budget: 0.0,
        };

        assert!(summary_belongs_to_team("GO Team Astrals", &team));
        assert!(summary_belongs_to_team("go team astrals", &team));
        assert!(summary_belongs_to_team("Team 42", &team));
        assert!(!summary_belongs_to_team("Free Agent", &team));
        assert!(!summary_belongs_to_team("Another Team", &team));
    }

    #[cfg(feature = "dev")]
    #[test]
    fn team_condition_probe_parses_management_fields() {
        let raw = r#"Athlete {
    name: "Test",
    management: AthleteManagementStat {
        stamina: 87,
        condition: 63.5,
        stress: 0,
    },
}"#;

        let (stamina, condition) = parse_team_condition_from_player_probe(raw).unwrap();
        assert_eq!(stamina, "87");
        assert_eq!(condition, "63.5");
    }

    #[cfg(feature = "dev")]
    #[test]
    fn team_condition_probe_rejects_missing_management_fields() {
        let raw = "Athlete { management: AthleteManagementStat { stress: 4, } }";
        assert!(parse_team_condition_from_player_probe(raw).is_err());
    }

    #[cfg(feature = "dev")]
    #[test]
    fn team_condition_editor_accepts_inclusive_zero_to_one_hundred() {
        assert!(validate_condition_editor_value("0", "Stamina").is_ok());
        assert!(validate_condition_editor_value("63.5", "Condition").is_ok());
        assert!(validate_condition_editor_value("100", "Condition").is_ok());
        assert!(validate_condition_editor_value("-1", "Stamina").is_err());
        assert!(validate_condition_editor_value("100.1", "Condition").is_err());
    }

    #[cfg(feature = "dev")]
    #[test]
    fn team_condition_entry_tracks_pending_changes() {
        let mut entry = TeamConditionEntry {
            player_id: 1,
            player_name: "Test".to_string(),
            stamina: "87".to_string(),
            condition: "63.5".to_string(),
            original_stamina: "87".to_string(),
            original_condition: "63.5".to_string(),
            write_status: "Ready".to_string(),
        };
        assert!(!entry.has_changes());
        entry.condition = "100".to_string();
        assert!(entry.has_changes());
    }

    #[cfg(feature = "dev")]
    #[test]
    fn team_management_parser_reads_all_read_only_sections() {
        let management = [
            "lineup\tTop\t43\tSiwoo",
            "lineup\tJungle\t486\tHizto",
            "watched_player\t43\tSiwoo",
            "no_transfer_player\t43\tSiwoo",
            "release_player\t99\tReserve",
            "watched_staff\t4\tCyanidefi",
            "release_staff\t5\tCoach",
            "metric\tpending_installments\t2",
            "metric\tresale_clauses\t1",
            "metric\tscout_dispatch\tActive",
            "metric\tmerchandise_products\t1",
            "metric\tchampion_tiers\t1",
            "metric\tpersonal_tactics\t1",
        ]
        .join("\n");
        let current_strategy = "focused\tBottom\nearly_jungle\tCounterJungle";
        let last_strategy = "focused\tAll\nearly_jungle\tGanking";
        let color_strategy = "focused\tAuto\nearly_jungle\tGanking";
        let merchandise = "3\t43\tSiwoo\t49\t66000\t151\t9966000\t151\t9966000\t15";
        let champion_setup = "soldier\tS\tAuto\tAuto\tAuto";
        let gaming_house = [
            "level\tLv3",
            "welfare\t30",
            "owned_furniture_types\t9",
            "owned_furniture_total\t12",
            "owned_wallpaper_types\t1",
            "owned_wallpaper_total\t18",
            "owned_wall_types\t0",
            "owned_wall_total\t0",
            "owned_window_types\t1",
            "owned_window_total\t1",
            "placed_furniture\t0",
            "placed_wallpapers\t0",
            "placed_walls\t0",
            "placed_windows\t0",
        ]
        .join("\n");
        let response = format!(
            "OK|TEAM_MANAGEMENT|85|{}|{}|{}|{}|{}|{}|{}",
            test_hex_encode(&management),
            test_hex_encode(current_strategy),
            test_hex_encode(last_strategy),
            test_hex_encode(color_strategy),
            test_hex_encode(merchandise),
            test_hex_encode(champion_setup),
            test_hex_encode(&gaming_house),
        );

        let parsed = parse_team_management_response(&response).unwrap();
        assert_eq!(parsed.team_id, 85);
        assert_eq!(parsed.lineup.len(), 2);
        assert_eq!(parsed.watched_players[0].name, "Siwoo");
        assert_eq!(parsed.pending_installments, 2);
        assert_eq!(parsed.resale_clauses, 1);
        assert_eq!(parsed.scout_dispatch, "Active");
        assert_eq!(parsed.current_strategy[0].value, "Bottom");
        assert_eq!(parsed.merchandise[0].stock, "49");
        assert_eq!(parsed.champion_setup[0].tier, "S");
        assert_eq!(parsed.gaming_house.owned_furniture_total, 12);
    }

    #[cfg(feature = "dev")]
    #[test]
    fn team_management_parser_accepts_empty_ai_team_collections() {
        let response = format!(
            "OK|TEAM_MANAGEMENT|92|{}|{}|{}|{}|{}|{}|{}",
            test_hex_encode("lineup\tTop\t873\tPlayer A\nmetric\tpending_installments\t0\nmetric\tresale_clauses\t0\nmetric\tscout_dispatch\tNone\nmetric\tmerchandise_products\t0\nmetric\tchampion_tiers\t0\nmetric\tpersonal_tactics\t0"),
            test_hex_encode("focused\tTop"),
            test_hex_encode("focused\tBottom"),
            test_hex_encode("focused\tAuto"),
            test_hex_encode(""),
            test_hex_encode(""),
            test_hex_encode("level\tLv3\nwelfare\t52"),
        );

        let parsed = parse_team_management_response(&response).unwrap();
        assert!(parsed.merchandise.is_empty());
        assert!(parsed.champion_setup.is_empty());
        assert_eq!(parsed.scout_dispatch, "None");
        assert_eq!(parsed.gaming_house.level, "Lv3");
    }

    #[cfg(feature = "dev")]
    #[test]
    fn champion_mastery_name_uses_at_most_two_lines() {
        let display = champion_mastery_card_display_name("Poison Dart Hunter");
        assert!(display.lines().count() <= 2);
        assert!(!display.is_empty());
    }

    #[cfg(feature = "dev")]
    #[test]
    fn champion_mastery_unbroken_long_name_is_ellipsized() {
        let display = champion_mastery_card_display_name(
            "AnExtremelyLongModChampionNameWithoutSpaces",
        );
        assert_eq!(display.lines().count(), 2);
        assert!(display.ends_with('…'));
    }

    #[cfg(feature = "dev")]
    #[test]
    fn champion_mastery_columns_follow_only_the_supplied_width() {
        assert_eq!(champion_mastery_columns_for_width(560.0), 2);
        assert_eq!(champion_mastery_columns_for_width(800.0), 4);
        assert_eq!(champion_mastery_columns_for_width(1080.0), 5);
    }

    #[cfg(feature = "dev")]
    #[test]
    fn champion_mastery_card_geometry_is_fixed_and_compact() {
        assert_eq!(CHAMPION_MASTERY_CARD_INNER_WIDTH, 172.0);
        assert_eq!(CHAMPION_MASTERY_CARD_INNER_HEIGHT, 38.0);
        assert_eq!(CHAMPION_MASTERY_CARD_OUTER_WIDTH, 184.0);
        assert_eq!(CHAMPION_MASTERY_CARD_HORIZONTAL_GAP, 8.0);
    }

    #[test]
    fn range_selection_selects_a_contiguous_range_in_both_directions() {
        let ordered_ids = [1, 2, 3, 4, 5, 6];
        let mut selected_ids = BTreeSet::from([1, 6]);

        apply_id_range_selection(&ordered_ids, 5, 2, true, &mut selected_ids);

        assert_eq!(selected_ids, BTreeSet::from([1, 2, 3, 4, 5, 6]));
    }

    #[test]
    fn range_selection_deselects_only_the_requested_range() {
        let ordered_ids = [1, 2, 3, 4, 5, 6];
        let mut selected_ids = BTreeSet::from([1, 2, 3, 4, 5, 6]);

        apply_id_range_selection(&ordered_ids, 2, 5, false, &mut selected_ids);

        assert_eq!(selected_ids, BTreeSet::from([1, 6]));
    }

    #[test]
    fn loaded_dataset_status_uses_the_search_table_count() {
        assert_eq!(
            ModifierApp::loaded_dataset_status("Player", 1014),
            "Player data loaded: 1014"
        );
        assert_eq!(
            ModifierApp::loaded_dataset_status("Staff", 327),
            "Staff data loaded: 327"
        );
        assert_eq!(
            ModifierApp::loaded_dataset_status("Team", 64),
            "Team data loaded: 64"
        );
    }

    #[test]
    fn recruitment_player_search_matches_only_player_name_or_id() {
        assert!(ModifierApp::recruitment_player_matches_search(
            "Faker", 123, "fak"
        ));
        assert!(ModifierApp::recruitment_player_matches_search(
            "Faker", 123, "123"
        ));
        assert!(!ModifierApp::recruitment_player_matches_search(
            "Faker", 123, "t1"
        ));
    }

    #[test]
    fn matching_bridge_is_compatible() {
        assert!(ModifierApp::compatibility_issue_for(
            REQUIRED_BRIDGE_VERSION,
            Some(BRIDGE_PROTOCOL_VERSION),
            Some(SUPPORTED_TFM2_VERSION),
        )
        .is_none());
    }

    #[test]
    fn unknown_future_bridge_without_protocol_is_unverified_warning() {
        let issue = ModifierApp::compatibility_issue_for("0.2.50", None, None).unwrap();
        assert_eq!(issue.severity, CompatibilitySeverity::Warning);
        assert_eq!(issue.action, CompatibilityAction::EditorUpdate);
        assert_eq!(issue.reason, CompatibilityReason::UnverifiedLegacyBridge);
    }

    #[test]
    fn older_pre_migration_bridge_is_not_supported() {
        let issue = ModifierApp::compatibility_issue_for(
            "0.2.38",
            Some(BRIDGE_PROTOCOL_VERSION),
            Some(SUPPORTED_TFM2_VERSION),
        )
        .unwrap();
        assert_eq!(issue.severity, CompatibilitySeverity::NotSupported);
        assert_eq!(issue.action, CompatibilityAction::BridgeUpdate);
        assert_eq!(issue.reason, CompatibilityReason::KnownUnsupportedCombination);
    }

    #[test]
    fn previous_bridge_without_name_commands_is_hard_blocked() {
        let issue = ModifierApp::compatibility_issue_for(
            "0.2.48",
            Some(BRIDGE_PROTOCOL_VERSION),
            Some(SUPPORTED_TFM2_VERSION),
        )
        .unwrap();
        assert_eq!(issue.severity, CompatibilitySeverity::NotSupported);
        assert_eq!(issue.action, CompatibilityAction::BridgeUpdate);
        assert_eq!(issue.reason, CompatibilityReason::KnownUnsupportedCombination);
    }

    #[test]
    fn known_old_bridge_is_not_supported_even_without_protocol_data() {
        let issue = ModifierApp::compatibility_issue_for("0.2.30", None, None).unwrap();
        assert_eq!(issue.severity, CompatibilitySeverity::NotSupported);
        assert_eq!(issue.action, CompatibilityAction::BridgeUpdate);
        assert_eq!(
            issue.reason,
            CompatibilityReason::KnownUnsupportedCombination
        );
    }

    #[cfg(not(feature = "dev"))]
    #[test]
    fn community_required_bridge_is_not_caught_by_a_future_bridge_rule() {
        assert!(unsupported_bridge_rule_for(REQUIRED_BRIDGE_VERSION).is_none());
        assert!(ModifierApp::compatibility_issue_for(
            REQUIRED_BRIDGE_VERSION,
            Some(BRIDGE_PROTOCOL_VERSION),
            Some(SUPPORTED_TFM2_VERSION),
        )
        .is_none());
    }

    #[cfg(feature = "dev")]
    #[test]
    fn development_required_bridge_is_not_caught_by_community_future_bridge_rule() {
        assert!(unsupported_bridge_rule_for(REQUIRED_BRIDGE_VERSION).is_none());
        assert!(ModifierApp::compatibility_issue_for(
            REQUIRED_BRIDGE_VERSION,
            Some(BRIDGE_PROTOCOL_VERSION),
            Some(SUPPORTED_TFM2_VERSION),
        )
        .is_none());
    }

    #[cfg(feature = "dev")]
    #[test]
    fn future_development_bridge_with_current_protocol_is_warning_only() {
        let issue = ModifierApp::compatibility_issue_for(
            "0.2.50",
            Some(BRIDGE_PROTOCOL_VERSION),
            Some(SUPPORTED_TFM2_VERSION),
        )
        .unwrap();
        assert_eq!(issue.severity, CompatibilitySeverity::Warning);
        assert_eq!(issue.action, CompatibilityAction::EditorUpdate);
        assert_eq!(issue.reason, CompatibilityReason::VersionMismatch);
    }

    #[test]
    fn unsafe_old_protocol_is_not_supported() {
        let issue = ModifierApp::compatibility_issue_for(
            REQUIRED_BRIDGE_VERSION,
            Some(0),
            Some(SUPPORTED_TFM2_VERSION),
        )
        .unwrap();
        assert_eq!(issue.severity, CompatibilitySeverity::NotSupported);
        assert_eq!(issue.action, CompatibilityAction::VerifyInstallation);
        assert_eq!(issue.reason, CompatibilityReason::ProtocolMismatch);
    }

    #[test]
    fn unsupported_future_protocol_requires_verification() {
        let issue = ModifierApp::compatibility_issue_for(
            REQUIRED_BRIDGE_VERSION,
            Some(BRIDGE_PROTOCOL_VERSION + 1),
            Some(SUPPORTED_TFM2_VERSION),
        )
        .unwrap();
        assert_eq!(issue.severity, CompatibilitySeverity::NotSupported);
        assert_eq!(issue.action, CompatibilityAction::VerifyInstallation);
        assert_eq!(issue.reason, CompatibilityReason::ProtocolMismatch);
    }

    #[test]
    fn wrong_tfm2_target_has_separate_warning() {
        let issue = ModifierApp::compatibility_issue_for(
            REQUIRED_BRIDGE_VERSION,
            Some(BRIDGE_PROTOCOL_VERSION),
            Some("0.5.3"),
        )
        .unwrap();
        assert_eq!(issue.severity, CompatibilitySeverity::Warning);
        assert_eq!(issue.action, CompatibilityAction::GameVersionMismatch);
        assert_eq!(issue.reason, CompatibilityReason::GameTargetMismatch);
    }

    #[cfg(feature = "dev")]
    #[test]
    fn team_history_parser_reads_match_analysis_and_rating() {
        let raw = r##"=== SELECTED TEAM RECORD ===
Team key: 85
Team {
    news: [
        News {
            ty: PreMatchAnalysis {
                enemy_team_id: 89,
                match_id: 787,
                tactics: [
                    PreMatchTacticEntry {
                        category: "early_jungle",
                        value: "counter_jungle",
                    },
                ],
                champion_picks: [
                    PreMatchChampionEntry {
                        champion: "ninja",
                        position: 1,
                        wins: 2,
                        losses: 1,
                    },
                ],
                has_match_history: true,
                star_player_id: 273,
                analysis_level: 93,
                insights: [
                    PreMatchInsight {
                        section: "strategy",
                        texts: [
                            PreMatchInsightText {
                                i18n_key: "#asset/base/text/news?pre_match_analysis.insight.strategy.recommend_champion",
                                binds: [],
                                champion_keys: ["gunner", "ice_mage"],
                            },
                        ],
                    },
                ],
            },
            title: "analysis",
            title_bind: [],
            author: "staff",
            date: 2026-02-07T15:30:00,
            is_read: true,
            is_sent: true,
            is_favorite: false,
        },
        News {
            ty: MatchReport {
                match_id: 787,
                is_practice: false,
                my_team_id: 85,
                enemy_team_id: 89,
                is_win: true,
                my_team_score: 2,
                enemy_team_score: 0,
                set_data: [
                    MatchSetArticleData {
                        game_tick: 33788,
                        team1_total_kill: 24,
                        team2_total_kill: 5,
                        team1_total_gold: 49198,
                        team2_total_gold: 37418,
                        mvp_athlete_id: 43,
                        mvp_champion: "cavalry_knight",
                        mvp_kills: 7,
                        mvp_deaths: 0,
                        mvp_assists: 6,
                        is_team1_win: true,
                        was_comeback: false,
                        is_team1_blue: false,
                    },
                ],
                article_pattern: CleanSweep,
                set_patterns: [DominantWin],
            },
            title: "match",
            title_bind: [],
            author: "report",
            date: 2026-02-07T16:30:00,
            is_read: true,
            is_sent: true,
            is_favorite: false,
        },
        News {
            ty: TeamRatingRankingReport {
                rankings: [
                    TeamRatingRankEntry {
                        team_id: 25,
                        rating: 1644,
                    },
                    TeamRatingRankEntry {
                        team_id: 85,
                        rating: 1442,
                    },
                ],
            },
            title: "rating",
            title_bind: [],
            author: "system",
            date: 2026-02-15T00:00:00,
            is_read: true,
            is_sent: true,
            is_favorite: false,
        },
    ],
}"##;

        let parsed = parse_team_history_probe(raw, 85, &[], &[]).unwrap();
        assert_eq!(parsed.matches.len(), 1);
        assert_eq!(parsed.matches[0].match_id, 787);
        assert_eq!(parsed.matches[0].my_score, 2);
        assert_eq!(parsed.matches[0].sets.len(), 1);
        assert_eq!(parsed.matches[0].sets[0].mvp_player_id, 43);
        assert_eq!(parsed.analyses.len(), 1);
        assert_eq!(parsed.analyses[0].tactics.len(), 1);
        assert_eq!(parsed.analyses[0].champion_picks.len(), 1);
        assert_eq!(parsed.analyses[0].insights.len(), 1);
        assert_eq!(parsed.latest_rating, Some(1442));
        assert_eq!(parsed.latest_rank, Some(2));
    }

    #[cfg(feature = "dev")]
    #[test]
    fn team_history_parser_accepts_team_without_history() {
        let raw = "=== SELECTED TEAM RECORD ===\nTeam key: 61\nTeam { news: [] }";
        let parsed = parse_team_history_probe(raw, 61, &[], &[]).unwrap();
        assert!(parsed.matches.is_empty());
        assert!(parsed.analyses.is_empty());
        assert_eq!(parsed.latest_rating, None);
        assert_eq!(parsed.latest_rank, None);
    }

}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([920.0, 760.0])
            .with_min_inner_size([720.0, 620.0]),
        ..Default::default()
    };

    let title = window_title();

    eframe::run_native(
        &title,
        options,
        Box::new(|cc| {
            let mut visuals = egui::Visuals::dark();
            visuals.selection.bg_fill = egui::Color32::from_rgb(35, 92, 170);
            visuals.selection.stroke = egui::Stroke::new(
                1.0_f32,
                egui::Color32::from_rgb(145, 195, 255),
            );
            cc.egui_ctx.set_visuals(visuals);
            Ok(Box::new(ModifierApp::default()))
        }),
    )
}
