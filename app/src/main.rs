#![cfg_attr(all(windows, not(feature = "dev")), windows_subsystem = "windows")]

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::time::Duration;

use eframe::egui;
use egui_extras::{Column, TableBuilder};

const BRIDGE_ADDR: &str = "127.0.0.1:28452";
#[cfg(feature = "dev")]
const APP_VERSION: &str = "0.3.9b";
#[cfg(not(feature = "dev"))]
const APP_VERSION: &str = "0.3.0";

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

#[cfg(feature = "dev")]
fn player_editor_intro_text() -> &'static str {
    "Attributes, positions, potential, contract finance, Champion Mastery, and selectable Communication regions."
}

#[cfg(not(feature = "dev"))]
fn player_editor_intro_text() -> &'static str {
    "Edit player attributes, positions, potential, active contracts, Champion Mastery, and Communication."
}

#[cfg(feature = "dev")]
fn salary_info_text() -> &'static str {
    "Salary currently uses TFM2's internal money units. Currency detection/conversion from the active save is planned; salary writes already persist through Proceed."
}

#[cfg(not(feature = "dev"))]
fn salary_info_text() -> &'static str {
    "Salary and transfer-fee fields use the amounts stored in the active career."
}

#[cfg(feature = "dev")]
fn potential_info_text() -> &'static str {
    "Actual Potential is the hidden 1–100 value. Grade presets set fixed values; editing the numeric value updates the grade automatically."
}

#[cfg(not(feature = "dev"))]
fn potential_info_text() -> &'static str {
    "Actual Potential is hidden in-game and normally represented through scout evaluation. Warning: changes to Actual Potential cannot currently be reverted by the editor. Saving your career before editing is recommended."
}

#[cfg(feature = "dev")]
fn transfer_runtime_text() -> &'static str {
    "Runtime toggle. Enabled state is re-applied automatically after loading another save."
}

#[cfg(not(feature = "dev"))]
fn transfer_runtime_text() -> &'static str {
    "Runtime toggle."
}

#[cfg(feature = "dev")]
fn recruitment_player_management_text() -> &'static str {
    "Move contracted players directly between teams, sign free agents through Edit Contract, or end a current contract to make the player a Free Agent."
}

#[cfg(not(feature = "dev"))]
fn recruitment_player_management_text() -> &'static str {
    "Move contracted players, create contracts for free-agent signings, or set a contracted player to Free Agent."
}

#[cfg(feature = "dev")]
fn search_intro_text() -> &'static str {
    "Expandable database-style search, lists, and future historical snapshots."
}

#[cfg(not(feature = "dev"))]
fn search_intro_text() -> &'static str {
    "Search, filter, sort, and compare player data."
}

#[cfg(feature = "dev")]
fn advanced_search_info_text() -> &'static str {
    "Import/Export uses normal files. Saved filters are stored in the local filters folder beside TFM2 Editor.exe. Enabled conditions combine with Quick Filters."
}

#[cfg(not(feature = "dev"))]
fn advanced_search_info_text() -> &'static str {
    "Combine filters to narrow the player database. Saved filters are stored locally in the filters folder."
}

#[cfg(feature = "dev")]
fn champion_mastery_help_text() -> &'static str {
    "Editing a value automatically checks that champion. Apply Selected writes only checked champions. ChampionProficiency.floor is left unchanged."
}

#[cfg(not(feature = "dev"))]
fn champion_mastery_help_text() -> &'static str {
    "Editing a value automatically checks that champion. Apply Selected saves only checked champions."
}

#[cfg(feature = "dev")]
fn search_rating_info_text() -> &'static str {
    "Actual Rating uses TFM2's rating when available; ≈ values are the average of the 12 attributes. Potential Rating is derived directly from hidden Actual Potential."
}

#[cfg(not(feature = "dev"))]
fn search_rating_info_text() -> &'static str {
    "Actual Rating: ≈ values are the average of the 12 attributes. Potential Rating is based on hidden Actual Potential, not scout evaluation."
}

#[cfg(feature = "dev")]
fn move_player_tooltip_text() -> &'static str {
    "Direct contract-team move. Clears pending transfer/recruit requests for that player. Test new game versions with a backup save first."
}

#[cfg(not(feature = "dev"))]
fn move_player_tooltip_text() -> &'static str {
    "Move the selected contracted player to the destination team."
}

#[cfg(feature = "dev")]
fn transfer_success_tooltip_text() -> &'static str {
    "Forces the latest transfer and player-contract negotiation papers for your current team to Accepted during management ticks."
}

#[cfg(not(feature = "dev"))]
fn transfer_success_tooltip_text() -> &'static str {
    "Makes transfer negotiations succeed."
}

#[cfg(feature = "dev")]
fn instant_retry_tooltip_text() -> &'static str {
    "Moves recruitment retry cooldowns back to the request's last action date, allowing an immediate new offer after rejection."
}

#[cfg(not(feature = "dev"))]
fn instant_retry_tooltip_text() -> &'static str {
    "Allows an immediate new offer after a rejected negotiation."
}

#[cfg(feature = "dev")]
fn champion_inactive_info_text() -> &'static str {
    "These champions already exist in the player's proficiency data but are not currently in the active save pool. They remain editable."
}

#[cfg(not(feature = "dev"))]
fn champion_inactive_info_text() -> &'static str {
    "These champions are not currently available in this save, but their mastery values can still be edited."
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppTab {
    Economy,
    PlayerEditor,
    StaffEditor,
    Recruitment,
    Search,
}

impl AppTab {
    const ALL: [Self; 5] = [
        Self::Economy,
        Self::PlayerEditor,
        Self::StaffEditor,
        Self::Recruitment,
        Self::Search,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Economy => "Economy",
            Self::PlayerEditor => "Player Editor",
            Self::StaffEditor => "Staff Editor",
            Self::Recruitment => "Recruitment",
            Self::Search => "Search",
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

    fn label(self) -> &'static str {
        match self {
            Self::Players => "Player Management",
            Self::Staff => "Staff Management",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchTab {
    Players,
    #[cfg(feature = "dev")]
    Staff,
    #[cfg(feature = "dev")]
    Teams,
    #[cfg(feature = "dev")]
    Lists,
    #[cfg(feature = "dev")]
    History,
}

impl SearchTab {
    #[cfg(feature = "dev")]
    const ALL: [Self; 5] = [
        Self::Players,
        Self::Staff,
        Self::Teams,
        Self::Lists,
        Self::History,
    ];

    #[cfg(not(feature = "dev"))]
    const ALL: [Self; 1] = [Self::Players];

    fn label(self) -> &'static str {
        match self {
            Self::Players => "Players",
            #[cfg(feature = "dev")]
            Self::Staff => "Staff",
            #[cfg(feature = "dev")]
            Self::Teams => "Teams",
            #[cfg(feature = "dev")]
            Self::Lists => "Lists",
            #[cfg(feature = "dev")]
            Self::History => "History",
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
    fn label(self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Id => "ID",
            Self::Age => "Age",
            Self::Team => "Team",
            Self::Position => "Position",
            Self::ActualRating => "Actual Rating",
            Self::PotentialRating => "Potential Rating",
            Self::ActualPotential => "Actual Potential",
            Self::Salary => "Salary",
            Self::ContractEnd => "Contract End",
            Self::LastHitting => "Last Hitting",
            Self::SkillshotDodging => "Skillshot Dodging",
            Self::SkillshotAccuracy => "Skillshot Accuracy",
            Self::InputSpeed => "Input Speed",
            Self::Positioning => "Positioning",
            Self::Judgment => "Judgment",
            Self::Mental => "Mental",
            Self::Focus => "Focus",
            Self::Calls => "Calls",
            Self::Roaming => "Roaming",
            Self::Aggression => "Aggression",
            Self::Ego => "Ego",
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
                AdvancedRangeFilter::new("age", "Age", ""),
                AdvancedRangeFilter::new("salary", "Salary", ""),
                AdvancedRangeFilter::new("transfer_fee", "Transfer Fee", ""),
                AdvancedRangeFilter::new("actual_rating", "Actual Rating", ""),
                AdvancedRangeFilter::new("last_hit", "Last Hitting", ""),
                AdvancedRangeFilter::new("skill_avoid", "Skillshot Dodging", ""),
                AdvancedRangeFilter::new("skill_hit", "Skillshot Accuracy", ""),
                AdvancedRangeFilter::new("control_speed", "Input Speed", ""),
                AdvancedRangeFilter::new("positioning", "Positioning", ""),
                AdvancedRangeFilter::new("judgement", "Judgment", ""),
                AdvancedRangeFilter::new("mental", "Mental", ""),
                AdvancedRangeFilter::new("concentration", "Focus", ""),
                AdvancedRangeFilter::new("order", "Calls", ""),
                AdvancedRangeFilter::new("roaming", "Roaming", ""),
                AdvancedRangeFilter::new("aggressive", "Aggression", ""),
                AdvancedRangeFilter::new("ego", "Ego", ""),
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
            format!("position_enabled={}", self.position_enabled),
            format!("position={}", self.position.replace('\n', " " ).replace('\r', " ")),
            format!("region_enabled={}", self.region_enabled),
            format!("region={}", self.region.replace('\n', " " ).replace('\r', " ")),
            format!("free_agents_only={}", self.free_agents_only),
        ];

        for range in &self.ranges {
            lines.push(format!("range.{}.enabled={}", range.key, range.enabled));
            lines.push(format!("range.{}.min={}", range.key, range.min.replace('\n', " " ).replace('\r', " ")));
            lines.push(format!("range.{}.max={}", range.key, range.max.replace('\n', " " ).replace('\r', " ")));
        }

        lines.join("\n") + "\n"
    }

    fn import_text(&mut self, text: &str) {
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
    }
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
}

impl StaffSummary {
    fn label(&self) -> String {
        format!(
            "{} · {} · {}",
            self.name,
            display_staff_role(&self.role),
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

#[derive(Debug, Clone)]
struct TeamSummary {
    id: usize,
    display_name: String,
    manager_name: String,
    league_id: usize,
    is_player_team: bool,
}

impl TeamSummary {
    fn label(&self) -> String {
        let name = if self.display_name.trim().is_empty() {
            format!("Team {}", self.id)
        } else {
            self.display_name.clone()
        };

        if self.is_player_team {
            format!("{name} · My Team · League {}", self.league_id)
        } else {
            format!("{name} · League {}", self.league_id)
        }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SquadStatusChoice {
    Core,
    Important,
    General,
    Sub,
}

impl Default for SquadStatusChoice {
    fn default() -> Self {
        Self::General
    }
}

impl SquadStatusChoice {
    const ALL: [Self; 4] = [Self::Core, Self::Important, Self::General, Self::Sub];

    fn internal(self) -> &'static str {
        match self {
            Self::Core => "Core",
            Self::Important => "Important",
            Self::General => "General",
            Self::Sub => "Sub",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Core => "Core Player",
            Self::Important => "Important Player",
            Self::General => "Starter",
            Self::Sub => "Substitute",
        }
    }

    fn from_internal(value: &str) -> Self {
        match value.trim() {
            "Core" => Self::Core,
            "Important" => Self::Important,
            "Sub" => Self::Sub,
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

    fn label(self) -> &'static str {
        match self {
            Self::Top => "Top",
            Self::Jungle => "Jungle",
            Self::Mid => "Mid",
            Self::Bottom => "Bottom",
            Self::Support => "Support",
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

fn selected_multi_filter_label<const N: usize>(
    empty_label: &str,
    labels: &[&str; N],
    selected: &[bool; N],
) -> String {
    let active = labels
        .iter()
        .zip(selected.iter())
        .filter_map(|(label, is_selected)| is_selected.then_some(*label))
        .collect::<Vec<_>>();

    match active.as_slice() {
        [] => empty_label.to_string(),
        [only] => (*only).to_string(),
        _ => format!("{} selected", active.len()),
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

fn render_actual_rating(ui: &mut egui::Ui, player: &PlayerSummary) {
    match effective_actual_rating(player) {
        Some((value, false)) => {
            #[cfg(feature = "dev")]
            {
                ui.label(format!("{value:.1}"))
                    .on_hover_text("TFM2 AthleteReport stat_score.");
            }
            #[cfg(not(feature = "dev"))]
            {
                ui.label(format!("{value:.1}"));
            }
        }
        Some((value, true)) => {
            let response = ui.label(format!("≈{value:.1}"));
            #[cfg(feature = "dev")]
            response.on_hover_text("Calculated fallback: the average of the 12 attributes.");
            #[cfg(not(feature = "dev"))]
            response.on_hover_text("Average of the 12 player attributes.");
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
        other if other.is_empty() => "Unknown".to_string(),
        other => other.to_string(),
    }
}

struct ModifierApp {
    active_tab: AppTab,
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
    bridge_version: String,
}

impl Default for ModifierApp {
    fn default() -> Self {
        let mut app = Self {
            active_tab: AppTab::Economy,
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
            status: "Starting…".to_string(),
            bridge_version: "-".to_string(),
        };
        app.reload_saved_filters();
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

impl ModifierApp {
    fn request(command: &str) -> Result<String, String> {
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

    fn refresh_connection(&mut self) {
        match Self::request("PING") {
            Ok(response) => {
                let parts: Vec<&str> = response.split('|').collect();
                if parts.len() >= 3 && parts[0] == "OK" && parts[1] == "PONG" {
                    self.connected = true;
                    self.bridge_version = parts[2].to_string();
                    self.status = "Connected to TFM2".to_string();
                } else {
                    self.connected = false;
                    self.status = format!("Unexpected bridge response: {response}");
                }
            }
            Err(error) => {
                self.connected = false;
                self.bridge_version = "-".to_string();
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

        self.economy.money = pretty_number(parts[2]);
        self.economy.transfer_budget = pretty_number(parts[3]);
        self.economy.salary_budget = pretty_number(parts[4]);
        Ok(())
    }

    fn refresh_economy(&mut self) {
        match Self::request("GET_ECONOMY") {
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
            parse_number(&self.economy.money),
            parse_number(&self.economy.transfer_budget),
            parse_number(&self.economy.salary_budget),
        ];

        if parsed.iter().any(Result::is_err) {
            self.status = "All economy fields must contain valid numbers".to_string();
            return;
        }

        let values: Vec<f64> = parsed.into_iter().map(Result::unwrap).collect();
        let command = format!(
            "SET_ECONOMY|{}|{}|{}",
            values[0], values[1], values[2]
        );

        match Self::request(&command) {
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
        match Self::request("GET_PLAYERS") {
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
                }
                Err(error) => self.status = human_error(&error),
            },
            Err(error) => {
                self.connected = false;
                self.status = error;
            }
        }
    }

    fn refresh_staff(&mut self) {
        match Self::request("GET_STAFFS") {
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
                }
                Err(error) => self.status = human_error(&error),
            },
            Err(error) => {
                self.connected = false;
                self.status = error;
            }
        }
    }

    fn refresh_selected_staff(&mut self) {
        let Some(id) = self.selected_staff_id else {
            self.selected_staff = None;
            return;
        };

        match Self::request(&format!("GET_STAFF|{id}")) {
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

        match Self::request(&command) {
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

        let Ok(annual_salary) = parse_number(&staff.annual_salary) else {
            self.status = "Salary must contain a valid number".to_string();
            return;
        };
        if annual_salary < 0.0 {
            self.status = "Salary cannot be negative".to_string();
            return;
        }

        let staff_id = staff.id;
        let staff_name = staff.name.clone();
        let command = format!("SET_STAFF_SALARY|{staff_id}|{annual_salary}");

        match Self::request(&command) {
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
        match Self::request(&format!("GET_CONTRACT_DEFAULTS|STAFF|{team_id}")) {
            Ok(response) => match parse_contract_defaults_response(&response) {
                Ok((start_date, end_date, annual_salary)) => {
                    self.staff_contract_form = ContractEditorForm {
                        team_id: Some(team_id),
                        start_date,
                        end_date,
                        annual_salary,
                        transfer_fee: "0".to_string(),
                        league_rank: "1".to_string(),
                        pog_bonus: "0".to_string(),
                        league_bonus: "0".to_string(),
                        match_bonus: "0".to_string(),
                        win_bonus: "0".to_string(),
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
                "0".to_string()
            } else {
                staff.annual_salary.clone()
            },
            transfer_fee: "0".to_string(),
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
        let Ok(annual_salary) = parse_number(&self.staff_contract_form.annual_salary) else {
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
            "SET_STAFF_CONTRACT|{staff_id}|{team_id}|{start_date}|{end_date}|{annual_salary}"
        );
        match Self::request(&command) {
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
        match Self::request(&command) {
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
        match Self::request(&command) {
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

        match Self::request(&command) {
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

        match Self::request(&format!("GET_PLAYER|{id}")) {
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

        match Self::request(&format!("GET_CHAMPION_MASTERY_PROBE|{athlete_id}")) {
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

        match Self::request(&command) {
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
                egui::Stroke::new(
                    1.0_f32,
                    egui::Color32::from_gray(82),
                )
            } else {
                egui::Stroke::new(
                    1.0_f32,
                    egui::Color32::from_gray(170),
                )
            }
        } else {
            ui.visuals().widgets.noninteractive.bg_stroke
        };

        let response = egui::Frame::group(ui.style())
            .fill(fill)
            .stroke(stroke)
            .show(ui, |ui| {
                // Match the compact v0.2.24 cards.
                ui.set_min_width(125.0);
                ui.set_max_width(145.0);

                ui.horizontal(|ui| {
                    ui.checkbox(&mut champion.selected, "");
                    ui.label(
                        egui::RichText::new(&champion.display_name).strong(),
                    );
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

                // Keep card height constant while values are edited in bulk.
                // Current/pending values remain available in the hover tooltip.
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
                "{state}\nID: {}\nCurrent mastery: {}\nPending mastery: {}\nRaw value: {}\nRaw floor: {}",
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
            .map(|player| player.name.as_str())
            .unwrap_or("Selected Player");

        egui::Window::new(format!("Champion Mastery — {player_name}"))
            .id(egui::Id::new("champion_mastery_grid_v030"))
            .open(&mut open)
            .resizable(true)
            .default_size(egui::vec2(1080.0, 720.0))
            .show(ctx, |ui| {
                let active_count = self
                    .champion_mastery_entries
                    .iter()
                    .filter(|entry| entry.active)
                    .count();
                let inactive_count =
                    self.champion_mastery_entries.len() - active_count;
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

                    if ui.button("Check Active").clicked() {
                        for entry in &mut self.champion_mastery_entries {
                            entry.selected = entry.active;
                        }
                    }

                    if ui.button("Check Inactive").clicked() {
                        for entry in &mut self.champion_mastery_entries {
                            entry.selected = !entry.active;
                        }
                    }

                    if ui.button("Check All").clicked() {
                        for entry in &mut self.champion_mastery_entries {
                            entry.selected = true;
                        }
                    }

                    if ui.button("Clear Checks").clicked() {
                        for entry in &mut self.champion_mastery_entries {
                            entry.selected = false;
                        }
                    }

                    if ui.button("Refresh").clicked() {
                        refresh_requested = true;
                    }
                });

                ui.add_space(6.0);

                ui.horizontal_wrapped(|ui| {
                    ui.label("Bulk Mastery");
                    ui.add(
                        egui::Slider::new(
                            &mut self.champion_mastery_bulk_value,
                            0..=100,
                        )
                        .show_value(true),
                    );

                    if ui
                        .add_enabled(
                            selected_count > 0,
                            egui::Button::new("Set Checked"),
                        )
                        .clicked()
                    {
                        for entry in &mut self.champion_mastery_entries {
                            if entry.selected {
                                entry.edit_mastery =
                                    self.champion_mastery_bulk_value;
                            }
                        }
                    }

                    ui.separator();

                    if ui
                        .add_enabled(
                            self.connected && selected_count > 0,
                            egui::Button::new("Apply Selected"),
                        )
                        .clicked()
                    {
                        apply_requested = true;
                    }
                });

                ui.add_space(6.0);
                ui.weak(champion_mastery_help_text());

                ui.add_space(8.0);
                ui.separator();

                // Measure the Champion Mastery window's own content width here,
                // BEFORE entering ScrollArea. This avoids both the main-window
                // viewport bug and ScrollArea desired-size feedback.
                let local_width = ui.available_width().max(180.0);

                // A v0.2.24 card is ~145 px wide plus Frame margins/spacing.
                // Use a conservative 165 px slot so rows never request more
                // width than the current Champion Mastery window.
                let slot_width = 165.0_f32;
                let cards_per_row =
                    (local_width / slot_width).floor().max(1.0) as usize;

                egui::ScrollArea::vertical()
                    .id_salt("champion_mastery_grid_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.heading("Active Champions");
                        ui.label(
                            "Currently available in this save. Highlighted for quick identification.",
                        );
                        ui.add_space(4.0);

                        let active_indices = self
                            .champion_mastery_entries
                            .iter()
                            .enumerate()
                            .filter_map(|(index, entry)| entry.active.then_some(index))
                            .collect::<Vec<_>>();

                        for row in active_indices.chunks(cards_per_row) {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 10.0;

                                for &index in row {
                                    let champion =
                                        &mut self.champion_mastery_entries[index];
                                    Self::render_champion_mastery_card(
                                        ui,
                                        champion,
                                        true,
                                    );
                                }
                            });
                            ui.add_space(10.0);
                        }

                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(8.0);

                        ui.heading(
                            "Inactive / Not Yet Available in This Save",
                        );
                        ui.label(champion_inactive_info_text());
                        ui.add_space(4.0);

                        let inactive_indices = self
                            .champion_mastery_entries
                            .iter()
                            .enumerate()
                            .filter_map(|(index, entry)| (!entry.active).then_some(index))
                            .collect::<Vec<_>>();

                        for row in inactive_indices.chunks(cards_per_row) {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 10.0;

                                for &index in row {
                                    let champion =
                                        &mut self.champion_mastery_entries[index];
                                    Self::render_champion_mastery_card(
                                        ui,
                                        champion,
                                        false,
                                    );
                                }
                            });
                            ui.add_space(10.0);
                        }
                    });
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
            .unwrap_or_else(|| "No player selected".to_string());
        let teams = self.teams.clone();

        egui::Window::new("Edit Player Contract")
            .id(egui::Id::new("edit_player_contract_v039"))
            .open(&mut open)
            .resizable(true)
            .collapsible(false)
            .default_width(590.0)
            .show(ctx, |ui| {
                ui.strong(target);
                let active_team_id = self.selected_player.as_ref().and_then(|player| player.contract_team_id);
                if self.player_contract_mode == ContractEditorMode::MoveFreeAgent {
                    ui.weak("Mode: Create an active contract and move this free-agent player to the selected team.");
                } else {
                    match (active_team_id, self.player_contract_form.team_id) {
                        (None, Some(_)) => ui.weak("Mode: Create an active contract for this free-agent player."),
                        (Some(current), Some(selected)) if current != selected => ui.weak("Mode: Move the player and apply the edited active contract."),
                        (Some(_), _) => ui.weak("Mode: Edit the current active contract."),
                        _ => ui.weak("Select a team before applying the contract."),
                    };
                }
                ui.add_space(8.0);

                let selected_team = self
                    .player_contract_form
                    .team_id
                    .and_then(|id| teams.iter().find(|team| team.id == id))
                    .map(TeamSummary::label)
                    .unwrap_or_else(|| "Select team...".to_string());

                egui::Grid::new("edit_player_contract_main_grid")
                    .num_columns(2)
                    .spacing([22.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Team");
                        egui::ComboBox::from_id_salt("edit_player_contract_team")
                            .selected_text(selected_team)
                            .width(350.0)
                            .height(300.0)
                            .show_ui(ui, |ui| {
                                for team in &teams {
                                    ui.selectable_value(
                                        &mut self.player_contract_form.team_id,
                                        Some(team.id),
                                        team.label(),
                                    );
                                }
                            });
                        ui.end_row();

                        ui.label("Contract Start");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.player_contract_form.start_date)
                                .desired_width(180.0)
                                .hint_text("YYYY-MM-DD"),
                        );
                        ui.end_row();

                        ui.label("Contract End");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.player_contract_form.end_date)
                                .desired_width(180.0)
                                .hint_text("YYYY-MM-DD"),
                        );
                        ui.end_row();

                        ui.label("Annual Salary");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.player_contract_form.annual_salary)
                                .desired_width(180.0),
                        );
                        ui.end_row();

                        ui.label("Transfer Fee");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.player_contract_form.transfer_fee)
                                .desired_width(180.0),
                        );
                        ui.end_row();

                        ui.label("Squad Status");
                        egui::ComboBox::from_id_salt("edit_player_contract_squad_status")
                            .selected_text(self.player_contract_form.squad_status.label())
                            .width(220.0)
                            .show_ui(ui, |ui| {
                                for status in SquadStatusChoice::ALL {
                                    ui.selectable_value(
                                        &mut self.player_contract_form.squad_status,
                                        status,
                                        status.label(),
                                    );
                                }
                            });
                        ui.end_row();
                    });

                ui.add_space(10.0);
                ui.separator();
                ui.strong("Active Contract Bonuses");
                ui.add_space(5.0);

                egui::Grid::new("edit_player_contract_bonus_grid")
                    .num_columns(3)
                    .spacing([14.0, 7.0])
                    .show(ui, |ui| {
                        ui.checkbox(&mut self.player_contract_form.pog_enabled, "POG Award Bonus");
                        ui.add_enabled(
                            self.player_contract_form.pog_enabled,
                            egui::TextEdit::singleline(&mut self.player_contract_form.pog_bonus)
                                .desired_width(150.0),
                        );
                        ui.weak("Amount");
                        ui.end_row();

                        ui.checkbox(&mut self.player_contract_form.league_enabled, "League Rank Bonus");
                        ui.add_enabled(
                            self.player_contract_form.league_enabled,
                            egui::TextEdit::singleline(&mut self.player_contract_form.league_bonus)
                                .desired_width(150.0),
                        );
                        ui.horizontal(|ui| {
                            ui.weak("Rank");
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

                        ui.checkbox(&mut self.player_contract_form.match_enabled, "Match Appearance Bonus");
                        ui.add_enabled(
                            self.player_contract_form.match_enabled,
                            egui::TextEdit::singleline(&mut self.player_contract_form.match_bonus)
                                .desired_width(150.0),
                        );
                        ui.weak("Amount");
                        ui.end_row();

                        ui.checkbox(&mut self.player_contract_form.win_enabled, "Match Win Bonus");
                        ui.add_enabled(
                            self.player_contract_form.win_enabled,
                            egui::TextEdit::singleline(&mut self.player_contract_form.win_bonus)
                                .desired_width(150.0),
                        );
                        ui.weak("Amount");
                        ui.end_row();
                    });

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    let apply_label = if self.player_contract_mode == ContractEditorMode::MoveFreeAgent {
                        "Apply Contract & Move Player"
                    } else {
                        "Apply Contract"
                    };
                    if ui
                        .add_enabled(self.connected, egui::Button::new(apply_label))
                        .clicked()
                    {
                        apply_requested = true;
                    }
                    let reset_help = if self.player_contract_mode == ContractEditorMode::MoveFreeAgent {
                        "Restore the automatic free-agent contract defaults and keep the selected destination team."
                    } else {
                        "Reload the current live contract values."
                    };
                    if ui.button("Reset").on_hover_text(reset_help).clicked() {
                        reset_requested = true;
                    }
                    #[cfg(feature = "dev")]
                    if ui
                        .add_enabled(self.connected, egui::Button::new("Capture Contract Flow"))
                        .clicked()
                    {
                        self.load_player_contract_probe();
                    }
                    if ui.button("Cancel").clicked() {
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
            .map(|staff| format!("{} · {} · ID {}", staff.name, display_staff_role(&staff.role), staff.id))
            .unwrap_or_else(|| "No staff selected".to_string());
        let teams = self.teams.clone();

        egui::Window::new("Edit Staff Contract")
            .id(egui::Id::new("edit_staff_contract_v039"))
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .default_width(520.0)
            .show(ctx, |ui| {
                ui.strong(target);
                let active_team_id = self.selected_staff.as_ref().and_then(|staff| staff.contract_team_id);
                if self.staff_contract_mode == ContractEditorMode::MoveFreeAgent {
                    ui.weak("Mode: Create an active contract and move this free-agent staff member to the selected team.");
                } else {
                    match (active_team_id, self.staff_contract_form.team_id) {
                        (None, Some(_)) => ui.weak("Mode: Create an active contract for this free-agent staff member."),
                        (Some(current), Some(selected)) if current != selected => ui.weak("Mode: Move staff to another team and apply the edited active contract. Old recruitment requests are cleared when the team changes."),
                        (Some(_), _) => ui.weak("Mode: Edit the current active staff contract in place while preserving existing contract data."),
                        _ => ui.weak("Select a team before applying the active contract."),
                    };
                }
                ui.add_space(8.0);

                let selected_team = self
                    .staff_contract_form
                    .team_id
                    .and_then(|id| teams.iter().find(|team| team.id == id))
                    .map(TeamSummary::label)
                    .unwrap_or_else(|| "Select team...".to_string());

                egui::Grid::new("staff_contract_builder_grid")
                    .num_columns(2)
                    .spacing([22.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Team");
                        egui::ComboBox::from_id_salt("staff_contract_team")
                            .selected_text(selected_team)
                            .width(330.0)
                            .height(300.0)
                            .show_ui(ui, |ui| {
                                for team in &teams {
                                    ui.selectable_value(
                                        &mut self.staff_contract_form.team_id,
                                        Some(team.id),
                                        team.label(),
                                    );
                                }
                            });
                        ui.end_row();

                        ui.label("Start Date");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.staff_contract_form.start_date)
                                .desired_width(180.0)
                                .hint_text("YYYY-MM-DD"),
                        );
                        ui.end_row();

                        ui.label("End Date");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.staff_contract_form.end_date)
                                .desired_width(180.0)
                                .hint_text("YYYY-MM-DD"),
                        );
                        ui.end_row();

                        ui.label("Annual Salary");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.staff_contract_form.annual_salary)
                                .desired_width(180.0),
                        );
                        ui.end_row();
                    });

                ui.add_space(8.0);
                ui.weak("This edits the active staff contract directly.");
                ui.horizontal(|ui| {
                    let apply_label = if self.staff_contract_mode == ContractEditorMode::MoveFreeAgent {
                        "Apply Contract & Move Staff"
                    } else {
                        "Apply Contract"
                    };
                    if ui
                        .add_enabled(self.connected, egui::Button::new(apply_label))
                        .clicked()
                    {
                        apply_requested = true;
                    }
                    let reset_help = if self.staff_contract_mode == ContractEditorMode::MoveFreeAgent {
                        "Restore the automatic free-agent contract defaults and keep the selected destination team."
                    } else {
                        "Reload the current live contract values."
                    };
                    if ui.button("Reset").on_hover_text(reset_help).clicked() {
                        reset_requested = true;
                    }
                    #[cfg(feature = "dev")]
                    if ui
                        .add_enabled(self.connected, egui::Button::new("Capture Contract Flow"))
                        .clicked()
                    {
                        self.load_staff_contract_probe();
                    }
                    if ui.button("Cancel").clicked() {
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
                    if ui.button("Clear All").clicked() {
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
                    if ui.button("Clear All").clicked() {
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

        match Self::request(&command) {
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

        match Self::request(&command) {
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

        match Self::request(&command) {
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

        let Ok(annual_salary) = parse_number(&player.annual_salary) else {
            self.status = "Salary must contain a valid number".to_string();
            return;
        };
        if annual_salary < 0.0 {
            self.status = "Salary cannot be negative".to_string();
            return;
        }

        let command = format!("SET_PLAYER_SALARY|{}|{}", player.id, annual_salary);
        match Self::request(&command) {
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
        match Self::request(&format!("GET_CONTRACT_DEFAULTS|PLAYER|{team_id}")) {
            Ok(response) => match parse_contract_defaults_response(&response) {
                Ok((start_date, end_date, annual_salary)) => {
                    self.player_contract_form = ContractEditorForm {
                        team_id: Some(team_id),
                        start_date,
                        end_date,
                        annual_salary,
                        transfer_fee: "0".to_string(),
                        squad_status: SquadStatusChoice::General,
                        pog_enabled: false,
                        pog_bonus: "0".to_string(),
                        league_enabled: false,
                        league_bonus: "0".to_string(),
                        league_rank: "1".to_string(),
                        match_enabled: false,
                        match_bonus: "0".to_string(),
                        win_enabled: false,
                        win_bonus: "0".to_string(),
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
                "0".to_string()
            } else {
                player.annual_salary.clone()
            },
            transfer_fee: if player.transfer_fee.trim().is_empty() {
                "0".to_string()
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
        let Ok(annual_salary) = parse_number(&self.player_contract_form.annual_salary) else {
            self.status = "Salary must contain a valid number".to_string();
            return;
        };
        let Ok(transfer_fee) = parse_number(&self.player_contract_form.transfer_fee) else {
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
            let value = parse_number(raw).map_err(|_| format!("{label} must contain a valid number"))?;
            if value < 0.0 {
                return Err(format!("{label} cannot be negative"));
            }
            Ok(value)
        };
        let pog_bonus = match bonus_value(self.player_contract_form.pog_enabled, &self.player_contract_form.pog_bonus, "POG Award Bonus") {
            Ok(value) => value,
            Err(error) => { self.status = error; return; }
        };
        let league_bonus = match bonus_value(self.player_contract_form.league_enabled, &self.player_contract_form.league_bonus, "League Rank Bonus") {
            Ok(value) => value,
            Err(error) => { self.status = error; return; }
        };
        let match_bonus = match bonus_value(self.player_contract_form.match_enabled, &self.player_contract_form.match_bonus, "Match Appearance Bonus") {
            Ok(value) => value,
            Err(error) => { self.status = error; return; }
        };
        let win_bonus = match bonus_value(self.player_contract_form.win_enabled, &self.player_contract_form.win_bonus, "Match Win Bonus") {
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
            "SET_PLAYER_CONTRACT|{athlete_id}|{team_id}|{start_date}|{end_date}|{annual_salary}|{transfer_fee}|{}|{}|{pog_bonus}|{}|{league_bonus}|{league_rank}|{}|{match_bonus}|{}|{win_bonus}",
            self.player_contract_form.squad_status.internal(),
            bool_digit(self.player_contract_form.pog_enabled),
            bool_digit(self.player_contract_form.league_enabled),
            bool_digit(self.player_contract_form.match_enabled),
            bool_digit(self.player_contract_form.win_enabled),
        );
        match Self::request(&command) {
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
        match Self::request(&command) {
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
        ui.heading("Economy");
        ui.label("Edit the active team's economy. Apply now writes both the client snapshot and authoritative server state.");
        ui.add_space(8.0);

        egui::Grid::new("economy_grid")
            .num_columns(2)
            .spacing([16.0, 8.0])
            .show(ui, |ui| {
                ui.label("Money");
                ui.text_edit_singleline(&mut self.economy.money);
                ui.end_row();

                ui.label("Transfer Budget");
                ui.text_edit_singleline(&mut self.economy.transfer_budget);
                ui.end_row();

                ui.label("Salary Budget");
                ui.text_edit_singleline(&mut self.economy.salary_budget);
                ui.end_row();
            });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(self.connected, egui::Button::new("Refresh"))
                .clicked()
            {
                self.refresh_economy();
            }

            if ui
                .add_enabled(self.connected, egui::Button::new("Apply Economy"))
                .clicked()
            {
                self.apply_economy();
            }

            if ui
                .add_enabled(self.connected, egui::Button::new("Set all to 1.2T"))
                .clicked()
            {
                let value = "1200000000000".to_string();
                self.economy.money = value.clone();
                self.economy.transfer_budget = value.clone();
                self.economy.salary_budget = value;
                self.apply_economy();
            }
        });
    }

    fn render_player_editor_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Player Editor");
        ui.label(player_editor_intro_text());
        ui.weak("Recommended: Save your career before making changes with the editor.");
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.label("Search");
            ui.add(
                egui::TextEdit::singleline(&mut self.player_search)
                    .hint_text("Type player name...")
                    .desired_width(250.0),
            );

            if ui
                .add_enabled(!self.player_search.is_empty(), egui::Button::new("Clear"))
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

        ui.horizontal(|ui| {
            ui.label("Player");

            let selected_text = self
                .selected_player_id
                .and_then(|id| self.players.iter().find(|player| player.id == id))
                .map(|player| player.name.clone())
                .unwrap_or_else(|| "Select player".to_string());

            let before = self.selected_player_id;
            ui.add_enabled_ui(self.connected && !self.players.is_empty(), |ui| {
                egui::ComboBox::from_id_salt("player_select")
                    .selected_text(selected_text)
                    .width(250.0)
                    .show_ui(ui, |ui| {
                        let mut shown = 0usize;
                        for player in &self.players {
                            if !search.is_empty() && !player.name.to_lowercase().contains(&search) {
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
                            ui.label("No matching players");
                        }
                    });
            });

            ui.label(format!("{match_count} / {}", self.players.len()));

            if self.selected_player_id != before {
                self.refresh_selected_player();
            }

            if ui
                .add_enabled(self.connected, egui::Button::new("Refresh Players"))
                .clicked()
            {
                self.refresh_players();
            }

            if ui
                .add_enabled(
                    self.connected && self.selected_player_id.is_some(),
                    egui::Button::new("Refresh Selected"),
                )
                .clicked()
            {
                self.refresh_selected_player();
            }
        });

        ui.add_space(8.0);
        let mut apply_player_clicked = false;
        let mut max_all_clicked = false;
        let mut apply_positions_clicked = false;
        let mut apply_potential_clicked = false;
        let mut apply_salary_clicked = false;
        let mut open_contract_clicked = false;

        if let Some(player) = self.selected_player.as_mut() {
            ui.label(format!("{}  ·  ID {}", player.name, player.id));
            ui.label("Attributes: 1–100");
            ui.add_space(4.0);

            egui::Grid::new("player_stats_grid")
                .num_columns(4)
                .spacing([18.0, 7.0])
                .striped(true)
                .show(ui, |ui| {
                    stat_edit_cell(ui, "Last Hitting", &mut player.last_hit);
                    stat_edit_cell(ui, "Skillshot Dodging", &mut player.skill_avoid);
                    ui.end_row();

                    stat_edit_cell(ui, "Skillshot Accuracy", &mut player.skill_hit);
                    stat_edit_cell(ui, "Input Speed", &mut player.control_speed);
                    ui.end_row();

                    stat_edit_cell(ui, "Positioning", &mut player.positioning);
                    stat_edit_cell(ui, "Judgment", &mut player.judgement);
                    ui.end_row();

                    stat_edit_cell(ui, "Mental", &mut player.mental);
                    stat_edit_cell(ui, "Focus", &mut player.concentration);
                    ui.end_row();

                    stat_edit_cell(ui, "Calls", &mut player.order);
                    stat_edit_cell(ui, "Roaming", &mut player.roaming);
                    ui.end_row();

                    stat_edit_cell(ui, "Aggression", &mut player.aggressive);
                    stat_edit_cell(ui, "Ego", &mut player.ego);
                    ui.end_row();
                });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(self.connected, egui::Button::new("Apply Attributes"))
                    .clicked()
                {
                    apply_player_clicked = true;
                }

                if ui
                    .add_enabled(self.connected, egui::Button::new("Max All"))
                    .clicked()
                {
                    max_all_clicked = true;
                }
            });

            ui.add_space(12.0);
            ui.separator();
            ui.heading("Positions");
            ui.label("Up to three active positions are supported.");

            let mut clear_all_positions_clicked = false;
            if let Some(positions) = self.player_positions.as_mut() {
                let slot_labels = ["Primary", "Secondary", "Tertiary"];
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
                                        .map(PositionChoice::label)
                                        .unwrap_or("None"),
                                )
                                .width(130.0)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut slot.position, None, "None");
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
                                                position.label(),
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
                                ui.add_enabled(false, egui::Button::new("—").min_size(egui::vec2(150.0, 0.0)));
                            }
                            ui.end_row();
                        }
                    });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(self.connected, egui::Button::new("Apply Positions"))
                        .clicked()
                    {
                        apply_positions_clicked = true;
                    }
                    if ui
                        .add_enabled(self.connected, egui::Button::new("Clear All"))
                        .clicked()
                    {
                        clear_all_positions_clicked = true;
                    }
                });
            }

            if clear_all_positions_clicked {
                if let Some(positions) = self.player_positions.as_mut() {
                    positions.clear_all();
                    self.status = "All positions set to None. Click Apply Positions to save.".to_string();
                }
            }
            ui.add_space(12.0);
            ui.separator();
            ui.heading("Potential");
            ui.label(potential_info_text());
            if let Some(potential) = self.player_potential.as_mut() {
                egui::Grid::new("player_potential_grid")
                    .num_columns(2)
                    .spacing([18.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Potential Grade");

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

                        ui.label("Actual Potential");
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

                        ui.label("Current Value");
                        if potential.current_raw == potential.edit_raw {
                            ui.label(potential.current_raw.to_string());
                        } else {
                            ui.label(format!(
                                "{}  →  {}",
                                potential.current_raw,
                                potential.edit_raw
                            ));
                        }
                        ui.end_row();
                    });

                ui.add_space(4.0);
                ui.weak(
                    "Grade presets: Very Low 1 · Low 30 · Normal 50 · High 70 · Very High 100",
                );

                ui.add_space(6.0);
                if ui
                    .add_enabled(self.connected, egui::Button::new("Apply Potential"))
                    .clicked()
                {
                    apply_potential_clicked = true;
                }
            }

            ui.add_space(12.0);
            ui.separator();
            ui.heading("Contract & Finance");
            ui.label(salary_info_text());
            if player.annual_salary.trim().is_empty() {
                ui.label("Free Agent / no active contract");
            } else {
                egui::Grid::new("player_contract_finance_grid")
                    .num_columns(2)
                    .spacing([18.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Team");
                        let team_label = player
                            .contract_team_id
                            .and_then(|id| self.teams.iter().find(|team| team.id == id))
                            .map(|team| team.display_name.clone())
                            .filter(|name| !name.trim().is_empty())
                            .or_else(|| player.contract_team_id.map(|id| format!("Team {id}")))
                            .unwrap_or_else(|| "—".to_string());
                        ui.strong(team_label);
                        ui.end_row();

                        ui.label("Contract Start");
                        ui.strong(display_contract_date(&player.contract_start_date));
                        ui.end_row();

                        ui.label("Contract End");
                        ui.strong(display_contract_date(&player.contract_end_date));
                        ui.end_row();

                        ui.label("Annual Salary");
                        ui.add(
                            egui::TextEdit::singleline(&mut player.annual_salary)
                                .desired_width(180.0),
                        );
                        ui.end_row();

                        ui.label("Weekly Salary");
                        ui.strong(pretty_or_dash(&player.weekly_salary));
                        ui.end_row();

                        ui.label("Transfer Fee");
                        ui.strong(pretty_or_dash(&player.transfer_fee));
                        ui.end_row();

                        ui.label("Squad Status");
                        let status = SquadStatusChoice::from_internal(&player.squad_status);
                        ui.strong(status.label());
                        ui.end_row();

                        ui.label("POG Award Bonus");
                        ui.strong(contract_bonus_display(&player.incentive_pog_bonus));
                        ui.end_row();

                        ui.label("League Rank Bonus");
                        ui.strong(if player.incentive_league_bonus.trim().is_empty() {
                            "Disabled".to_string()
                        } else {
                            format!("{} · Rank {}", pretty_number(&player.incentive_league_bonus), pretty_or_dash(&player.incentive_league_rank))
                        });
                        ui.end_row();

                        ui.label("Match Appearance Bonus");
                        ui.strong(contract_bonus_display(&player.incentive_match_bonus));
                        ui.end_row();

                        ui.label("Match Win Bonus");
                        ui.strong(contract_bonus_display(&player.incentive_win_bonus));
                        ui.end_row();
                    });
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        self.connected && !player.annual_salary.trim().is_empty(),
                        egui::Button::new("Apply Salary"),
                    )
                    .clicked()
                {
                    apply_salary_clicked = true;
                }
                if ui
                    .add_enabled(self.connected, egui::Button::new("Edit Contract"))
                    .on_hover_text("Create and apply a complete finalized contract.")
                    .clicked()
                {
                    open_contract_clicked = true;
                }
            });
            ui.weak("Edit Contract changes the active contract, bonuses, squad status, and transfer fee.");
            ui.add_space(12.0);
            ui.separator();
            ui.heading("Champion Mastery");

            ui.label(
                "Edit individual Champion Mastery values or apply changes to multiple champions at once.",
            );
            if ui
                .add_enabled(
                    self.connected && self.selected_player_id.is_some(),
                    egui::Button::new("Open Champion Mastery"),
                )
                .clicked()
            {
                self.load_champion_mastery();
            }

            self.render_communication_section(ui);
        } else {
            ui.label("No player data loaded.");
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


    fn render_staff_editor_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Staff Editor");
        ui.label("Edit staff attributes, active contracts, salary, and Communication.");
        ui.add_space(8.0);

        let mut refresh_staff_clicked = false;
        let mut selection_changed = false;
        let mut apply_staff_clicked = false;
        let mut max_all_clicked = false;
        let mut apply_salary_clicked = false;
        let mut open_contract_clicked = false;
        let mut apply_communication_clicked = false;
        let mut max_communication_clicked = false;

        ui.horizontal(|ui| {
            ui.label("Search");
            ui.add(
                egui::TextEdit::singleline(&mut self.staff_search)
                    .hint_text("Type staff name...")
                    .desired_width(250.0),
            );

            if ui
                .add_enabled(!self.staff_search.is_empty(), egui::Button::new("Clear"))
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
            .map(|staff| (staff.id, staff.label()))
            .collect::<Vec<_>>();
        let match_count = filtered_staff.len();

        ui.horizontal(|ui| {
            ui.label("Staff");
            let selected_label = self
                .selected_staff_id
                .and_then(|id| self.staffs.iter().find(|staff| staff.id == id))
                .map(StaffSummary::label)
                .unwrap_or_else(|| "Select staff...".to_string());

            ui.add_enabled_ui(self.connected && !self.staffs.is_empty(), |ui| {
                egui::ComboBox::from_id_salt("staff_editor_select")
                    .selected_text(selected_label)
                    .width(420.0)
                    .height(320.0)
                    .show_ui(ui, |ui| {
                        ui.set_min_width(420.0);

                        if filtered_staff.is_empty() {
                            ui.label("No matching staff");
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
                .add_enabled(self.connected, egui::Button::new("Refresh Staff"))
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
                    ui.label("Name");
                    ui.strong(&staff.name);
                    ui.label("ID");
                    ui.strong(staff.id.to_string());
                    ui.end_row();

                    ui.label("Role");
                    ui.strong(display_staff_role(&staff.role));
                    ui.label("Age");
                    ui.strong(&staff.age);
                    ui.end_row();

                    ui.label("Team");
                    ui.strong(&staff.team);
                    ui.end_row();
                });

            ui.add_space(12.0);
            ui.heading("Attributes");
            ui.label("Staff attributes use the same 1–100 edit range as player attributes.");

            egui::Grid::new("staff_attributes_grid")
                .num_columns(4)
                .spacing([20.0, 5.0])
                .show(ui, |ui| {
                    stat_edit_cell(ui, "Ban/Pick", &mut staff.banpick);
                    stat_edit_cell(ui, "Strategy", &mut staff.strategy);
                    ui.end_row();

                    stat_edit_cell(ui, "Negotiation", &mut staff.negotiation);
                    stat_edit_cell(ui, "Ability Analysis", &mut staff.judge_ability);
                    ui.end_row();

                    stat_edit_cell(ui, "Potential Analysis", &mut staff.judge_potential);
                    stat_edit_cell(ui, "Feedback", &mut staff.feedback);
                    ui.end_row();

                    stat_edit_cell(ui, "Power Analysis", &mut staff.power_analysis);
                    stat_edit_cell(ui, "Control Coaching", &mut staff.control_coaching);
                    ui.end_row();

                    stat_edit_cell(ui, "Judgment Coaching", &mut staff.judgment_coaching);
                    stat_edit_cell(ui, "Mental Coaching", &mut staff.mental_coaching);
                    ui.end_row();
                });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(self.connected, egui::Button::new("Apply Attributes"))
                    .clicked()
                {
                    apply_staff_clicked = true;
                }

                if ui
                    .add_enabled(self.connected, egui::Button::new("Max All"))
                    .clicked()
                {
                    max_all_clicked = true;
                }
            });

            ui.add_space(12.0);
            ui.separator();
            ui.heading("Contract & Finance");
            ui.label("Salary fields use the amounts stored in the active career.");

            if staff.annual_salary.trim().is_empty() {
                ui.label("Free Agent / no active contract");
            } else {
                egui::Grid::new("staff_contract_grid")
                    .num_columns(2)
                    .spacing([24.0, 7.0])
                    .show(ui, |ui| {
                        ui.label("Annual Salary");
                        ui.add(
                            egui::TextEdit::singleline(&mut staff.annual_salary)
                                .desired_width(180.0),
                        );
                        ui.end_row();

                        ui.label("Contract Start");
                        ui.strong(display_contract_date(&staff.contract_start_date));
                        ui.end_row();

                        ui.label("Contract End");
                        ui.strong(display_contract_date(&staff.contract_end_date));
                        ui.end_row();
                    });
            }

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        self.connected && !staff.annual_salary.trim().is_empty(),
                        egui::Button::new("Apply Salary"),
                    )
                    .clicked()
                {
                    apply_salary_clicked = true;
                }
                if ui
                    .add_enabled(self.connected, egui::Button::new("Edit Contract"))
                    .on_hover_text("Create and apply a complete finalized staff contract.")
                    .clicked()
                {
                    open_contract_clicked = true;
                }
            });
            ui.weak("Edit Contract changes the active staff contract.");

            ui.add_space(12.0);
            ui.separator();
            ui.heading("Communication");
            ui.label("Select a league region and set its direct 0–100 Communication value. Applying a region that is not stored yet creates it for the staff member.");

            let previous_region = self.staff_communication_region_id;
            egui::Grid::new("staff_communication_editor_grid")
                .num_columns(2)
                .spacing([24.0, 7.0])
                .show(ui, |ui| {
                    ui.label("Region");
                    egui::ComboBox::from_id_salt("staff_communication_region_select")
                        .selected_text(staff_communication_region_label(
                            self.staff_communication_region_id,
                        ))
                        .width(240.0)
                        .show_ui(ui, |ui| {
                            for (region_id, _) in COMMUNICATION_REGIONS {
                                ui.selectable_value(
                                    &mut self.staff_communication_region_id,
                                    region_id,
                                    staff_communication_region_label(region_id),
                                );
                            }
                        });
                    ui.end_row();

                    ui.label("Communication");
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
            if selected_region_is_stored {
                ui.weak("This region is already stored for the selected staff member.");
            } else {
                ui.weak("This region is not stored yet. Apply Communication will add it.");
            }

            if !staff.communication.is_empty() {
                let stored_regions = staff
                    .communication
                    .iter()
                    .map(|entry| {
                        format!(
                            "{}: {}",
                            staff_communication_region_label(entry.region_id),
                            entry.value
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" · ");
                ui.weak(format!("Stored regions: {stored_regions}"));
            }

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(self.connected, egui::Button::new("Apply Communication"))
                    .clicked()
                {
                    apply_communication_clicked = true;
                }
                if ui
                    .add_enabled(self.connected, egui::Button::new("Max Selected"))
                    .clicked()
                {
                    max_communication_clicked = true;
                }
            });

            ui.add_space(12.0);
            ui.separator();
            ui.weak("Potential Analysis measures how well this staff member evaluates player potential; it is not the staff member's own potential.");
        } else if self.staffs.is_empty() {
            ui.label("No staff data loaded. Load a career and click Refresh Staff.");
        } else {
            ui.label("Select a staff member.");
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
                "{} set to 100. Click Apply Communication to save.",
                staff_communication_region_label(self.staff_communication_region_id)
            );
        }

        if apply_staff_clicked {
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
        ui.heading("Communication Level");
        ui.label("Player Communication has two separate values in TFM2: the actual 0–100 proficiency used by the player profile, and pending training XP stored for weekly progression.");

        let mut apply_clicked = false;
        let mut max_clicked = false;

        if let Some(communication) = self.player_communication.as_ref() {
            if let Some(primary_region) = communication.primary_region {
                ui.horizontal(|ui| {
                    ui.label("Native region");
                    ui.strong(player_communication_region_label(primary_region));
                });
            } else {
                ui.weak("Native region could not be resolved for this player.");
            }

            let previous_region = self.player_communication_region_id;
            egui::Grid::new("player_communication_editor_grid")
                .num_columns(2)
                .spacing([16.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Region");
                    egui::ComboBox::from_id_salt("player_communication_region_select")
                        .selected_text(player_communication_region_label(
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
                                    player_communication_region_label(region_id),
                                );
                            }
                        });
                    ui.end_row();

                    ui.label("Actual Communication");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.player_communication_value)
                            .desired_width(90.0),
                    );
                    ui.end_row();
                });

            if self.player_communication_region_id != previous_region {
                self.player_communication_value = player_communication_value_for_region(
                    communication,
                    self.player_communication_region_id,
                );
            }

            let pending_xp = communication
                .xp_entries
                .iter()
                .find(|(region_id, _)| *region_id == self.player_communication_region_id)
                .map(|(_, value)| *value)
                .unwrap_or(0);
            ui.label(format!("Pending training XP for selected region: {pending_xp}"));

            let selected_region_exists = communication
                .entries
                .iter()
                .any(|(region_id, _)| *region_id == self.player_communication_region_id);
            if !selected_region_exists {
                ui.weak("This learned region has no actual Communication value yet. Applying 0 removes it.");
            }

            if !communication.entries.is_empty() {
                ui.add_space(6.0);
                ui.strong("Actual learned regions");
                for (region_id, value) in &communication.entries {
                    ui.label(format!(
                        "{}: {} / 100",
                        player_communication_region_label(*region_id),
                        value
                    ));
                }
            } else {
                ui.label("No non-native actual Communication values are stored for this player yet.");
            }

            if !communication.xp_entries.is_empty() {
                ui.add_space(6.0);
                ui.strong("Pending training XP");
                for (region_id, value) in &communication.xp_entries {
                    ui.label(format!(
                        "{}: {} XP",
                        player_communication_region_label(*region_id),
                        value
                    ));
                }
            }

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(self.connected, egui::Button::new("Apply Actual Communication"))
                    .clicked()
                {
                    apply_clicked = true;
                }
                if ui
                    .add_enabled(self.connected, egui::Button::new("Max Selected"))
                    .clicked()
                {
                    max_clicked = true;
                }
            });
            ui.weak("Actual Communication is the value shown in the player profile. Pending XP is updated separately by TFM2's training progression.");
        }

        if max_clicked {
            self.player_communication_value = "100".to_string();
            self.apply_player_communication();
        } else if apply_clicked {
            self.apply_player_communication();
        }
    }



    fn refresh_recruitment_settings(&mut self) {
        match Self::request("GET_RECRUITMENT_SETTINGS") {
            Ok(response) => {
                let parts: Vec<&str> = response.split('|').collect();
                if parts.len() >= 4 && parts[0] == "OK" && parts[1] == "RECRUITMENT" {
                    self.transfer_always_success = parts[2] == "1";
                    self.recruitment_instant_retry = parts[3] == "1";
                }
            }
            Err(_) => {}
        }
    }

    fn refresh_teams(&mut self) {
        match Self::request("GET_TEAMS") {
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
                    self.status = format!("Loaded {} teams", self.teams.len());
                }
                Err(error) => self.status = human_error(&error),
            },
            Err(error) => {
                self.connected = false;
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

        match Self::request(&format!("MOVE_PLAYER_TO_TEAM|{athlete_id}|{team_id}")) {
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

        match Self::request(&format!("SET_PLAYER_FREE_AGENT|{athlete_id}")) {
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

        match Self::request(&format!("MOVE_STAFF_TO_TEAM|{staff_id}|{team_id}")) {
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

        match Self::request(&format!("SET_STAFF_FREE_AGENT|{staff_id}")) {
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
        match Self::request(&command) {
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
        match Self::request(&command) {
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
        ui.heading("Recruitment");
        ui.label("Recruitment, negotiation, and direct roster tools for the active career.");
        ui.add_space(12.0);

        ui.group(|ui| {
            ui.strong("Transfer Negotiation");
            ui.add_space(6.0);

            let previous = self.transfer_always_success;
            let response = ui.checkbox(
                &mut self.transfer_always_success,
                "Transfer Always Success",
            );
            response.on_hover_text(transfer_success_tooltip_text());

            if self.transfer_always_success != previous {
                self.set_transfer_always_success(self.transfer_always_success);
            }

            ui.label(transfer_runtime_text());
        });

        ui.add_space(10.0);
        ui.group(|ui| {
            ui.strong("Recruitment Retry");
            ui.add_space(6.0);

            let previous = self.recruitment_instant_retry;
            let response = ui.checkbox(
                &mut self.recruitment_instant_retry,
                "Instant Retry (No Negotiation Cooldown)",
            );
            response.on_hover_text(instant_retry_tooltip_text());

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
                ui.selectable_value(
                    &mut self.recruitment_management_tab,
                    tab,
                    tab.label(),
                );
            }
        });
        ui.separator();

        match self.recruitment_management_tab {
            RecruitmentManagementTab::Players => {
                ui.group(|ui| {
                    ui.strong("Player Management");
                    ui.label(recruitment_player_management_text());
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.label("Search");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.recruitment_player_search)
                                .desired_width(220.0)
                                .hint_text("Player name or ID"),
                        );
                        if ui.button("Clear").clicked() {
                            self.recruitment_player_search.clear();
                        }
                        if ui.button("Refresh Players").clicked() {
                            self.refresh_players();
                        }
                    });

                    let query = self.recruitment_player_search.trim().to_lowercase();
                    let filtered_players = self
                        .players
                        .iter()
                        .filter(|player| {
                            query.is_empty()
                                || player.name.to_lowercase().contains(&query)
                                || player.team.to_lowercase().contains(&query)
                                || player.id.to_string().contains(&query)
                        })
                        .collect::<Vec<_>>();

                    let selected_player_text = self
                        .recruitment_player_id
                        .and_then(|id| self.players.iter().find(|player| player.id == id))
                        .map(|player| format!("{} · {} · ID {}", player.name, player.team, player.id))
                        .unwrap_or_else(|| "Select player".to_string());

                    ui.horizontal(|ui| {
                        ui.label("Player");
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
                        ui.label(format!("{} matches", filtered_players.len()));
                    });

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label("Team Search");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.recruitment_team_search)
                                .desired_width(220.0)
                                .hint_text("Team or manager name"),
                        );
                        if ui.button("Clear").clicked() {
                            self.recruitment_team_search.clear();
                        }
                        if ui.button("Refresh Teams").clicked() {
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
                        .map(TeamSummary::label)
                        .unwrap_or_else(|| "Select destination team".to_string());

                    ui.horizontal(|ui| {
                        ui.label("Destination Team");
                        egui::ComboBox::from_id_salt("recruitment_player_team_v039")
                            .selected_text(selected_team_text)
                            .width(420.0)
                            .show_ui(ui, |ui| {
                                for team in &filtered_teams {
                                    ui.selectable_value(
                                        &mut self.recruitment_team_id,
                                        Some(team.id),
                                        team.label(),
                                    );
                                }
                            });
                        ui.label(format!("{} matches", filtered_teams.len()));
                    });

                    if let Some(my_team) = self.teams.iter().find(|team| team.is_player_team) {
                        ui.label(format!("My Team: {}", my_team.display_name));
                    }

                    let selected_is_free_agent = self
                        .recruitment_player_id
                        .and_then(|id| self.players.iter().find(|player| player.id == id))
                        .is_some_and(|player| player.team == "Free Agent");
                    let action_label = if selected_is_free_agent {
                        "Create Contract & Move Player"
                    } else {
                        "Move Contracted Player"
                    };
                    let can_move = self.connected
                        && self.recruitment_player_id.is_some()
                        && self.recruitment_team_id.is_some();

                    ui.add_space(8.0);
                    if ui
                        .add_enabled(can_move, egui::Button::new(action_label))
                        .on_hover_text(if selected_is_free_agent {
                            "Opens Edit Contract with automatic defaults. The player moves to the selected team only after the contract is applied."
                        } else {
                            move_player_tooltip_text()
                        })
                        .clicked()
                    {
                        self.move_recruitment_player_to_team();
                    }

                    if ui
                        .add_enabled(
                            self.connected
                                && self.recruitment_player_id.is_some()
                                && !selected_is_free_agent,
                            egui::Button::new("Set Player to Free Agent"),
                        )
                        .on_hover_text("Ends the selected player's current contract and removes the player from the team roster.")
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
                            ui.label("This removes the active contract. Salary, transfer fee, contract dates, bonuses, and current contract requests will be cleared.");
                            ui.horizontal(|ui| {
                                if ui.button("Confirm Free Agent").clicked() {
                                    self.set_recruitment_player_free_agent(confirm_id);
                                    self.free_agent_confirmation_player_id = None;
                                }
                                if ui.button("Cancel").clicked() {
                                    self.free_agent_confirmation_player_id = None;
                                }
                            });
                        });
                    }
                });
            }
            RecruitmentManagementTab::Staff => {
                ui.group(|ui| {
                    ui.strong("Staff Management");
                    ui.label("Move contracted staff directly, or create an active contract before signing a free agent.");
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.label("Search");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.recruitment_staff_search)
                                .desired_width(220.0)
                                .hint_text("Staff name, role, team, or ID"),
                        );
                        if ui.button("Clear").clicked() {
                            self.recruitment_staff_search.clear();
                        }
                        if ui.button("Refresh Staff").clicked() {
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
                        .map(StaffSummary::label)
                        .unwrap_or_else(|| "Select staff".to_string());

                    ui.horizontal(|ui| {
                        ui.label("Staff");
                        egui::ComboBox::from_id_salt("recruitment_staff_v039")
                            .selected_text(selected_staff_text)
                            .width(420.0)
                            .show_ui(ui, |ui| {
                                for staff in &filtered_staff {
                                    ui.selectable_value(
                                        &mut self.recruitment_staff_id,
                                        Some(staff.id),
                                        staff.label(),
                                    );
                                }
                            });
                        ui.label(format!("{} matches", filtered_staff.len()));
                    });

                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label("Team Search");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.recruitment_team_search)
                                .desired_width(220.0)
                                .hint_text("Team or manager name"),
                        );
                        if ui.button("Clear").clicked() {
                            self.recruitment_team_search.clear();
                        }
                        if ui.button("Refresh Teams").clicked() {
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
                        .map(TeamSummary::label)
                        .unwrap_or_else(|| "Select destination team".to_string());

                    ui.horizontal(|ui| {
                        ui.label("Destination Team");
                        egui::ComboBox::from_id_salt("recruitment_staff_team_v039")
                            .selected_text(selected_team_text)
                            .width(420.0)
                            .show_ui(ui, |ui| {
                                for team in &filtered_teams {
                                    ui.selectable_value(
                                        &mut self.recruitment_team_id,
                                        Some(team.id),
                                        team.label(),
                                    );
                                }
                            });
                        ui.label(format!("{} matches", filtered_teams.len()));
                    });

                    let selected_is_free_agent = self
                        .recruitment_staff_id
                        .and_then(|id| self.staffs.iter().find(|staff| staff.id == id))
                        .is_some_and(|staff| staff.team == "Free Agent");
                    let action_label = if selected_is_free_agent {
                        "Create Contract & Move Staff"
                    } else {
                        "Move Contracted Staff"
                    };
                    let can_move = self.connected
                        && self.recruitment_staff_id.is_some()
                        && self.recruitment_team_id.is_some();

                    ui.add_space(8.0);
                    if ui
                        .add_enabled(can_move, egui::Button::new(action_label))
                        .on_hover_text(if selected_is_free_agent {
                            "Opens Edit Contract with automatic defaults. The staff member moves to the selected team only after the contract is applied."
                        } else {
                            "Moves the active staff contract to the selected team. Contract terms can be edited separately."
                        })
                        .clicked()
                    {
                        self.move_recruitment_staff_to_team();
                    }

                    if ui
                        .add_enabled(
                            self.connected
                                && self.recruitment_staff_id.is_some()
                                && !selected_is_free_agent,
                            egui::Button::new("Set Staff to Free Agent"),
                        )
                        .on_hover_text("Ends the selected staff member's current contract and removes the staff assignment.")
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
                            ui.label("This removes the active staff contract, salary, dates, and contract requests.");
                            ui.horizontal(|ui| {
                                if ui.button("Confirm Free Agent").clicked() {
                                    self.set_recruitment_staff_free_agent(confirm_id);
                                    self.free_agent_confirmation_staff_id = None;
                                }
                                if ui.button("Cancel").clicked() {
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
            .set_file_name(&format!(
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


    fn render_search_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Search");
        ui.label(search_intro_text());
        ui.add_space(8.0);

        #[cfg(feature = "dev")]
        {
            ui.horizontal_wrapped(|ui| {
            let import_list = ui.add_enabled(false, egui::Button::new("Import List"));
            import_list.on_hover_text("Saved-list import is reserved for the Lists backend.");
            let export_list = ui.add_enabled(false, egui::Button::new("Export List"));
            export_list.on_hover_text("Saved-list export is reserved for the Lists backend.");

            ui.separator();
            ui.label("List");
            ui.add_enabled_ui(false, |ui| {
                egui::ComboBox::from_id_salt("search_active_list")
                    .selected_text("All Results")
                    .width(150.0)
                    .show_ui(ui, |ui| {
                        let _ = ui.selectable_label(true, "All Results");
                    });
            });

            ui.add_enabled(false, egui::Button::new("New List"));
            ui.add_enabled(false, egui::Button::new("Save List"));
        });
        }

        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            for tab in SearchTab::ALL {
                ui.selectable_value(&mut self.search_tab, tab, tab.label());
            }
        });

        ui.separator();
        ui.add_space(6.0);

        match self.search_tab {
            SearchTab::Players => self.render_player_search_page(ui),
            #[cfg(feature = "dev")]
            SearchTab::Staff => {
                #[cfg(feature = "dev")]
                {

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.columns(2, |columns| {
                            columns[0].group(|ui| {
                                ui.strong("Staff Filters");
                                ui.add_space(6.0);
                                ui.add_enabled(false, egui::Button::new("Name"));
                                ui.add_enabled(false, egui::Button::new("Role"));
                                ui.add_enabled(false, egui::Button::new("Team"));
                                ui.add_enabled(false, egui::Button::new("Region"));
                            });
                            columns[1].group(|ui| {
                                ui.strong("Attribute Filters");
                                ui.add_space(6.0);
                                ui.add_enabled(false, egui::Button::new("Negotiation  Min  —  Max"));
                                ui.add_enabled(false, egui::Button::new("Analysis     Min  —  Max"));
                                ui.add_enabled(false, egui::Button::new("Coaching     Min  —  Max"));
                                ui.add_enabled(false, egui::Button::new("Salary       Min  —  Max"));
                            });
                        });

                        ui.add_space(10.0);
                        ui.group(|ui| {
                            ui.set_min_width(ui.available_width());
                            ui.strong("Staff List");
                            ui.label("Staff database support is planned after the Player Search model is established.");
                            ui.add_enabled(false, egui::Button::new("Staff results table"));
                        });
                    });

                }

                #[cfg(not(feature = "dev"))]
                {
                    ui.heading("Staff");
                    ui.label("Under development.");
                }
            }
            #[cfg(feature = "dev")]
            SearchTab::Teams => {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.columns(2, |columns| {
                            columns[0].group(|ui| {
                                ui.strong("Team Filters");
                                ui.add_space(6.0);
                                ui.add_enabled(false, egui::Button::new("Name"));
                                ui.add_enabled(false, egui::Button::new("League"));
                                ui.add_enabled(false, egui::Button::new("Region"));
                            });
                            columns[1].group(|ui| {
                                ui.strong("Finance / Roster Filters");
                                ui.add_space(6.0);
                                ui.add_enabled(false, egui::Button::new("Balance      Min  —  Max"));
                                ui.add_enabled(false, egui::Button::new("Roster Size  Min  —  Max"));
                                ui.add_enabled(false, egui::Button::new("Team Rating  Min  —  Max"));
                            });
                        });

                        ui.add_space(10.0);
                        ui.group(|ui| {
                            ui.set_min_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.strong("Team List");
                                ui.label(format!("{} teams loaded", self.teams.len()));
                            });

                            egui::ScrollArea::horizontal()
                                .id_salt("search_teams_horizontal")
                                .auto_shrink([false, true])
                                .show(ui, |ui| {
                                    egui::Grid::new("search_teams_table")
                                        .striped(true)
                                        .min_col_width(90.0)
                                        .spacing([14.0, 5.0])
                                        .show(ui, |ui| {
                                            ui.strong("Team");
                                            ui.strong("ID");
                                            ui.strong("League");
                                            ui.strong("Manager");
                                            ui.strong("Balance");
                                            ui.strong("Roster");
                                            ui.strong("Rating");
                                            ui.end_row();

                                            for team in self.teams.iter().take(250) {
                                                ui.label(if team.display_name.trim().is_empty() {
                                                    format!("Team {}", team.id)
                                                } else {
                                                    team.display_name.clone()
                                                });
                                                ui.label(team.id.to_string());
                                                ui.label(team.league_id.to_string());
                                                ui.label(&team.manager_name);
                                                ui.weak("—");
                                                ui.weak("—");
                                                ui.weak("—");
                                                ui.end_row();
                                            }
                                        });
                                });
                        });
                    });
            }
            #[cfg(feature = "dev")]
            SearchTab::Lists => {
                #[cfg(feature = "dev")]
                {

                ui.columns(2, |columns| {
                    columns[0].group(|ui| {
                        ui.strong("Saved Lists");
                        ui.add_space(6.0);
                        ui.add_enabled(false, egui::Button::new("All Players"));
                        ui.add_enabled(false, egui::Button::new("Create List"));
                        ui.add_enabled(false, egui::Button::new("Rename"));
                        ui.add_enabled(false, egui::Button::new("Delete"));
                    });
                    columns[1].group(|ui| {
                        ui.strong("List Options");
                        ui.add_space(6.0);
                        ui.add_enabled(false, egui::Button::new("Add selected result"));
                        ui.add_enabled(false, egui::Button::new("Remove selected result"));
                        ui.add_enabled(false, egui::Button::new("Import into list"));
                        ui.add_enabled(false, egui::Button::new("Export list"));
                    });
                });

                ui.add_space(10.0);
                ui.group(|ui| {
                    ui.set_min_width(ui.available_width());
                    ui.strong("List Contents");
                    ui.label("Saved shortlists will use the same expandable table layout as Player Search.");
                    ui.add_enabled(false, egui::Button::new("No saved list selected"));
                });

                }

                #[cfg(not(feature = "dev"))]
                {
                    ui.heading("Lists");
                    ui.label("Under development.");
                }
            }
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

    fn render_player_search_page(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                ui.strong("Quick Filters");
                ui.separator();

                let advanced_button = egui::Button::new(
                    egui::RichText::new("Advanced Search")
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
                    ui.label(
                        egui::RichText::new(format!("{advanced_count} active"))
                            .strong()
                            .color(ui.visuals().selection.stroke.color),
                    );
                }

                ui.separator();
                ui.weak("Fast filters for the full player database.");
            });
            ui.add_space(5.0);

            ui.horizontal_wrapped(|ui| {
                ui.label("Name");
                ui.add(
                    egui::TextEdit::singleline(&mut self.search_preview_filter)
                        .desired_width(190.0)
                        .hint_text("Player name"),
                );

                ui.separator();
                ui.label("Team");
                egui::ComboBox::from_id_salt("search_quick_team")
                    .selected_text(self.search_team_filter.as_str())
                    .width(150.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.search_team_filter, "Any Team".to_string(), "Any Team");
                        for team in &self.teams {
                            let label = if team.display_name.trim().is_empty() {
                                format!("Team {}", team.id)
                            } else {
                                team.display_name.clone()
                            };
                            ui.selectable_value(&mut self.search_team_filter, label.clone(), label);
                        }
                    });

                ui.label("Region");
                let region_label = selected_multi_filter_label(
                    "Any Region",
                    &REGION_FILTER_NAMES,
                    &self.search_region_filters,
                );
                ui.menu_button(region_label, |ui| {
                    if ui.button("Clear").clicked() {
                        self.search_region_filters = [false; 6];
                    }
                    ui.separator();
                    for (index, label) in REGION_FILTER_NAMES.iter().enumerate() {
                        ui.checkbox(&mut self.search_region_filters[index], *label);
                    }
                });

                ui.label("Position");
                let position_label = selected_multi_filter_label(
                    "Any Position",
                    &POSITION_FILTER_NAMES,
                    &self.search_position_filters,
                );
                ui.menu_button(position_label, |ui| {
                    if ui.button("Clear").clicked() {
                        self.search_position_filters = [false; 5];
                    }
                    ui.separator();
                    for (index, label) in POSITION_FILTER_NAMES.iter().enumerate() {
                        ui.checkbox(&mut self.search_position_filters[index], *label);
                    }
                });
            });

            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.label("Age");
                ui.add(
                    egui::TextEdit::singleline(&mut self.search_age_min)
                        .desired_width(58.0)
                        .hint_text("Min"),
                );
                ui.label("to");
                ui.add(
                    egui::TextEdit::singleline(&mut self.search_age_max)
                        .desired_width(58.0)
                        .hint_text("Max"),
                );

                ui.separator();
                ui.label("Actual Potential");
                ui.add(
                    egui::TextEdit::singleline(&mut self.search_actual_potential_min)
                        .desired_width(58.0)
                        .hint_text("Min"),
                );
                ui.label("to");
                ui.add(
                    egui::TextEdit::singleline(&mut self.search_actual_potential_max)
                        .desired_width(58.0)
                        .hint_text("Max"),
                );

                ui.separator();
                ui.checkbox(&mut self.search_free_agents_only, "Free Agents Only");
            });
        });

        ui.add_space(8.0);

        let mut refresh_players_requested = false;
        let mut reset_columns_requested = false;
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

        let mut filtered_players = self
            .players
            .iter()
            .filter(|player| {
                if !query.is_empty() && !player.name.to_lowercase().contains(&query) {
                    return false;
                }

                let age = player.age.parse::<f64>().ok();
                if age_min.is_some_and(|min| age.map_or(true, |value| value < min)) {
                    return false;
                }
                if age_max.is_some_and(|max| age.map_or(true, |value| value > max)) {
                    return false;
                }

                let potential = player.actual_potential.parse::<f64>().ok();
                if potential_min.is_some_and(|min| potential.map_or(true, |value| value < min)) {
                    return false;
                }
                if potential_max.is_some_and(|max| potential.map_or(true, |value| value > max)) {
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
            .collect::<Vec<_>>();

        filtered_players.sort_by(|a, b| {
            compare_player_summaries(a, b, sort_column, sort_ascending)
        });

        let available_height = ui.available_height().max(180.0);
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), available_height),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.group(|ui| {
                    ui.set_min_width(ui.available_width());
                    ui.set_min_height((ui.available_height() - 2.0).max(160.0));

                    ui.horizontal_wrapped(|ui| {
                        ui.strong("Player List");
                        ui.label(format!("{} results", filtered_players.len()));
                        if ui.button("Refresh Players").clicked() {
                            refresh_players_requested = true;
                        }
                        if ui.button("Reset Columns").clicked() {
                            reset_columns_requested = true;
                        }
                        ui.separator();
                        ui.weak("Click a header to sort. Drag column separators to resize; double-click a separator to auto-size.");
                    });
                    ui.weak(search_rating_info_text());
                    ui.add_space(4.0);

                    let table_height = (ui.available_height() - 26.0).max(120.0);
                    let viewport_width = ui.available_width();
                    let table_min_width = 2860.0_f32.max(viewport_width);

                    egui::ScrollArea::horizontal()
                        .id_salt("search_players_table_horizontal")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.set_min_width(table_min_width);

                            let widths = [
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

                            let mut table = TableBuilder::new(ui)
                                .id_salt("search_players_resizable_table")
                                .striped(true)
                                .resizable(true)
                                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
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

                            table
                                .header(22.0, |mut header| {
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
                                            );
                                        });
                                    }
                                    header.col(|ui| {
                                        ui.strong("History");
                                    });
                                })
                                .body(|body| {
                                    body.rows(21.0, filtered_players.len(), |mut row| {
                                        let player = filtered_players[row.index()];
                                        row.col(|ui| { ui.label(&player.name); });
                                        row.col(|ui| { ui.label(player.id.to_string()); });
                                        row.col(|ui| { ui.label(value_or_dash(&player.age)); });
                                        row.col(|ui| { ui.label(value_or_dash(&player.team)); });
                                        row.col(|ui| { ui.label(value_or_dash(&player.position)); });
                                        row.col(|ui| { render_actual_rating(ui, player); });
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
                                            ui.add_enabled(false, egui::Button::new("Open"));
                                        });
                                    });
                                });
                        });
                });
            },
        );

        self.player_sort_column = sort_column;
        self.player_sort_ascending = sort_ascending;
        if refresh_players_requested {
            self.refresh_players();
        }
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

        egui::Window::new("Advanced Player Search")
            // New persistent ID intentionally resets the undersized window rect
            // that could be remembered from v0.2.17.
            .id(egui::Id::new("advanced_player_search_window_v0218"))
            .open(&mut open)
            .resizable(true)
            .default_size(egui::vec2(1040.0, 720.0))
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.button("New Filter").clicked() {
                        reset_filter = true;
                    }
                    if ui.button("Import Filter").clicked() {
                        import_filter = true;
                    }
                    if ui.button("Export Filter").clicked() {
                        export_filter = true;
                    }

                    ui.separator();
                    ui.weak("Saved filters are listed on the left.");
                });

                ui.add_space(4.0);
                ui.weak(advanced_search_info_text());
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
                                left_ui.strong("Saved Filters");
                                left_ui.separator();

                                let list_height = left_ui.available_height();
                                egui::ScrollArea::vertical()
                                    .id_salt("saved_player_filters_scroll")
                                    .auto_shrink([false, false])
                                    .max_height(list_height)
                                    .show(left_ui, |ui| {
                                        ui.set_min_height(list_height);

                                        if self.saved_filters.is_empty() {
                                            ui.weak("No saved filters");
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
                                            "Position",
                                            &mut self.advanced_player_search.position_enabled,
                                            &mut self.advanced_player_search.position,
                                            &[
                                                "No Condition",
                                                "Top",
                                                "Jungle",
                                                "Mid",
                                                "Bottom",
                                                "Support",
                                            ],
                                            "advanced_position_choice",
                                        );
                                        advanced_choice_filter_row(
                                            ui,
                                            "Region",
                                            &mut self.advanced_player_search.region_enabled,
                                            &mut self.advanced_player_search.region,
                                            &[
                                                "No Condition",
                                                "Korea",
                                                "China",
                                                "Europe",
                                                "North America",
                                                "South America",
                                                "Japan",
                                            ],
                                            "advanced_region_choice",
                                        );

                                        for range in &mut self.advanced_player_search.ranges {
                                            advanced_range_filter_row(ui, range);
                                        }

                                        advanced_boolean_filter_row(
                                            ui,
                                            "Free Agents Only",
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
                    if ui.button("Reset").clicked() {
                        reset_filter = true;
                    }
                    if ui.button("Confirm").clicked() {
                        self.status = format!(
                            "Advanced Search applied: {} active condition(s)",
                            self.advanced_player_search.active_condition_count()
                        );
                        close_after = true;
                    }

                    ui.separator();

                    if ui.button("Save Filter").clicked() {
                        self.filter_name_draft =
                            self.selected_saved_filter.clone().unwrap_or_default();
                        self.filter_name_popup_open = true;
                    }

                    let update = ui.add_enabled(
                        self.selected_saved_filter.is_some(),
                        egui::Button::new("Update Filter"),
                    );
                    if update.clicked() {
                        self.update_selected_filter();
                    }

                    let delete = ui.add_enabled(
                        self.selected_saved_filter.is_some(),
                        egui::Button::new("Delete Filter"),
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

        egui::Window::new("Save Player Filter")
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .default_width(360.0)
            .show(ctx, |ui| {
                ui.label("Filter name");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.filter_name_draft)
                        .desired_width(f32::INFINITY)
                        .hint_text("e.g. EU Young Prospects"),
                );

                if response.lost_focus()
                    && ui.input(|input| input.key_pressed(egui::Key::Enter))
                {
                    save_requested = true;
                }

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        save_requested = true;
                    }
                    if ui.button("Cancel").clicked() {
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



}

impl eframe::App for ModifierApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("app_header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("TFM2 Editor");
                ui.weak("Made by jal-io");
                ui.separator();
                ui.label(display_version());

                #[cfg(feature = "dev")]
                {
                    ui.label(
                        egui::RichText::new("DEV BUILD")
                            .strong()
                            .color(ui.visuals().selection.stroke.color),
                    );
                }
                ui.separator();

                let status_text = if self.connected {
                    "● Connected"
                } else {
                    "● Disconnected"
                };
                ui.label(status_text);
                ui.label(format!("Bridge {}", self.bridge_version));

                if ui.button("Reconnect").clicked() {
                    self.refresh_connection();
                    if self.connected {
                        self.refresh_economy();
                        self.refresh_players();
                        self.refresh_staff();
                        self.refresh_teams();
                        self.refresh_recruitment_settings();
                    }
                }
            });

            ui.add_space(6.0);
            let tab_before_click = self.active_tab;
            ui.horizontal(|ui| {
                for tab in AppTab::ALL {
                    ui.selectable_value(&mut self.active_tab, tab, tab.label());
                }
            });
            if tab_before_click != self.active_tab
                && self.active_tab == AppTab::Search
                && self.connected
            {
                self.refresh_players();
            }
            ui.add_space(4.0);
        });

        egui::TopBottomPanel::bottom("app_status").show(ctx, |ui| {
            ui.separator();
            ui.label(format!("Status: {}", self.status));
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
                        AppTab::Recruitment => self.render_recruitment_tab(ui),
                        AppTab::Search => unreachable!(),
                    });
            }
        });

        self.render_advanced_search_window(ctx);

        self.render_champion_mastery_window(ctx);

        self.render_player_contract_window(ctx);
        #[cfg(feature = "dev")]
        self.render_player_contract_probe_window(ctx);

        self.render_staff_contract_window(ctx);
        #[cfg(feature = "dev")]
        self.render_staff_contract_probe_window(ctx);
    }
}



fn player_sort_header(
    ui: &mut egui::Ui,
    column: PlayerSortColumn,
    sort_column: &mut PlayerSortColumn,
    sort_ascending: &mut bool,
) {
    let is_active = *sort_column == column;
    let arrow = if is_active {
        if *sort_ascending { " ↑" } else { " ↓" }
    } else {
        ""
    };

    if ui.button(format!("{}{}", column.label(), arrow)).clicked() {
        if is_active {
            *sort_ascending = !*sort_ascending;
        } else {
            *sort_column = column;
            *sort_ascending = true;
        }
    }
}


fn parse_filter_number(value: &str) -> Option<f64> {
    let normalized = value.trim().replace(' ', "").replace(',', ".");
    if normalized.is_empty() {
        None
    } else {
        normalized.parse::<f64>().ok()
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

    fn parse_number_for_sort(value: &str) -> Option<f64> {
        let normalized = value
            .trim()
            .replace(' ', "")
            .replace(',', ".");
        if normalized.is_empty() || normalized == "—" || normalized == "-" {
            None
        } else {
            normalized.parse::<f64>().ok()
        }
    }

    fn compare_number(a: &str, b: &str, ascending: bool) -> Ordering {
        match (parse_number_for_sort(a), parse_number_for_sort(b)) {
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
    choices: &[&str],
    id: &'static str,
) {
    ui.horizontal(|ui| {
        ui.add_sized([20.0, 24.0], egui::Checkbox::without_text(enabled));
        ui.add_sized([138.0, 24.0], egui::Label::new(label));
        ui.add_enabled_ui(*enabled, |ui| {
            egui::ComboBox::from_id_salt(id)
                .selected_text(value.as_str())
                .width(198.0)
                .show_ui(ui, |ui| {
                    for choice in choices {
                        ui.selectable_value(value, (*choice).to_string(), *choice);
                    }
                });
        });
    });
}

fn advanced_range_filter_row(ui: &mut egui::Ui, filter: &mut AdvancedRangeFilter) {
    ui.horizontal(|ui| {
        ui.add_sized([20.0, 24.0], egui::Checkbox::without_text(&mut filter.enabled));
        ui.add_sized([138.0, 24.0], egui::Label::new(filter.label));
        ui.add_enabled(
            filter.enabled,
            egui::TextEdit::singleline(&mut filter.min)
                .desired_width(82.0)
                .hint_text("Min"),
        );
        ui.add_sized([14.0, 24.0], egui::Label::new("~"));
        ui.add_enabled(
            filter.enabled,
            egui::TextEdit::singleline(&mut filter.max)
                .desired_width(82.0)
                .hint_text("Max"),
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
    if raw % 10 == 0 {
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
            salary: hex_decode(fields[9])?,
            transfer_fee: hex_decode(fields[10])?,
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
        if fields.len() != 5 {
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
        annual_salary: pretty_number(parts[17]),
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
        pretty_number(parts[4]),
    ))
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
        if fields.len() != 5 {
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
        teams.push(TeamSummary {
            id,
            display_name,
            manager_name,
            league_id,
            is_player_team,
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
        annual_salary: pretty_number(parts[22]),
        weekly_salary: pretty_number(parts[23]),
        contract_team_id: parse_optional_usize(parts[24])?,
        contract_start_date: display_contract_date(&hex_decode(parts[25])?),
        contract_end_date: display_contract_date(&hex_decode(parts[26])?),
        transfer_fee: pretty_number(parts[27]),
        squad_status: hex_decode(parts[28])?,
        incentive_pog_bonus: pretty_number(parts[29]),
        incentive_league_bonus: pretty_number(parts[30]),
        incentive_league_rank: pretty_number(parts[31]),
        incentive_match_bonus: pretty_number(parts[32]),
        incentive_win_bonus: pretty_number(parts[33]),
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

fn hex_decode(encoded: &str) -> Result<String, String> {
    if encoded.len() % 2 != 0 {
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

fn parse_number(input: &str) -> Result<f64, ()> {
    let normalized = input
        .trim()
        .replace(' ', "")
        .replace('_', "")
        .replace(',', ".");

    normalized
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
        .ok_or(())
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

fn contract_bonus_display(raw: &str) -> String {
    if raw.trim().is_empty() {
        "Disabled".to_string()
    } else {
        pretty_number(raw)
    }
}

fn value_or_zero(raw: &str) -> String {
    let value = raw.trim();
    if value.is_empty() {
        "0".to_string()
    } else {
        pretty_number(value)
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
        "NO_REGIONS_DETECTED" => "No region IDs could be detected in the current save".to_string(),
        "INVALID_REGION" => "The selected Communication region is not available in the current save".to_string(),
        "SERVER_COMMAND_FAILED" => "Could not send the change to TFM2 management/server state".to_string(),
        "GAME_RESPONSE_TIMEOUT" => "The game did not respond to the bridge command".to_string(),
        other => format!("Bridge error: {other}"),
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
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::new(ModifierApp::default()))
        }),
    )
}
