use std::cmp::Reverse;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use mod_api::*;
use game_core::{Contract, Incentive, PaperState, SquadStatus, StaffRole};

const MOD_ID: &str = "tfm2_modifier_bridge";
const BRIDGE_ADDR: &str = "127.0.0.1:28452";
const BRIDGE_VERSION: &str = "0.2.59";
const BRIDGE_PROTOCOL_VERSION: u32 = 19;
const TFM2_TARGET_VERSION: &str = "0.5.5";
const GLOBAL_HISTORY_RECORD_CAP: usize = 10_000;

#[derive(Debug, Clone, Copy)]
struct EconomyValues {
    money: f64,
    transfer_budget: f64,
    salary_budget: f64,
}

#[derive(Debug, Clone, Copy)]
enum ContractDefaultsEntity {
    Player,
    Staff,
}

#[derive(Debug, Clone)]
struct ContractDefaults {
    start_date: String,
    end_date: String,
    annual_salary: String,
}

#[derive(Debug, Clone)]
struct PlayerListEntry {
    id: usize,
    name: String,
    age: String,
    team: String,
    region: String,
    position: String,
    actual_rating: String,
    potential_rating: String,
    actual_potential: String,
    annual_salary: String,
    transfer_fee: String,
    contract_end_date: String,
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
struct StaffListEntry {
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
    contract_end_date: String,
    communication: String,
}

#[derive(Debug, Clone)]
struct StaffSnapshot {
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
    contract_team_id: String,
    contract_start_date: String,
    contract_end_date: String,
    communication_raw: String,
}

#[derive(Debug, Clone)]
struct StaffStatValues {
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
}


#[derive(Debug, Clone)]
struct EntityNameValue {
    name: String,
}

#[derive(Debug, Clone)]
struct StaffSalaryValue {
    annual_salary: String,
}

#[derive(Debug, Clone)]
struct StaffContractEndValue {
    end_date: String,
}

#[derive(Debug, Clone)]
struct StaffContractValue {
    team_id: usize,
    start_date: String,
    end_date: String,
    annual_salary: String,
}

#[derive(Debug, Clone)]
struct StaffCommunicationValues {
    entries: Vec<(usize, u16)>,
}

#[derive(Debug, Clone)]
struct TeamListEntry {
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
    stadium_name: String,
    stadium_capacity: String,
    total_home_attendance: String,
    home_match_count: String,
    total_entrance_income: String,
    popularity: String,
    fan_expectation: String,
    fan_satisfaction: String,
    fan_count: String,
    fan_momentum: String,
    gaming_house_level: String,
    welfare: String,
    total_balance: f64,
    transfer_budget: f64,
    salary_budget: f64,
}

#[derive(Debug, Clone)]
struct TeamManagementSnapshot {
    team_id: usize,
    management: String,
    current_strategy: String,
    last_strategy: String,
    team_color_strategy: String,
    merchandise: String,
    champion_setup: String,
    gaming_house: String,
}

#[derive(Debug, Clone)]
struct TeamMerchandiseWriteValue {
    product_type: String,
    athlete_id: usize,
    stock: String,
    sell_price: String,
}

#[derive(Debug, Clone)]
struct TeamFansWriteValue {
    popularity: String,
    fan_count: String,
    fan_expectation: String,
    fan_satisfaction: String,
    fan_momentum: String,
}

#[derive(Debug, Clone)]
struct PlayerSnapshot {
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
    contract_team_id: String,
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

#[derive(Debug, Clone)]
struct PlayerStatValues {
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

#[derive(Debug, Clone, Copy)]
struct PlayerPositionValues {
    top: u16,
    jungle: u16,
    mid: u16,
    bottom: u16,
    support: u16,
}

#[derive(Debug, Clone, Copy)]
struct PlayerPotentialValue {
    potential: u16,
}

#[derive(Debug, Clone)]
struct PlayerSalaryValue {
    annual_salary: String,
}

#[derive(Debug, Clone)]
struct PlayerContractEndValue {
    end_date: String,
}

#[derive(Debug, Clone)]
struct PlayerContractValue {
    team_id: usize,
    start_date: String,
    end_date: String,
    annual_salary: String,
    transfer_fee: String,
    squad_status: String,
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

#[derive(Debug, Clone)]
struct PlayerConditionValue {
    stamina: String,
    condition: String,
}

#[derive(Debug, Clone)]
struct PlayerConditionSnapshot {
    athlete_id: usize,
    stamina: String,
    condition: String,
}

#[derive(Debug, Clone, Copy)]
struct PlayerCommunicationValue {
    region_id: usize,
    level: usize,
}

#[derive(Debug, Clone)]
struct ChampionMasteryValue {
    champion_id: String,
    mastery: u16,
}

enum GameRequest {
    GetEconomy {
        reply: Sender<String>,
    },
    SetEconomy {
        values: EconomyValues,
        reply: Sender<String>,
    },
    GetPlayers {
        reply: Sender<String>,
    },
    GetStaffs {
        reply: Sender<String>,
    },
    GetStaff {
        staff_id: usize,
        reply: Sender<String>,
    },
    GetStaffContractProbe {
        staff_id: usize,
        reply: Sender<String>,
    },
    SetStaffName {
        staff_id: usize,
        values: EntityNameValue,
        reply: Sender<String>,
    },
    SetStaffStats {
        staff_id: usize,
        values: StaffStatValues,
        reply: Sender<String>,
    },
    SetStaffSalary {
        staff_id: usize,
        values: StaffSalaryValue,
        reply: Sender<String>,
    },
    SetStaffContractEnd {
        staff_id: usize,
        values: StaffContractEndValue,
        reply: Sender<String>,
    },
    SetStaffContract {
        staff_id: usize,
        values: StaffContractValue,
        reply: Sender<String>,
    },
    SetStaffCommunication {
        staff_id: usize,
        values: StaffCommunicationValues,
        reply: Sender<String>,
    },
    GetTeams {
        reply: Sender<String>,
    },
    GetTeamProbe {
        team_id: usize,
        reply: Sender<String>,
    },
    GetTeamManagement {
        team_id: usize,
        reply: Sender<String>,
    },
    SetTeamMerchandise {
        team_id: usize,
        values: TeamMerchandiseWriteValue,
        reply: Sender<String>,
    },
    SetTeamFans {
        team_id: usize,
        values: TeamFansWriteValue,
        reply: Sender<String>,
    },
    GetTeamFanMomentumProbe {
        team_id: usize,
        reply: Sender<String>,
    },
    GetTeamStrategyOptions {
        reply: Sender<String>,
    },
    GetTeamReplayStrategies {
        team_id: usize,
        replay_ids: Vec<usize>,
        reply: Sender<String>,
    },
    ProbeSwapTeamStrategy {
        team_id: usize,
        reply: Sender<String>,
    },
    SetTeamStrategy {
        team_id: usize,
        raw_strategy: String,
        reply: Sender<String>,
    },
    GetContractDefaults {
        entity: ContractDefaultsEntity,
        team_id: usize,
        reply: Sender<String>,
    },
    MoveStaffToTeam {
        staff_id: usize,
        team_id: usize,
        role: Option<String>,
        reply: Sender<String>,
    },
    SetStaffFreeAgent {
        staff_id: usize,
        reply: Sender<String>,
    },
    MovePlayerToTeam {
        athlete_id: usize,
        team_id: usize,
        reply: Sender<String>,
    },
    SetPlayerFreeAgent {
        athlete_id: usize,
        reply: Sender<String>,
    },
    GetPlayer {
        athlete_id: usize,
        reply: Sender<String>,
    },
    GetPlayerContractProbe {
        athlete_id: usize,
        reply: Sender<String>,
    },
    SetPlayerName {
        athlete_id: usize,
        values: EntityNameValue,
        reply: Sender<String>,
    },
    SetPlayerCondition {
        athlete_id: usize,
        values: PlayerConditionValue,
        reply: Sender<String>,
    },
    SetPlayerStats {
        athlete_id: usize,
        values: PlayerStatValues,
        reply: Sender<String>,
    },
    SetPlayerPositions {
        athlete_id: usize,
        values: PlayerPositionValues,
        reply: Sender<String>,
    },
    SetPlayerPotential {
        athlete_id: usize,
        values: PlayerPotentialValue,
        reply: Sender<String>,
    },
    SetPlayerSalary {
        athlete_id: usize,
        values: PlayerSalaryValue,
        reply: Sender<String>,
    },
    SetPlayerContractEnd {
        athlete_id: usize,
        values: PlayerContractEndValue,
        reply: Sender<String>,
    },
    SetPlayerContract {
        athlete_id: usize,
        values: PlayerContractValue,
        reply: Sender<String>,
    },
    SetPlayerCommunication {
        athlete_id: usize,
        values: PlayerCommunicationValue,
        reply: Sender<String>,
    },
    SetPlayerCommunicationMax {
        athlete_id: usize,
        reply: Sender<String>,
    },
    SetTransferAlwaysSuccess {
        enabled: bool,
        reply: Sender<String>,
    },
    SetRecruitmentInstantRetry {
        enabled: bool,
        reply: Sender<String>,
    },
    GetChampionMasteryProbe {
        athlete_id: usize,
        reply: Sender<String>,
    },
    SetChampionMastery {
        athlete_id: usize,
        values: Vec<ChampionMasteryValue>,
        reply: Sender<String>,
    },
    GetGlobalLeagues {
        reply: Sender<String>,
    },
    GetGlobalLeagueCompetition {
        league_id: usize,
        reply: Sender<String>,
    },
    GetGlobalTeamSchedule {
        team_id: usize,
        reply: Sender<String>,
    },
    GetGlobalTeamHistory {
        team_id: usize,
        reply: Sender<String>,
    },
}

#[derive(Debug, Clone)]
struct GlobalMatchRecord {
    json: String,
    completed: bool,
}

#[derive(Debug, Clone, Default)]
struct GlobalHistoryCaptureMetrics {
    league_source_records: usize,
    league_retained_records: usize,
    league_bytes: usize,
    league_competition_source_records: usize,
    league_competition_retained_records: usize,
    league_competition_bytes: usize,
    match_source_records: usize,
    match_scanned_records: usize,
    match_retained_records: usize,
    match_dropped_records: usize,
    match_oldest_retained_id: Option<usize>,
    match_newest_retained_id: Option<usize>,
    match_indexed_teams: usize,
    match_index_entries: usize,
    match_bytes: usize,
    snapshot_bytes: usize,
    largest_record_bytes: usize,
    capture_micros: u64,
}

#[derive(Debug, Clone, Default)]
struct GlobalHistoryRequestMetric {
    requested_id: Option<usize>,
    records_returned: usize,
    response_bytes: usize,
    response_micros: u64,
}

#[derive(Debug, Clone, Default)]
struct GlobalHistoryResponseMetrics {
    get_leagues: Option<GlobalHistoryRequestMetric>,
    get_league_competition: Option<GlobalHistoryRequestMetric>,
    get_team_schedule: Option<GlobalHistoryRequestMetric>,
    get_team_history: Option<GlobalHistoryRequestMetric>,
}

#[derive(Debug, Default)]
struct GlobalHistorySnapshot {
    capture_index: usize,
    league_records: Vec<String>,
    league_competition_records: HashMap<usize, String>,
    match_records: Vec<GlobalMatchRecord>,
    team_match_indices: HashMap<usize, Vec<usize>>,
    metrics: GlobalHistoryCaptureMetrics,
}

static SERVER_STARTED: AtomicBool = AtomicBool::new(false);
static GLOBAL_HISTORY_CAPTURE_INDEX: AtomicUsize = AtomicUsize::new(0);
static GLOBAL_HISTORY_SNAPSHOT: OnceLock<Mutex<Option<GlobalHistorySnapshot>>> = OnceLock::new();
static GLOBAL_HISTORY_RESPONSE_METRICS: OnceLock<Mutex<GlobalHistoryResponseMetrics>> = OnceLock::new();
static TRANSFER_ALWAYS_SUCCESS: AtomicBool = AtomicBool::new(false);
static TRANSFER_ALWAYS_SUCCESS_TEAM_ID: AtomicUsize = AtomicUsize::new(usize::MAX);
static RECRUITMENT_INSTANT_RETRY: AtomicBool = AtomicBool::new(false);
static RECRUITMENT_INSTANT_RETRY_TEAM_ID: AtomicUsize = AtomicUsize::new(usize::MAX);
static REQUEST_RX: OnceLock<Mutex<Receiver<GameRequest>>> = OnceLock::new();

#[derive(Debug, Clone)]
struct ContractEndOverride {
    values: PlayerContractEndValue,
    remaining_after_ticks: u8,
}

static CONTRACT_END_OVERRIDES: OnceLock<Mutex<HashMap<usize, ContractEndOverride>>> = OnceLock::new();

#[derive(Debug, Clone)]
struct StaffContractEndOverride {
    values: StaffContractEndValue,
    remaining_after_ticks: u8,
}

static STAFF_CONTRACT_END_OVERRIDES: OnceLock<Mutex<HashMap<usize, StaffContractEndOverride>>> = OnceLock::new();

fn contract_end_overrides() -> &'static Mutex<HashMap<usize, ContractEndOverride>> {
    CONTRACT_END_OVERRIDES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn queue_contract_end_override(athlete_id: usize, values: PlayerContractEndValue) {
    if let Ok(mut overrides) = contract_end_overrides().lock() {
        overrides.insert(
            athlete_id,
            ContractEndOverride {
                values,
                remaining_after_ticks: 3,
            },
        );
    }
}

fn clear_contract_end_overrides() {
    if let Ok(mut overrides) = contract_end_overrides().lock() {
        overrides.clear();
    }
}

fn enforce_contract_end_overrides(ctx: &mut ServerModContext, decrement: bool) {
    let pending = if let Ok(overrides) = contract_end_overrides().lock() {
        overrides
            .iter()
            .map(|(athlete_id, entry)| (*athlete_id, entry.values.clone()))
            .collect::<Vec<_>>()
    } else {
        return;
    };

    for (athlete_id, values) in pending {
        if let Some(athlete) = ctx.database.athletes.get_mut(athlete_id) {
            let _ = apply_contract_end_to_athlete(athlete, &values);
        }
    }

    if decrement {
        if let Ok(mut overrides) = contract_end_overrides().lock() {
            overrides.retain(|_, entry| {
                entry.remaining_after_ticks = entry.remaining_after_ticks.saturating_sub(1);
                entry.remaining_after_ticks > 0
            });
        }
    }
}

fn staff_contract_end_overrides() -> &'static Mutex<HashMap<usize, StaffContractEndOverride>> {
    STAFF_CONTRACT_END_OVERRIDES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn queue_staff_contract_end_override(staff_id: usize, values: StaffContractEndValue) {
    if let Ok(mut overrides) = staff_contract_end_overrides().lock() {
        overrides.insert(
            staff_id,
            StaffContractEndOverride {
                values,
                remaining_after_ticks: 3,
            },
        );
    }
}

fn clear_staff_contract_end_overrides() {
    if let Ok(mut overrides) = staff_contract_end_overrides().lock() {
        overrides.clear();
    }
}

fn enforce_staff_contract_end_overrides(ctx: &mut ServerModContext, decrement: bool) {
    let pending = if let Ok(overrides) = staff_contract_end_overrides().lock() {
        overrides
            .iter()
            .map(|(staff_id, entry)| (*staff_id, entry.values.clone()))
            .collect::<Vec<_>>()
    } else {
        return;
    };

    for (staff_id, values) in pending {
        if let Some(staff) = ctx.database.staffs.get_mut(staff_id) {
            let _ = apply_staff_contract_end(&mut staff.contract, &values);
        }
    }

    if decrement {
        if let Ok(mut overrides) = staff_contract_end_overrides().lock() {
            overrides.retain(|_, entry| {
                entry.remaining_after_ticks = entry.remaining_after_ticks.saturating_sub(1);
                entry.remaining_after_ticks > 0
            });
        }
    }
}

#[derive(Debug, Clone)]
struct ActivePlayerContractOverride {
    values: PlayerContractValue,
    remaining_after_ticks: u8,
}

static ACTIVE_PLAYER_CONTRACT_OVERRIDES: OnceLock<Mutex<HashMap<usize, ActivePlayerContractOverride>>> = OnceLock::new();

#[derive(Debug, Clone)]
struct ActiveStaffContractOverride {
    values: StaffContractValue,
    remaining_after_ticks: u8,
}

static ACTIVE_STAFF_CONTRACT_OVERRIDES: OnceLock<Mutex<HashMap<usize, ActiveStaffContractOverride>>> = OnceLock::new();

fn active_player_contract_overrides() -> &'static Mutex<HashMap<usize, ActivePlayerContractOverride>> {
    ACTIVE_PLAYER_CONTRACT_OVERRIDES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn active_staff_contract_overrides() -> &'static Mutex<HashMap<usize, ActiveStaffContractOverride>> {
    ACTIVE_STAFF_CONTRACT_OVERRIDES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn queue_active_player_contract_override(athlete_id: usize, values: PlayerContractValue) {
    if let Ok(mut overrides) = active_player_contract_overrides().lock() {
        overrides.insert(
            athlete_id,
            ActivePlayerContractOverride {
                values,
                remaining_after_ticks: 3,
            },
        );
    }
}

fn queue_active_staff_contract_override(staff_id: usize, values: StaffContractValue) {
    if let Ok(mut overrides) = active_staff_contract_overrides().lock() {
        overrides.insert(
            staff_id,
            ActiveStaffContractOverride {
                values,
                remaining_after_ticks: 3,
            },
        );
    }
}

fn clear_active_contract_overrides() {
    if let Ok(mut overrides) = active_player_contract_overrides().lock() {
        overrides.clear();
    }
    if let Ok(mut overrides) = active_staff_contract_overrides().lock() {
        overrides.clear();
    }
}

fn enforce_active_player_contract_overrides(ctx: &mut ServerModContext, decrement: bool) {
    let pending = if let Ok(overrides) = active_player_contract_overrides().lock() {
        overrides
            .iter()
            .map(|(athlete_id, entry)| (*athlete_id, entry.values.clone()))
            .collect::<Vec<_>>()
    } else {
        return;
    };

    for (athlete_id, values) in pending {
        if let Some(athlete) = ctx.database.athletes.get_mut(athlete_id) {
            let _ = apply_player_contract_values(athlete, &values);
        }
    }

    if decrement {
        if let Ok(mut overrides) = active_player_contract_overrides().lock() {
            overrides.retain(|_, entry| {
                entry.remaining_after_ticks = entry.remaining_after_ticks.saturating_sub(1);
                entry.remaining_after_ticks > 0
            });
        }
    }
}

fn enforce_active_staff_contract_overrides(ctx: &mut ServerModContext, decrement: bool) {
    let pending = if let Ok(overrides) = active_staff_contract_overrides().lock() {
        overrides
            .iter()
            .map(|(staff_id, entry)| (*staff_id, entry.values.clone()))
            .collect::<Vec<_>>()
    } else {
        return;
    };

    for (staff_id, values) in pending {
        if let Some(staff) = ctx.database.staffs.get_mut(staff_id) {
            let _ = apply_active_contract_fields(
                &mut staff.contract,
                values.team_id,
                &values.start_date,
                &values.end_date,
                &values.annual_salary,
                "0",
            );
        }
    }

    if decrement {
        if let Ok(mut overrides) = active_staff_contract_overrides().lock() {
            overrides.retain(|_, entry| {
                entry.remaining_after_ticks = entry.remaining_after_ticks.saturating_sub(1);
                entry.remaining_after_ticks > 0
            });
        }
    }
}

fn response_ok_economy(values: EconomyValues) -> String {
    format!(
        "OK|ECONOMY|{}|{}|{}",
        values.money,
        values.transfer_budget,
        values.salary_budget
    )
}

fn response_ok_contract_defaults(values: &ContractDefaults) -> String {
    format!(
        "OK|CONTRACT_DEFAULTS|{}|{}|{}",
        values.start_date, values.end_date, values.annual_salary
    )
}

fn hex_encode_into(output: &mut String, value: &str) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for byte in value.as_bytes() {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
}

fn hex_encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len().saturating_mul(2));
    hex_encode_into(&mut encoded, value);
    encoded
}

fn hex_join_records(records: &[String]) -> String {
    let encoded_bytes = records
        .iter()
        .map(|record| record.len().saturating_mul(2))
        .sum::<usize>();
    let separators = records.len().saturating_sub(1);
    let mut payload = String::with_capacity(encoded_bytes.saturating_add(separators));
    for (index, record) in records.iter().enumerate() {
        if index != 0 {
            payload.push(';');
        }
        hex_encode_into(&mut payload, record);
    }
    payload
}

fn hex_decode(encoded: &str) -> Result<String, &'static str> {
    if !encoded.len().is_multiple_of(2) {
        return Err("INVALID_NAME_ENCODING");
    }

    let mut bytes = Vec::with_capacity(encoded.len() / 2);
    for pair in encoded.as_bytes().chunks_exact(2) {
        let high = hex_value(pair[0]).ok_or("INVALID_NAME_ENCODING")?;
        let low = hex_value(pair[1]).ok_or("INVALID_NAME_ENCODING")?;
        bytes.push((high << 4) | low);
    }

    String::from_utf8(bytes).map_err(|_| "INVALID_NAME_ENCODING")
}

fn hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn validate_entity_name(value: &str) -> Result<String, &'static str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("NAME_EMPTY");
    }
    if trimmed.chars().count() > 100 {
        return Err("NAME_TOO_LONG");
    }
    if trimmed.chars().any(char::is_control) {
        return Err("NAME_CONTROL_CHARACTER");
    }
    Ok(trimmed.to_string())
}

fn entity_name_payload(entity_id: usize, values: &EntityNameValue) -> Vec<u8> {
    format!("{}|{}", entity_id, hex_encode(&values.name)).into_bytes()
}

fn parse_server_entity_name(payload: &[u8]) -> Result<(usize, EntityNameValue), &'static str> {
    let text = std::str::from_utf8(payload).map_err(|_| "INVALID_PAYLOAD")?;
    let (entity_id, encoded_name) = text.split_once('|').ok_or("MISSING_VALUE")?;
    let name = validate_entity_name(&hex_decode(encoded_name)?)?;
    Ok((
        entity_id.parse::<usize>().map_err(|_| "INVALID_ID")?,
        EntityNameValue { name },
    ))
}

fn response_ok_players(players: &[PlayerListEntry]) -> String {
    let payload = players
        .iter()
        .map(|player| {
            [
                player.id.to_string(),
                hex_encode(&player.name),
                hex_encode(&player.age),
                hex_encode(&player.team),
                hex_encode(&player.region),
                hex_encode(&player.position),
                hex_encode(&player.actual_rating),
                hex_encode(&player.potential_rating),
                hex_encode(&player.actual_potential),
                hex_encode(&player.annual_salary),
                hex_encode(&player.transfer_fee),
                hex_encode(&player.contract_end_date),
                hex_encode(&player.last_hit),
                hex_encode(&player.skill_avoid),
                hex_encode(&player.skill_hit),
                hex_encode(&player.control_speed),
                hex_encode(&player.positioning),
                hex_encode(&player.judgement),
                hex_encode(&player.mental),
                hex_encode(&player.concentration),
                hex_encode(&player.order),
                hex_encode(&player.roaming),
                hex_encode(&player.aggressive),
                hex_encode(&player.ego),
            ]
            .join(":")
        })
        .collect::<Vec<_>>()
        .join(";");

    format!("OK|PLAYERS|{}|{}", players.len(), payload)
}

fn response_ok_staffs(staffs: &[StaffListEntry]) -> String {
    let payload = staffs
        .iter()
        .map(|staff| {
            [
                staff.id.to_string(),
                hex_encode(&staff.name),
                hex_encode(&staff.age),
                hex_encode(&staff.team),
                hex_encode(&staff.role),
                hex_encode(&staff.banpick),
                hex_encode(&staff.strategy),
                hex_encode(&staff.negotiation),
                hex_encode(&staff.judge_ability),
                hex_encode(&staff.judge_potential),
                hex_encode(&staff.feedback),
                hex_encode(&staff.power_analysis),
                hex_encode(&staff.control_coaching),
                hex_encode(&staff.judgment_coaching),
                hex_encode(&staff.mental_coaching),
                hex_encode(&staff.annual_salary),
                hex_encode(&staff.contract_end_date),
                hex_encode(&staff.communication),
            ]
            .join(":")
        })
        .collect::<Vec<_>>()
        .join(";");

    format!("OK|STAFFS|{}|{}", staffs.len(), payload)
}

fn response_ok_staff(staff: StaffSnapshot) -> String {
    format!(
        concat!(
            "OK|STAFF|{}|{}|{}|{}|{}|",
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|",
            "{}|{}|{}|{}|{}"
        ),
        staff.id,
        hex_encode(&staff.name),
        hex_encode(&staff.age),
        hex_encode(&staff.role),
        hex_encode(&staff.team),
        staff.banpick,
        staff.strategy,
        staff.negotiation,
        staff.judge_ability,
        staff.judge_potential,
        staff.feedback,
        staff.power_analysis,
        staff.control_coaching,
        staff.judgment_coaching,
        staff.mental_coaching,
        staff.annual_salary,
        staff.contract_team_id,
        hex_encode(&staff.contract_start_date),
        hex_encode(&staff.contract_end_date),
        hex_encode(&staff.communication_raw),
    )
}

fn response_ok_staff_contract_probe(raw: &str) -> String {
    format!("OK|STAFF_CONTRACT_PROBE|{}", hex_encode(raw))
}

fn response_ok_player_contract_probe(raw: &str) -> String {
    format!("OK|PLAYER_CONTRACT_PROBE|{}", hex_encode(raw))
}

fn response_ok_player_condition(snapshot: PlayerConditionSnapshot) -> String {
    format!(
        "OK|PLAYER_CONDITION|{}|{}|{}",
        snapshot.athlete_id, snapshot.stamina, snapshot.condition
    )
}

fn response_ok_teams(teams: &[TeamListEntry]) -> String {
    let payload = teams
        .iter()
        .map(|team| format!(
            "{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}:{}",
            team.id,
            hex_encode(&team.display_name),
            hex_encode(&team.manager_name),
            team.league_id,
            if team.is_player_team { 1 } else { 0 },
            team.roster_size,
            team.staff_count,
            team.roster_rating
                .map(|value| value.to_string())
                .unwrap_or_default(),
            hex_encode(&team.merchandise_facility_grade),
            hex_encode(&team.stadium_grade),
            hex_encode(&team.training_facility_grade),
            team.total_balance,
            team.transfer_budget,
            team.salary_budget,
            hex_encode(&team.stadium_name),
            team.stadium_capacity,
            team.total_home_attendance,
            team.home_match_count,
            team.total_entrance_income,
            team.popularity,
            hex_encode(&team.fan_expectation),
            hex_encode(&team.fan_satisfaction),
            team.fan_count,
            team.fan_momentum,
            hex_encode(&team.gaming_house_level),
            team.welfare,
        ))
        .collect::<Vec<_>>()
        .join(";");

    format!("OK|TEAMS|{}|{}", teams.len(), payload)
}

fn response_ok_team_probe(raw: &str) -> String {
    format!("OK|TEAM_PROBE|{}", hex_encode(raw))
}

fn response_ok_team_management(snapshot: TeamManagementSnapshot) -> String {
    format!(
        "OK|TEAM_MANAGEMENT|{}|{}|{}|{}|{}|{}|{}|{}",
        snapshot.team_id,
        hex_encode(&snapshot.management),
        hex_encode(&snapshot.current_strategy),
        hex_encode(&snapshot.last_strategy),
        hex_encode(&snapshot.team_color_strategy),
        hex_encode(&snapshot.merchandise),
        hex_encode(&snapshot.champion_setup),
        hex_encode(&snapshot.gaming_house),
    )
}

fn response_ok_team_strategy_options(raw: &str) -> String {
    format!("OK|TEAM_STRATEGY_OPTIONS|{}", hex_encode(raw))
}

fn response_ok_team_fan_momentum_probe(raw: &str) -> String {
    format!("OK|TEAM_FAN_MOMENTUM_PROBE|{}", hex_encode(raw))
}

fn response_ok_team_replay_strategies(raw: &str) -> String {
    format!("OK|TEAM_REPLAY_STRATEGIES|{}", hex_encode(raw))
}

fn response_ok_player(player: PlayerSnapshot) -> String {
    [
        "OK".to_string(),
        "PLAYER".to_string(),
        player.id.to_string(),
        hex_encode(&player.name),
        player.last_hit,
        player.skill_avoid,
        player.skill_hit,
        player.control_speed,
        player.positioning,
        player.judgement,
        player.mental,
        player.concentration,
        player.order,
        player.roaming,
        player.aggressive,
        player.ego,
        player.top,
        player.jungle,
        player.mid,
        player.bottom,
        player.support,
        player.potential,
        player.annual_salary,
        player.weekly_salary,
        player.contract_team_id,
        hex_encode(&player.contract_start_date),
        hex_encode(&player.contract_end_date),
        player.transfer_fee,
        hex_encode(&player.squad_status),
        player.incentive_pog_bonus,
        player.incentive_league_bonus,
        player.incentive_league_rank,
        player.incentive_match_bonus,
        player.incentive_win_bonus,
        player.primary_region,
        hex_encode(&player.communication_raw),
        hex_encode(&player.communication_xp_raw),
    ]
    .join("|")
}

fn read_economy(scene: &mut Scene) -> Result<EconomyValues, &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };

    let team_id = data.player_team_id();
    let mut db = data.db_mut();
    let Some(team) = db.teams.get_mut(&team_id) else {
        return Err("PLAYER_TEAM_NOT_FOUND");
    };

    Ok(EconomyValues {
        money: team.total_balance,
        transfer_budget: team.transfer_budget,
        salary_budget: team.salary_budget,
    })
}

fn apply_economy_to_team(team: &mut Team, values: EconomyValues) {
    team.total_balance = values.money;
    team.transfer_budget = values.transfer_budget;
    team.salary_budget = values.salary_budget;
}

fn economy_payload(team_id: usize, values: EconomyValues) -> Vec<u8> {
    format!(
        "{}|{}|{}|{}",
        team_id, values.money, values.transfer_budget, values.salary_budget
    )
    .into_bytes()
}

fn parse_server_economy(payload: &[u8]) -> Result<(usize, EconomyValues), &'static str> {
    let text = std::str::from_utf8(payload).map_err(|_| "INVALID_PAYLOAD")?;
    let mut parts = text.split('|');
    let team_id = parse_usize(parts.next())?;
    let values = EconomyValues {
        money: parse_f64(parts.next())?,
        transfer_budget: parse_f64(parts.next())?,
        salary_budget: parse_f64(parts.next())?,
    };
    Ok((team_id, values))
}

fn write_economy(scene: &mut Scene, values: EconomyValues) -> Result<EconomyValues, &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };

    let team_id = data.player_team_id();

    // ClientDatabase is only a client-side snapshot. Send the same values to the
    // authoritative management/server Database so normal spending, Proceed and
    // save/autosave continue from the edited economy rather than restoring the
    // old server values.
    if !data.send_mod_command(MOD_ID, "set_economy", economy_payload(team_id, values)) {
        return Err("SERVER_COMMAND_FAILED");
    }

    // Mirror the edit client-side immediately so the game UI and modifier update
    // without waiting for the next server synchronization.
    let mut db = data.db_mut();
    let Some(team) = db.teams.get_mut(&team_id) else {
        return Err("PLAYER_TEAM_NOT_FOUND");
    };

    apply_economy_to_team(team, values);

    Ok(EconomyValues {
        money: team.total_balance,
        transfer_budget: team.transfer_budget,
        salary_budget: team.salary_budget,
    })
}

fn stat_as_f64<T: ToString>(value: &T) -> f64 {
    value.to_string().parse::<f64>().unwrap_or(0.0)
}

fn player_region_name(athlete: &Athlete) -> String {
    match athlete.get_primary_region() {
        Some(0) => "Korea".to_string(),
        Some(1) => "China".to_string(),
        Some(2) => "Europe".to_string(),
        Some(3) => "North America".to_string(),
        Some(4) => "South America".to_string(),
        Some(5) => "Japan".to_string(),
        Some(id) => format!("Region {id}"),
        None => String::new(),
    }
}

fn player_position_summary(athlete: &Athlete) -> String {
    let stat = &athlete.stat;
    let mut positions = vec![
        ("Top", stat_as_f64(&stat.top)),
        ("Jungle", stat_as_f64(&stat.jungle)),
        ("Mid", stat_as_f64(&stat.mid)),
        ("Bottom", stat_as_f64(&stat.bottom)),
        ("Support", stat_as_f64(&stat.support)),
    ];

    positions.retain(|(_, value)| *value > 0.0);
    positions.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.0.cmp(b.0))
    });

    if positions.is_empty() {
        "None".to_string()
    } else {
        positions
            .into_iter()
            .take(3)
            .map(|(label, _)| label)
            .collect::<Vec<_>>()
            .join(" / ")
    }
}

fn read_players(scene: &mut Scene) -> Result<Vec<PlayerListEntry>, &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };

    let db = data.db();
    let player_research = &db.research_data;

    let mut players = db
        .athletes
        .iter()
        .map(|(id, athlete)| {
            let team = match &athlete.contract {
                Contract::InContract { team_id, .. } => db
                    .teams
                    .get(team_id)
                    .map(|team| db.team_display_name(team).to_string())
                    .unwrap_or_else(|| format!("Team {}", team_id)),
                Contract::FreeAgent { .. } => "Free Agent".to_string(),
            };

            let (actual_rating, potential_rating) = player_research
                .athlete_report
                .get(id)
                .map(|report| (report.stat_score.to_string(), report.potential_score.to_string()))
                .unwrap_or_else(|| (String::new(), String::new()));

            PlayerListEntry {
                id: *id,
                name: athlete.name.to_string(),
                age: athlete.age.to_string(),
                team,
                region: player_region_name(athlete),
                position: player_position_summary(athlete),
                actual_rating,
                potential_rating,
                actual_potential: athlete.hidden.potential.to_string(),
                annual_salary: annual_salary_raw(athlete),
                transfer_fee: transfer_fee_raw(athlete),
                contract_end_date: contract_end_date_raw(athlete),
                last_hit: athlete.stat.last_hit.to_string(),
                skill_avoid: athlete.stat.skill_avoid.to_string(),
                skill_hit: athlete.stat.skill_hit.to_string(),
                control_speed: athlete.stat.control_speed.to_string(),
                positioning: athlete.stat.positioning.to_string(),
                judgement: athlete.stat.judgement.to_string(),
                mental: athlete.stat.mental.to_string(),
                concentration: athlete.stat.concentration.to_string(),
                order: athlete.stat.order.to_string(),
                roaming: athlete.stat.roaming.to_string(),
                aggressive: athlete.stat.aggressive.to_string(),
                ego: athlete.stat.ego.to_string(),
            }
        })
        .collect::<Vec<_>>();

    players.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.id.cmp(&b.id))
    });

    Ok(players)
}

fn read_staffs(scene: &mut Scene) -> Result<Vec<StaffListEntry>, &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };

    let db = data.db();
    let mut staffs = db
        .staffs
        .iter()
        .map(|(id, staff)| {
            let team = match &staff.contract {
                Contract::InContract { team_id, .. } => db
                    .teams
                    .get(team_id)
                    .map(|team| db.team_display_name(team).to_string())
                    .unwrap_or_else(|| format!("Team {}", team_id)),
                Contract::FreeAgent { .. } => "Free Agent".to_string(),
            };

            let (annual_salary, contract_end_date) = match &staff.contract {
                Contract::InContract {
                    weekly_salary,
                    end_date,
                    ..
                } => (
                    weekly_salary
                        .to_string()
                        .parse::<f64>()
                        .map(|weekly| (weekly * 52.0).to_string())
                        .unwrap_or_default(),
                    end_date.to_string(),
                ),
                Contract::FreeAgent { .. } => (String::new(), String::new()),
            };

            let communication = staff
                .language
                .values()
                .filter_map(|value| value.to_string().parse::<f64>().ok())
                .max_by(|left, right| left.total_cmp(right))
                .map(|value| value.to_string())
                .unwrap_or_default();

            StaffListEntry {
                id: *id,
                name: staff.name.to_string(),
                age: staff.age.to_string(),
                team,
                role: format!("{:?}", staff.role),
                banpick: staff.stat.banpick.to_string(),
                strategy: staff.stat.strategy.to_string(),
                negotiation: staff.stat.negotiation.to_string(),
                judge_ability: staff.stat.judge_ability.to_string(),
                judge_potential: staff.stat.judge_potential.to_string(),
                feedback: staff.stat.feedback.to_string(),
                power_analysis: staff.stat.power_analysis.to_string(),
                control_coaching: staff.stat.control_coaching.to_string(),
                judgment_coaching: staff.stat.judgment_coaching.to_string(),
                mental_coaching: staff.stat.mental_coaching.to_string(),
                annual_salary,
                contract_end_date,
                communication,
            }
        })
        .collect::<Vec<_>>();

    staffs.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.id.cmp(&b.id))
    });

    Ok(staffs)
}

fn read_staff(scene: &mut Scene, staff_id: usize) -> Result<StaffSnapshot, &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };

    let db = data.db();
    let Some(staff) = db.staffs.get(&staff_id) else {
        return Err("STAFF_NOT_FOUND");
    };
    let stat = &staff.stat;

    let team = match &staff.contract {
        Contract::InContract { team_id, .. } => db
            .teams
            .get(team_id)
            .map(|team| db.team_display_name(team).to_string())
            .unwrap_or_else(|| format!("Team {}", team_id)),
        Contract::FreeAgent { .. } => "Free Agent".to_string(),
    };

    let (annual_salary, contract_team_id, contract_start_date, contract_end_date) =
        match &staff.contract {
            Contract::InContract {
                team_id,
                weekly_salary,
                start_date,
                end_date,
                ..
            } => (
                weekly_salary
                    .to_string()
                    .parse::<f64>()
                    .map(|weekly| (weekly * 52.0).to_string())
                    .unwrap_or_default(),
                team_id.to_string(),
                start_date.to_string(),
                end_date.to_string(),
            ),
            Contract::FreeAgent { .. } => (
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ),
        };

    let mut communication_entries = staff
        .language
        .iter()
        .map(|(region_id, value)| (*region_id, value.to_string()))
        .collect::<Vec<_>>();
    communication_entries.sort_by_key(|(region_id, _)| *region_id);
    let communication_raw = communication_entries
        .into_iter()
        .map(|(region_id, value)| format!("{}={}", region_id, value))
        .collect::<Vec<_>>()
        .join(";");

    Ok(StaffSnapshot {
        id: staff_id,
        name: staff.name.to_string(),
        age: staff.age.to_string(),
        role: format!("{:?}", staff.role),
        team,
        banpick: stat.banpick.to_string(),
        strategy: stat.strategy.to_string(),
        negotiation: stat.negotiation.to_string(),
        judge_ability: stat.judge_ability.to_string(),
        judge_potential: stat.judge_potential.to_string(),
        feedback: stat.feedback.to_string(),
        power_analysis: stat.power_analysis.to_string(),
        control_coaching: stat.control_coaching.to_string(),
        judgment_coaching: stat.judgment_coaching.to_string(),
        mental_coaching: stat.mental_coaching.to_string(),
        annual_salary,
        contract_team_id,
        contract_start_date,
        contract_end_date,
        communication_raw,
    })
}

fn write_staff_name(
    scene: &mut Scene,
    staff_id: usize,
    values: EntityNameValue,
) -> Result<(), &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };

    let name = validate_entity_name(&values.name)?;
    let values = EntityNameValue { name };

    if !data.send_mod_command(
        MOD_ID,
        "set_staff_name",
        entity_name_payload(staff_id, &values),
    ) {
        return Err("SERVER_COMMAND_FAILED");
    }

    let mut db = data.db_mut();
    let Some(staff) = db.staffs.get_mut(&staff_id) else {
        return Err("STAFF_NOT_FOUND");
    };
    staff.name = values.name;
    Ok(())
}

fn validate_staff_stats(values: &StaffStatValues) -> Result<(), &'static str> {
    for value in [
        &values.banpick,
        &values.strategy,
        &values.negotiation,
        &values.judge_ability,
        &values.judge_potential,
        &values.feedback,
        &values.power_analysis,
        &values.control_coaching,
        &values.judgment_coaching,
        &values.mental_coaching,
    ] {
        validate_stat_text(value)?;
    }
    Ok(())
}

fn staff_stats_payload(staff_id: usize, values: &StaffStatValues) -> Vec<u8> {
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        staff_id,
        values.banpick,
        values.strategy,
        values.negotiation,
        values.judge_ability,
        values.judge_potential,
        values.feedback,
        values.power_analysis,
        values.control_coaching,
        values.judgment_coaching,
        values.mental_coaching,
    )
    .into_bytes()
}

fn parse_server_staff_stats(payload: &[u8]) -> Result<(usize, StaffStatValues), &'static str> {
    let text = std::str::from_utf8(payload).map_err(|_| "INVALID_PAYLOAD")?;
    let mut parts = text.split('|');
    let staff_id = parse_usize(parts.next())?;
    let values = StaffStatValues {
        banpick: parts.next().ok_or("MISSING_VALUE")?.to_string(),
        strategy: parts.next().ok_or("MISSING_VALUE")?.to_string(),
        negotiation: parts.next().ok_or("MISSING_VALUE")?.to_string(),
        judge_ability: parts.next().ok_or("MISSING_VALUE")?.to_string(),
        judge_potential: parts.next().ok_or("MISSING_VALUE")?.to_string(),
        feedback: parts.next().ok_or("MISSING_VALUE")?.to_string(),
        power_analysis: parts.next().ok_or("MISSING_VALUE")?.to_string(),
        control_coaching: parts.next().ok_or("MISSING_VALUE")?.to_string(),
        judgment_coaching: parts.next().ok_or("MISSING_VALUE")?.to_string(),
        mental_coaching: parts.next().ok_or("MISSING_VALUE")?.to_string(),
    };
    validate_staff_stats(&values)?;
    Ok((staff_id, values))
}

fn write_staff_stats(
    scene: &mut Scene,
    staff_id: usize,
    values: StaffStatValues,
) -> Result<(), &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };

    validate_staff_stats(&values)?;

    if !data.send_mod_command(
        MOD_ID,
        "set_staff_stats",
        staff_stats_payload(staff_id, &values),
    ) {
        return Err("SERVER_COMMAND_FAILED");
    }

    // Mirror the authoritative write into the current client snapshot so the
    // Staff Editor updates immediately instead of waiting for the next sync.
    let mut db = data.db_mut();
    let Some(staff) = db.staffs.get_mut(&staff_id) else {
        return Err("STAFF_NOT_FOUND");
    };

    let stat = &mut staff.stat;
    stat.banpick = parse_stat_value(&values.banpick)?;
    stat.strategy = parse_stat_value(&values.strategy)?;
    stat.negotiation = parse_stat_value(&values.negotiation)?;
    stat.judge_ability = parse_stat_value(&values.judge_ability)?;
    stat.judge_potential = parse_stat_value(&values.judge_potential)?;
    stat.feedback = parse_stat_value(&values.feedback)?;
    stat.power_analysis = parse_stat_value(&values.power_analysis)?;
    stat.control_coaching = parse_stat_value(&values.control_coaching)?;
    stat.judgment_coaching = parse_stat_value(&values.judgment_coaching)?;
    stat.mental_coaching = parse_stat_value(&values.mental_coaching)?;
    Ok(())
}


fn staff_salary_payload(staff_id: usize, values: &StaffSalaryValue) -> Vec<u8> {
    format!("{}|{}", staff_id, values.annual_salary).into_bytes()
}

fn parse_server_staff_salary(
    payload: &[u8],
) -> Result<(usize, StaffSalaryValue), &'static str> {
    let text = std::str::from_utf8(payload).map_err(|_| "INVALID_PAYLOAD")?;
    let (staff_id, annual_salary) = text.split_once('|').ok_or("MISSING_VALUE")?;
    let values = StaffSalaryValue {
        annual_salary: annual_salary.to_string(),
    };
    validate_annual_salary(&values.annual_salary)?;
    Ok((
        staff_id.parse::<usize>().map_err(|_| "INVALID_ID")?,
        values,
    ))
}

fn apply_staff_salary_contract(
    contract: &mut Contract,
    values: &StaffSalaryValue,
) -> Result<(), &'static str> {
    let annual = validate_annual_salary(&values.annual_salary)?;
    let weekly_text = (annual / 52.0).to_string();
    match contract {
        Contract::InContract { weekly_salary, .. } => {
            *weekly_salary = weekly_text
                .parse()
                .map_err(|_| "SALARY_TYPE_ERROR")?;
            Ok(())
        }
        Contract::FreeAgent { .. } => Err("STAFF_FREE_AGENT"),
    }
}

fn write_staff_salary(
    scene: &mut Scene,
    staff_id: usize,
    values: StaffSalaryValue,
) -> Result<(), &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };

    validate_annual_salary(&values.annual_salary)?;
    {
        let db = data.db();
        let Some(staff) = db.staffs.get(&staff_id) else {
            return Err("STAFF_NOT_FOUND");
        };
        if matches!(&staff.contract, Contract::FreeAgent { .. }) {
            return Err("STAFF_FREE_AGENT");
        }
    }

    if !data.send_mod_command(
        MOD_ID,
        "set_staff_salary",
        staff_salary_payload(staff_id, &values),
    ) {
        return Err("SERVER_COMMAND_FAILED");
    }

    let mut db = data.db_mut();
    let Some(staff) = db.staffs.get_mut(&staff_id) else {
        return Err("STAFF_NOT_FOUND");
    };
    apply_staff_salary_contract(&mut staff.contract, &values)
}

fn staff_contract_end_payload(staff_id: usize, values: &StaffContractEndValue) -> Vec<u8> {
    format!("{}|{}", staff_id, values.end_date).into_bytes()
}

fn write_staff_contract_end(
    scene: &mut Scene,
    staff_id: usize,
    values: StaffContractEndValue,
) -> Result<StaffSnapshot, &'static str> {
    validate_contract_end_date_text(&values.end_date)?;
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };

    if !data.send_mod_command(
        MOD_ID,
        "set_staff_contract_end",
        staff_contract_end_payload(staff_id, &values),
    ) {
        return Err("SERVER_COMMAND_FAILED");
    }

    let mut db = data.db_mut();
    let Some(staff) = db.staffs.get_mut(&staff_id) else {
        return Err("STAFF_NOT_FOUND");
    };
    apply_staff_contract_end(&mut staff.contract, &values)?;
    drop(db);
    read_staff(scene, staff_id)
}

fn parse_server_staff_contract_end(
    payload: &[u8],
) -> Result<(usize, StaffContractEndValue), &'static str> {
    let text = std::str::from_utf8(payload).map_err(|_| "INVALID_PAYLOAD")?;
    let mut parts = text.split('|');
    let staff_id = parse_usize(parts.next())?;
    let end_date = parts.next().ok_or("MISSING_VALUE")?.to_string();
    validate_contract_end_date_text(&end_date)?;
    Ok((staff_id, StaffContractEndValue { end_date }))
}

fn validate_staff_communication(
    values: &StaffCommunicationValues,
) -> Result<(), &'static str> {
    if values.entries.is_empty() {
        return Err("NO_COMMUNICATION_REGIONS");
    }

    let mut seen = HashSet::new();
    for (region_id, value) in &values.entries {
        if !seen.insert(*region_id) {
            return Err("DUPLICATE_REGION");
        }
        if *value > 100 {
            return Err("COMMUNICATION_OUT_OF_RANGE");
        }
    }
    Ok(())
}

fn staff_communication_payload(
    staff_id: usize,
    values: &StaffCommunicationValues,
) -> Vec<u8> {
    let entries = values
        .entries
        .iter()
        .map(|(region_id, value)| format!("{}={}", region_id, value))
        .collect::<Vec<_>>()
        .join(";");
    format!("{}|{}", staff_id, entries).into_bytes()
}

fn parse_staff_communication_entries(
    raw: &str,
) -> Result<StaffCommunicationValues, &'static str> {
    let mut entries = Vec::new();
    for entry in raw.split(';').filter(|entry| !entry.trim().is_empty()) {
        let (region_id, value) = entry.split_once('=').ok_or("INVALID_COMMUNICATION")?;
        entries.push((
            region_id.parse::<usize>().map_err(|_| "INVALID_REGION")?,
            value.parse::<u16>().map_err(|_| "INVALID_COMMUNICATION")?,
        ));
    }
    let values = StaffCommunicationValues { entries };
    validate_staff_communication(&values)?;
    Ok(values)
}

fn parse_server_staff_communication(
    payload: &[u8],
) -> Result<(usize, StaffCommunicationValues), &'static str> {
    let text = std::str::from_utf8(payload).map_err(|_| "INVALID_PAYLOAD")?;
    let (staff_id, entries) = text.split_once('|').ok_or("MISSING_VALUE")?;
    Ok((
        staff_id.parse::<usize>().map_err(|_| "INVALID_ID")?,
        parse_staff_communication_entries(entries)?,
    ))
}

fn write_staff_communication(
    scene: &mut Scene,
    staff_id: usize,
    values: StaffCommunicationValues,
) -> Result<(), &'static str> {
    validate_staff_communication(&values)?;

    let available_regions = detected_region_ids(scene)?;
    for (region_id, _) in &values.entries {
        if !available_regions.contains(region_id) {
            return Err("INVALID_REGION");
        }
    }

    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };

    {
        let db = data.db();
        if !db.staffs.contains_key(&staff_id) {
            return Err("STAFF_NOT_FOUND");
        }
    }

    if !data.send_mod_command(
        MOD_ID,
        "set_staff_communication",
        staff_communication_payload(staff_id, &values),
    ) {
        return Err("SERVER_COMMAND_FAILED");
    }

    let mut db = data.db_mut();
    let Some(staff) = db.staffs.get_mut(&staff_id) else {
        return Err("STAFF_NOT_FOUND");
    };
    for (region_id, value) in &values.entries {
        let parsed = value
            .to_string()
            .parse()
            .map_err(|_| "COMMUNICATION_TYPE_ERROR")?;
        staff.language.insert(*region_id, parsed);
    }
    Ok(())
}

fn read_staff_contract_probe(
    scene: &mut Scene,
    staff_id: usize,
) -> Result<String, &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };
    let db = data.db();
    let Some(staff) = db.staffs.get(&staff_id) else {
        return Err("STAFF_NOT_FOUND");
    };

    let mut raw = format!(
        "=== STAFF CONTRACT FLOW PROBE ===\nStaff key: {staff_id}\n\n=== FULL STAFF RECORD ===\n{staff:#?}"
    );
    if let Contract::InContract { team_id, .. } = &staff.contract {
        if let Some(team) = db.teams.get(team_id) {
            raw.push_str(&format!(
                "\n\n=== CURRENT CONTRACT TEAM RECORD ===\nTeam key: {team_id}\n{team:#?}"
            ));
        }
    }
    Ok(raw)
}

fn read_player_contract_probe(
    scene: &mut Scene,
    athlete_id: usize,
) -> Result<String, &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };
    let db = data.db();
    let Some(athlete) = db.athletes.get(&athlete_id) else {
        return Err("PLAYER_NOT_FOUND");
    };

    let mut raw = format!(
        "=== PLAYER CONTRACT FLOW PROBE ===\nAthlete key: {athlete_id}\n\n=== FULL ATHLETE RECORD ===\n{athlete:#?}"
    );
    if let Contract::InContract { team_id, .. } = &athlete.contract {
        if let Some(team) = db.teams.get(team_id) {
            raw.push_str(&format!(
                "\n\n=== CURRENT CONTRACT TEAM RECORD ===\nTeam key: {team_id}\n{team:#?}"
            ));
        }
    }
    Ok(raw)
}

fn read_team_probe(scene: &mut Scene, team_id: usize) -> Result<String, &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };

    let db = data.db();
    let Some(team) = db.teams.get(&team_id) else {
        return Err("TEAM_NOT_FOUND");
    };

    let mut raw = format!(
        "=== SELECTED TEAM RECORD ===\nTeam key: {team_id}\nDisplay name: {}\n{team:#?}",
        db.team_display_name(team)
    );
    raw.push_str("\n\n");
    raw.push_str(&global_history_robustness_report());
    Ok(raw)
}

fn sanitize_tsv_cell(value: &str) -> String {
    value
        .replace(['\t', '\r', '\n'], " ")
}

fn append_tsv_row(output: &mut String, cells: &[String]) {
    if !output.is_empty() {
        output.push('\n');
    }
    output.push_str(
        &cells
            .iter()
            .map(|cell| sanitize_tsv_cell(cell))
            .collect::<Vec<_>>()
            .join("\t"),
    );
}

fn debug_option_or_auto<T: std::fmt::Debug>(value: &Option<T>) -> String {
    value
        .as_ref()
        .map(|entry| format!("{entry:?}"))
        .unwrap_or_else(|| "Auto".to_string())
}

fn read_team_management(
    scene: &mut Scene,
    team_id: usize,
) -> Result<TeamManagementSnapshot, &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };

    let db = data.db();
    let Some(team) = db.teams.get(&team_id) else {
        return Err("TEAM_NOT_FOUND");
    };

    let mut management = String::new();
    let lineup_slots = ["Top", "Jungle", "Mid", "Bottom", "Support"];
    for (index, athlete_id) in team.last_starting.iter().enumerate() {
        let slot = lineup_slots.get(index).copied().unwrap_or("Unknown");
        match athlete_id {
            Some(athlete_id) => {
                let name = db
                    .athletes
                    .get(athlete_id)
                    .map(|athlete| athlete.name.to_string())
                    .unwrap_or_else(|| format!("Player {athlete_id}"));
                append_tsv_row(
                    &mut management,
                    &[
                        "lineup".to_string(),
                        slot.to_string(),
                        athlete_id.to_string(),
                        name,
                    ],
                );
            }
            None => append_tsv_row(
                &mut management,
                &[
                    "lineup".to_string(),
                    slot.to_string(),
                    String::new(),
                    String::new(),
                ],
            ),
        }
    }

    for athlete_id in &team.watched_athletes {
        let name = db
            .athletes
            .get(athlete_id)
            .map(|athlete| athlete.name.to_string())
            .unwrap_or_else(|| format!("Player {athlete_id}"));
        append_tsv_row(
            &mut management,
            &["watched_player".to_string(), athlete_id.to_string(), name],
        );
    }
    for athlete_id in &team.no_transfer_athletes {
        let name = db
            .athletes
            .get(athlete_id)
            .map(|athlete| athlete.name.to_string())
            .unwrap_or_else(|| format!("Player {athlete_id}"));
        append_tsv_row(
            &mut management,
            &["no_transfer_player".to_string(), athlete_id.to_string(), name],
        );
    }
    for athlete_id in &team.release_list_athletes {
        let name = db
            .athletes
            .get(athlete_id)
            .map(|athlete| athlete.name.to_string())
            .unwrap_or_else(|| format!("Player {athlete_id}"));
        append_tsv_row(
            &mut management,
            &["release_player".to_string(), athlete_id.to_string(), name],
        );
    }
    for staff_id in &team.watched_staffs {
        let name = db
            .staffs
            .get(staff_id)
            .map(|staff| staff.name.to_string())
            .unwrap_or_else(|| format!("Staff {staff_id}"));
        append_tsv_row(
            &mut management,
            &["watched_staff".to_string(), staff_id.to_string(), name],
        );
    }
    for staff_id in &team.release_list_staffs {
        let name = db
            .staffs
            .get(staff_id)
            .map(|staff| staff.name.to_string())
            .unwrap_or_else(|| format!("Staff {staff_id}"));
        append_tsv_row(
            &mut management,
            &["release_staff".to_string(), staff_id.to_string(), name],
        );
    }

    let management_metrics = [
        ("pending_installments", team.pending_installments.len().to_string()),
        ("resale_clauses", team.resale_clauses.len().to_string()),
        (
            "scout_dispatch",
            if team.scout_dispatch.is_some() {
                "Active".to_string()
            } else {
                "None".to_string()
            },
        ),
        ("merchandise_products", team.merchandise_products.len().to_string()),
        ("champion_tiers", team.champion_tiers.len().to_string()),
        (
            "personal_tactics",
            team.champion_personal_tactics.len().to_string(),
        ),
    ];
    for (key, value) in management_metrics {
        append_tsv_row(
            &mut management,
            &["metric".to_string(), key.to_string(), value],
        );
    }

    let mut current_strategy = String::new();
    let current_values = [
        ("focused", format!("{:?}", team.strategy.focused)),
        ("early_jungle", format!("{:?}", team.strategy.early_jungle)),
        ("early_serpen", format!("{:?}", team.strategy.early_serpen)),
        (
            "early_serpen_top",
            format!("{:?}", team.strategy.early_serpen_top),
        ),
        (
            "object_buildup",
            format!("{:?}", team.strategy.object_buildup),
        ),
        (
            "object_battle",
            format!("{:?}", team.strategy.object_battle),
        ),
        ("morgard_use", format!("{:?}", team.strategy.morgard_use)),
        ("tower_press", format!("{:?}", team.strategy.tower_press)),
        (
            "morgard_defense",
            format!("{:?}", team.strategy.morgard_defense),
        ),
        (
            "object_finish",
            format!("{:?}", team.strategy.object_finish),
        ),
        ("minion_wave", format!("{:?}", team.strategy.minion_wave)),
        ("game_finish", format!("{:?}", team.strategy.game_finish)),
    ];
    for (key, value) in current_values {
        append_tsv_row(&mut current_strategy, &[key.to_string(), value]);
    }

    let mut last_strategy = String::new();
    let last_values = [
        ("focused", format!("{:?}", team.last_strategy.focused)),
        (
            "early_jungle",
            format!("{:?}", team.last_strategy.early_jungle),
        ),
        (
            "early_serpen",
            format!("{:?}", team.last_strategy.early_serpen),
        ),
        (
            "early_serpen_top",
            format!("{:?}", team.last_strategy.early_serpen_top),
        ),
        (
            "object_buildup",
            format!("{:?}", team.last_strategy.object_buildup),
        ),
        (
            "object_battle",
            format!("{:?}", team.last_strategy.object_battle),
        ),
        (
            "morgard_use",
            format!("{:?}", team.last_strategy.morgard_use),
        ),
        (
            "tower_press",
            format!("{:?}", team.last_strategy.tower_press),
        ),
        (
            "morgard_defense",
            format!("{:?}", team.last_strategy.morgard_defense),
        ),
        (
            "object_finish",
            format!("{:?}", team.last_strategy.object_finish),
        ),
        (
            "minion_wave",
            format!("{:?}", team.last_strategy.minion_wave),
        ),
        (
            "game_finish",
            format!("{:?}", team.last_strategy.game_finish),
        ),
    ];
    for (key, value) in last_values {
        append_tsv_row(&mut last_strategy, &[key.to_string(), value]);
    }

    let mut team_color_strategy = String::new();
    let color_values = [
        ("focused", debug_option_or_auto(&team.team_color_strategy.focused)),
        (
            "early_jungle",
            debug_option_or_auto(&team.team_color_strategy.early_jungle),
        ),
        (
            "early_serpen",
            debug_option_or_auto(&team.team_color_strategy.early_serpen),
        ),
        (
            "early_serpen_top",
            debug_option_or_auto(&team.team_color_strategy.early_serpen_top),
        ),
        (
            "object_buildup",
            debug_option_or_auto(&team.team_color_strategy.object_buildup),
        ),
        (
            "object_battle",
            debug_option_or_auto(&team.team_color_strategy.object_battle),
        ),
        (
            "morgard_use",
            debug_option_or_auto(&team.team_color_strategy.morgard_use),
        ),
        (
            "tower_press",
            debug_option_or_auto(&team.team_color_strategy.tower_press),
        ),
        (
            "morgard_defense",
            debug_option_or_auto(&team.team_color_strategy.morgard_defense),
        ),
        (
            "object_finish",
            debug_option_or_auto(&team.team_color_strategy.object_finish),
        ),
        (
            "minion_wave",
            debug_option_or_auto(&team.team_color_strategy.minion_wave),
        ),
        (
            "game_finish",
            debug_option_or_auto(&team.team_color_strategy.game_finish),
        ),
    ];
    for (key, value) in color_values {
        append_tsv_row(&mut team_color_strategy, &[key.to_string(), value]);
    }

    let mut merchandise = String::new();
    for product in &team.merchandise_products {
        let athlete_name = db
            .athletes
            .get(&product.athlete_id)
            .map(|athlete| athlete.name.to_string())
            .unwrap_or_else(|| format!("Player {}", product.athlete_id));
        append_tsv_row(
            &mut merchandise,
            &[
                product.product_type.to_string(),
                product.athlete_id.to_string(),
                athlete_name,
                product.stock.to_string(),
                product.sell_price.to_string(),
                product.yearly_sales.to_string(),
                product.yearly_revenue.to_string(),
                product.total_sales.to_string(),
                product.total_revenue.to_string(),
                product.daily_purchase_rate.to_string(),
            ],
        );
    }

    let mut champion_ids = BTreeSet::new();
    champion_ids.extend(team.champion_tiers.keys().cloned());
    champion_ids.extend(team.champion_personal_tactics.keys().cloned());
    let mut champion_setup = String::new();
    for champion_id in champion_ids {
        let tier = team
            .champion_tiers
            .get(&champion_id)
            .map(|value| format!("{value:?}"))
            .unwrap_or_default();
        let tactics = team.champion_personal_tactics.get(&champion_id);
        let tactic_1 = tactics
            .and_then(|values| values.first())
            .map(|value| format!("{value:?}"))
            .unwrap_or_default();
        let tactic_2 = tactics
            .and_then(|values| values.get(1))
            .map(|value| format!("{value:?}"))
            .unwrap_or_default();
        let tactic_3 = tactics
            .and_then(|values| values.get(2))
            .map(|value| format!("{value:?}"))
            .unwrap_or_default();
        append_tsv_row(
            &mut champion_setup,
            &[champion_id, tier, tactic_1, tactic_2, tactic_3],
        );
    }

    let inventory = &team.gaming_house_inventory;
    let customization = &team.gaming_house_customization;
    let owned_furniture_total = inventory
        .furniture
        .iter()
        .map(|item| item.count)
        .sum::<usize>();
    let owned_wallpaper_total = inventory
        .wallpapers
        .iter()
        .map(|item| item.count)
        .sum::<usize>();
    let owned_wall_total = inventory
        .walls
        .iter()
        .map(|item| item.count)
        .sum::<usize>();
    let owned_window_total = inventory
        .windows
        .iter()
        .map(|item| item.count)
        .sum::<usize>();

    let mut gaming_house = String::new();
    let gaming_metrics = [
        ("level", format!("{:?}", team.gaming_house_level)),
        ("welfare", team.welfare.to_string()),
        ("owned_furniture_types", inventory.furniture.len().to_string()),
        ("owned_furniture_total", owned_furniture_total.to_string()),
        ("owned_wallpaper_types", inventory.wallpapers.len().to_string()),
        ("owned_wallpaper_total", owned_wallpaper_total.to_string()),
        ("owned_wall_types", inventory.walls.len().to_string()),
        ("owned_wall_total", owned_wall_total.to_string()),
        ("owned_window_types", inventory.windows.len().to_string()),
        ("owned_window_total", owned_window_total.to_string()),
        ("placed_furniture", customization.furniture.len().to_string()),
        ("placed_wallpapers", customization.wallpapers.len().to_string()),
        ("placed_walls", customization.walls.len().to_string()),
        ("placed_windows", customization.windows.len().to_string()),
    ];
    for (key, value) in gaming_metrics {
        append_tsv_row(&mut gaming_house, &[key.to_string(), value]);
    }

    Ok(TeamManagementSnapshot {
        team_id,
        management,
        current_strategy,
        last_strategy,
        team_color_strategy,
        merchandise,
        champion_setup,
        gaming_house,
    })
}

fn read_teams(scene: &mut Scene) -> Result<Vec<TeamListEntry>, &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };

    let player_team_id = data.player_team_id();
    let db = data.db();
    let mut teams = db
        .teams
        .iter()
        .map(|(id, team)| {
            let roster = db
                .athletes
                .values()
                .filter(|athlete| {
                    matches!(
                        &athlete.contract,
                        Contract::InContract { team_id, .. } if *team_id == *id
                    )
                })
                .collect::<Vec<_>>();
            let roster_size = roster.len();
            let roster_player_fans = roster
                .iter()
                .filter_map(|athlete| athlete.management.fan_count.to_string().parse::<u128>().ok())
                .sum::<u128>();
            let displayed_fan_count = team
                .fan_count
                .to_string()
                .parse::<u128>()
                .ok()
                .and_then(|base| displayed_team_fan_count(base, roster_player_fans).ok())
                .map(|value| value.to_string())
                .unwrap_or_else(|| team.fan_count.to_string());
            let staff_count = db
                .staffs
                .iter()
                .filter(|(_, staff)| {
                    matches!(
                        &staff.contract,
                        Contract::InContract { team_id, .. } if *team_id == *id
                    )
                })
                .count();

            let roster_rating = if roster.is_empty() {
                None
            } else {
                let total = roster
                    .iter()
                    .map(|athlete| {
                        [
                            athlete.stat.last_hit.to_string(),
                            athlete.stat.skill_avoid.to_string(),
                            athlete.stat.skill_hit.to_string(),
                            athlete.stat.control_speed.to_string(),
                            athlete.stat.positioning.to_string(),
                            athlete.stat.judgement.to_string(),
                            athlete.stat.mental.to_string(),
                            athlete.stat.concentration.to_string(),
                            athlete.stat.order.to_string(),
                            athlete.stat.roaming.to_string(),
                            athlete.stat.aggressive.to_string(),
                            athlete.stat.ego.to_string(),
                        ]
                        .iter()
                        .filter_map(|value| value.parse::<f64>().ok())
                        .sum::<f64>()
                            / 12.0
                    })
                    .sum::<f64>();
                Some(total / roster_size as f64)
            };

            TeamListEntry {
                id: *id,
                display_name: db.team_display_name(team).to_string(),
                manager_name: team.manager_name.to_string(),
                league_id: team.league_id,
                is_player_team: *id == player_team_id,
                roster_size,
                staff_count,
                roster_rating,
                merchandise_facility_grade: format!("{:?}", team.merchandise_facility_grade),
                stadium_grade: format!("{:?}", team.stadium.grade),
                training_facility_grade: format!("{:?}", team.training_facility_grade),
                stadium_name: team.stadium.name.to_string(),
                stadium_capacity: team.stadium.capacity.to_string(),
                total_home_attendance: team.total_home_attendance.to_string(),
                home_match_count: team.home_match_count.to_string(),
                total_entrance_income: team.total_entrance_income.to_string(),
                popularity: team.popularity.to_string(),
                fan_expectation: format!("{:?}", team.fan_expectation),
                fan_satisfaction: format!("{:?}", team.fan_satisfaction),
                fan_count: displayed_fan_count,
                fan_momentum: team.fan_momentum.to_string(),
                gaming_house_level: format!("{:?}", team.gaming_house_level),
                welfare: team.welfare.to_string(),
                total_balance: team.total_balance,
                transfer_budget: team.transfer_budget,
                salary_budget: team.salary_budget,
            }
        })
        .collect::<Vec<_>>();

    teams.sort_by(|a, b| {
        b.is_player_team
            .cmp(&a.is_player_team)
            .then_with(|| a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()))
            .then_with(|| a.league_id.cmp(&b.league_id))
            .then_with(|| a.id.cmp(&b.id))
    });

    Ok(teams)
}


fn global_history_state() -> &'static Mutex<Option<GlobalHistorySnapshot>> {
    GLOBAL_HISTORY_SNAPSHOT.get_or_init(|| Mutex::new(None))
}

fn global_history_response_metrics() -> &'static Mutex<GlobalHistoryResponseMetrics> {
    GLOBAL_HISTORY_RESPONSE_METRICS.get_or_init(|| Mutex::new(GlobalHistoryResponseMetrics::default()))
}

fn duration_micros_u64(duration: Duration) -> u64 {
    u64::try_from(duration.as_micros()).unwrap_or(u64::MAX)
}

fn record_global_history_response_metric(
    kind: &str,
    requested_id: Option<usize>,
    records_returned: usize,
    response_bytes: usize,
    started: Instant,
) {
    let metric = GlobalHistoryRequestMetric {
        requested_id,
        records_returned,
        response_bytes,
        response_micros: duration_micros_u64(started.elapsed()),
    };
    if let Ok(mut metrics) = global_history_response_metrics().lock() {
        match kind {
            "GET_LEAGUES" => metrics.get_leagues = Some(metric),
            "GET_LEAGUE_COMPETITION" => metrics.get_league_competition = Some(metric),
            "GET_TEAM_SCHEDULE" => metrics.get_team_schedule = Some(metric),
            "GET_TEAM_HISTORY" => metrics.get_team_history = Some(metric),
            _ => {}
        }
    }
}

fn format_global_history_request_metric(
    label: &str,
    metric: &Option<GlobalHistoryRequestMetric>,
) -> String {
    let Some(metric) = metric else {
        return format!("{label}: not requested yet");
    };
    let requested = metric
        .requested_id
        .map(|value| format!(" requested_id={value}"))
        .unwrap_or_default();
    format!(
        "{label}:{requested} records={} response_bytes={} response_time_us={}",
        metric.records_returned, metric.response_bytes, metric.response_micros
    )
}

fn global_history_robustness_report() -> String {
    let capture = global_history_state()
        .lock()
        .ok()
        .and_then(|state| state.as_ref().map(|snapshot| (snapshot.capture_index, snapshot.metrics.clone())));
    let responses = global_history_response_metrics()
        .lock()
        .map(|metrics| metrics.clone())
        .unwrap_or_default();

    let mut report = String::from("=== GLOBAL HISTORY ROBUSTNESS ===\n");
    if let Some((capture_index, metrics)) = capture {
        report.push_str(&format!(
            "capture_index={capture_index} record_cap={GLOBAL_HISTORY_RECORD_CAP} capture_time_us={}\n",
            metrics.capture_micros
        ));
        report.push_str(&format!(
            "leagues: source={} retained={} bytes={}\n",
            metrics.league_source_records, metrics.league_retained_records, metrics.league_bytes
        ));
        report.push_str(&format!(
            "league_competitions: source={} retained={} bytes={}\n",
            metrics.league_competition_source_records,
            metrics.league_competition_retained_records,
            metrics.league_competition_bytes
        ));
        report.push_str(&format!(
            "matches: source={} scanned={} retained={} dropped={} bytes={}\n",
            metrics.match_source_records,
            metrics.match_scanned_records,
            metrics.match_retained_records,
            metrics.match_dropped_records,
            metrics.match_bytes
        ));
        report.push_str(&format!(
            "match_window: policy=highest_id_recent oldest_id={} newest_id={} indexed_teams={} index_entries={}\n",
            metrics
                .match_oldest_retained_id
                .map(|value| value.to_string())
                .unwrap_or_else(|| "none".to_string()),
            metrics
                .match_newest_retained_id
                .map(|value| value.to_string())
                .unwrap_or_else(|| "none".to_string()),
            metrics.match_indexed_teams,
            metrics.match_index_entries
        ));
        report.push_str(&format!(
            "snapshot: retained_records={} bytes={} largest_record_bytes={}\n",
            metrics
                .league_retained_records
                .saturating_add(metrics.league_competition_retained_records)
                .saturating_add(metrics.match_retained_records),
            metrics.snapshot_bytes,
            metrics.largest_record_bytes
        ));
    } else {
        report.push_str("capture: not ready\n");
    }
    report.push_str(&format!(
        "{}\n{}\n{}\n{}",
        format_global_history_request_metric("GET_LEAGUES", &responses.get_leagues),
        format_global_history_request_metric(
            "GET_LEAGUE_COMPETITION",
            &responses.get_league_competition
        ),
        format_global_history_request_metric("GET_TEAM_SCHEDULE", &responses.get_team_schedule),
        format_global_history_request_metric("GET_TEAM_HISTORY", &responses.get_team_history),
    ));
    report
}

fn clear_global_history_snapshot() {
    GLOBAL_HISTORY_CAPTURE_INDEX.store(0, Ordering::SeqCst);
    if let Ok(mut state) = global_history_state().lock() {
        *state = None;
    }
    if let Ok(mut metrics) = global_history_response_metrics().lock() {
        *metrics = GlobalHistoryResponseMetrics::default();
    }
}

fn global_json_with_id(
    record_id: usize,
    mut value: serde_json::Value,
) -> Result<String, &'static str> {
    let object = value.as_object_mut().ok_or("GLOBAL_RECORD_NOT_OBJECT")?;
    object.insert(
        "id".to_string(),
        serde_json::Value::Number(serde_json::Number::from(record_id as u64)),
    );

    // TFM2 0.5.5 production Bridge: keep serde_json for Value construction,
    // then encode text with the Bridge-local serializer used by global history.
    Ok(global_json_value_to_string(&value))
}

fn global_json_write_string(output: &mut String, value: &str) {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    output.push('"');
    for ch in value.chars() {
        match ch {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{08}' => output.push_str("\\b"),
            '\u{0c}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            ch if ch <= '\u{1f}' => {
                let code = ch as u32;
                output.push_str("\\u00");
                output.push(HEX[((code >> 4) & 0x0f) as usize] as char);
                output.push(HEX[(code & 0x0f) as usize] as char);
            }
            ch => output.push(ch),
        }
    }
    output.push('"');
}

fn global_json_write_value(output: &mut String, value: &serde_json::Value) {
    match value {
        serde_json::Value::Null => output.push_str("null"),
        serde_json::Value::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
        serde_json::Value::Number(value) => output.push_str(&value.to_string()),
        serde_json::Value::String(value) => global_json_write_string(output, value),
        serde_json::Value::Array(values) => {
            output.push('[');
            for (index, value) in values.iter().enumerate() {
                if index != 0 {
                    output.push(',');
                }
                global_json_write_value(output, value);
            }
            output.push(']');
        }
        serde_json::Value::Object(values) => {
            output.push('{');
            for (index, (key, value)) in values.iter().enumerate() {
                if index != 0 {
                    output.push(',');
                }
                global_json_write_string(output, key);
                output.push(':');
                global_json_write_value(output, value);
            }
            output.push('}');
        }
    }
}

fn global_json_value_to_string(value: &serde_json::Value) -> String {
    let mut output = String::new();
    global_json_write_value(&mut output, value);
    output
}

fn global_json_normal_team_id(value: &serde_json::Value, field: &str) -> Option<usize> {
    value
        .get(field)
        .and_then(|team| team.get("Normal"))
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

fn global_json_is_completed_match(value: &serde_json::Value) -> bool {
    value
        .get("running_state")
        .and_then(|state| state.get("End"))
        .is_some()
}

fn global_json_copy_fields(
    source: &serde_json::Value,
    fields: &[&str],
) -> serde_json::Map<String, serde_json::Value> {
    let mut projected = serde_json::Map::new();
    for field in fields {
        if let Some(value) = source.get(*field) {
            projected.insert((*field).to_string(), value.clone());
        }
    }
    projected
}

fn global_compact_league_value(value: &serde_json::Value) -> serde_json::Value {
    serde_json::Value::Object(global_json_copy_fields(
        value,
        &["region_id", "division", "name"],
    ))
}

fn global_compact_league_competition_value(value: &serde_json::Value) -> serde_json::Value {
    let mut projected = global_json_copy_fields(value, &["league_type", "finalized"]);

    let mut standings = serde_json::Map::new();
    if let Some(source_standings) = value.get("standings").and_then(serde_json::Value::as_object) {
        for (team_id, stats) in source_standings {
            standings.insert(
                team_id.clone(),
                serde_json::Value::Object(global_json_copy_fields(
                    stats,
                    &["win", "lose", "set_win", "set_lose", "kill", "death", "assist"],
                )),
            );
        }
    }
    projected.insert("standings".to_string(), serde_json::Value::Object(standings));

    let mut statistics = serde_json::Map::new();
    if let Some(source_statistics) = value.get("statistics").and_then(serde_json::Value::as_object) {
        for (player_id, stats) in source_statistics {
            let mut player = global_json_copy_fields(
                stats,
                &[
                    "matches",
                    "wins",
                    "kills",
                    "deaths",
                    "assists",
                    "mvp",
                    "rating",
                    "gold",
                    "dealing",
                    "healing",
                    "tanking",
                    "solo_kill",
                    "solo_killed",
                ],
            );
            let mut champion_detail = serde_json::Map::new();
            if let Some(source_champions) = stats
                .get("champion_detail")
                .and_then(serde_json::Value::as_object)
            {
                for (champion_id, champion_stats) in source_champions {
                    champion_detail.insert(
                        champion_id.clone(),
                        serde_json::Value::Object(global_json_copy_fields(
                            champion_stats,
                            &["matches", "wins", "rating", "dealing", "healing", "tanking"],
                        )),
                    );
                }
            }
            player.insert(
                "champion_detail".to_string(),
                serde_json::Value::Object(champion_detail),
            );
            statistics.insert(player_id.clone(), serde_json::Value::Object(player));
        }
    }
    projected.insert(
        "statistics".to_string(),
        serde_json::Value::Object(statistics),
    );

    serde_json::Value::Object(projected)
}

fn global_compact_match_value(value: &serde_json::Value) -> serde_json::Value {
    let mut projected = global_json_copy_fields(value, &["date", "is_practice", "replays"]);

    for field in ["team1", "team2"] {
        if let Some(team_id) = global_json_normal_team_id(value, field) {
            let mut team = serde_json::Map::new();
            team.insert(
                "Normal".to_string(),
                serde_json::Value::Number(serde_json::Number::from(team_id as u64)),
            );
            projected.insert(field.to_string(), serde_json::Value::Object(team));
        }
    }

    if let Some(running_state) = value.get("running_state") {
        if let Some(end) = running_state.get("End") {
            let mut state = serde_json::Map::new();
            state.insert(
                "End".to_string(),
                serde_json::Value::Object(global_json_copy_fields(
                    end,
                    &["team1_score", "team2_score", "winner"],
                )),
            );
            projected.insert("running_state".to_string(), serde_json::Value::Object(state));
        } else if running_state.as_str() == Some("Running") {
            projected.insert("running_state".to_string(), running_state.clone());
        }
    }

    serde_json::Value::Object(projected)
}

fn select_recent_global_match_ids(mut record_ids: Vec<usize>, cap: usize) -> Vec<usize> {
    record_ids.sort_unstable_by_key(|record_id| Reverse(*record_id));
    record_ids.truncate(cap);
    record_ids.sort_unstable();
    record_ids
}

fn index_global_match_for_team(
    team_match_indices: &mut HashMap<usize, Vec<usize>>,
    team_id: Option<usize>,
    record_index: usize,
) {
    if let Some(team_id) = team_id {
        team_match_indices.entry(team_id).or_default().push(record_index);
    }
}

fn capture_global_history_snapshot(ctx: &ServerModContext) {
    let started = Instant::now();
    let capture_index = GLOBAL_HISTORY_CAPTURE_INDEX
        .fetch_add(1, Ordering::SeqCst)
        .saturating_add(1);
    let mut snapshot = GlobalHistorySnapshot {
        capture_index,
        ..GlobalHistorySnapshot::default()
    };

    let league_ids = ctx.database.leagues.keys();
    snapshot.metrics.league_source_records = league_ids.len();
    for record_id in league_ids.into_iter().take(GLOBAL_HISTORY_RECORD_CAP) {
        let Some(league) = ctx.database.leagues.get(record_id) else {
            continue;
        };
        let Ok(value) = serde_json::to_value(league) else {
            continue;
        };
        let compact = global_compact_league_value(&value);
        let Ok(json) = global_json_with_id(record_id, compact) else {
            continue;
        };
        snapshot.metrics.league_bytes = snapshot.metrics.league_bytes.saturating_add(json.len());
        snapshot.metrics.largest_record_bytes = snapshot.metrics.largest_record_bytes.max(json.len());
        snapshot.league_records.push(json);
    }
    snapshot.metrics.league_retained_records = snapshot.league_records.len();

    let league_competition_ids = ctx.database.league_competitions.keys();
    snapshot.metrics.league_competition_source_records = league_competition_ids.len();
    for record_id in league_competition_ids
        .into_iter()
        .take(GLOBAL_HISTORY_RECORD_CAP)
    {
        let Some(competition) = ctx.database.league_competitions.get(record_id) else {
            continue;
        };
        let Ok(value) = serde_json::to_value(competition) else {
            continue;
        };
        let compact = global_compact_league_competition_value(&value);
        let Ok(json) = global_json_with_id(record_id, compact) else {
            continue;
        };
        snapshot.metrics.league_competition_bytes = snapshot
            .metrics
            .league_competition_bytes
            .saturating_add(json.len());
        snapshot.metrics.largest_record_bytes = snapshot.metrics.largest_record_bytes.max(json.len());
        snapshot.league_competition_records.insert(record_id, json);
    }
    snapshot.metrics.league_competition_retained_records =
        snapshot.league_competition_records.len();

    let match_ids = ctx.database.matches.keys();
    snapshot.metrics.match_source_records = match_ids.len();
    let retained_match_ids = select_recent_global_match_ids(match_ids, GLOBAL_HISTORY_RECORD_CAP);
    snapshot.metrics.match_dropped_records = snapshot
        .metrics
        .match_source_records
        .saturating_sub(retained_match_ids.len());
    snapshot.metrics.match_oldest_retained_id = retained_match_ids.first().copied();
    snapshot.metrics.match_newest_retained_id = retained_match_ids.last().copied();

    for record_id in retained_match_ids {
        snapshot.metrics.match_scanned_records = snapshot.metrics.match_scanned_records.saturating_add(1);
        let Some(match_info) = ctx.database.matches.get(record_id) else {
            continue;
        };
        let Ok(value) = serde_json::to_value(match_info) else {
            continue;
        };
        let team1_id = global_json_normal_team_id(&value, "team1");
        let team2_id = global_json_normal_team_id(&value, "team2");
        let completed = global_json_is_completed_match(&value);
        let compact = global_compact_match_value(&value);
        let Ok(json) = global_json_with_id(record_id, compact) else {
            continue;
        };
        snapshot.metrics.match_bytes = snapshot.metrics.match_bytes.saturating_add(json.len());
        snapshot.metrics.largest_record_bytes = snapshot.metrics.largest_record_bytes.max(json.len());
        let record_index = snapshot.match_records.len();
        snapshot.match_records.push(GlobalMatchRecord { json, completed });
        index_global_match_for_team(&mut snapshot.team_match_indices, team1_id, record_index);
        if team2_id != team1_id {
            index_global_match_for_team(&mut snapshot.team_match_indices, team2_id, record_index);
        }
    }
    snapshot.metrics.match_retained_records = snapshot.match_records.len();
    snapshot.metrics.match_indexed_teams = snapshot.team_match_indices.len();
    snapshot.metrics.match_index_entries = snapshot
        .team_match_indices
        .values()
        .map(Vec::len)
        .sum();
    snapshot.metrics.snapshot_bytes = snapshot
        .metrics
        .league_bytes
        .saturating_add(snapshot.metrics.league_competition_bytes)
        .saturating_add(snapshot.metrics.match_bytes);
    snapshot.metrics.capture_micros = duration_micros_u64(started.elapsed());

    if let Ok(mut state) = global_history_state().lock() {
        *state = Some(snapshot);
    }
}

fn read_global_leagues() -> Result<(usize, Vec<String>), &'static str> {
    let state = global_history_state()
        .lock()
        .map_err(|_| "GLOBAL_HISTORY_LOCK_FAILED")?;
    let snapshot = state.as_ref().ok_or("GLOBAL_HISTORY_NOT_READY")?;
    Ok((snapshot.capture_index, snapshot.league_records.clone()))
}

fn read_global_league_competition(
    league_id: usize,
) -> Result<(usize, String), &'static str> {
    let state = global_history_state()
        .lock()
        .map_err(|_| "GLOBAL_HISTORY_LOCK_FAILED")?;
    let snapshot = state.as_ref().ok_or("GLOBAL_HISTORY_NOT_READY")?;
    let json = snapshot
        .league_competition_records
        .get(&league_id)
        .cloned()
        .ok_or("LEAGUE_COMPETITION_NOT_FOUND")?;
    Ok((snapshot.capture_index, json))
}

fn filter_global_team_records(
    snapshot: &GlobalHistorySnapshot,
    team_id: usize,
    completed_only: bool,
) -> Vec<String> {
    let Some(record_indices) = snapshot.team_match_indices.get(&team_id) else {
        return Vec::new();
    };

    record_indices
        .iter()
        .filter_map(|record_index| snapshot.match_records.get(*record_index))
        .filter(|record| !completed_only || record.completed)
        .map(|record| record.json.clone())
        .collect()
}

fn read_global_team_matches(
    team_id: usize,
    completed_only: bool,
) -> Result<(usize, Vec<String>), &'static str> {
    let state = global_history_state()
        .lock()
        .map_err(|_| "GLOBAL_HISTORY_LOCK_FAILED")?;
    let snapshot = state.as_ref().ok_or("GLOBAL_HISTORY_NOT_READY")?;
    let records = filter_global_team_records(snapshot, team_id, completed_only);
    Ok((snapshot.capture_index, records))
}

fn current_player_team_id(scene: &mut Scene) -> Result<usize, &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };
    Ok(data.player_team_id())
}

fn response_ok_global_leagues(capture_index: usize, records: &[String]) -> String {
    let payload = hex_join_records(records);
    format!(
        "OK|GLOBAL_LEAGUES|{BRIDGE_VERSION}|{TFM2_TARGET_VERSION}|{capture_index}|{}|{}",
        records.len(), payload
    )
}

fn response_ok_global_league_competition(
    capture_index: usize,
    league_id: usize,
    json: &str,
) -> String {
    format!(
        "OK|GLOBAL_LEAGUE_COMPETITION|{BRIDGE_VERSION}|{TFM2_TARGET_VERSION}|{capture_index}|{league_id}|{}",
        hex_encode(json)
    )
}

fn response_ok_global_team_records(
    kind: &str,
    capture_index: usize,
    player_team_id: usize,
    team_id: usize,
    records: &[String],
) -> String {
    let payload = hex_join_records(records);
    format!(
        "OK|{kind}|{BRIDGE_VERSION}|{TFM2_TARGET_VERSION}|{capture_index}|{player_team_id}|{team_id}|{}|{}",
        records.len(), payload
    )
}

fn contract_annual_salary(contract: &Contract) -> Option<f64> {
    match contract {
        Contract::InContract { weekly_salary, .. } => weekly_salary
            .to_string()
            .parse::<f64>()
            .ok()
            .map(|weekly| weekly * 52.0)
            .filter(|annual| annual.is_finite() && *annual >= 0.0),
        Contract::FreeAgent { .. } => None,
    }
}

fn median_salary(mut salaries: Vec<f64>) -> f64 {
    salaries.retain(|value| value.is_finite() && *value >= 0.0);
    if salaries.is_empty() {
        return 0.0;
    }
    salaries.sort_by(|left, right| left.total_cmp(right));
    let middle = salaries.len() / 2;
    if salaries.len().is_multiple_of(2) {
        (salaries[middle - 1] + salaries[middle]) / 2.0
    } else {
        salaries[middle]
    }
}

fn parse_replay_id_list(raw: &str) -> Result<Vec<usize>, &'static str> {
    let mut replay_ids = Vec::new();
    for value in raw.split(',').map(str::trim).filter(|value| !value.is_empty()) {
        let replay_id = value.parse::<usize>().map_err(|_| "INVALID_REPLAY_ID")?;
        if !replay_ids.contains(&replay_id) {
            replay_ids.push(replay_id);
        }
    }
    if replay_ids.is_empty() {
        return Err("NO_REPLAY_IDS");
    }
    Ok(replay_ids)
}

fn read_team_replay_strategies(
    scene: &mut Scene,
    team_id: usize,
    replay_ids: &[usize],
) -> Result<String, &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };
    if data.player_team_id() != team_id {
        return Err("PLAYER_TEAM_ONLY");
    }

    let db = data.db();
    if !db.teams.contains_key(&team_id) {
        return Err("TEAM_NOT_FOUND");
    }

    macro_rules! strategy_snapshot {
        ($strategy:expr) => {
            format!(
                "focused={:?};early_jungle={:?};early_serpen={:?};early_serpen_top={:?};object_buildup={:?};object_battle={:?};morgard_use={:?};tower_press={:?};morgard_defense={:?};object_finish={:?};minion_wave={:?};game_finish={:?}",
                &$strategy.focused,
                &$strategy.early_jungle,
                &$strategy.early_serpen,
                &$strategy.early_serpen_top,
                &$strategy.object_buildup,
                &$strategy.object_battle,
                &$strategy.morgard_use,
                &$strategy.tower_press,
                &$strategy.morgard_defense,
                &$strategy.object_finish,
                &$strategy.minion_wave,
                &$strategy.game_finish,
            )
        };
    }

    let mut raw = String::new();
    for replay_id in replay_ids {
        let Some(replay) = db.match_replays.get(replay_id) else {
            append_tsv_row(
                &mut raw,
                &[
                    replay_id.to_string(),
                    "MISSING".to_string(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                    String::new(),
                ],
            );
            continue;
        };

        let blue_team_id = replay.blue_team_id;
        let red_team_id = replay.red_team_id;
        let (side, own_strategy, own_set_win) = if blue_team_id == team_id {
            (
                "Blue",
                strategy_snapshot!(replay.blue_strategy),
                replay.blue_team_win,
            )
        } else if red_team_id == team_id {
            (
                "Red",
                strategy_snapshot!(replay.red_strategy),
                !replay.blue_team_win,
            )
        } else {
            append_tsv_row(
                &mut raw,
                &[
                    replay_id.to_string(),
                    "TEAM_NOT_IN_REPLAY".to_string(),
                    String::new(),
                    blue_team_id.to_string(),
                    red_team_id.to_string(),
                    String::new(),
                    String::new(),
                ],
            );
            continue;
        };

        append_tsv_row(
            &mut raw,
            &[
                replay_id.to_string(),
                "OK".to_string(),
                side.to_string(),
                blue_team_id.to_string(),
                red_team_id.to_string(),
                if own_set_win { "1".to_string() } else { "0".to_string() },
                own_strategy,
            ],
        );
    }
    Ok(raw)
}

fn team_strategy_probe_payload(team_id: usize) -> Vec<u8> {
    team_id.to_string().into_bytes()
}

fn parse_team_strategy_probe_payload(payload: &[u8]) -> Result<usize, &'static str> {
    let text = std::str::from_utf8(payload).map_err(|_| "INVALID_PAYLOAD")?;
    text.trim().parse::<usize>().map_err(|_| "INVALID_ID")
}

fn parse_team_strategy_values(raw: &str) -> Result<HashMap<String, String>, &'static str> {
    let mut values = HashMap::new();
    for line in raw.lines() {
        let Some((key, value)) = line.split_once('\t') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        if !key.is_empty() && !value.is_empty() {
            values.insert(key.to_string(), value.to_string());
        }
    }
    for key in [
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
    ] {
        if !values.contains_key(key) {
            return Err("MISSING_STRATEGY_VALUE");
        }
    }
    Ok(values)
}

fn team_strategy_set_payload(team_id: usize, raw_strategy: &str) -> Vec<u8> {
    format!("{team_id}|{}", hex_encode(raw_strategy)).into_bytes()
}

fn parse_team_strategy_set_payload(
    payload: &[u8],
) -> Result<(usize, String), &'static str> {
    let text = std::str::from_utf8(payload).map_err(|_| "INVALID_PAYLOAD")?;
    let mut parts = text.splitn(2, '|');
    let team_id = parts
        .next()
        .ok_or("MISSING_VALUE")?
        .parse::<usize>()
        .map_err(|_| "INVALID_ID")?;
    let raw_strategy = hex_decode(parts.next().ok_or("MISSING_VALUE")?)?;
    parse_team_strategy_values(&raw_strategy)?;
    Ok((team_id, raw_strategy))
}

fn probe_swap_team_strategy_client(
    scene: &mut Scene,
    team_id: usize,
) -> Result<TeamManagementSnapshot, &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };

    {
        let db = data.db();
        if !db.teams.contains_key(&team_id) {
            return Err("TEAM_NOT_FOUND");
        }
    }

    if !data.send_mod_command(
        MOD_ID,
        "probe_swap_team_strategy",
        team_strategy_probe_payload(team_id),
    ) {
        return Err("SERVER_COMMAND_FAILED");
    }

    {
        let mut db = data.db_mut();
        let Some(team) = db.teams.get_mut(&team_id) else {
            return Err("TEAM_NOT_FOUND");
        };
        std::mem::swap(&mut team.strategy, &mut team.last_strategy);
    }

    read_team_management(scene, team_id)
}


fn set_team_strategy_client(
    scene: &mut Scene,
    team_id: usize,
    raw_strategy: &str,
) -> Result<TeamManagementSnapshot, &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };
    if data.player_team_id() != team_id {
        return Err("PLAYER_TEAM_ONLY");
    }
    let values = parse_team_strategy_values(raw_strategy)?;

    let new_strategy = {
        let db = data.db();
        let Some(target) = db.teams.get(&team_id) else {
            return Err("TEAM_NOT_FOUND");
        };
        let mut strategy = target.strategy;

        macro_rules! resolve_field {
            ($field:ident, $key:literal) => {{
                let desired = values.get($key).ok_or("MISSING_STRATEGY_VALUE")?;
                db.teams
                    .iter()
                    .find_map(|(_, candidate)| {
                        if format!("{:?}", candidate.strategy.$field) == desired.as_str() {
                            Some(candidate.strategy.$field.clone())
                        } else if format!("{:?}", candidate.last_strategy.$field) == desired.as_str() {
                            Some(candidate.last_strategy.$field.clone())
                        } else {
                            candidate
                                .team_color_strategy
                                .$field
                                .as_ref()
                                .filter(|value| format!("{:?}", value) == desired.as_str())
                                .cloned()
                        }
                    })
                    .ok_or("UNKNOWN_STRATEGY_VALUE")?
            }};
        }

        strategy.focused = resolve_field!(focused, "focused");
        strategy.early_jungle = resolve_field!(early_jungle, "early_jungle");
        strategy.early_serpen = resolve_field!(early_serpen, "early_serpen");
        strategy.early_serpen_top = resolve_field!(early_serpen_top, "early_serpen_top");
        strategy.object_buildup = resolve_field!(object_buildup, "object_buildup");
        strategy.object_battle = resolve_field!(object_battle, "object_battle");
        strategy.morgard_use = resolve_field!(morgard_use, "morgard_use");
        strategy.tower_press = resolve_field!(tower_press, "tower_press");
        strategy.morgard_defense = resolve_field!(morgard_defense, "morgard_defense");
        strategy.object_finish = resolve_field!(object_finish, "object_finish");
        strategy.minion_wave = resolve_field!(minion_wave, "minion_wave");
        strategy.game_finish = resolve_field!(game_finish, "game_finish");
        strategy
    };

    if !data.send_mod_command(
        MOD_ID,
        "set_team_strategy",
        team_strategy_set_payload(team_id, raw_strategy),
    ) {
        return Err("SERVER_COMMAND_FAILED");
    }

    {
        let mut db = data.db_mut();
        let Some(team) = db.teams.get_mut(&team_id) else {
            return Err("TEAM_NOT_FOUND");
        };
        team.strategy = new_strategy;
    }

    read_team_management(scene, team_id)
}

fn validate_team_merchandise_write(values: &TeamMerchandiseWriteValue) -> Result<(), &'static str> {
    if values.product_type.trim().is_empty() {
        return Err("INVALID_MERCHANDISE_TYPE");
    }
    values
        .stock
        .trim()
        .parse::<u128>()
        .map_err(|_| "INVALID_MERCHANDISE_STOCK")?;
    let sell_price = values
        .sell_price
        .trim()
        .parse::<f64>()
        .map_err(|_| "INVALID_MERCHANDISE_SELL_PRICE")?;
    if !sell_price.is_finite() || sell_price < 0.0 {
        return Err("MERCHANDISE_SELL_PRICE_OUT_OF_RANGE");
    }
    Ok(())
}

fn team_merchandise_write_payload(
    team_id: usize,
    values: &TeamMerchandiseWriteValue,
) -> Vec<u8> {
    format!(
        "{}|{}|{}|{}|{}",
        team_id,
        hex_encode(&values.product_type),
        values.athlete_id,
        values.stock.trim(),
        values.sell_price.trim(),
    )
    .into_bytes()
}

fn parse_team_merchandise_write_payload(
    payload: &[u8],
) -> Result<(usize, TeamMerchandiseWriteValue), &'static str> {
    let text = std::str::from_utf8(payload).map_err(|_| "INVALID_PAYLOAD")?;
    let mut parts = text.split('|');
    let team_id = parse_usize(parts.next())?;
    let values = TeamMerchandiseWriteValue {
        product_type: hex_decode(parts.next().ok_or("MISSING_VALUE")?)?,
        athlete_id: parse_usize(parts.next())?,
        stock: parts.next().ok_or("MISSING_VALUE")?.to_string(),
        sell_price: parts.next().ok_or("MISSING_VALUE")?.to_string(),
    };
    validate_team_merchandise_write(&values)?;
    Ok((team_id, values))
}

fn set_team_merchandise_client(
    scene: &mut Scene,
    team_id: usize,
    values: &TeamMerchandiseWriteValue,
) -> Result<(), &'static str> {
    validate_team_merchandise_write(values)?;
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };
    if data.player_team_id() != team_id {
        return Err("PLAYER_TEAM_ONLY");
    }
    {
        let db = data.db();
        let Some(team) = db.teams.get(&team_id) else {
            return Err("TEAM_NOT_FOUND");
        };
        if !team.merchandise_products.iter().any(|product| {
            product.athlete_id == values.athlete_id
                && product.product_type.to_string() == values.product_type
        }) {
            return Err("MERCHANDISE_PRODUCT_NOT_FOUND");
        }
    }
    if !data.send_mod_command(
        MOD_ID,
        "set_team_merchandise",
        team_merchandise_write_payload(team_id, values),
    ) {
        return Err("SERVER_COMMAND_FAILED");
    }
    let mut db = data.db_mut();
    let Some(team) = db.teams.get_mut(&team_id) else {
        return Err("TEAM_NOT_FOUND");
    };
    let Some(product) = team.merchandise_products.iter_mut().find(|product| {
        product.athlete_id == values.athlete_id
            && product.product_type.to_string() == values.product_type
    }) else {
        return Err("MERCHANDISE_PRODUCT_NOT_FOUND");
    };
    product.stock = values
        .stock
        .trim()
        .parse()
        .map_err(|_| "MERCHANDISE_STOCK_TYPE_ERROR")?;
    product.sell_price = values
        .sell_price
        .trim()
        .parse()
        .map_err(|_| "MERCHANDISE_SELL_PRICE_TYPE_ERROR")?;
    Ok(())
}

fn displayed_team_fan_count(base_team_fans: u128, roster_player_fans: u128) -> Result<u128, &'static str> {
    base_team_fans
        .checked_add(roster_player_fans)
        .ok_or("FAN_COUNT_OVERFLOW")
}

fn base_team_fan_count_from_displayed(
    displayed_fans: u128,
    roster_player_fans: u128,
) -> Result<u128, &'static str> {
    displayed_fans
        .checked_sub(roster_player_fans)
        .ok_or("FAN_COUNT_BELOW_PLAYER_FANS")
}

fn validate_team_fans_write(values: &TeamFansWriteValue) -> Result<(), &'static str> {
    if values.popularity.trim().is_empty() {
        return Err("INVALID_FAN_POPULARITY");
    }
    values
        .fan_count
        .trim()
        .parse::<u128>()
        .map_err(|_| "INVALID_FAN_COUNT")?;
    if values.fan_expectation.trim().is_empty() {
        return Err("INVALID_FAN_EXPECTATION");
    }
    if values.fan_satisfaction.trim().is_empty() {
        return Err("INVALID_FAN_SATISFACTION");
    }
    let fan_momentum = values
        .fan_momentum
        .trim()
        .parse::<i32>()
        .map_err(|_| "INVALID_FAN_MOMENTUM")?;
    if !(-5..=5).contains(&fan_momentum) {
        return Err("FAN_MOMENTUM_OUT_OF_RANGE");
    }
    Ok(())
}

fn team_fans_write_payload(team_id: usize, values: &TeamFansWriteValue) -> Vec<u8> {
    format!(
        "{}|{}|{}|{}|{}|{}",
        team_id,
        values.popularity.trim(),
        values.fan_count.trim(),
        hex_encode(values.fan_expectation.trim()),
        hex_encode(values.fan_satisfaction.trim()),
        values.fan_momentum.trim(),
    )
    .into_bytes()
}

fn parse_team_fans_write_payload(
    payload: &[u8],
) -> Result<(usize, TeamFansWriteValue), &'static str> {
    let text = std::str::from_utf8(payload).map_err(|_| "INVALID_PAYLOAD")?;
    let mut parts = text.split('|');
    let team_id = parse_usize(parts.next())?;
    let values = TeamFansWriteValue {
        popularity: parts.next().ok_or("MISSING_VALUE")?.to_string(),
        fan_count: parts.next().ok_or("MISSING_VALUE")?.to_string(),
        fan_expectation: hex_decode(parts.next().ok_or("MISSING_VALUE")?)?,
        fan_satisfaction: hex_decode(parts.next().ok_or("MISSING_VALUE")?)?,
        fan_momentum: parts.next().ok_or("MISSING_VALUE")?.to_string(),
    };
    validate_team_fans_write(&values)?;
    Ok((team_id, values))
}

fn set_team_fans_client(
    scene: &mut Scene,
    team_id: usize,
    values: &TeamFansWriteValue,
) -> Result<(), &'static str> {
    validate_team_fans_write(values)?;
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };
    if data.player_team_id() != team_id {
        return Err("PLAYER_TEAM_ONLY");
    }
    let (resolved_expectation, resolved_satisfaction, roster_player_fans) = {
        let db = data.db();
        if !db.teams.contains_key(&team_id) {
            return Err("TEAM_NOT_FOUND");
        }
        let expectation = db
            .teams
            .iter()
            .find_map(|(_, candidate)| {
                if format!("{:?}", candidate.fan_expectation) == values.fan_expectation.trim() {
                    Some(candidate.fan_expectation)
                } else {
                    None
                }
            })
            .ok_or("UNKNOWN_FAN_EXPECTATION")?;
        let satisfaction = db
            .teams
            .iter()
            .find_map(|(_, candidate)| {
                if format!("{:?}", candidate.fan_satisfaction) == values.fan_satisfaction.trim() {
                    Some(candidate.fan_satisfaction)
                } else {
                    None
                }
            })
            .ok_or("UNKNOWN_FAN_SATISFACTION")?;
        let player_fans = db
            .athletes
            .values()
            .filter_map(|athlete| {
                if matches!(
                    &athlete.contract,
                    Contract::InContract { team_id: athlete_team_id, .. } if *athlete_team_id == team_id
                ) {
                    athlete.management.fan_count.to_string().parse::<u128>().ok()
                } else {
                    None
                }
            })
            .sum::<u128>();
        (expectation, satisfaction, player_fans)
    };
    let displayed_fan_count = values
        .fan_count
        .trim()
        .parse::<u128>()
        .map_err(|_| "INVALID_FAN_COUNT")?;
    let base_fan_count =
        base_team_fan_count_from_displayed(displayed_fan_count, roster_player_fans)?;
    if !data.send_mod_command(
        MOD_ID,
        "set_team_fans",
        team_fans_write_payload(team_id, values),
    ) {
        return Err("SERVER_COMMAND_FAILED");
    }
    let mut db = data.db_mut();
    let Some(team) = db.teams.get_mut(&team_id) else {
        return Err("TEAM_NOT_FOUND");
    };
    team.popularity = values
        .popularity
        .trim()
        .parse()
        .map_err(|_| "FAN_POPULARITY_TYPE_ERROR")?;
    team.fan_count = base_fan_count
        .to_string()
        .parse()
        .map_err(|_| "FAN_COUNT_TYPE_ERROR")?;
    team.fan_expectation = resolved_expectation;
    team.fan_satisfaction = resolved_satisfaction;
    team.fan_momentum = values
        .fan_momentum
        .trim()
        .parse()
        .map_err(|_| "FAN_MOMENTUM_TYPE_ERROR")?;
    Ok(())
}

fn read_team_fan_momentum_probe(
    scene: &mut Scene,
    team_id: usize,
) -> Result<String, &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };
    let db = data.db();
    let Some(team) = db.teams.get(&team_id) else {
        return Err("TEAM_NOT_FOUND");
    };
    let rust_type = std::any::type_name_of_val(&team.fan_momentum);
    let mut observed = db
        .teams
        .iter()
        .map(|(id, candidate)| (*id, candidate.fan_momentum.to_string()))
        .collect::<Vec<_>>();
    observed.sort_by_key(|(id, _)| *id);
    let numeric = observed
        .iter()
        .filter_map(|(_, value)| value.parse::<f64>().ok())
        .filter(|value| value.is_finite())
        .collect::<Vec<_>>();
    let observed_min = numeric.iter().copied().reduce(f64::min);
    let observed_max = numeric.iter().copied().reduce(f64::max);
    let signed_hint = rust_type
        .rsplit("::")
        .next()
        .is_some_and(|name| name.starts_with('i') || name.starts_with('f'));

    let mut raw = String::new();
    append_tsv_row(&mut raw, &["team_id".to_string(), team_id.to_string()]);
    append_tsv_row(&mut raw, &["rust_type".to_string(), rust_type.to_string()]);
    append_tsv_row(
        &mut raw,
        &["current_raw".to_string(), team.fan_momentum.to_string()],
    );
    append_tsv_row(
        &mut raw,
        &["signed_type_hint".to_string(), signed_hint.to_string()],
    );
    append_tsv_row(
        &mut raw,
        &[
            "observed_min".to_string(),
            observed_min.map(|value| value.to_string()).unwrap_or_default(),
        ],
    );
    append_tsv_row(
        &mut raw,
        &[
            "observed_max".to_string(),
            observed_max.map(|value| value.to_string()).unwrap_or_default(),
        ],
    );
    append_tsv_row(
        &mut raw,
        &[
            "display_scaling".to_string(),
            "Bridge raw ToString output; no display scaling is applied.".to_string(),
        ],
    );
    for (id, value) in observed {
        append_tsv_row(
            &mut raw,
            &["observed_team".to_string(), id.to_string(), value],
        );
    }
    Ok(raw)
}

fn read_team_strategy_options(scene: &mut Scene) -> Result<String, &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };

    let db = data.db();
    let mut options = BTreeSet::<(String, String)>::new();

    for team in db.teams.values() {
        macro_rules! collect_field {
            ($field:ident, $key:literal) => {{
                options.insert(($key.to_string(), format!("{:?}", team.strategy.$field)));
                options.insert((
                    $key.to_string(),
                    format!("{:?}", team.last_strategy.$field),
                ));
                if let Some(value) = team.team_color_strategy.$field.as_ref() {
                    options.insert(($key.to_string(), format!("{value:?}")));
                }
            }};
        }

        collect_field!(focused, "focused");
        collect_field!(early_jungle, "early_jungle");
        collect_field!(early_serpen, "early_serpen");
        collect_field!(early_serpen_top, "early_serpen_top");
        collect_field!(object_buildup, "object_buildup");
        collect_field!(object_battle, "object_battle");
        collect_field!(morgard_use, "morgard_use");
        collect_field!(tower_press, "tower_press");
        collect_field!(morgard_defense, "morgard_defense");
        collect_field!(object_finish, "object_finish");
        collect_field!(minion_wave, "minion_wave");
        collect_field!(game_finish, "game_finish");
    }

    let mut raw = String::new();
    for (key, value) in options {
        append_tsv_row(&mut raw, &[key, value]);
    }
    Ok(raw)
}

fn read_contract_defaults(
    scene: &mut Scene,
    entity: ContractDefaultsEntity,
    destination_team_id: usize,
) -> Result<ContractDefaults, &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };

    let db = data.db();
    if !db.teams.contains_key(&destination_team_id) {
        return Err("DESTINATION_TEAM_NOT_FOUND");
    }

    // Team news is generated from the management timeline and gives us a public,
    // save-specific date source without relying on private client internals.
    let mut latest_date = db
        .teams
        .values()
        .flat_map(|team| team.news.iter())
        .filter_map(|news| news.date.to_string().get(..10).map(|value| value.to_string()))
        .max();

    // A very early save can have little or no news. Active contract starts provide
    // a safe fallback and are always valid ISO dates in a normal career database.
    for athlete in db.athletes.values() {
        if let Contract::InContract { start_date, .. } = &athlete.contract {
            if let Some(date) = start_date.to_string().get(..10).map(|value| value.to_string()) {
                if latest_date.as_ref().is_none_or(|current| date.as_str() > current.as_str()) {
                    latest_date = Some(date);
                }
            }
        }
    }
    for staff in db.staffs.values() {
        if let Contract::InContract { start_date, .. } = &staff.contract {
            if let Some(date) = start_date.to_string().get(..10).map(|value| value.to_string()) {
                if latest_date.as_ref().is_none_or(|current| date.as_str() > current.as_str()) {
                    latest_date = Some(date);
                }
            }
        }
    }

    let start_date = latest_date.unwrap_or_else(|| "2026-01-01".to_string());
    let start_year = start_date
        .get(..4)
        .and_then(|year| year.parse::<i32>().ok())
        .ok_or("INVALID_GAME_DATE")?;
    let end_date = format!("{}-12-31", start_year + 3);

    let team_salaries = match entity {
        ContractDefaultsEntity::Player => db
            .athletes
            .values()
            .filter_map(|athlete| match &athlete.contract {
                Contract::InContract { team_id, .. } if *team_id == destination_team_id => {
                    contract_annual_salary(&athlete.contract)
                }
                _ => None,
            })
            .collect::<Vec<_>>(),
        ContractDefaultsEntity::Staff => db
            .staffs
            .values()
            .filter_map(|staff| match &staff.contract {
                Contract::InContract { team_id, .. } if *team_id == destination_team_id => {
                    contract_annual_salary(&staff.contract)
                }
                _ => None,
            })
            .collect::<Vec<_>>(),
    };

    let salaries = if team_salaries.is_empty() {
        match entity {
            ContractDefaultsEntity::Player => db
                .athletes
                .values()
                .filter_map(|athlete| contract_annual_salary(&athlete.contract))
                .collect::<Vec<_>>(),
            ContractDefaultsEntity::Staff => db
                .staffs
                .values()
                .filter_map(|staff| contract_annual_salary(&staff.contract))
                .collect::<Vec<_>>(),
        }
    } else {
        team_salaries
    };

    Ok(ContractDefaults {
        start_date,
        end_date,
        annual_salary: median_salary(salaries).to_string(),
    })
}

fn parse_staff_role(raw: &str) -> Result<StaffRole, &'static str> {
    match raw.trim() {
        "HeadCoach" => Ok(StaffRole::HeadCoach),
        "TrainingCoach" => Ok(StaffRole::TrainingCoach),
        "Scouter" => Ok(StaffRole::Scouter),
        "Analyst" => Ok(StaffRole::Analyst),
        _ => Err("INVALID_STAFF_ROLE"),
    }
}

fn validate_optional_staff_role(role: Option<&str>) -> Result<(), &'static str> {
    if let Some(role) = role {
        let _ = parse_staff_role(role)?;
    }
    Ok(())
}

fn move_staff_to_team_client(
    scene: &mut Scene,
    staff_id: usize,
    destination_team_id: usize,
    role: Option<String>,
) -> Result<(), &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };
    validate_optional_staff_role(role.as_deref())?;

    {
        let db = data.db();
        if !db.teams.contains_key(&destination_team_id) {
            return Err("TEAM_NOT_FOUND");
        }
        if !db.staffs.contains_key(&staff_id) {
            return Err("STAFF_NOT_FOUND");
        }
    }

    {
        let mut db = data.db_mut();
        let Some(staff) = db.staffs.get_mut(&staff_id) else {
            return Err("STAFF_NOT_FOUND");
        };
        match &mut staff.contract {
            Contract::InContract {
                team_id,
                transfer_requests,
                recruit_requests,
                ..
            } => {
                *team_id = destination_team_id;
                transfer_requests.clear();
                recruit_requests.clear();
            }
            Contract::FreeAgent { .. } => return Err("STAFF_FREE_AGENT_NEEDS_CONTRACT"),
        }
        if let Some(role) = role.as_deref() {
            staff.role = parse_staff_role(role)?;
        }
    }

    let payload = if let Some(role) = role.as_deref() {
        format!("{}|{}|{}", staff_id, destination_team_id, role).into_bytes()
    } else {
        format!("{}|{}", staff_id, destination_team_id).into_bytes()
    };
    if !data.send_mod_command(MOD_ID, "move_staff_to_team", payload) {
        return Err("SERVER_COMMAND_FAILED");
    }
    Ok(())
}

fn parse_move_staff_payload(
    payload: &[u8],
) -> Result<(usize, usize, Option<String>), &'static str> {
    let text = std::str::from_utf8(payload).map_err(|_| "INVALID_PAYLOAD")?;
    let mut parts = text.split('|');
    let staff_id = parse_usize(parts.next())?;
    let team_id = parse_usize(parts.next())?;
    let role = parts
        .next()
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string);
    validate_optional_staff_role(role.as_deref())?;
    Ok((staff_id, team_id, role))
}

fn move_staff_to_team_server(
    ctx: &mut ServerModContext,
    staff_id: usize,
    destination_team_id: usize,
    role: Option<String>,
) -> Result<(), &'static str> {
    validate_optional_staff_role(role.as_deref())?;
    if ctx.database.teams.get(destination_team_id).is_none() {
        return Err("TEAM_NOT_FOUND");
    }
    let Some(staff) = ctx.database.staffs.get_mut(staff_id) else {
        return Err("STAFF_NOT_FOUND");
    };
    match &mut staff.contract {
        Contract::InContract {
            team_id,
            transfer_requests,
            recruit_requests,
            ..
        } => {
            *team_id = destination_team_id;
            transfer_requests.clear();
            recruit_requests.clear();
            if let Some(role) = role.as_deref() {
                staff.role = parse_staff_role(role)?;
            }
            Ok(())
        }
        Contract::FreeAgent { .. } => Err("STAFF_FREE_AGENT_NEEDS_CONTRACT"),
    }
}

fn set_staff_free_agent_client(scene: &mut Scene, staff_id: usize) -> Result<(), &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };
    {
        let mut db = data.db_mut();
        let Some(staff) = db.staffs.get_mut(&staff_id) else {
            return Err("STAFF_NOT_FOUND");
        };
        if matches!(&staff.contract, Contract::FreeAgent { .. }) {
            return Err("STAFF_ALREADY_FREE_AGENT");
        }
        staff.contract = Contract::FreeAgent { requests: Vec::new() };
    }
    if !data.send_mod_command(MOD_ID, "set_staff_free_agent", staff_id.to_string().into_bytes()) {
        return Err("SERVER_COMMAND_FAILED");
    }
    Ok(())
}

fn set_staff_free_agent_server(
    ctx: &mut ServerModContext,
    staff_id: usize,
) -> Result<(), &'static str> {
    let Some(staff) = ctx.database.staffs.get_mut(staff_id) else {
        return Err("STAFF_NOT_FOUND");
    };
    if matches!(&staff.contract, Contract::FreeAgent { .. }) {
        return Err("STAFF_ALREADY_FREE_AGENT");
    }
    staff.contract = Contract::FreeAgent { requests: Vec::new() };
    Ok(())
}

fn move_player_to_team_client(
    scene: &mut Scene,
    athlete_id: usize,
    destination_team_id: usize,
) -> Result<(), &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };

    {
        let db = data.db();
        if !db.teams.contains_key(&destination_team_id) {
            return Err("TEAM_NOT_FOUND");
        }
        if !db.athletes.contains_key(&athlete_id) {
            return Err("PLAYER_NOT_FOUND");
        }
    }

    {
        let mut db = data.db_mut();
        let Some(athlete) = db.athletes.get_mut(&athlete_id) else {
            return Err("PLAYER_NOT_FOUND");
        };

        match &mut athlete.contract {
            Contract::InContract {
                team_id,
                transfer_requests,
                recruit_requests,
                ..
            } => {
                *team_id = destination_team_id;
                transfer_requests.clear();
                recruit_requests.clear();
            }
            Contract::FreeAgent { .. } => {
                return Err("FREE_AGENT_MOVE_NOT_SUPPORTED_YET");
            }
        }
    }

    let payload = format!("{}|{}", athlete_id, destination_team_id).into_bytes();
    if !data.send_mod_command(MOD_ID, "move_player_to_team", payload) {
        return Err("SERVER_COMMAND_FAILED");
    }

    Ok(())
}

fn parse_move_player_payload(payload: &[u8]) -> Result<(usize, usize), &'static str> {
    let text = std::str::from_utf8(payload).map_err(|_| "INVALID_PAYLOAD")?;
    let mut parts = text.split('|');
    let athlete_id = parts
        .next()
        .ok_or("MISSING_VALUE")?
        .parse::<usize>()
        .map_err(|_| "INVALID_ID")?;
    let team_id = parts
        .next()
        .ok_or("MISSING_VALUE")?
        .parse::<usize>()
        .map_err(|_| "INVALID_ID")?;
    Ok((athlete_id, team_id))
}

fn move_player_to_team_server(
    ctx: &mut ServerModContext,
    athlete_id: usize,
    destination_team_id: usize,
) -> Result<(), &'static str> {
    if ctx.database.teams.get(destination_team_id).is_none() {
        return Err("TEAM_NOT_FOUND");
    }

    let Some(athlete) = ctx.database.athletes.get_mut(athlete_id) else {
        return Err("PLAYER_NOT_FOUND");
    };

    match &mut athlete.contract {
        Contract::InContract {
            team_id,
            transfer_requests,
            recruit_requests,
            ..
        } => {
            *team_id = destination_team_id;
            transfer_requests.clear();
            recruit_requests.clear();
            Ok(())
        }
        Contract::FreeAgent { .. } => Err("FREE_AGENT_MOVE_NOT_SUPPORTED_YET"),
    }
}

fn set_player_free_agent_client(
    scene: &mut Scene,
    athlete_id: usize,
) -> Result<(), &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };

    {
        let mut db = data.db_mut();
        let Some(athlete) = db.athletes.get_mut(&athlete_id) else {
            return Err("PLAYER_NOT_FOUND");
        };
        if matches!(&athlete.contract, Contract::FreeAgent { .. }) {
            return Err("PLAYER_ALREADY_FREE_AGENT");
        }
        athlete.contract = Contract::FreeAgent { requests: Vec::new() };
    }

    if !data.send_mod_command(
        MOD_ID,
        "set_player_free_agent",
        athlete_id.to_string().into_bytes(),
    ) {
        return Err("SERVER_COMMAND_FAILED");
    }

    Ok(())
}

fn parse_player_free_agent_payload(payload: &[u8]) -> Result<usize, &'static str> {
    let text = std::str::from_utf8(payload).map_err(|_| "INVALID_PAYLOAD")?;
    text.parse::<usize>().map_err(|_| "INVALID_ID")
}

fn set_player_free_agent_server(
    ctx: &mut ServerModContext,
    athlete_id: usize,
) -> Result<(), &'static str> {
    let Some(athlete) = ctx.database.athletes.get_mut(athlete_id) else {
        return Err("PLAYER_NOT_FOUND");
    };
    if matches!(&athlete.contract, Contract::FreeAgent { .. }) {
        return Err("PLAYER_ALREADY_FREE_AGENT");
    }
    athlete.contract = Contract::FreeAgent { requests: Vec::new() };
    Ok(())
}

fn communication_raw(athlete: &Athlete) -> String {
    let mut values = athlete
        .stat
        .language
        .iter()
        .map(|(region_id, value)| (*region_id, *value))
        .collect::<Vec<_>>();
    values.sort_by_key(|(region_id, _)| *region_id);
    values
        .into_iter()
        .map(|(region_id, value)| format!("{}:{}", region_id, value))
        .collect::<Vec<_>>()
        .join(",")
}

fn communication_xp_raw(athlete: &Athlete) -> String {
    let mut values = athlete
        .training_exp
        .language_by_region
        .iter()
        .map(|(region_id, value)| (*region_id, *value))
        .collect::<Vec<_>>();
    values.sort_by_key(|(region_id, _)| *region_id);
    values
        .into_iter()
        .map(|(region_id, value)| format!("{}:{}", region_id, value))
        .collect::<Vec<_>>()
        .join(",")
}

fn primary_region_raw(athlete: &Athlete) -> String {
    athlete
        .get_primary_region()
        .map(|region_id| region_id.to_string())
        .unwrap_or_default()
}

fn detected_region_ids(scene: &mut Scene) -> Result<Vec<usize>, &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };

    let db = data.db();
    let mut region_ids = Vec::new();
    for athlete in db.athletes.values() {
        if let Some(region_id) = athlete.get_primary_region() {
            region_ids.push(region_id);
        }
        region_ids.extend(athlete.stat.language.keys().copied());
        region_ids.extend(athlete.training_exp.language_by_region.keys().copied());
    }
    region_ids.sort_unstable();
    region_ids.dedup();
    Ok(region_ids)
}

fn read_champion_mastery_probe(
    scene: &mut Scene,
    athlete_id: usize,
) -> Result<(String, String), &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };

    let db = data.db();
    let Some(athlete) = db.athletes.get(&athlete_id) else {
        return Err("PLAYER_NOT_FOUND");
    };

    // Second-pass diagnostic: inspect the actual champion proficiency state.
    let mastery_state = format!(
        "champion_proficiency:\n{:#?}\n\nrecent_champions:\n{:#?}",
        athlete.champion_proficiency,
        athlete.recent_champions,
    );

    // The active save owns the dynamic champion pool. Never hardcode champions.
    let available_champions = format!("{:#?}", db.available_champions);

    Ok((mastery_state, available_champions))
}

fn validate_champion_mastery_values(
    values: &[ChampionMasteryValue],
) -> Result<(), &'static str> {
    if values.is_empty() {
        return Err("NO_CHAMPIONS_SELECTED");
    }

    for value in values {
        if value.champion_id.is_empty()
            || !value
                .champion_id
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        {
            return Err("INVALID_CHAMPION_ID");
        }

        if value.mastery > 100 {
            return Err("MASTERY_OUT_OF_RANGE");
        }
    }

    Ok(())
}

fn champion_mastery_payload(
    athlete_id: usize,
    values: &[ChampionMasteryValue],
) -> Vec<u8> {
    let entries = values
        .iter()
        .map(|value| format!("{}:{}", value.champion_id, value.mastery))
        .collect::<Vec<_>>()
        .join(";");

    format!("{}|{}", athlete_id, entries).into_bytes()
}

fn apply_champion_mastery_to_athlete(
    athlete: &mut Athlete,
    values: &[ChampionMasteryValue],
) -> Result<(), &'static str> {
    validate_champion_mastery_values(values)?;

    for value in values {
        let Some(proficiency) =
            athlete.champion_proficiency.get_mut(&value.champion_id)
        else {
            return Err("CHAMPION_NOT_FOUND");
        };

        // Confirmed mapping: in-game mastery 90 == raw ChampionProficiency.value 900.
        // Do not touch ChampionProficiency.floor.
        proficiency.value = (value.mastery as i32 * 10) as _;
    }

    Ok(())
}

fn write_champion_mastery(
    scene: &mut Scene,
    athlete_id: usize,
    values: Vec<ChampionMasteryValue>,
) -> Result<usize, &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };

    validate_champion_mastery_values(&values)?;

    if !data.send_mod_command(
        MOD_ID,
        "set_champion_mastery",
        champion_mastery_payload(athlete_id, &values),
    ) {
        return Err("SERVER_COMMAND_FAILED");
    }

    let mut db = data.db_mut();
    let Some(athlete) = db.athletes.get_mut(&athlete_id) else {
        return Err("PLAYER_NOT_FOUND");
    };

    apply_champion_mastery_to_athlete(athlete, &values)?;
    Ok(values.len())
}

fn parse_server_champion_mastery(
    payload: &[u8],
) -> Result<(usize, Vec<ChampionMasteryValue>), &'static str> {
    let text = std::str::from_utf8(payload).map_err(|_| "INVALID_PAYLOAD")?;
    let (athlete_id, entries) = text.split_once('|').ok_or("MISSING_VALUE")?;
    let athlete_id = athlete_id
        .parse::<usize>()
        .map_err(|_| "INVALID_ID")?;

    let mut values = Vec::new();
    for entry in entries.split(';').filter(|entry| !entry.trim().is_empty()) {
        let (champion_id, mastery) =
            entry.split_once(':').ok_or("INVALID_MASTERY_ENTRY")?;

        values.push(ChampionMasteryValue {
            champion_id: champion_id.to_string(),
            mastery: mastery
                .parse::<u16>()
                .map_err(|_| "INVALID_MASTERY")?,
        });
    }

    validate_champion_mastery_values(&values)?;
    Ok((athlete_id, values))
}

fn response_ok_champion_mastery_probe(
    mastery_state: &str,
    available_champions: &str,
) -> String {
    format!(
        "OK|MASTERY_PROBE|{}|{}",
        hex_encode(mastery_state),
        hex_encode(available_champions)
    )
}

fn annual_salary_raw(athlete: &Athlete) -> String {
    match &athlete.contract {
        Contract::InContract { weekly_salary, .. } => weekly_salary
            .to_string()
            .parse::<f64>()
            .map(|weekly| (weekly * 52.0).to_string())
            .unwrap_or_default(),
        Contract::FreeAgent { .. } => String::new(),
    }
}

fn weekly_salary_raw(athlete: &Athlete) -> String {
    match &athlete.contract {
        Contract::InContract { weekly_salary, .. } => weekly_salary.to_string(),
        Contract::FreeAgent { .. } => String::new(),
    }
}

fn squad_status_raw(athlete: &Athlete) -> String {
    format!("{:?}", athlete.squad_status)
}

fn incentive_values_raw(athlete: &Athlete) -> (String, String, String, String, String) {
    let mut pog = String::new();
    let mut league_bonus = String::new();
    let mut league_rank = String::new();
    let mut match_bonus = String::new();
    let mut win_bonus = String::new();

    if let Contract::InContract { incentives, .. } = &athlete.contract {
        for incentive in incentives {
            match incentive {
                Incentive::OnPog { bonus } => pog = bonus.to_string(),
                Incentive::OnLeagueRank { bonus, rank } => {
                    league_bonus = bonus.to_string();
                    league_rank = rank.to_string();
                }
                Incentive::OnMatch { bonus, .. } => match_bonus = bonus.to_string(),
                Incentive::OnWin { bonus } => win_bonus = bonus.to_string(),
            }
        }
    }

    (pog, league_bonus, league_rank, match_bonus, win_bonus)
}

fn transfer_fee_raw(athlete: &Athlete) -> String {
    match &athlete.contract {
        Contract::InContract { transfer_fee, .. } => transfer_fee.to_string(),
        Contract::FreeAgent { .. } => String::new(),
    }
}

fn contract_team_id_raw(athlete: &Athlete) -> String {
    match &athlete.contract {
        Contract::InContract { team_id, .. } => team_id.to_string(),
        Contract::FreeAgent { .. } => String::new(),
    }
}

fn contract_start_date_raw(athlete: &Athlete) -> String {
    match &athlete.contract {
        Contract::InContract { start_date, .. } => start_date.to_string(),
        Contract::FreeAgent { .. } => String::new(),
    }
}

fn contract_end_date_raw(athlete: &Athlete) -> String {
    match &athlete.contract {
        Contract::InContract { end_date, .. } => end_date.to_string(),
        Contract::FreeAgent { .. } => String::new(),
    }
}

fn validate_contract_end_date_text(value: &str) -> Result<&str, &'static str> {
    let value = value.trim();
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes.iter().enumerate().any(|(i, b)| i != 4 && i != 7 && !b.is_ascii_digit())
    {
        return Err("INVALID_CONTRACT_DATE");
    }
    Ok(value)
}

fn player_contract_end_payload(athlete_id: usize, values: &PlayerContractEndValue) -> Vec<u8> {
    format!("{}|{}", athlete_id, values.end_date).into_bytes()
}

fn apply_contract_end_to_contract(
    contract: &mut Contract,
    requested: &str,
    free_agent_error: &'static str,
) -> Result<(), &'static str> {
    // Reuse the exact full-contract serialization already validated by the
    // Player/Staff Contract Editor. The wire command remains YYYY-MM-DD, while
    // the game-facing Contract end value is normalized to end-of-day.
    let candidate = contract_end_text(requested)?;
    match contract {
        Contract::InContract { end_date, .. } => {
            *end_date = candidate.parse().map_err(|_| "INVALID_CONTRACT_DATE")?;
            Ok(())
        }
        Contract::FreeAgent { .. } => Err(free_agent_error),
    }
}

fn apply_contract_end_to_athlete(
    athlete: &mut Athlete,
    values: &PlayerContractEndValue,
) -> Result<(), &'static str> {
    apply_contract_end_to_contract(
        &mut athlete.contract,
        &values.end_date,
        "PLAYER_FREE_AGENT",
    )
}

fn apply_staff_contract_end(
    contract: &mut Contract,
    values: &StaffContractEndValue,
) -> Result<(), &'static str> {
    apply_contract_end_to_contract(contract, &values.end_date, "STAFF_FREE_AGENT")
}

fn validate_contract_dates(start_date: &str, end_date: &str) -> Result<(), &'static str> {
    let start_date = validate_contract_end_date_text(start_date)?;
    let end_date = validate_contract_end_date_text(end_date)?;
    if end_date < start_date {
        return Err("CONTRACT_END_BEFORE_START");
    }
    Ok(())
}

fn contract_start_text(value: &str) -> Result<String, &'static str> {
    let value = validate_contract_end_date_text(value)?;
    Ok(format!("{value}T00:00:00"))
}

fn contract_end_text(value: &str) -> Result<String, &'static str> {
    let value = validate_contract_end_date_text(value)?;
    Ok(format!("{value}T23:59:59"))
}

fn build_active_contract(
    team_id: usize,
    start_date: &str,
    end_date: &str,
    annual_salary: &str,
    transfer_fee: &str,
) -> Result<Contract, &'static str> {
    validate_contract_dates(start_date, end_date)?;
    let annual = validate_annual_salary(annual_salary)?;
    let transfer = transfer_fee
        .parse::<f64>()
        .map_err(|_| "INVALID_TRANSFER_FEE")?;
    if !transfer.is_finite() || transfer < 0.0 {
        return Err("TRANSFER_FEE_OUT_OF_RANGE");
    }

    let weekly_text = (annual / 52.0).to_string();
    let start_text = contract_start_text(start_date)?;
    let end_text = contract_end_text(end_date)?;

    Ok(Contract::InContract {
        team_id,
        start_date: start_text.parse().map_err(|_| "INVALID_CONTRACT_DATE")?,
        end_date: end_text.parse().map_err(|_| "INVALID_CONTRACT_DATE")?,
        weekly_salary: weekly_text.parse().map_err(|_| "SALARY_TYPE_ERROR")?,
        transfer_fee: transfer.to_string().parse().map_err(|_| "TRANSFER_FEE_TYPE_ERROR")?,
        incentives: Vec::new(),
        transfer_requests: Vec::new(),
        recruit_requests: Vec::new(),
    })
}

fn apply_active_contract_fields(
    contract: &mut Contract,
    new_team_id: usize,
    start_date: &str,
    end_date: &str,
    annual_salary: &str,
    transfer_fee: &str,
) -> Result<(), &'static str> {
    validate_contract_dates(start_date, end_date)?;
    let annual = validate_annual_salary(annual_salary)?;
    let transfer = transfer_fee
        .parse::<f64>()
        .map_err(|_| "INVALID_TRANSFER_FEE")?;
    if !transfer.is_finite() || transfer < 0.0 {
        return Err("TRANSFER_FEE_OUT_OF_RANGE");
    }

    if matches!(contract, Contract::FreeAgent { .. }) {
        *contract = build_active_contract(
            new_team_id,
            start_date,
            end_date,
            annual_salary,
            transfer_fee,
        )?;
        return Ok(());
    }

    let start_text = contract_start_text(start_date)?;
    let end_text = contract_end_text(end_date)?;
    let weekly_text = (annual / 52.0).to_string();

    match contract {
        Contract::InContract {
            team_id,
            start_date: current_start_date,
            end_date: current_end_date,
            weekly_salary,
            transfer_fee: current_transfer_fee,
            transfer_requests,
            recruit_requests,
            ..
        } => {
            let team_changed = *team_id != new_team_id;
            *team_id = new_team_id;
            *current_start_date = start_text
                .parse()
                .map_err(|_| "INVALID_CONTRACT_DATE")?;
            *current_end_date = end_text
                .parse()
                .map_err(|_| "INVALID_CONTRACT_DATE")?;
            *weekly_salary = weekly_text
                .parse()
                .map_err(|_| "SALARY_TYPE_ERROR")?;
            *current_transfer_fee = transfer
                .to_string()
                .parse()
                .map_err(|_| "TRANSFER_FEE_TYPE_ERROR")?;

            if team_changed {
                transfer_requests.clear();
                recruit_requests.clear();
            }
            Ok(())
        }
        Contract::FreeAgent { .. } => unreachable!(),
    }
}

fn parse_contract_bool(value: &str) -> Result<bool, &'static str> {
    match value {
        "1" => Ok(true),
        "0" => Ok(false),
        _ => Err("INVALID_BOOLEAN"),
    }
}

fn parse_squad_status(value: &str) -> Result<SquadStatus, &'static str> {
    match value {
        "Core" => Ok(SquadStatus::Core),
        "Important" => Ok(SquadStatus::Important),
        "General" => Ok(SquadStatus::General),
        "Sub" => Ok(SquadStatus::Sub),
        "Prospect" => Ok(SquadStatus::Prospect),
        _ => Err("INVALID_SQUAD_STATUS"),
    }
}

fn validate_contract_bonus(value: &str) -> Result<(), &'static str> {
    let bonus = value.parse::<f64>().map_err(|_| "INVALID_CONTRACT_BONUS")?;
    if !bonus.is_finite() || bonus < 0.0 {
        return Err("CONTRACT_BONUS_OUT_OF_RANGE");
    }
    Ok(())
}

fn validate_player_contract_extras(values: &PlayerContractValue) -> Result<(), &'static str> {
    let _ = parse_squad_status(&values.squad_status)?;
    if values.pog_enabled {
        validate_contract_bonus(&values.pog_bonus)?;
    }
    if values.league_enabled {
        validate_contract_bonus(&values.league_bonus)?;
        let rank = values
            .league_rank
            .parse::<usize>()
            .map_err(|_| "INVALID_LEAGUE_RANK")?;
        if !(1..=10).contains(&rank) {
            return Err("INVALID_LEAGUE_RANK");
        }
    }
    if values.match_enabled {
        validate_contract_bonus(&values.match_bonus)?;
    }
    if values.win_enabled {
        validate_contract_bonus(&values.win_bonus)?;
    }
    Ok(())
}

fn build_player_incentives(values: &PlayerContractValue) -> Result<Vec<Incentive>, &'static str> {
    validate_player_contract_extras(values)?;
    let mut incentives = Vec::new();
    if values.pog_enabled {
        incentives.push(Incentive::OnPog {
            bonus: values.pog_bonus.parse().map_err(|_| "INVALID_CONTRACT_BONUS")?,
        });
    }
    if values.league_enabled {
        incentives.push(Incentive::OnLeagueRank {
            bonus: values.league_bonus.parse().map_err(|_| "INVALID_CONTRACT_BONUS")?,
            rank: values.league_rank.parse().map_err(|_| "INVALID_LEAGUE_RANK")?,
        });
    }
    if values.match_enabled {
        incentives.push(Incentive::OnMatch {
            bonus: values.match_bonus.parse().map_err(|_| "INVALID_CONTRACT_BONUS")?,
            match_id: 0,
        });
    }
    if values.win_enabled {
        incentives.push(Incentive::OnWin {
            bonus: values.win_bonus.parse().map_err(|_| "INVALID_CONTRACT_BONUS")?,
        });
    }
    Ok(incentives)
}

fn apply_player_contract_values(
    athlete: &mut Athlete,
    values: &PlayerContractValue,
) -> Result<(), &'static str> {
    apply_active_contract_fields(
        &mut athlete.contract,
        values.team_id,
        &values.start_date,
        &values.end_date,
        &values.annual_salary,
        &values.transfer_fee,
    )?;

    athlete.squad_status = parse_squad_status(&values.squad_status)?;
    let new_incentives = build_player_incentives(values)?;
    match &mut athlete.contract {
        Contract::InContract { incentives, .. } => {
            *incentives = new_incentives;
            Ok(())
        }
        Contract::FreeAgent { .. } => Err("PLAYER_FREE_AGENT"),
    }
}

fn player_contract_payload(athlete_id: usize, values: &PlayerContractValue) -> Vec<u8> {
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        athlete_id,
        values.team_id,
        values.start_date,
        values.end_date,
        values.annual_salary,
        values.transfer_fee,
        values.squad_status,
        if values.pog_enabled { 1 } else { 0 },
        values.pog_bonus,
        if values.league_enabled { 1 } else { 0 },
        values.league_bonus,
        values.league_rank,
        if values.match_enabled { 1 } else { 0 },
        values.match_bonus,
        if values.win_enabled { 1 } else { 0 },
        values.win_bonus,
    )
    .into_bytes()
}

fn parse_server_player_contract(
    payload: &[u8],
) -> Result<(usize, PlayerContractValue), &'static str> {
    let text = std::str::from_utf8(payload).map_err(|_| "INVALID_PAYLOAD")?;
    let mut parts = text.split('|');
    let athlete_id = parse_usize(parts.next())?;
    let values = PlayerContractValue {
        team_id: parse_usize(parts.next())?,
        start_date: parts.next().ok_or("MISSING_VALUE")?.to_string(),
        end_date: parts.next().ok_or("MISSING_VALUE")?.to_string(),
        annual_salary: parts.next().ok_or("MISSING_VALUE")?.to_string(),
        transfer_fee: parts.next().ok_or("MISSING_VALUE")?.to_string(),
        squad_status: parts.next().ok_or("MISSING_VALUE")?.to_string(),
        pog_enabled: parse_contract_bool(parts.next().ok_or("MISSING_VALUE")?)?,
        pog_bonus: parts.next().ok_or("MISSING_VALUE")?.to_string(),
        league_enabled: parse_contract_bool(parts.next().ok_or("MISSING_VALUE")?)?,
        league_bonus: parts.next().ok_or("MISSING_VALUE")?.to_string(),
        league_rank: parts.next().ok_or("MISSING_VALUE")?.to_string(),
        match_enabled: parse_contract_bool(parts.next().ok_or("MISSING_VALUE")?)?,
        match_bonus: parts.next().ok_or("MISSING_VALUE")?.to_string(),
        win_enabled: parse_contract_bool(parts.next().ok_or("MISSING_VALUE")?)?,
        win_bonus: parts.next().ok_or("MISSING_VALUE")?.to_string(),
    };
    validate_contract_dates(&values.start_date, &values.end_date)?;
    validate_annual_salary(&values.annual_salary)?;
    let transfer = values
        .transfer_fee
        .parse::<f64>()
        .map_err(|_| "INVALID_TRANSFER_FEE")?;
    if !transfer.is_finite() || transfer < 0.0 {
        return Err("TRANSFER_FEE_OUT_OF_RANGE");
    }
    validate_player_contract_extras(&values)?;
    Ok((athlete_id, values))
}

fn write_player_contract(
    scene: &mut Scene,
    athlete_id: usize,
    values: PlayerContractValue,
) -> Result<PlayerSnapshot, &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };

    {
        let db = data.db();
        if !db.teams.contains_key(&values.team_id) {
            return Err("TEAM_NOT_FOUND");
        }
        if !db.athletes.contains_key(&athlete_id) {
            return Err("PLAYER_NOT_FOUND");
        }
    }

    if !data.send_mod_command(
        MOD_ID,
        "set_player_contract",
        player_contract_payload(athlete_id, &values),
    ) {
        return Err("SERVER_COMMAND_FAILED");
    }

    let mut db = data.db_mut();
    let Some(athlete) = db.athletes.get_mut(&athlete_id) else {
        return Err("PLAYER_NOT_FOUND");
    };
    apply_player_contract_values(athlete, &values)?;

    // Return the exact client-side object we just edited. Re-reading after
    // dropping the guard can race a server snapshot and hide a successful
    // local write behind the previous contract values.
    let snapshot = snapshot_from_athlete(athlete_id, athlete);
    if snapshot.contract_team_id != values.team_id.to_string()
        || !snapshot.contract_start_date.starts_with(&values.start_date)
        || !snapshot.contract_end_date.starts_with(&values.end_date)
    {
        return Err("CONTRACT_WRITE_NOT_APPLIED");
    }
    Ok(snapshot)
}

fn apply_player_contract_server(
    ctx: &mut ServerModContext,
    athlete_id: usize,
    values: &PlayerContractValue,
) -> Result<(), &'static str> {
    if ctx.database.teams.get(values.team_id).is_none() {
        return Err("TEAM_NOT_FOUND");
    }
    let Some(athlete) = ctx.database.athletes.get_mut(athlete_id) else {
        return Err("PLAYER_NOT_FOUND");
    };
    apply_player_contract_values(athlete, values)?;
    if let Ok(mut overrides) = contract_end_overrides().lock() {
        overrides.remove(&athlete_id);
    }
    Ok(())
}

fn staff_contract_payload(staff_id: usize, values: &StaffContractValue) -> Vec<u8> {
    format!(
        "{}|{}|{}|{}|{}",
        staff_id,
        values.team_id,
        values.start_date,
        values.end_date,
        values.annual_salary,
    )
    .into_bytes()
}

fn parse_server_staff_contract(
    payload: &[u8],
) -> Result<(usize, StaffContractValue), &'static str> {
    let text = std::str::from_utf8(payload).map_err(|_| "INVALID_PAYLOAD")?;
    let mut parts = text.split('|');
    let staff_id = parse_usize(parts.next())?;
    let values = StaffContractValue {
        team_id: parse_usize(parts.next())?,
        start_date: parts.next().ok_or("MISSING_VALUE")?.to_string(),
        end_date: parts.next().ok_or("MISSING_VALUE")?.to_string(),
        annual_salary: parts.next().ok_or("MISSING_VALUE")?.to_string(),
    };
    validate_contract_dates(&values.start_date, &values.end_date)?;
    validate_annual_salary(&values.annual_salary)?;
    Ok((staff_id, values))
}

fn write_staff_contract(
    scene: &mut Scene,
    staff_id: usize,
    values: StaffContractValue,
) -> Result<StaffSnapshot, &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };

    {
        let db = data.db();
        if !db.teams.contains_key(&values.team_id) {
            return Err("TEAM_NOT_FOUND");
        }
        if !db.staffs.contains_key(&staff_id) {
            return Err("STAFF_NOT_FOUND");
        }
    }

    if !data.send_mod_command(
        MOD_ID,
        "set_staff_contract",
        staff_contract_payload(staff_id, &values),
    ) {
        return Err("SERVER_COMMAND_FAILED");
    }

    let mut db = data.db_mut();
    let Some(staff) = db.staffs.get_mut(&staff_id) else {
        return Err("STAFF_NOT_FOUND");
    };
    apply_active_contract_fields(
        &mut staff.contract,
        values.team_id,
        &values.start_date,
        &values.end_date,
        &values.annual_salary,
        "0",
    )?;
    drop(db);

    let snapshot = read_staff(scene, staff_id)?;
    if snapshot.contract_team_id != values.team_id.to_string()
        || !snapshot.contract_start_date.starts_with(&values.start_date)
        || !snapshot.contract_end_date.starts_with(&values.end_date)
    {
        return Err("CONTRACT_WRITE_NOT_APPLIED");
    }
    Ok(snapshot)
}

fn apply_staff_contract_server(
    ctx: &mut ServerModContext,
    staff_id: usize,
    values: &StaffContractValue,
) -> Result<(), &'static str> {
    if ctx.database.teams.get(values.team_id).is_none() {
        return Err("TEAM_NOT_FOUND");
    }
    let Some(staff) = ctx.database.staffs.get_mut(staff_id) else {
        return Err("STAFF_NOT_FOUND");
    };
    apply_active_contract_fields(
        &mut staff.contract,
        values.team_id,
        &values.start_date,
        &values.end_date,
        &values.annual_salary,
        "0",
    )?;
    if let Ok(mut overrides) = staff_contract_end_overrides().lock() {
        overrides.remove(&staff_id);
    }
    Ok(())
}

fn validate_annual_salary(value: &str) -> Result<f64, &'static str> {
    let annual = value.parse::<f64>().map_err(|_| "INVALID_SALARY")?;
    if !annual.is_finite() || annual < 0.0 {
        return Err("SALARY_OUT_OF_RANGE");
    }
    Ok(annual)
}

fn player_salary_payload(athlete_id: usize, values: &PlayerSalaryValue) -> Vec<u8> {
    format!("{}|{}", athlete_id, values.annual_salary).into_bytes()
}

fn apply_salary_to_athlete(
    athlete: &mut Athlete,
    values: &PlayerSalaryValue,
) -> Result<(), &'static str> {
    let annual = validate_annual_salary(&values.annual_salary)?;
    let weekly_text = (annual / 52.0).to_string();
    match &mut athlete.contract {
        Contract::InContract { weekly_salary, .. } => {
            *weekly_salary = weekly_text
                .parse()
                .map_err(|_| "SALARY_TYPE_ERROR")?;
            Ok(())
        }
        Contract::FreeAgent { .. } => Err("PLAYER_FREE_AGENT"),
    }
}

fn read_player(scene: &mut Scene, athlete_id: usize) -> Result<PlayerSnapshot, &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };

    let db = data.db();
    let Some(athlete) = db.athletes.get(&athlete_id) else {
        return Err("PLAYER_NOT_FOUND");
    };

    let stat = &athlete.stat;
    let (incentive_pog_bonus, incentive_league_bonus, incentive_league_rank, incentive_match_bonus, incentive_win_bonus) = incentive_values_raw(athlete);

    Ok(PlayerSnapshot {
        id: athlete_id,
        name: athlete.name.to_string(),
        last_hit: stat.last_hit.to_string(),
        skill_avoid: stat.skill_avoid.to_string(),
        skill_hit: stat.skill_hit.to_string(),
        control_speed: stat.control_speed.to_string(),
        positioning: stat.positioning.to_string(),
        judgement: stat.judgement.to_string(),
        mental: stat.mental.to_string(),
        concentration: stat.concentration.to_string(),
        order: stat.order.to_string(),
        roaming: stat.roaming.to_string(),
        aggressive: stat.aggressive.to_string(),
        ego: stat.ego.to_string(),
        top: stat.top.to_string(),
        jungle: stat.jungle.to_string(),
        mid: stat.mid.to_string(),
        bottom: stat.bottom.to_string(),
        support: stat.support.to_string(),
        potential: athlete.hidden.potential.to_string(),
        annual_salary: annual_salary_raw(athlete),
        weekly_salary: weekly_salary_raw(athlete),
        contract_team_id: contract_team_id_raw(athlete),
        contract_start_date: contract_start_date_raw(athlete),
        contract_end_date: contract_end_date_raw(athlete),
        transfer_fee: transfer_fee_raw(athlete),
        squad_status: squad_status_raw(athlete),
        incentive_pog_bonus,
        incentive_league_bonus,
        incentive_league_rank,
        incentive_match_bonus,
        incentive_win_bonus,
        primary_region: primary_region_raw(athlete),
        communication_raw: communication_raw(athlete),
        communication_xp_raw: communication_xp_raw(athlete),
    })
}

fn write_player_name(
    scene: &mut Scene,
    athlete_id: usize,
    values: EntityNameValue,
) -> Result<(), &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };

    let name = validate_entity_name(&values.name)?;
    let values = EntityNameValue { name };

    if !data.send_mod_command(
        MOD_ID,
        "set_player_name",
        entity_name_payload(athlete_id, &values),
    ) {
        return Err("SERVER_COMMAND_FAILED");
    }

    let mut db = data.db_mut();
    let Some(athlete) = db.athletes.get_mut(&athlete_id) else {
        return Err("PLAYER_NOT_FOUND");
    };
    athlete.name = values.name;
    Ok(())
}

fn validate_stat_text(value: &str) -> Result<(), &'static str> {
    let numeric = value.parse::<f64>().map_err(|_| "INVALID_STAT")?;
    if !numeric.is_finite()
        || numeric.fract().abs() > f64::EPSILON
        || !(1.0..=100.0).contains(&numeric)
    {
        return Err("STAT_OUT_OF_RANGE");
    }
    Ok(())
}

fn validate_player_stats(values: &PlayerStatValues) -> Result<(), &'static str> {
    for value in [
        &values.last_hit,
        &values.skill_avoid,
        &values.skill_hit,
        &values.control_speed,
        &values.positioning,
        &values.judgement,
        &values.mental,
        &values.concentration,
        &values.order,
        &values.roaming,
        &values.aggressive,
        &values.ego,
    ] {
        validate_stat_text(value)?;
    }
    Ok(())
}

fn player_stats_payload(athlete_id: usize, values: &PlayerStatValues) -> Vec<u8> {
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
        athlete_id,
        values.last_hit,
        values.skill_avoid,
        values.skill_hit,
        values.control_speed,
        values.positioning,
        values.judgement,
        values.mental,
        values.concentration,
        values.order,
        values.roaming,
        values.aggressive,
        values.ego,
    )
    .into_bytes()
}

fn apply_stats_to_athlete(
    athlete: &mut Athlete,
    values: &PlayerStatValues,
) -> Result<(), &'static str> {
    let stat = &mut athlete.stat;
    stat.last_hit = parse_stat_value(&values.last_hit)?;
    stat.skill_avoid = parse_stat_value(&values.skill_avoid)?;
    stat.skill_hit = parse_stat_value(&values.skill_hit)?;
    stat.control_speed = parse_stat_value(&values.control_speed)?;
    stat.positioning = parse_stat_value(&values.positioning)?;
    stat.judgement = parse_stat_value(&values.judgement)?;
    stat.mental = parse_stat_value(&values.mental)?;
    stat.concentration = parse_stat_value(&values.concentration)?;
    stat.order = parse_stat_value(&values.order)?;
    stat.roaming = parse_stat_value(&values.roaming)?;
    stat.aggressive = parse_stat_value(&values.aggressive)?;
    stat.ego = parse_stat_value(&values.ego)?;
    Ok(())
}

fn snapshot_from_athlete(athlete_id: usize, athlete: &Athlete) -> PlayerSnapshot {
    let stat = &athlete.stat;
    let (incentive_pog_bonus, incentive_league_bonus, incentive_league_rank, incentive_match_bonus, incentive_win_bonus) = incentive_values_raw(athlete);
    PlayerSnapshot {
        id: athlete_id,
        name: athlete.name.to_string(),
        last_hit: stat.last_hit.to_string(),
        skill_avoid: stat.skill_avoid.to_string(),
        skill_hit: stat.skill_hit.to_string(),
        control_speed: stat.control_speed.to_string(),
        positioning: stat.positioning.to_string(),
        judgement: stat.judgement.to_string(),
        mental: stat.mental.to_string(),
        concentration: stat.concentration.to_string(),
        order: stat.order.to_string(),
        roaming: stat.roaming.to_string(),
        aggressive: stat.aggressive.to_string(),
        ego: stat.ego.to_string(),
        top: stat.top.to_string(),
        jungle: stat.jungle.to_string(),
        mid: stat.mid.to_string(),
        bottom: stat.bottom.to_string(),
        support: stat.support.to_string(),
        potential: athlete.hidden.potential.to_string(),
        annual_salary: annual_salary_raw(athlete),
        weekly_salary: weekly_salary_raw(athlete),
        contract_team_id: contract_team_id_raw(athlete),
        contract_start_date: contract_start_date_raw(athlete),
        contract_end_date: contract_end_date_raw(athlete),
        transfer_fee: transfer_fee_raw(athlete),
        squad_status: squad_status_raw(athlete),
        incentive_pog_bonus,
        incentive_league_bonus,
        incentive_league_rank,
        incentive_match_bonus,
        incentive_win_bonus,
        primary_region: primary_region_raw(athlete),
        communication_raw: communication_raw(athlete),
        communication_xp_raw: communication_xp_raw(athlete),
    }
}

fn write_player_stats(
    scene: &mut Scene,
    athlete_id: usize,
    values: PlayerStatValues,
) -> Result<PlayerSnapshot, &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };

    validate_player_stats(&values)?;

    // ClientDatabase is only the client-side snapshot. Ask the management/server
    // side to change the authoritative Database so the values survive Proceed,
    // simulation and normal save/autosave flows.
    if !data.send_mod_command(
        MOD_ID,
        "set_player_stats",
        player_stats_payload(athlete_id, &values),
    ) {
        return Err("SERVER_COMMAND_FAILED");
    }

    // Mirror the edit into the client snapshot immediately so the modifier and
    // current Player Info view do not need to wait for the next server sync.
    let mut db = data.db_mut();
    let Some(athlete) = db.athletes.get_mut(&athlete_id) else {
        return Err("PLAYER_NOT_FOUND");
    };

    apply_stats_to_athlete(athlete, &values)?;
    Ok(snapshot_from_athlete(athlete_id, athlete))
}

fn parse_server_player_stats(payload: &[u8]) -> Result<(usize, PlayerStatValues), &'static str> {
    let text = std::str::from_utf8(payload).map_err(|_| "INVALID_PAYLOAD")?;
    let mut parts = text.split('|');
    let athlete_id = parse_usize(parts.next())?;
    let values = PlayerStatValues {
        last_hit: parts.next().ok_or("MISSING_VALUE")?.to_string(),
        skill_avoid: parts.next().ok_or("MISSING_VALUE")?.to_string(),
        skill_hit: parts.next().ok_or("MISSING_VALUE")?.to_string(),
        control_speed: parts.next().ok_or("MISSING_VALUE")?.to_string(),
        positioning: parts.next().ok_or("MISSING_VALUE")?.to_string(),
        judgement: parts.next().ok_or("MISSING_VALUE")?.to_string(),
        mental: parts.next().ok_or("MISSING_VALUE")?.to_string(),
        concentration: parts.next().ok_or("MISSING_VALUE")?.to_string(),
        order: parts.next().ok_or("MISSING_VALUE")?.to_string(),
        roaming: parts.next().ok_or("MISSING_VALUE")?.to_string(),
        aggressive: parts.next().ok_or("MISSING_VALUE")?.to_string(),
        ego: parts.next().ok_or("MISSING_VALUE")?.to_string(),
    };
    validate_player_stats(&values)?;
    Ok((athlete_id, values))
}


fn validate_condition_number(value: &str, invalid: &'static str) -> Result<(), &'static str> {
    let parsed = value.parse::<f64>().map_err(|_| invalid)?;
    if !parsed.is_finite() || !(0.0..=100.0).contains(&parsed) {
        return Err("CONDITION_OUT_OF_RANGE");
    }
    Ok(())
}

fn validate_player_condition(values: &PlayerConditionValue) -> Result<(), &'static str> {
    validate_condition_number(&values.stamina, "INVALID_STAMINA")?;
    validate_condition_number(&values.condition, "INVALID_CONDITION")?;
    Ok(())
}

fn player_condition_payload(athlete_id: usize, values: &PlayerConditionValue) -> Vec<u8> {
    format!("{}|{}|{}", athlete_id, values.stamina, values.condition).into_bytes()
}

fn apply_condition_to_athlete(
    athlete_id: usize,
    athlete: &mut Athlete,
    values: &PlayerConditionValue,
) -> Result<PlayerConditionSnapshot, &'static str> {
    validate_player_condition(values)?;
    athlete.management.stamina = values
        .stamina
        .parse()
        .map_err(|_| "INVALID_STAMINA")?;
    athlete.management.condition = values
        .condition
        .parse()
        .map_err(|_| "INVALID_CONDITION")?;

    Ok(PlayerConditionSnapshot {
        athlete_id,
        stamina: athlete.management.stamina.to_string(),
        condition: athlete.management.condition.to_string(),
    })
}

fn write_player_condition(
    scene: &mut Scene,
    athlete_id: usize,
    values: PlayerConditionValue,
) -> Result<PlayerConditionSnapshot, &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };

    validate_player_condition(&values)?;
    if !data.send_mod_command(
        MOD_ID,
        "set_player_condition",
        player_condition_payload(athlete_id, &values),
    ) {
        return Err("SERVER_COMMAND_FAILED");
    }

    let mut db = data.db_mut();
    let Some(athlete) = db.athletes.get_mut(&athlete_id) else {
        return Err("PLAYER_NOT_FOUND");
    };
    apply_condition_to_athlete(athlete_id, athlete, &values)
}

fn parse_server_player_condition(
    payload: &[u8],
) -> Result<(usize, PlayerConditionValue), &'static str> {
    let text = std::str::from_utf8(payload).map_err(|_| "INVALID_PAYLOAD")?;
    let mut parts = text.split('|');
    let athlete_id = parse_usize(parts.next())?;
    let values = PlayerConditionValue {
        stamina: parts.next().ok_or("MISSING_VALUE")?.to_string(),
        condition: parts.next().ok_or("MISSING_VALUE")?.to_string(),
    };
    validate_player_condition(&values)?;
    Ok((athlete_id, values))
}

fn validate_position_value(value: u16) -> Result<(), &'static str> {
    if value > 100 {
        return Err("POSITION_OUT_OF_RANGE");
    }
    Ok(())
}

fn validate_player_positions(values: &PlayerPositionValues) -> Result<(), &'static str> {
    let positions = [
        values.top,
        values.jungle,
        values.mid,
        values.bottom,
        values.support,
    ];
    for value in positions {
        validate_position_value(value)?;
    }
    if positions.into_iter().filter(|value| *value > 0).count() > 3 {
        return Err("TOO_MANY_POSITIONS");
    }
    Ok(())
}

fn player_positions_payload(athlete_id: usize, values: PlayerPositionValues) -> Vec<u8> {
    format!(
        "{}|{}|{}|{}|{}|{}",
        athlete_id, values.top, values.jungle, values.mid, values.bottom, values.support
    )
    .into_bytes()
}

fn parse_position_value<T>(value: u16) -> Result<T, &'static str>
where
    T: std::str::FromStr,
{
    validate_position_value(value)?;
    value.to_string().parse::<T>().map_err(|_| "INVALID_POSITION")
}

fn apply_positions_to_athlete(
    athlete: &mut Athlete,
    values: PlayerPositionValues,
) -> Result<(), &'static str> {
    validate_player_positions(&values)?;

    // TFM2 stores one independent proficiency value for every role. A value
    // of 0 means that the role is absent. This is why v0.1.5 left ghost roles
    // behind: it reset unselected roles to 1 instead of removing them.
    let stat = &mut athlete.stat;
    stat.top = parse_position_value(values.top)?;
    stat.jungle = parse_position_value(values.jungle)?;
    stat.mid = parse_position_value(values.mid)?;
    stat.bottom = parse_position_value(values.bottom)?;
    stat.support = parse_position_value(values.support)?;
    Ok(())
}

fn write_player_positions(
    scene: &mut Scene,
    athlete_id: usize,
    values: PlayerPositionValues,
) -> Result<PlayerSnapshot, &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };

    validate_player_positions(&values)?;

    if !data.send_mod_command(
        MOD_ID,
        "set_player_positions",
        player_positions_payload(athlete_id, values),
    ) {
        return Err("SERVER_COMMAND_FAILED");
    }

    let mut db = data.db_mut();
    let Some(athlete) = db.athletes.get_mut(&athlete_id) else {
        return Err("PLAYER_NOT_FOUND");
    };

    apply_positions_to_athlete(athlete, values)?;
    Ok(snapshot_from_athlete(athlete_id, athlete))
}

fn parse_server_player_positions(
    payload: &[u8],
) -> Result<(usize, PlayerPositionValues), &'static str> {
    let text = std::str::from_utf8(payload).map_err(|_| "INVALID_PAYLOAD")?;
    let mut parts = text.split('|');
    let athlete_id = parse_usize(parts.next())?;
    let values = PlayerPositionValues {
        top: parse_u16(parts.next(), "INVALID_POSITION")?,
        jungle: parse_u16(parts.next(), "INVALID_POSITION")?,
        mid: parse_u16(parts.next(), "INVALID_POSITION")?,
        bottom: parse_u16(parts.next(), "INVALID_POSITION")?,
        support: parse_u16(parts.next(), "INVALID_POSITION")?,
    };
    validate_player_positions(&values)?;
    Ok((athlete_id, values))
}

fn validate_player_potential(values: PlayerPotentialValue) -> Result<(), &'static str> {
    if !(1..=100).contains(&values.potential) {
        return Err("POTENTIAL_OUT_OF_RANGE");
    }
    Ok(())
}

fn player_potential_payload(athlete_id: usize, values: PlayerPotentialValue) -> Vec<u8> {
    format!("{}|{}", athlete_id, values.potential).into_bytes()
}

fn apply_potential_to_athlete(
    athlete: &mut Athlete,
    values: PlayerPotentialValue,
) -> Result<(), &'static str> {
    validate_player_potential(values)?;
    athlete.hidden.potential = parse_stat_value(&values.potential.to_string())?;
    Ok(())
}

fn write_player_potential(
    scene: &mut Scene,
    athlete_id: usize,
    values: PlayerPotentialValue,
) -> Result<PlayerSnapshot, &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };

    validate_player_potential(values)?;

    if !data.send_mod_command(
        MOD_ID,
        "set_player_potential",
        player_potential_payload(athlete_id, values),
    ) {
        return Err("SERVER_COMMAND_FAILED");
    }

    let mut db = data.db_mut();
    {
        let Some(athlete) = db.athletes.get_mut(&athlete_id) else {
            return Err("PLAYER_NOT_FOUND");
        };
        apply_potential_to_athlete(athlete, values)?;
    }

    let Some(athlete) = db.athletes.get(&athlete_id) else {
        return Err("PLAYER_NOT_FOUND");
    };
    Ok(snapshot_from_athlete(athlete_id, athlete))
}

fn parse_server_player_potential(
    payload: &[u8],
) -> Result<(usize, PlayerPotentialValue), &'static str> {
    let text = std::str::from_utf8(payload).map_err(|_| "INVALID_PAYLOAD")?;
    let mut parts = text.split('|');
    let athlete_id = parse_usize(parts.next())?;
    let values = PlayerPotentialValue {
        potential: parse_u16(parts.next(), "INVALID_POTENTIAL")?,
    };
    validate_player_potential(values)?;
    Ok((athlete_id, values))
}

fn write_player_salary(
    scene: &mut Scene,
    athlete_id: usize,
    values: PlayerSalaryValue,
) -> Result<PlayerSnapshot, &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };

    validate_annual_salary(&values.annual_salary)?;

    if !data.send_mod_command(
        MOD_ID,
        "set_player_salary",
        player_salary_payload(athlete_id, &values),
    ) {
        return Err("SERVER_COMMAND_FAILED");
    }

    let mut db = data.db_mut();
    let Some(athlete) = db.athletes.get_mut(&athlete_id) else {
        return Err("PLAYER_NOT_FOUND");
    };
    apply_salary_to_athlete(athlete, &values)?;
    Ok(snapshot_from_athlete(athlete_id, athlete))
}

fn parse_server_player_salary(
    payload: &[u8],
) -> Result<(usize, PlayerSalaryValue), &'static str> {
    let text = std::str::from_utf8(payload).map_err(|_| "INVALID_PAYLOAD")?;
    let mut parts = text.split('|');
    let athlete_id = parse_usize(parts.next())?;
    let annual_salary = parts.next().ok_or("MISSING_VALUE")?.to_string();
    validate_annual_salary(&annual_salary)?;
    Ok((athlete_id, PlayerSalaryValue { annual_salary }))
}

fn write_player_contract_end(
    scene: &mut Scene,
    athlete_id: usize,
    values: PlayerContractEndValue,
) -> Result<PlayerSnapshot, &'static str> {
    validate_contract_end_date_text(&values.end_date)?;
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };

    if !data.send_mod_command(
        MOD_ID,
        "set_player_contract_end",
        player_contract_end_payload(athlete_id, &values),
    ) {
        return Err("SERVER_COMMAND_FAILED");
    }

    let mut db = data.db_mut();
    let Some(athlete) = db.athletes.get_mut(&athlete_id) else {
        return Err("PLAYER_NOT_FOUND");
    };
    apply_contract_end_to_athlete(athlete, &values)?;
    drop(db);
    read_player(scene, athlete_id)
}

fn parse_server_player_contract_end(
    payload: &[u8],
) -> Result<(usize, PlayerContractEndValue), &'static str> {
    let text = std::str::from_utf8(payload).map_err(|_| "INVALID_PAYLOAD")?;
    let mut parts = text.split('|');
    let athlete_id = parse_usize(parts.next())?;
    let end_date = parts.next().ok_or("MISSING_VALUE")?.to_string();
    validate_contract_end_date_text(&end_date)?;
    Ok((athlete_id, PlayerContractEndValue { end_date }))
}

fn validate_player_communication(values: PlayerCommunicationValue) -> Result<(), &'static str> {
    if values.level > 100 {
        return Err("COMMUNICATION_OUT_OF_RANGE");
    }
    Ok(())
}

fn player_communication_payload(athlete_id: usize, values: PlayerCommunicationValue) -> Vec<u8> {
    format!("{}|{}|{}", athlete_id, values.region_id, values.level).into_bytes()
}

fn apply_communication_to_athlete(
    athlete: &mut Athlete,
    values: PlayerCommunicationValue,
) -> Result<(), &'static str> {
    validate_player_communication(values)?;
    if values.level == 0 {
        athlete.stat.language.remove(&values.region_id);
    } else {
        athlete.stat.language.insert(values.region_id, values.level);
    }
    Ok(())
}

fn write_player_communication(
    scene: &mut Scene,
    athlete_id: usize,
    values: PlayerCommunicationValue,
) -> Result<PlayerSnapshot, &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };
    validate_player_communication(values)?;

    if !data.send_mod_command(
        MOD_ID,
        "set_player_communication",
        player_communication_payload(athlete_id, values),
    ) {
        return Err("SERVER_COMMAND_FAILED");
    }

    let mut db = data.db_mut();
    let Some(athlete) = db.athletes.get_mut(&athlete_id) else {
        return Err("PLAYER_NOT_FOUND");
    };
    apply_communication_to_athlete(athlete, values)?;
    Ok(snapshot_from_athlete(athlete_id, athlete))
}

fn parse_server_player_communication(
    payload: &[u8],
) -> Result<(usize, PlayerCommunicationValue), &'static str> {
    let text = std::str::from_utf8(payload).map_err(|_| "INVALID_PAYLOAD")?;
    let mut parts = text.split('|');
    let athlete_id = parse_usize(parts.next())?;
    let values = PlayerCommunicationValue {
        region_id: parse_usize(parts.next())?,
        level: parts.next().ok_or("MISSING_VALUE")?.parse::<usize>().map_err(|_| "INVALID_COMMUNICATION")?,
    };
    validate_player_communication(values)?;
    Ok((athlete_id, values))
}

fn player_communication_max_payload(athlete_id: usize, region_ids: &[usize]) -> Vec<u8> {
    let ids = region_ids
        .iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(",");
    format!("{}|{}", athlete_id, ids).into_bytes()
}

fn apply_communication_max_to_athlete(athlete: &mut Athlete, region_ids: &[usize]) {
    let primary_region = athlete.get_primary_region();
    for region_id in region_ids {
        if Some(*region_id) != primary_region {
            athlete.stat.language.insert(*region_id, 100);
        }
    }
}

fn write_player_communication_max(
    scene: &mut Scene,
    athlete_id: usize,
) -> Result<PlayerSnapshot, &'static str> {
    let region_ids = detected_region_ids(scene)?;
    if region_ids.is_empty() {
        return Err("NO_REGIONS_DETECTED");
    }

    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };

    if !data.send_mod_command(
        MOD_ID,
        "set_player_communication_max",
        player_communication_max_payload(athlete_id, &region_ids),
    ) {
        return Err("SERVER_COMMAND_FAILED");
    }

    let mut db = data.db_mut();
    let Some(athlete) = db.athletes.get_mut(&athlete_id) else {
        return Err("PLAYER_NOT_FOUND");
    };
    apply_communication_max_to_athlete(athlete, &region_ids);
    Ok(snapshot_from_athlete(athlete_id, athlete))
}

fn parse_server_player_communication_max(
    payload: &[u8],
) -> Result<(usize, Vec<usize>), &'static str> {
    let text = std::str::from_utf8(payload).map_err(|_| "INVALID_PAYLOAD")?;
    let (athlete_id, ids) = text.split_once('|').ok_or("MISSING_VALUE")?;
    let athlete_id = athlete_id.parse::<usize>().map_err(|_| "INVALID_ID")?;
    let mut region_ids = Vec::new();
    if !ids.trim().is_empty() {
        for id in ids.split(',') {
            region_ids.push(id.parse::<usize>().map_err(|_| "INVALID_ID")?);
        }
    }
    region_ids.sort_unstable();
    region_ids.dedup();
    if region_ids.is_empty() {
        return Err("NO_REGIONS_DETECTED");
    }
    Ok((athlete_id, region_ids))
}


fn response_ok_recruitment_settings() -> String {
    format!(
        "OK|RECRUITMENT|{}|{}",
        if TRANSFER_ALWAYS_SUCCESS.load(Ordering::SeqCst) { 1 } else { 0 },
        if RECRUITMENT_INSTANT_RETRY.load(Ordering::SeqCst) { 1 } else { 0 },
    )
}

fn set_transfer_always_success(
    scene: &mut Scene,
    enabled: bool,
) -> Result<(), &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };

    let team_id = data.player_team_id();
    TRANSFER_ALWAYS_SUCCESS.store(enabled, Ordering::SeqCst);
    TRANSFER_ALWAYS_SUCCESS_TEAM_ID.store(team_id, Ordering::SeqCst);

    let payload = format!("{}|{}", team_id, if enabled { 1 } else { 0 }).into_bytes();
    if !data.send_mod_command(MOD_ID, "set_transfer_always_success", payload) {
        return Err("SERVER_COMMAND_FAILED");
    }

    Ok(())
}

fn set_recruitment_instant_retry(
    scene: &mut Scene,
    enabled: bool,
) -> Result<(), &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };

    let team_id = data.player_team_id();
    RECRUITMENT_INSTANT_RETRY.store(enabled, Ordering::SeqCst);
    RECRUITMENT_INSTANT_RETRY_TEAM_ID.store(team_id, Ordering::SeqCst);

    let payload = format!("{}|{}", team_id, if enabled { 1 } else { 0 }).into_bytes();
    if !data.send_mod_command(MOD_ID, "set_recruitment_instant_retry", payload) {
        return Err("SERVER_COMMAND_FAILED");
    }

    Ok(())
}

fn sync_recruitment_runtime_toggles(scene: &mut Scene) {
    let Scene::InGame { data } = scene else {
        return;
    };

    let team_id = data.player_team_id();

    // Loading another save restarts the management server, but the native DLL stays
    // loaded. Keep the user's runtime preferences and re-send them once the new
    // career is available instead of requiring an OFF/ON toggle in the editor.
    if TRANSFER_ALWAYS_SUCCESS.load(Ordering::SeqCst)
        && TRANSFER_ALWAYS_SUCCESS_TEAM_ID.load(Ordering::SeqCst) != team_id
    {
        let payload = format!("{}|1", team_id).into_bytes();
        let _ = data.send_mod_command(MOD_ID, "set_transfer_always_success", payload);
    }

    if RECRUITMENT_INSTANT_RETRY.load(Ordering::SeqCst)
        && RECRUITMENT_INSTANT_RETRY_TEAM_ID.load(Ordering::SeqCst) != team_id
    {
        let payload = format!("{}|1", team_id).into_bytes();
        let _ = data.send_mod_command(MOD_ID, "set_recruitment_instant_retry", payload);
    }
}

fn clear_player_recruitment_cooldowns(ctx: &mut ServerModContext) {
    if !RECRUITMENT_INSTANT_RETRY.load(Ordering::SeqCst) {
        return;
    }

    let team_id = RECRUITMENT_INSTANT_RETRY_TEAM_ID.load(Ordering::SeqCst);
    if team_id == usize::MAX {
        return;
    }

    for athlete in ctx.database.athletes.iter_mut() {
        match &mut athlete.contract {
            Contract::InContract {
                transfer_requests,
                recruit_requests,
                ..
            } => {
                for request in transfer_requests.iter_mut() {
                    if request.team_id == team_id {
                        // Moving cooldown_until back to the request's last action date
                        // makes the request immediately retryable without fabricating
                        // a game date in the bridge.
                        request.cooldown_until = Some(request.last_date);
                    }
                }
                for request in recruit_requests.iter_mut() {
                    if request.team_id == team_id {
                        request.cooldown_until = Some(request.last_date);
                    }
                }
            }
            Contract::FreeAgent { requests } => {
                for request in requests.iter_mut() {
                    if request.team_id == team_id {
                        request.cooldown_until = Some(request.last_date);
                    }
                }
            }
        }
    }
}

fn force_transfer_request_success(ctx: &mut ServerModContext) {
    if !TRANSFER_ALWAYS_SUCCESS.load(Ordering::SeqCst) {
        return;
    }

    let team_id = TRANSFER_ALWAYS_SUCCESS_TEAM_ID.load(Ordering::SeqCst);
    if team_id == usize::MAX {
        return;
    }

    for athlete in ctx.database.athletes.iter_mut() {
        match &mut athlete.contract {
            Contract::InContract {
                transfer_requests,
                recruit_requests,
                ..
            } => {
                // A contracted-player acquisition has two sides: the seller's
                // transfer paper and the player's recruit/salary paper. Force
                // only requests belonging to the human-controlled team.
                for request in transfer_requests.iter_mut() {
                    if request.team_id != team_id {
                        continue;
                    }
                    if let Some(paper) = request.phase.last_mut() {
                        paper.state = PaperState::Accepted;
                    }
                }

                for request in recruit_requests.iter_mut() {
                    if request.team_id != team_id {
                        continue;
                    }
                    if let Some(paper) = request.phase.last_mut() {
                        paper.state = PaperState::Accepted;
                    }
                }
            }
            Contract::FreeAgent { requests } => {
                for request in requests.iter_mut() {
                    if request.team_id != team_id {
                        continue;
                    }
                    if let Some(paper) = request.phase.last_mut() {
                        paper.state = PaperState::Accepted;
                    }
                }
            }
        }
    }
}

fn process_game_requests(scene: &mut Scene) {
    let Some(rx) = REQUEST_RX.get() else {
        return;
    };

    let Ok(rx) = rx.lock() else {
        return;
    };

    loop {
        let request = match rx.try_recv() {
            Ok(request) => request,
            Err(mpsc::TryRecvError::Empty) => break,
            Err(mpsc::TryRecvError::Disconnected) => break,
        };

        match request {
            GameRequest::GetEconomy { reply } => {
                let response = match read_economy(scene) {
                    Ok(values) => response_ok_economy(values),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::SetEconomy { values, reply } => {
                let response = match write_economy(scene, values) {
                    Ok(values) => response_ok_economy(values),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::GetPlayers { reply } => {
                let response = match read_players(scene) {
                    Ok(players) => response_ok_players(&players),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::GetStaffs { reply } => {
                let response = match read_staffs(scene) {
                    Ok(staffs) => response_ok_staffs(&staffs),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::GetStaff { staff_id, reply } => {
                let response = match read_staff(scene, staff_id) {
                    Ok(staff) => response_ok_staff(staff),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::GetStaffContractProbe { staff_id, reply } => {
                let response = match read_staff_contract_probe(scene, staff_id) {
                    Ok(raw) => response_ok_staff_contract_probe(&raw),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::SetStaffName {
                staff_id,
                values,
                reply,
            } => {
                let response = match write_staff_name(scene, staff_id, values) {
                    Ok(()) => "OK|STAFF_NAME".to_string(),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::SetStaffStats {
                staff_id,
                values,
                reply,
            } => {
                let response = match write_staff_stats(scene, staff_id, values) {
                    Ok(()) => "OK|STAFF_STATS".to_string(),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::SetStaffSalary {
                staff_id,
                values,
                reply,
            } => {
                let response = match write_staff_salary(scene, staff_id, values) {
                    Ok(()) => "OK|STAFF_SALARY".to_string(),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::SetStaffContractEnd {
                staff_id,
                values,
                reply,
            } => {
                let response = match write_staff_contract_end(scene, staff_id, values) {
                    Ok(staff) => response_ok_staff(staff),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::SetStaffContract {
                staff_id,
                values,
                reply,
            } => {
                let response = match write_staff_contract(scene, staff_id, values) {
                    Ok(staff) => response_ok_staff(staff),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::SetStaffCommunication {
                staff_id,
                values,
                reply,
            } => {
                let response = match write_staff_communication(scene, staff_id, values) {
                    Ok(()) => "OK|STAFF_COMMUNICATION".to_string(),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::GetTeams { reply } => {
                let response = match read_teams(scene) {
                    Ok(teams) => response_ok_teams(&teams),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::GetTeamProbe { team_id, reply } => {
                let response = match read_team_probe(scene, team_id) {
                    Ok(raw) => response_ok_team_probe(&raw),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::GetTeamManagement { team_id, reply } => {
                let response = match read_team_management(scene, team_id) {
                    Ok(snapshot) => response_ok_team_management(snapshot),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::SetTeamMerchandise {
                team_id,
                values,
                reply,
            } => {
                let response = match set_team_merchandise_client(scene, team_id, &values) {
                    Ok(()) => "OK|TEAM_MERCHANDISE".to_string(),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::SetTeamFans {
                team_id,
                values,
                reply,
            } => {
                let response = match set_team_fans_client(scene, team_id, &values) {
                    Ok(()) => "OK|TEAM_FANS".to_string(),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::GetTeamFanMomentumProbe { team_id, reply } => {
                let response = match read_team_fan_momentum_probe(scene, team_id) {
                    Ok(raw) => response_ok_team_fan_momentum_probe(&raw),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::GetTeamStrategyOptions { reply } => {
                let response = match read_team_strategy_options(scene) {
                    Ok(raw) => response_ok_team_strategy_options(&raw),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::GetTeamReplayStrategies {
                team_id,
                replay_ids,
                reply,
            } => {
                let response = match read_team_replay_strategies(scene, team_id, &replay_ids) {
                    Ok(raw) => response_ok_team_replay_strategies(&raw),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::ProbeSwapTeamStrategy { team_id, reply } => {
                let response = match probe_swap_team_strategy_client(scene, team_id) {
                    Ok(snapshot) => response_ok_team_management(snapshot),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::SetTeamStrategy {
                team_id,
                raw_strategy,
                reply,
            } => {
                let response = match set_team_strategy_client(scene, team_id, &raw_strategy) {
                    Ok(snapshot) => response_ok_team_management(snapshot),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::GetContractDefaults {
                entity,
                team_id,
                reply,
            } => {
                let response = match read_contract_defaults(scene, entity, team_id) {
                    Ok(values) => response_ok_contract_defaults(&values),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::MoveStaffToTeam {
                staff_id,
                team_id,
                role,
                reply,
            } => {
                let response = match move_staff_to_team_client(scene, staff_id, team_id, role) {
                    Ok(()) => "OK|MOVE_STAFF".to_string(),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::SetStaffFreeAgent { staff_id, reply } => {
                let response = match set_staff_free_agent_client(scene, staff_id) {
                    Ok(()) => "OK|STAFF_FREE_AGENT".to_string(),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::MovePlayerToTeam {
                athlete_id,
                team_id,
                reply,
            } => {
                let response = match move_player_to_team_client(scene, athlete_id, team_id) {
                    Ok(()) => "OK|MOVE_PLAYER".to_string(),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::SetPlayerFreeAgent { athlete_id, reply } => {
                let response = match set_player_free_agent_client(scene, athlete_id) {
                    Ok(()) => "OK|PLAYER_FREE_AGENT".to_string(),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::GetPlayer { athlete_id, reply } => {
                let response = match read_player(scene, athlete_id) {
                    Ok(player) => response_ok_player(player),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::GetPlayerContractProbe { athlete_id, reply } => {
                let response = match read_player_contract_probe(scene, athlete_id) {
                    Ok(raw) => response_ok_player_contract_probe(&raw),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::SetPlayerName {
                athlete_id,
                values,
                reply,
            } => {
                let response = match write_player_name(scene, athlete_id, values) {
                    Ok(()) => "OK|PLAYER_NAME".to_string(),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::SetPlayerCondition {
                athlete_id,
                values,
                reply,
            } => {
                let response = match write_player_condition(scene, athlete_id, values) {
                    Ok(snapshot) => response_ok_player_condition(snapshot),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::GetChampionMasteryProbe { athlete_id, reply } => {
                let response = match read_champion_mastery_probe(scene, athlete_id) {
                    Ok((mastery_state, available_champions)) => {
                        response_ok_champion_mastery_probe(
                            &mastery_state,
                            &available_champions,
                        )
                    }
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::GetGlobalLeagues { reply } => {
                let started = Instant::now();
                let (response, records_returned) = match read_global_leagues() {
                    Ok((capture_index, records)) => {
                        let records_returned = records.len();
                        (response_ok_global_leagues(capture_index, &records), records_returned)
                    }
                    Err(reason) => (format!("ERR|{reason}"), 0),
                };
                record_global_history_response_metric(
                    "GET_LEAGUES",
                    None,
                    records_returned,
                    response.len(),
                    started,
                );
                let _ = reply.send(response);
            }
            GameRequest::GetGlobalLeagueCompetition { league_id, reply } => {
                let started = Instant::now();
                let (response, records_returned) = match read_global_league_competition(league_id) {
                    Ok((capture_index, json)) => (
                        response_ok_global_league_competition(capture_index, league_id, &json),
                        1,
                    ),
                    Err(reason) => (format!("ERR|{reason}"), 0),
                };
                record_global_history_response_metric(
                    "GET_LEAGUE_COMPETITION",
                    Some(league_id),
                    records_returned,
                    response.len(),
                    started,
                );
                let _ = reply.send(response);
            }
            GameRequest::GetGlobalTeamSchedule { team_id, reply } => {
                let started = Instant::now();
                let (response, records_returned) = match (
                    current_player_team_id(scene),
                    read_global_team_matches(team_id, false),
                ) {
                    (Ok(player_team_id), Ok((capture_index, records))) => {
                        let records_returned = records.len();
                        (
                            response_ok_global_team_records(
                                "GLOBAL_TEAM_SCHEDULE",
                                capture_index,
                                player_team_id,
                                team_id,
                                &records,
                            ),
                            records_returned,
                        )
                    }
                    (Err(reason), _) | (_, Err(reason)) => (format!("ERR|{reason}"), 0),
                };
                record_global_history_response_metric(
                    "GET_TEAM_SCHEDULE",
                    Some(team_id),
                    records_returned,
                    response.len(),
                    started,
                );
                let _ = reply.send(response);
            }
            GameRequest::GetGlobalTeamHistory { team_id, reply } => {
                let started = Instant::now();
                let (response, records_returned) = match (
                    current_player_team_id(scene),
                    read_global_team_matches(team_id, true),
                ) {
                    (Ok(player_team_id), Ok((capture_index, records))) => {
                        let records_returned = records.len();
                        (
                            response_ok_global_team_records(
                                "GLOBAL_TEAM_HISTORY",
                                capture_index,
                                player_team_id,
                                team_id,
                                &records,
                            ),
                            records_returned,
                        )
                    }
                    (Err(reason), _) | (_, Err(reason)) => (format!("ERR|{reason}"), 0),
                };
                record_global_history_response_metric(
                    "GET_TEAM_HISTORY",
                    Some(team_id),
                    records_returned,
                    response.len(),
                    started,
                );
                let _ = reply.send(response);
            }
            GameRequest::SetChampionMastery {
                athlete_id,
                values,
                reply,
            } => {
                let response = match write_champion_mastery(scene, athlete_id, values) {
                    Ok(count) => format!("OK|CHAMPION_MASTERY|{count}"),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::SetPlayerStats {
                athlete_id,
                values,
                reply,
            } => {
                let response = match write_player_stats(scene, athlete_id, values) {
                    Ok(player) => response_ok_player(player),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::SetPlayerPositions {
                athlete_id,
                values,
                reply,
            } => {
                let response = match write_player_positions(scene, athlete_id, values) {
                    Ok(player) => response_ok_player(player),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::SetPlayerPotential {
                athlete_id,
                values,
                reply,
            } => {
                let response = match write_player_potential(scene, athlete_id, values) {
                    Ok(player) => response_ok_player(player),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::SetPlayerSalary {
                athlete_id,
                values,
                reply,
            } => {
                let response = match write_player_salary(scene, athlete_id, values) {
                    Ok(player) => response_ok_player(player),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::SetPlayerContractEnd {
                athlete_id,
                values,
                reply,
            } => {
                let response = match write_player_contract_end(scene, athlete_id, values) {
                    Ok(player) => response_ok_player(player),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::SetPlayerContract {
                athlete_id,
                values,
                reply,
            } => {
                let response = match write_player_contract(scene, athlete_id, values) {
                    Ok(player) => response_ok_player(player),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::SetPlayerCommunication {
                athlete_id,
                values,
                reply,
            } => {
                let response = match write_player_communication(scene, athlete_id, values) {
                    Ok(player) => response_ok_player(player),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::SetPlayerCommunicationMax { athlete_id, reply } => {
                let response = match write_player_communication_max(scene, athlete_id) {
                    Ok(player) => response_ok_player(player),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::SetTransferAlwaysSuccess { enabled, reply } => {
                let response = match set_transfer_always_success(scene, enabled) {
                    Ok(()) => response_ok_recruitment_settings(),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
            GameRequest::SetRecruitmentInstantRetry { enabled, reply } => {
                let response = match set_recruitment_instant_retry(scene, enabled) {
                    Ok(()) => response_ok_recruitment_settings(),
                    Err(reason) => format!("ERR|{reason}"),
                };
                let _ = reply.send(response);
            }
        }
    }
}

fn parse_stat_value<T>(value: &str) -> Result<T, &'static str>
where
    T: std::str::FromStr,
{
    let numeric = value.parse::<f64>().map_err(|_| "INVALID_STAT")?;
    if !numeric.is_finite()
        || numeric.fract().abs() > f64::EPSILON
        || !(1.0..=100.0).contains(&numeric)
    {
        return Err("STAT_OUT_OF_RANGE");
    }

    value.parse::<T>().map_err(|_| "INVALID_STAT")
}

fn parse_f64(value: Option<&str>) -> Result<f64, &'static str> {
    let Some(value) = value else {
        return Err("MISSING_VALUE");
    };

    let value = value.parse::<f64>().map_err(|_| "INVALID_NUMBER")?;
    if !value.is_finite() {
        return Err("INVALID_NUMBER");
    }

    Ok(value)
}

fn parse_usize(value: Option<&str>) -> Result<usize, &'static str> {
    value
        .ok_or("MISSING_VALUE")?
        .parse::<usize>()
        .map_err(|_| "INVALID_ID")
}

fn parse_u16(value: Option<&str>, error: &'static str) -> Result<u16, &'static str> {
    value
        .ok_or("MISSING_VALUE")?
        .parse::<u16>()
        .map_err(|_| error)
}

fn send_game_request<F>(request_tx: &Sender<GameRequest>, build: F) -> String
where
    F: FnOnce(Sender<String>) -> GameRequest,
{
    let (reply_tx, reply_rx) = mpsc::channel();
    if request_tx.send(build(reply_tx)).is_err() {
        return "ERR|GAME_CHANNEL_UNAVAILABLE".to_string();
    }

    reply_rx
        .recv_timeout(Duration::from_secs(2))
        .unwrap_or_else(|_| "ERR|GAME_RESPONSE_TIMEOUT".to_string())
}

fn handle_client(mut stream: TcpStream, request_tx: &Sender<GameRequest>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(3)));

    let read_stream = match stream.try_clone() {
        Ok(stream) => stream,
        Err(_) => return,
    };

    let mut reader = BufReader::new(read_stream);
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() {
        return;
    }

    let line = line.trim();
    let mut parts = line.split('|');
    let command = parts.next().unwrap_or_default();

    let response = match command {
        "PING" => format!(
            "OK|PONG|{BRIDGE_VERSION}|{BRIDGE_PROTOCOL_VERSION}|{TFM2_TARGET_VERSION}"
        ),
        "GET_ECONOMY" => send_game_request(request_tx, |reply| GameRequest::GetEconomy { reply }),
        "SET_ECONOMY" => {
            let parsed: Result<EconomyValues, &'static str> = (|| {
                Ok(EconomyValues {
                    money: parse_f64(parts.next())?,
                    transfer_budget: parse_f64(parts.next())?,
                    salary_budget: parse_f64(parts.next())?,
                })
            })();

            match parsed {
                Ok(values) => send_game_request(request_tx, |reply| GameRequest::SetEconomy {
                    values,
                    reply,
                }),
                Err(reason) => format!("ERR|{reason}"),
            }
        }
        "GET_PLAYERS" => send_game_request(request_tx, |reply| GameRequest::GetPlayers { reply }),
        "GET_STAFFS" => send_game_request(request_tx, |reply| GameRequest::GetStaffs { reply }),
        "GET_STAFF" => {
            let staff_id = match parse_usize(parts.next()) {
                Ok(value) => value,
                Err(reason) => {
                    let _ = writeln!(stream, "ERR|{reason}");
                    let _ = stream.flush();
                    return;
                }
            };
            send_game_request(request_tx, |reply| GameRequest::GetStaff { staff_id, reply })
        }
        "GET_STAFF_CONTRACT_PROBE" => match parse_usize(parts.next()) {
            Ok(staff_id) => send_game_request(request_tx, |reply| {
                GameRequest::GetStaffContractProbe { staff_id, reply }
            }),
            Err(reason) => format!("ERR|{reason}"),
        },
        "SET_STAFF_NAME" => {
            let parsed: Result<(usize, EntityNameValue), &'static str> = (|| {
                let staff_id = parse_usize(parts.next())?;
                let encoded_name = parts.next().ok_or("MISSING_VALUE")?;
                let name = validate_entity_name(&hex_decode(encoded_name)?)?;
                Ok((staff_id, EntityNameValue { name }))
            })();
            match parsed {
                Ok((staff_id, values)) => send_game_request(request_tx, |reply| {
                    GameRequest::SetStaffName {
                        staff_id,
                        values,
                        reply,
                    }
                }),
                Err(reason) => format!("ERR|{reason}"),
            }
        },
        "SET_STAFF_STATS" => {
            let staff_id = match parse_usize(parts.next()) {
                Ok(value) => value,
                Err(reason) => {
                    let _ = writeln!(stream, "ERR|{reason}");
                    let _ = stream.flush();
                    return;
                }
            };

            let values = StaffStatValues {
                banpick: parts.next().unwrap_or_default().to_string(),
                strategy: parts.next().unwrap_or_default().to_string(),
                negotiation: parts.next().unwrap_or_default().to_string(),
                judge_ability: parts.next().unwrap_or_default().to_string(),
                judge_potential: parts.next().unwrap_or_default().to_string(),
                feedback: parts.next().unwrap_or_default().to_string(),
                power_analysis: parts.next().unwrap_or_default().to_string(),
                control_coaching: parts.next().unwrap_or_default().to_string(),
                judgment_coaching: parts.next().unwrap_or_default().to_string(),
                mental_coaching: parts.next().unwrap_or_default().to_string(),
            };

            match validate_staff_stats(&values) {
                Ok(()) => send_game_request(request_tx, |reply| GameRequest::SetStaffStats {
                    staff_id,
                    values,
                    reply,
                }),
                Err(reason) => format!("ERR|{reason}"),
            }
        }
        "SET_STAFF_SALARY" => {
            let staff_id = match parse_usize(parts.next()) {
                Ok(value) => value,
                Err(reason) => {
                    let _ = writeln!(stream, "ERR|{reason}");
                    let _ = stream.flush();
                    return;
                }
            };
            let values = StaffSalaryValue {
                annual_salary: parts.next().unwrap_or_default().to_string(),
            };
            match validate_annual_salary(&values.annual_salary) {
                Ok(_) => send_game_request(request_tx, |reply| GameRequest::SetStaffSalary {
                    staff_id,
                    values,
                    reply,
                }),
                Err(reason) => format!("ERR|{reason}"),
            }
        }
        "SET_STAFF_CONTRACT_END" => {
            let staff_id = match parse_usize(parts.next()) {
                Ok(value) => value,
                Err(reason) => {
                    let _ = writeln!(stream, "ERR|{reason}");
                    let _ = stream.flush();
                    return;
                }
            };
            let end_date = parts.next().unwrap_or_default().to_string();
            match validate_contract_end_date_text(&end_date) {
                Ok(_) => send_game_request(request_tx, |reply| GameRequest::SetStaffContractEnd {
                    staff_id,
                    values: StaffContractEndValue { end_date },
                    reply,
                }),
                Err(reason) => format!("ERR|{reason}"),
            }
        }
        "SET_STAFF_CONTRACT" => {
            let parsed: Result<(usize, StaffContractValue), &'static str> = (|| {
                let staff_id = parse_usize(parts.next())?;
                let values = StaffContractValue {
                    team_id: parse_usize(parts.next())?,
                    start_date: parts.next().ok_or("MISSING_VALUE")?.to_string(),
                    end_date: parts.next().ok_or("MISSING_VALUE")?.to_string(),
                    annual_salary: parts.next().ok_or("MISSING_VALUE")?.to_string(),
                };
                validate_contract_dates(&values.start_date, &values.end_date)?;
                validate_annual_salary(&values.annual_salary)?;
                Ok((staff_id, values))
            })();
            match parsed {
                Ok((staff_id, values)) => send_game_request(request_tx, |reply| {
                    GameRequest::SetStaffContract { staff_id, values, reply }
                }),
                Err(reason) => format!("ERR|{reason}"),
            }
        }
        "SET_STAFF_COMMUNICATION" => {
            let staff_id = match parse_usize(parts.next()) {
                Ok(value) => value,
                Err(reason) => {
                    let _ = writeln!(stream, "ERR|{reason}");
                    let _ = stream.flush();
                    return;
                }
            };
            let values = match parse_staff_communication_entries(parts.next().unwrap_or_default()) {
                Ok(values) => values,
                Err(reason) => {
                    let _ = writeln!(stream, "ERR|{reason}");
                    let _ = stream.flush();
                    return;
                }
            };
            send_game_request(request_tx, |reply| GameRequest::SetStaffCommunication {
                staff_id,
                values,
                reply,
            })
        }
        "GET_LEAGUES" => send_game_request(request_tx, |reply| GameRequest::GetGlobalLeagues { reply }),
        "GET_LEAGUE_COMPETITION" => match parse_usize(parts.next()) {
            Ok(league_id) => send_game_request(request_tx, |reply| {
                GameRequest::GetGlobalLeagueCompetition { league_id, reply }
            }),
            Err(reason) => format!("ERR|{reason}"),
        },
        "GET_TEAM_SCHEDULE" => match parse_usize(parts.next()) {
            Ok(team_id) => send_game_request(request_tx, |reply| {
                GameRequest::GetGlobalTeamSchedule { team_id, reply }
            }),
            Err(reason) => format!("ERR|{reason}"),
        },
        "GET_TEAM_HISTORY" => match parse_usize(parts.next()) {
            Ok(team_id) => send_game_request(request_tx, |reply| {
                GameRequest::GetGlobalTeamHistory { team_id, reply }
            }),
            Err(reason) => format!("ERR|{reason}"),
        },
        "GET_TEAMS" => send_game_request(request_tx, |reply| GameRequest::GetTeams { reply }),
        "GET_TEAM_PROBE" => match parse_usize(parts.next()) {
            Ok(team_id) => send_game_request(request_tx, |reply| {
                GameRequest::GetTeamProbe { team_id, reply }
            }),
            Err(reason) => format!("ERR|{reason}"),
        },
        "GET_TEAM_MANAGEMENT" => match parse_usize(parts.next()) {
            Ok(team_id) => send_game_request(request_tx, |reply| {
                GameRequest::GetTeamManagement { team_id, reply }
            }),
            Err(reason) => format!("ERR|{reason}"),
        },
        "SET_TEAM_MERCHANDISE" => {
            let parsed: Result<(usize, TeamMerchandiseWriteValue), &'static str> = (|| {
                let team_id = parse_usize(parts.next())?;
                let values = TeamMerchandiseWriteValue {
                    product_type: hex_decode(parts.next().ok_or("MISSING_VALUE")?)?,
                    athlete_id: parse_usize(parts.next())?,
                    stock: parts.next().ok_or("MISSING_VALUE")?.to_string(),
                    sell_price: parts.next().ok_or("MISSING_VALUE")?.to_string(),
                };
                validate_team_merchandise_write(&values)?;
                Ok((team_id, values))
            })();
            match parsed {
                Ok((team_id, values)) => send_game_request(request_tx, |reply| {
                    GameRequest::SetTeamMerchandise {
                        team_id,
                        values,
                        reply,
                    }
                }),
                Err(reason) => format!("ERR|{reason}"),
            }
        }
        "SET_TEAM_FANS" => {
            let parsed: Result<(usize, TeamFansWriteValue), &'static str> = (|| {
                let team_id = parse_usize(parts.next())?;
                let values = TeamFansWriteValue {
                    popularity: parts.next().ok_or("MISSING_VALUE")?.to_string(),
                    fan_count: parts.next().ok_or("MISSING_VALUE")?.to_string(),
                    fan_expectation: hex_decode(parts.next().ok_or("MISSING_VALUE")?)?,
                    fan_satisfaction: hex_decode(parts.next().ok_or("MISSING_VALUE")?)?,
                    fan_momentum: parts.next().ok_or("MISSING_VALUE")?.to_string(),
                };
                validate_team_fans_write(&values)?;
                Ok((team_id, values))
            })();
            match parsed {
                Ok((team_id, values)) => send_game_request(request_tx, |reply| {
                    GameRequest::SetTeamFans {
                        team_id,
                        values,
                        reply,
                    }
                }),
                Err(reason) => format!("ERR|{reason}"),
            }
        }
        "GET_TEAM_FAN_MOMENTUM_PROBE" => match parse_usize(parts.next()) {
            Ok(team_id) => send_game_request(request_tx, |reply| {
                GameRequest::GetTeamFanMomentumProbe { team_id, reply }
            }),
            Err(reason) => format!("ERR|{reason}"),
        },
        "GET_TEAM_STRATEGY_OPTIONS" => send_game_request(request_tx, |reply| {
            GameRequest::GetTeamStrategyOptions { reply }
        }),
        "GET_TEAM_REPLAY_STRATEGIES" => {
            let parsed: Result<(usize, Vec<usize>), &'static str> = (|| {
                let team_id = parse_usize(parts.next())?;
                let replay_ids = parse_replay_id_list(parts.next().ok_or("MISSING_VALUE")?)?;
                Ok((team_id, replay_ids))
            })();
            match parsed {
                Ok((team_id, replay_ids)) => send_game_request(request_tx, |reply| {
                    GameRequest::GetTeamReplayStrategies {
                        team_id,
                        replay_ids,
                        reply,
                    }
                }),
                Err(reason) => format!("ERR|{reason}"),
            }
        }
        "PROBE_SWAP_TEAM_STRATEGY" => match parse_usize(parts.next()) {
            Ok(team_id) => send_game_request(request_tx, |reply| {
                GameRequest::ProbeSwapTeamStrategy { team_id, reply }
            }),
            Err(reason) => format!("ERR|{reason}"),
        },
        "SET_TEAM_STRATEGY" => {
            let parsed: Result<(usize, String), &'static str> = (|| {
                let team_id = parse_usize(parts.next())?;
                let raw_strategy = hex_decode(parts.next().ok_or("MISSING_VALUE")?)?;
                parse_team_strategy_values(&raw_strategy)?;
                Ok((team_id, raw_strategy))
            })();
            match parsed {
                Ok((team_id, raw_strategy)) => send_game_request(request_tx, |reply| {
                    GameRequest::SetTeamStrategy {
                        team_id,
                        raw_strategy,
                        reply,
                    }
                }),
                Err(reason) => format!("ERR|{reason}"),
            }
        }
        "GET_CONTRACT_DEFAULTS" => {
            let parsed: Result<(ContractDefaultsEntity, usize), &'static str> = (|| {
                let entity = match parts.next().ok_or("MISSING_VALUE")? {
                    "PLAYER" => ContractDefaultsEntity::Player,
                    "STAFF" => ContractDefaultsEntity::Staff,
                    _ => return Err("INVALID_CONTRACT_ENTITY"),
                };
                let team_id = parse_usize(parts.next())?;
                Ok((entity, team_id))
            })();
            match parsed {
                Ok((entity, team_id)) => send_game_request(request_tx, |reply| {
                    GameRequest::GetContractDefaults {
                        entity,
                        team_id,
                        reply,
                    }
                }),
                Err(reason) => format!("ERR|{reason}"),
            }
        }
        "MOVE_STAFF_TO_TEAM" => {
            let staff_id = match parse_usize(parts.next()) {
                Ok(value) => value,
                Err(reason) => {
                    let _ = writeln!(stream, "ERR|{reason}");
                    let _ = stream.flush();
                    return;
                }
            };
            let team_id = match parse_usize(parts.next()) {
                Ok(value) => value,
                Err(reason) => {
                    let _ = writeln!(stream, "ERR|{reason}");
                    let _ = stream.flush();
                    return;
                }
            };
            let role = parts
                .next()
                .filter(|value| !value.trim().is_empty())
                .map(str::to_string);
            match validate_optional_staff_role(role.as_deref()) {
                Ok(()) => send_game_request(request_tx, |reply| GameRequest::MoveStaffToTeam {
                    staff_id,
                    team_id,
                    role,
                    reply,
                }),
                Err(reason) => format!("ERR|{reason}"),
            }
        }
        "SET_STAFF_FREE_AGENT" => match parse_usize(parts.next()) {
            Ok(staff_id) => send_game_request(request_tx, |reply| {
                GameRequest::SetStaffFreeAgent { staff_id, reply }
            }),
            Err(reason) => format!("ERR|{reason}"),
        },
        "MOVE_PLAYER_TO_TEAM" => {
            let athlete_id = match parse_usize(parts.next()) {
                Ok(value) => value,
                Err(reason) => {
                    let _ = writeln!(stream, "ERR|{reason}");
                    let _ = stream.flush();
                    return;
                }
            };
            let team_id = match parse_usize(parts.next()) {
                Ok(value) => value,
                Err(reason) => {
                    let _ = writeln!(stream, "ERR|{reason}");
                    let _ = stream.flush();
                    return;
                }
            };
            send_game_request(request_tx, |reply| GameRequest::MovePlayerToTeam {
                athlete_id,
                team_id,
                reply,
            })
        }
        "SET_PLAYER_FREE_AGENT" => match parse_usize(parts.next()) {
            Ok(athlete_id) => send_game_request(request_tx, |reply| {
                GameRequest::SetPlayerFreeAgent { athlete_id, reply }
            }),
            Err(reason) => format!("ERR|{reason}"),
        },
        "GET_PLAYER" => match parse_usize(parts.next()) {
            Ok(athlete_id) => send_game_request(request_tx, |reply| GameRequest::GetPlayer {
                athlete_id,
                reply,
            }),
            Err(reason) => format!("ERR|{reason}"),
        },
        "GET_PLAYER_CONTRACT_PROBE" => match parse_usize(parts.next()) {
            Ok(athlete_id) => send_game_request(request_tx, |reply| {
                GameRequest::GetPlayerContractProbe { athlete_id, reply }
            }),
            Err(reason) => format!("ERR|{reason}"),
        },
        "SET_PLAYER_NAME" => {
            let parsed: Result<(usize, EntityNameValue), &'static str> = (|| {
                let athlete_id = parse_usize(parts.next())?;
                let encoded_name = parts.next().ok_or("MISSING_VALUE")?;
                let name = validate_entity_name(&hex_decode(encoded_name)?)?;
                Ok((athlete_id, EntityNameValue { name }))
            })();
            match parsed {
                Ok((athlete_id, values)) => send_game_request(request_tx, |reply| {
                    GameRequest::SetPlayerName {
                        athlete_id,
                        values,
                        reply,
                    }
                }),
                Err(reason) => format!("ERR|{reason}"),
            }
        },
        "SET_PLAYER_CONDITION" => {
            let parsed: Result<(usize, PlayerConditionValue), &'static str> = (|| {
                let athlete_id = parse_usize(parts.next())?;
                let values = PlayerConditionValue {
                    stamina: parts.next().ok_or("MISSING_VALUE")?.to_string(),
                    condition: parts.next().ok_or("MISSING_VALUE")?.to_string(),
                };
                validate_player_condition(&values)?;
                Ok((athlete_id, values))
            })();
            match parsed {
                Ok((athlete_id, values)) => send_game_request(request_tx, |reply| {
                    GameRequest::SetPlayerCondition {
                        athlete_id,
                        values,
                        reply,
                    }
                }),
                Err(reason) => format!("ERR|{reason}"),
            }
        }
        "GET_CHAMPION_MASTERY_PROBE" => match parse_usize(parts.next()) {
            Ok(athlete_id) => send_game_request(request_tx, |reply| {
                GameRequest::GetChampionMasteryProbe { athlete_id, reply }
            }),
            Err(reason) => format!("ERR|{reason}"),
        },
        "SET_CHAMPION_MASTERY" => {
            let athlete_id = match parse_usize(parts.next()) {
                Ok(value) => value,
                Err(reason) => {
                    let _ = writeln!(stream, "ERR|{reason}");
                    let _ = stream.flush();
                    return;
                }
            };

            let entries = parts.next().unwrap_or_default();
            let mut values = Vec::new();

            for entry in entries.split(';').filter(|entry| !entry.trim().is_empty()) {
                let Some((champion_id, mastery)) = entry.split_once(':') else {
                    let _ = writeln!(stream, "ERR|INVALID_MASTERY_ENTRY");
                    let _ = stream.flush();
                    return;
                };

                let mastery = match mastery.parse::<u16>() {
                    Ok(value) => value,
                    Err(_) => {
                        let _ = writeln!(stream, "ERR|INVALID_MASTERY");
                        let _ = stream.flush();
                        return;
                    }
                };

                values.push(ChampionMasteryValue {
                    champion_id: champion_id.to_string(),
                    mastery,
                });
            }

            match validate_champion_mastery_values(&values) {
                Ok(()) => send_game_request(request_tx, |reply| {
                    GameRequest::SetChampionMastery {
                        athlete_id,
                        values,
                        reply,
                    }
                }),
                Err(reason) => format!("ERR|{reason}"),
            }
        }
        "SET_PLAYER_STATS" => {
            let athlete_id = match parse_usize(parts.next()) {
                Ok(value) => value,
                Err(reason) => {
                    let _ = writeln!(stream, "ERR|{reason}");
                    let _ = stream.flush();
                    return;
                }
            };

            let values = PlayerStatValues {
                last_hit: parts.next().unwrap_or_default().to_string(),
                skill_avoid: parts.next().unwrap_or_default().to_string(),
                skill_hit: parts.next().unwrap_or_default().to_string(),
                control_speed: parts.next().unwrap_or_default().to_string(),
                positioning: parts.next().unwrap_or_default().to_string(),
                judgement: parts.next().unwrap_or_default().to_string(),
                mental: parts.next().unwrap_or_default().to_string(),
                concentration: parts.next().unwrap_or_default().to_string(),
                order: parts.next().unwrap_or_default().to_string(),
                roaming: parts.next().unwrap_or_default().to_string(),
                aggressive: parts.next().unwrap_or_default().to_string(),
                ego: parts.next().unwrap_or_default().to_string(),
            };

            send_game_request(request_tx, |reply| GameRequest::SetPlayerStats {
                athlete_id,
                values,
                reply,
            })
        }
        "SET_PLAYER_POSITIONS" => {
            let athlete_id = match parse_usize(parts.next()) {
                Ok(value) => value,
                Err(reason) => {
                    let _ = writeln!(stream, "ERR|{reason}");
                    let _ = stream.flush();
                    return;
                }
            };
            let parsed: Result<PlayerPositionValues, &'static str> = (|| {
                Ok(PlayerPositionValues {
                    top: parse_u16(parts.next(), "INVALID_POSITION")?,
                    jungle: parse_u16(parts.next(), "INVALID_POSITION")?,
                    mid: parse_u16(parts.next(), "INVALID_POSITION")?,
                    bottom: parse_u16(parts.next(), "INVALID_POSITION")?,
                    support: parse_u16(parts.next(), "INVALID_POSITION")?,
                })
            })();

            match parsed {
                Ok(values) => send_game_request(request_tx, |reply| GameRequest::SetPlayerPositions {
                    athlete_id,
                    values,
                    reply,
                }),
                Err(reason) => format!("ERR|{reason}"),
            }
        }
        "SET_PLAYER_POTENTIAL" => {
            let athlete_id = match parse_usize(parts.next()) {
                Ok(value) => value,
                Err(reason) => {
                    let _ = writeln!(stream, "ERR|{reason}");
                    let _ = stream.flush();
                    return;
                }
            };
            let potential = match parse_u16(parts.next(), "INVALID_POTENTIAL") {
                Ok(value) => value,
                Err(reason) => {
                    let _ = writeln!(stream, "ERR|{reason}");
                    let _ = stream.flush();
                    return;
                }
            };
            send_game_request(request_tx, |reply| GameRequest::SetPlayerPotential {
                athlete_id,
                values: PlayerPotentialValue { potential },
                reply,
            })
        }
        "SET_PLAYER_SALARY" => {
            let athlete_id = match parse_usize(parts.next()) {
                Ok(value) => value,
                Err(reason) => {
                    let _ = writeln!(stream, "ERR|{reason}");
                    let _ = stream.flush();
                    return;
                }
            };
            let annual_salary = parts.next().unwrap_or_default().to_string();
            match validate_annual_salary(&annual_salary) {
                Ok(_) => send_game_request(request_tx, |reply| GameRequest::SetPlayerSalary {
                    athlete_id,
                    values: PlayerSalaryValue { annual_salary },
                    reply,
                }),
                Err(reason) => format!("ERR|{reason}"),
            }
        }
        "SET_PLAYER_CONTRACT_END" => {
            let athlete_id = match parse_usize(parts.next()) {
                Ok(value) => value,
                Err(reason) => {
                    let _ = writeln!(stream, "ERR|{reason}");
                    let _ = stream.flush();
                    return;
                }
            };
            let end_date = parts.next().unwrap_or_default().to_string();
            match validate_contract_end_date_text(&end_date) {
                Ok(_) => send_game_request(request_tx, |reply| GameRequest::SetPlayerContractEnd {
                    athlete_id,
                    values: PlayerContractEndValue { end_date },
                    reply,
                }),
                Err(reason) => format!("ERR|{reason}"),
            }
        }
        "SET_PLAYER_CONTRACT" => {
            let parsed: Result<(usize, PlayerContractValue), &'static str> = (|| {
                let athlete_id = parse_usize(parts.next())?;
                let values = PlayerContractValue {
                    team_id: parse_usize(parts.next())?,
                    start_date: parts.next().ok_or("MISSING_VALUE")?.to_string(),
                    end_date: parts.next().ok_or("MISSING_VALUE")?.to_string(),
                    annual_salary: parts.next().ok_or("MISSING_VALUE")?.to_string(),
                    transfer_fee: parts.next().ok_or("MISSING_VALUE")?.to_string(),
                    squad_status: parts.next().ok_or("MISSING_VALUE")?.to_string(),
                    pog_enabled: parse_contract_bool(parts.next().ok_or("MISSING_VALUE")?)?,
                    pog_bonus: parts.next().ok_or("MISSING_VALUE")?.to_string(),
                    league_enabled: parse_contract_bool(parts.next().ok_or("MISSING_VALUE")?)?,
                    league_bonus: parts.next().ok_or("MISSING_VALUE")?.to_string(),
                    league_rank: parts.next().ok_or("MISSING_VALUE")?.to_string(),
                    match_enabled: parse_contract_bool(parts.next().ok_or("MISSING_VALUE")?)?,
                    match_bonus: parts.next().ok_or("MISSING_VALUE")?.to_string(),
                    win_enabled: parse_contract_bool(parts.next().ok_or("MISSING_VALUE")?)?,
                    win_bonus: parts.next().ok_or("MISSING_VALUE")?.to_string(),
                };
                validate_contract_dates(&values.start_date, &values.end_date)?;
                validate_annual_salary(&values.annual_salary)?;
                let transfer = values.transfer_fee.parse::<f64>().map_err(|_| "INVALID_TRANSFER_FEE")?;
                if !transfer.is_finite() || transfer < 0.0 {
                    return Err("TRANSFER_FEE_OUT_OF_RANGE");
                }
                validate_player_contract_extras(&values)?;
                Ok((athlete_id, values))
            })();
            match parsed {
                Ok((athlete_id, values)) => send_game_request(request_tx, |reply| {
                    GameRequest::SetPlayerContract { athlete_id, values, reply }
                }),
                Err(reason) => format!("ERR|{reason}"),
            }
        }
        "SET_PLAYER_COMMUNICATION" => {
            let parsed: Result<(usize, PlayerCommunicationValue), &'static str> = (|| {
                let athlete_id = parse_usize(parts.next())?;
                let region_id = parse_usize(parts.next())?;
                let level = parts.next().ok_or("MISSING_VALUE")?.parse::<usize>().map_err(|_| "INVALID_COMMUNICATION")?;
                let values = PlayerCommunicationValue { region_id, level };
                validate_player_communication(values)?;
                Ok((athlete_id, values))
            })();
            match parsed {
                Ok((athlete_id, values)) => send_game_request(request_tx, |reply| {
                    GameRequest::SetPlayerCommunication { athlete_id, values, reply }
                }),
                Err(reason) => format!("ERR|{reason}"),
            }
        }
        "SET_PLAYER_COMMUNICATION_MAX" => match parse_usize(parts.next()) {
            Ok(athlete_id) => send_game_request(request_tx, |reply| {
                GameRequest::SetPlayerCommunicationMax { athlete_id, reply }
            }),
            Err(reason) => format!("ERR|{reason}"),
        },
        "GET_RECRUITMENT_SETTINGS" => response_ok_recruitment_settings(),
        "SET_TRANSFER_ALWAYS_SUCCESS" => {
            let enabled = match parts.next() {
                Some("1") => true,
                Some("0") => false,
                _ => {
                    let _ = writeln!(stream, "ERR|INVALID_BOOLEAN");
                    let _ = stream.flush();
                    return;
                }
            };
            send_game_request(request_tx, |reply| GameRequest::SetTransferAlwaysSuccess {
                enabled,
                reply,
            })
        }
        "SET_RECRUITMENT_INSTANT_RETRY" => {
            let enabled = match parts.next() {
                Some("1") => true,
                Some("0") => false,
                _ => {
                    let _ = writeln!(stream, "ERR|INVALID_BOOLEAN");
                    let _ = stream.flush();
                    return;
                }
            };
            send_game_request(request_tx, |reply| GameRequest::SetRecruitmentInstantRetry {
                enabled,
                reply,
            })
        }
        _ => "ERR|UNKNOWN_COMMAND".to_string(),
    };

    let _ = writeln!(stream, "{response}");
    let _ = stream.flush();
}

fn start_bridge_server() {
    if SERVER_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }

    let (request_tx, request_rx) = mpsc::channel::<GameRequest>();
    if REQUEST_RX.set(Mutex::new(request_rx)).is_err() {
        return;
    }

    thread::spawn(move || {
        let listener = match TcpListener::bind(BRIDGE_ADDR) {
            Ok(listener) => listener,
            Err(_) => return,
        };

        for incoming in listener.incoming() {
            let Ok(stream) = incoming else {
                continue;
            };
            handle_client(stream, &request_tx);
        }
    });
}

struct ModifierBridgeClient;

impl ModExtension for ModifierBridgeClient {
    fn post_update(
        &self,
        scene: &mut Scene,
        _ui: &mut GameUI,
        _assets: &mut Assets,
        _dt: f32,
    ) {
        sync_recruitment_runtime_toggles(scene);
        process_game_requests(scene);
    }
}

struct ModifierBridgeServer;

impl ModServerExtension for ModifierBridgeServer {
    fn on_server_start(&self, ctx: &mut ServerModContext) {
        clear_contract_end_overrides();
        clear_staff_contract_end_overrides();
        clear_active_contract_overrides();

        // Keep the two runtime toggle preferences across save/career loads. The
        // destination team is save-specific, so mark the team IDs stale; the client
        // extension will re-send enabled settings for the newly loaded player team.
        TRANSFER_ALWAYS_SUCCESS_TEAM_ID.store(usize::MAX, Ordering::SeqCst);
        RECRUITMENT_INSTANT_RETRY_TEAM_ID.store(usize::MAX, Ordering::SeqCst);

        clear_global_history_snapshot();
        capture_global_history_snapshot(ctx);
    }

    fn before_management_tick(&self, ctx: &mut ServerModContext) {
        enforce_active_player_contract_overrides(ctx, false);
        enforce_active_staff_contract_overrides(ctx, false);
        enforce_contract_end_overrides(ctx, false);
        enforce_staff_contract_end_overrides(ctx, false);
        force_transfer_request_success(ctx);
        clear_player_recruitment_cooldowns(ctx);
    }

    fn after_management_tick(&self, ctx: &mut ServerModContext) {
        enforce_active_player_contract_overrides(ctx, true);
        enforce_active_staff_contract_overrides(ctx, true);
        enforce_contract_end_overrides(ctx, true);
        enforce_staff_contract_end_overrides(ctx, true);
        force_transfer_request_success(ctx);
        clear_player_recruitment_cooldowns(ctx);
        capture_global_history_snapshot(ctx);
    }

    fn handle_command(
        &self,
        ctx: &mut ServerModContext,
        command: &ModServerCommand,
    ) -> ModServerCommandResult {
        if command.mod_id != MOD_ID {
            return ModServerCommandResult::Pass;
        }

        if command.command == "set_economy" {
            let Ok((team_id, values)) = parse_server_economy(&command.payload) else {
                return ModServerCommandResult::Handled;
            };

            if let Some(team) = ctx.database.teams.get_mut(team_id) {
                apply_economy_to_team(team, values);
            }

            return ModServerCommandResult::Handled;
        }

        if command.command == "set_transfer_always_success" {
            let Ok(payload) = std::str::from_utf8(&command.payload) else {
                return ModServerCommandResult::Handled;
            };
            let mut parts = payload.split('|');
            let Ok(team_id) = parts.next().unwrap_or_default().parse::<usize>() else {
                return ModServerCommandResult::Handled;
            };
            let enabled = matches!(parts.next(), Some("1"));
            TRANSFER_ALWAYS_SUCCESS_TEAM_ID.store(team_id, Ordering::SeqCst);
            TRANSFER_ALWAYS_SUCCESS.store(enabled, Ordering::SeqCst);
            if enabled {
                force_transfer_request_success(ctx);
            }
            return ModServerCommandResult::Handled;
        }

        if command.command == "set_recruitment_instant_retry" {
            let Ok(payload) = std::str::from_utf8(&command.payload) else {
                return ModServerCommandResult::Handled;
            };
            let mut parts = payload.split('|');
            let Ok(team_id) = parts.next().unwrap_or_default().parse::<usize>() else {
                return ModServerCommandResult::Handled;
            };
            let enabled = matches!(parts.next(), Some("1"));
            RECRUITMENT_INSTANT_RETRY_TEAM_ID.store(team_id, Ordering::SeqCst);
            RECRUITMENT_INSTANT_RETRY.store(enabled, Ordering::SeqCst);
            if enabled {
                clear_player_recruitment_cooldowns(ctx);
            }
            return ModServerCommandResult::Handled;
        }

        if command.command == "probe_swap_team_strategy" {
            let Ok(team_id) = parse_team_strategy_probe_payload(&command.payload) else {
                return ModServerCommandResult::Handled;
            };
            if let Some(team) = ctx.database.teams.get_mut(team_id) {
                std::mem::swap(&mut team.strategy, &mut team.last_strategy);
            }
            return ModServerCommandResult::Handled;
        }

        if command.command == "set_team_merchandise" {
            let Ok((team_id, values)) = parse_team_merchandise_write_payload(&command.payload) else {
                return ModServerCommandResult::Handled;
            };
            if let Some(team) = ctx.database.teams.get_mut(team_id) {
                if let Some(product) = team.merchandise_products.iter_mut().find(|product| {
                    product.athlete_id == values.athlete_id
                        && product.product_type.to_string() == values.product_type
                }) {
                    if let (Ok(stock), Ok(sell_price)) = (
                        values.stock.trim().parse(),
                        values.sell_price.trim().parse(),
                    ) {
                        product.stock = stock;
                        product.sell_price = sell_price;
                    }
                }
            }
            return ModServerCommandResult::Handled;
        }

        if command.command == "set_team_fans" {
            let Ok((team_id, values)) = parse_team_fans_write_payload(&command.payload) else {
                return ModServerCommandResult::Handled;
            };

            let resolved_expectation = ctx
                .database
                .teams
                .iter()
                .find_map(|candidate| {
                    if format!("{:?}", candidate.fan_expectation)
                        == values.fan_expectation.trim()
                    {
                        Some(candidate.fan_expectation)
                    } else {
                        None
                    }
                });
            let resolved_satisfaction = ctx
                .database
                .teams
                .iter()
                .find_map(|candidate| {
                    if format!("{:?}", candidate.fan_satisfaction)
                        == values.fan_satisfaction.trim()
                    {
                        Some(candidate.fan_satisfaction)
                    } else {
                        None
                    }
                });
            let roster_player_fans = ctx
                .database
                .athletes
                .iter()
                .filter_map(|athlete| {
                    if matches!(
                        &athlete.contract,
                        Contract::InContract { team_id: athlete_team_id, .. } if *athlete_team_id == team_id
                    ) {
                        athlete.management.fan_count.to_string().parse::<u128>().ok()
                    } else {
                        None
                    }
                })
                .sum::<u128>();
            let displayed_fan_count = values.fan_count.trim().parse::<u128>().ok();
            let base_fan_count = displayed_fan_count.and_then(|displayed| {
                base_team_fan_count_from_displayed(displayed, roster_player_fans).ok()
            });

            if let (
                Some(expectation),
                Some(satisfaction),
                Some(base_fan_count),
                Some(team),
            ) = (
                resolved_expectation,
                resolved_satisfaction,
                base_fan_count,
                ctx.database.teams.get_mut(team_id),
            ) {
                if let (Ok(popularity), Ok(fan_count), Ok(fan_momentum)) = (
                    values.popularity.trim().parse(),
                    base_fan_count.to_string().parse(),
                    values.fan_momentum.trim().parse(),
                ) {
                    team.popularity = popularity;
                    team.fan_count = fan_count;
                    team.fan_expectation = expectation;
                    team.fan_satisfaction = satisfaction;
                    team.fan_momentum = fan_momentum;
                }
            }
            return ModServerCommandResult::Handled;
        }

        if command.command == "set_team_strategy" {
            let Ok((team_id, raw_strategy)) =
                parse_team_strategy_set_payload(&command.payload)
            else {
                return ModServerCommandResult::Handled;
            };
            let Ok(values) = parse_team_strategy_values(&raw_strategy) else {
                return ModServerCommandResult::Handled;
            };

            let new_strategy = (|| {
                let db = &ctx.database;
                let target = db.teams.get(team_id).ok_or("TEAM_NOT_FOUND")?;
                let mut strategy = target.strategy;

                macro_rules! resolve_field {
                    ($field:ident, $key:literal) => {{
                        let desired = values.get($key).ok_or("MISSING_STRATEGY_VALUE")?;
                        db.teams
                            .iter()
                            .find_map(|candidate| {
                                if format!("{:?}", candidate.strategy.$field) == desired.as_str() {
                                    Some(candidate.strategy.$field.clone())
                                } else if format!("{:?}", candidate.last_strategy.$field) == desired.as_str() {
                                    Some(candidate.last_strategy.$field.clone())
                                } else {
                                    candidate
                                        .team_color_strategy
                                        .$field
                                        .as_ref()
                                        .filter(|value| format!("{:?}", value) == desired.as_str())
                                        .cloned()
                                }
                            })
                            .ok_or("UNKNOWN_STRATEGY_VALUE")?
                    }};
                }

                strategy.focused = resolve_field!(focused, "focused");
                strategy.early_jungle = resolve_field!(early_jungle, "early_jungle");
                strategy.early_serpen = resolve_field!(early_serpen, "early_serpen");
                strategy.early_serpen_top =
                    resolve_field!(early_serpen_top, "early_serpen_top");
                strategy.object_buildup = resolve_field!(object_buildup, "object_buildup");
                strategy.object_battle = resolve_field!(object_battle, "object_battle");
                strategy.morgard_use = resolve_field!(morgard_use, "morgard_use");
                strategy.tower_press = resolve_field!(tower_press, "tower_press");
                strategy.morgard_defense =
                    resolve_field!(morgard_defense, "morgard_defense");
                strategy.object_finish = resolve_field!(object_finish, "object_finish");
                strategy.minion_wave = resolve_field!(minion_wave, "minion_wave");
                strategy.game_finish = resolve_field!(game_finish, "game_finish");
                Ok::<_, &'static str>(strategy)
            })();

            if let Ok(strategy) = new_strategy {
                if let Some(team) = ctx.database.teams.get_mut(team_id) {
                    team.strategy = strategy;
                }
            }
            return ModServerCommandResult::Handled;
        }

        if command.command == "move_staff_to_team" {
            let Ok((staff_id, team_id, role)) = parse_move_staff_payload(&command.payload) else {
                return ModServerCommandResult::Handled;
            };
            let _ = move_staff_to_team_server(ctx, staff_id, team_id, role);
            return ModServerCommandResult::Handled;
        }

        if command.command == "set_staff_free_agent" {
            let Ok(staff_id) = parse_player_free_agent_payload(&command.payload) else {
                return ModServerCommandResult::Handled;
            };
            let _ = set_staff_free_agent_server(ctx, staff_id);
            return ModServerCommandResult::Handled;
        }

        if command.command == "move_player_to_team" {
            let Ok((athlete_id, team_id)) = parse_move_player_payload(&command.payload) else {
                return ModServerCommandResult::Handled;
            };
            let _ = move_player_to_team_server(ctx, athlete_id, team_id);
            return ModServerCommandResult::Handled;
        }

        if command.command == "set_player_free_agent" {
            let Ok(athlete_id) = parse_player_free_agent_payload(&command.payload) else {
                return ModServerCommandResult::Handled;
            };
            let _ = set_player_free_agent_server(ctx, athlete_id);
            return ModServerCommandResult::Handled;
        }

        if command.command == "set_player_condition" {
            let Ok((athlete_id, values)) = parse_server_player_condition(&command.payload) else {
                return ModServerCommandResult::Handled;
            };
            if let Some(athlete) = ctx.database.athletes.get_mut(athlete_id) {
                let _ = apply_condition_to_athlete(athlete_id, athlete, &values);
            }
            return ModServerCommandResult::Handled;
        }

        if command.command == "set_champion_mastery" {
            let Ok((athlete_id, values)) =
                parse_server_champion_mastery(&command.payload)
            else {
                return ModServerCommandResult::Handled;
            };

            if let Some(athlete) = ctx.database.athletes.get_mut(athlete_id) {
                let _ = apply_champion_mastery_to_athlete(athlete, &values);
            }

            return ModServerCommandResult::Handled;
        }

        if command.command == "set_staff_name" {
            let Ok((staff_id, values)) = parse_server_entity_name(&command.payload) else {
                return ModServerCommandResult::Handled;
            };
            if let Some(staff) = ctx.database.staffs.get_mut(staff_id) {
                staff.name = values.name;
            }
            return ModServerCommandResult::Handled;
        }

        if command.command == "set_staff_stats" {
            let Ok((staff_id, values)) = parse_server_staff_stats(&command.payload) else {
                return ModServerCommandResult::Handled;
            };

            if let Some(staff) = ctx.database.staffs.get_mut(staff_id) {
                let stat = &mut staff.stat;
                if let Ok(value) = parse_stat_value(&values.banpick) { stat.banpick = value; }
                if let Ok(value) = parse_stat_value(&values.strategy) { stat.strategy = value; }
                if let Ok(value) = parse_stat_value(&values.negotiation) { stat.negotiation = value; }
                if let Ok(value) = parse_stat_value(&values.judge_ability) { stat.judge_ability = value; }
                if let Ok(value) = parse_stat_value(&values.judge_potential) { stat.judge_potential = value; }
                if let Ok(value) = parse_stat_value(&values.feedback) { stat.feedback = value; }
                if let Ok(value) = parse_stat_value(&values.power_analysis) { stat.power_analysis = value; }
                if let Ok(value) = parse_stat_value(&values.control_coaching) { stat.control_coaching = value; }
                if let Ok(value) = parse_stat_value(&values.judgment_coaching) { stat.judgment_coaching = value; }
                if let Ok(value) = parse_stat_value(&values.mental_coaching) { stat.mental_coaching = value; }
            }

            return ModServerCommandResult::Handled;
        }

        if command.command == "set_staff_salary" {
            let Ok((staff_id, values)) = parse_server_staff_salary(&command.payload) else {
                return ModServerCommandResult::Handled;
            };
            if let Some(staff) = ctx.database.staffs.get_mut(staff_id) {
                let _ = apply_staff_salary_contract(&mut staff.contract, &values);
            }
            return ModServerCommandResult::Handled;
        }

        if command.command == "set_staff_contract_end" {
            let Ok((staff_id, values)) = parse_server_staff_contract_end(&command.payload) else {
                return ModServerCommandResult::Handled;
            };
            let applied = if let Some(staff) = ctx.database.staffs.get_mut(staff_id) {
                apply_staff_contract_end(&mut staff.contract, &values).is_ok()
            } else {
                false
            };
            if applied {
                queue_staff_contract_end_override(staff_id, values);
            }
            return ModServerCommandResult::Handled;
        }

        if command.command == "set_staff_contract" {
            let Ok((staff_id, values)) = parse_server_staff_contract(&command.payload) else {
                return ModServerCommandResult::Handled;
            };
            if apply_staff_contract_server(ctx, staff_id, &values).is_ok() {
                queue_active_staff_contract_override(staff_id, values);
            }
            return ModServerCommandResult::Handled;
        }

        if command.command == "set_staff_communication" {
            let Ok((staff_id, values)) = parse_server_staff_communication(&command.payload) else {
                return ModServerCommandResult::Handled;
            };
            if let Some(staff) = ctx.database.staffs.get_mut(staff_id) {
                for (region_id, value) in &values.entries {
                    if let Ok(parsed) = value.to_string().parse() {
                        staff.language.insert(*region_id, parsed);
                    }
                }
            }
            return ModServerCommandResult::Handled;
        }

        if command.command == "set_player_name" {
            let Ok((athlete_id, values)) = parse_server_entity_name(&command.payload) else {
                return ModServerCommandResult::Handled;
            };
            if let Some(athlete) = ctx.database.athletes.get_mut(athlete_id) {
                athlete.name = values.name;
            }
            return ModServerCommandResult::Handled;
        }

        if command.command == "set_player_stats" {
            let Ok((athlete_id, values)) = parse_server_player_stats(&command.payload) else {
                return ModServerCommandResult::Handled;
            };

            if let Some(athlete) = ctx.database.athletes.get_mut(athlete_id) {
                let _ = apply_stats_to_athlete(athlete, &values);
            }

            return ModServerCommandResult::Handled;
        }

        if command.command == "set_player_positions" {
            let Ok((athlete_id, values)) = parse_server_player_positions(&command.payload) else {
                return ModServerCommandResult::Handled;
            };

            if let Some(athlete) = ctx.database.athletes.get_mut(athlete_id) {
                let _ = apply_positions_to_athlete(athlete, values);
            }

            return ModServerCommandResult::Handled;
        }

        if command.command == "set_player_potential" {
            let Ok((athlete_id, values)) = parse_server_player_potential(&command.payload) else {
                return ModServerCommandResult::Handled;
            };

            if let Some(athlete) = ctx.database.athletes.get_mut(athlete_id) {
                let _ = apply_potential_to_athlete(athlete, values);
            }
            for research in ctx.database.team_research_datas.iter_mut() {
                if let Some(report) = research.athlete_report.get_mut(&athlete_id) {
                    report.potential_score = values.potential as _;
                }
            }

            return ModServerCommandResult::Handled;
        }

        if command.command == "set_player_salary" {
            let Ok((athlete_id, values)) = parse_server_player_salary(&command.payload) else {
                return ModServerCommandResult::Handled;
            };
            if let Some(athlete) = ctx.database.athletes.get_mut(athlete_id) {
                let _ = apply_salary_to_athlete(athlete, &values);
            }
            return ModServerCommandResult::Handled;
        }

        if command.command == "set_player_contract_end" {
            let Ok((athlete_id, values)) = parse_server_player_contract_end(&command.payload) else {
                return ModServerCommandResult::Handled;
            };

            let actual = if let Some(athlete) = ctx.database.athletes.get_mut(athlete_id) {
                if apply_contract_end_to_athlete(athlete, &values).is_ok() {
                    Some(contract_end_date_raw(athlete))
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(actual) = actual {
                queue_contract_end_override(athlete_id, values.clone());
                let _ = ctx.emit_event_to_command_sender(
                    command,
                    "contract_end_applied",
                    format!("{}|{}", athlete_id, actual).into_bytes(),
                );
            }

            return ModServerCommandResult::Handled;
        }

        if command.command == "set_player_contract" {
            let Ok((athlete_id, values)) = parse_server_player_contract(&command.payload) else {
                return ModServerCommandResult::Handled;
            };
            if apply_player_contract_server(ctx, athlete_id, &values).is_ok() {
                queue_active_player_contract_override(athlete_id, values);
            }
            return ModServerCommandResult::Handled;
        }

        if command.command == "set_player_communication" {
            let Ok((athlete_id, values)) = parse_server_player_communication(&command.payload) else {
                return ModServerCommandResult::Handled;
            };
            if let Some(athlete) = ctx.database.athletes.get_mut(athlete_id) {
                let _ = apply_communication_to_athlete(athlete, values);
            }
            return ModServerCommandResult::Handled;
        }

        if command.command == "set_player_communication_max" {
            let Ok((athlete_id, region_ids)) = parse_server_player_communication_max(&command.payload) else {
                return ModServerCommandResult::Handled;
            };
            if let Some(athlete) = ctx.database.athletes.get_mut(athlete_id) {
                apply_communication_max_to_athlete(athlete, &region_ids);
            }
            return ModServerCommandResult::Handled;
        }

        ModServerCommandResult::Pass
    }
}

#[cfg(test)]
mod global_history_wire_tests {
    use super::*;

    #[test]
    fn global_history_responses_use_production_bridge_identity() {
        let leagues = response_ok_global_leagues(7, &[r#"{"id":0,"name":"LCK"}"#.to_string()]);
        assert!(leagues.starts_with("OK|GLOBAL_LEAGUES|0.2.59|0.5.5|7|1|"));

        let competition = response_ok_global_league_competition(
            7,
            0,
            r#"{"id":0,"league_type":"Spring"}"#,
        );
        assert!(competition.starts_with(
            "OK|GLOBAL_LEAGUE_COMPETITION|0.2.59|0.5.5|7|0|"
        ));
    }

    #[test]
    fn global_team_response_preserves_capture_player_and_requested_team_ids() {
        let response = response_ok_global_team_records(
            "GLOBAL_TEAM_HISTORY",
            7,
            85,
            89,
            &[r#"{"id":787}"#.to_string()],
        );
        assert!(response.starts_with(
            "OK|GLOBAL_TEAM_HISTORY|0.2.59|0.5.5|7|85|89|1|"
        ));
    }

    #[test]
    fn global_match_json_helpers_decode_normal_team_and_completed_state() {
        let value: serde_json::Value = serde_json::from_str(
            r#"{"team1":{"Normal":85},"team2":{"Normal":89},"running_state":{"End":{"winner":85}}}"#,
        )
        .unwrap();
        assert_eq!(global_json_normal_team_id(&value, "team1"), Some(85));
        assert_eq!(global_json_normal_team_id(&value, "team2"), Some(89));
        assert!(global_json_is_completed_match(&value));
    }

    #[test]
    fn bridge_local_json_encoder_round_trips_nested_values_and_escaping() {
        let value = serde_json::json!({
            "null": null,
            "booleans": [true, false],
            "numbers": [0, 42, -3.5],
            "quote": "A\"B",
            "backslash": "A\\B",
            "controls": "line\nfeed\rcarriage\tend\u{0008}\u{000c}\u{0001}",
            "unicode": "René 홍길동 日本語",
            "array": [true, false, null, 42, {"nested": [1, 2, 3]}],
            "object": {"key\"\\": "value"}
        });

        let encoded = global_json_value_to_string(&value);
        assert_eq!(encoded, global_json_value_to_string(&value));
        assert!(!encoded.chars().any(|ch| ch <= '\u{001f}'));

        let decoded: serde_json::Value = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn global_json_with_id_uses_bridge_local_encoder_and_preserves_record_id() {
        let value = serde_json::json!({"name": "LCK", "active": true});
        let encoded = global_json_with_id(73, value.clone()).unwrap();
        assert_eq!(encoded, global_json_with_id(73, value).unwrap());

        let decoded: serde_json::Value = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded.get("id").and_then(serde_json::Value::as_u64), Some(73));
        assert_eq!(decoded.get("name").and_then(serde_json::Value::as_str), Some("LCK"));
        assert_eq!(decoded.get("active").and_then(serde_json::Value::as_bool), Some(true));
    }

    #[test]
    fn global_history_snapshot_filters_requested_team_and_completed_state() {
        let snapshot = GlobalHistorySnapshot {
            capture_index: 3,
            league_records: Vec::new(),
            league_competition_records: HashMap::new(),
            match_records: vec![
                GlobalMatchRecord {
                    json: r#"{"id":787}"#.to_string(),
                    completed: true,
                },
                GlobalMatchRecord {
                    json: r#"{"id":788}"#.to_string(),
                    completed: false,
                },
                GlobalMatchRecord {
                    json: r#"{"id":789}"#.to_string(),
                    completed: true,
                },
            ],
            team_match_indices: HashMap::from([
                (85, vec![0, 1]),
                (89, vec![0]),
                (90, vec![1]),
                (91, vec![2]),
                (92, vec![2]),
            ]),
            metrics: GlobalHistoryCaptureMetrics::default(),
        };

        assert_eq!(filter_global_team_records(&snapshot, 85, false).len(), 2);
        assert_eq!(filter_global_team_records(&snapshot, 85, true).len(), 1);
        assert_eq!(filter_global_team_records(&snapshot, 89, true).len(), 1);
        assert!(filter_global_team_records(&snapshot, 99, false).is_empty());
    }

    #[test]
    fn recent_match_selector_is_bounded_deterministic_and_keeps_highest_ids() {
        assert!(select_recent_global_match_ids(Vec::new(), 10).is_empty());
        assert_eq!(select_recent_global_match_ids(vec![42], 10), vec![42]);

        let ascending = (0..12_500).collect::<Vec<_>>();
        let mut shuffled = ascending.clone();
        shuffled.reverse();
        let selected = select_recent_global_match_ids(ascending, GLOBAL_HISTORY_RECORD_CAP);
        let selected_again = select_recent_global_match_ids(shuffled, GLOBAL_HISTORY_RECORD_CAP);

        assert_eq!(selected.len(), GLOBAL_HISTORY_RECORD_CAP);
        assert_eq!(selected, selected_again);
        assert_eq!(selected.first().copied(), Some(2_500));
        assert_eq!(selected.last().copied(), Some(12_499));
    }

    #[test]
    fn compact_league_competition_projection_preserves_editor_fields_only() {
        let source = serde_json::json!({
            "league_type": "Spring",
            "finalized": true,
            "unused_root": {"large": [1, 2, 3]},
            "standings": {
                "85": {
                    "win": 4, "lose": 1, "set_win": 9, "set_lose": 3,
                    "kill": 120, "death": 80, "assist": 300, "unused": 999
                }
            },
            "statistics": {
                "11": {
                    "matches": 5, "wins": 4, "kills": 30, "deaths": 8, "assists": 41,
                    "mvp": 2, "rating": 91, "gold": 12345, "dealing": 45678,
                    "healing": 10, "tanking": 9000, "solo_kill": 3, "solo_killed": 1,
                    "unused": "drop",
                    "champion_detail": {
                        "swordman": {
                            "matches": 3, "wins": 3, "rating": 95, "dealing": 222,
                            "healing": 0, "tanking": 100, "unused": 123
                        }
                    }
                }
            }
        });

        let compact = global_compact_league_competition_value(&source);
        assert_eq!(compact.get("league_type"), source.get("league_type"));
        assert_eq!(compact.get("finalized"), source.get("finalized"));
        assert!(compact.get("unused_root").is_none());
        assert!(compact["standings"]["85"].get("unused").is_none());
        assert!(compact["statistics"]["11"].get("unused").is_none());
        assert!(compact["statistics"]["11"]["champion_detail"]["swordman"]
            .get("unused")
            .is_none());
        assert_eq!(compact["statistics"]["11"]["rating"], 91);
        assert_eq!(compact["standings"]["85"]["set_win"], 9);
    }

    #[test]
    fn compact_match_projection_preserves_schedule_and_history_contract() {
        let source = serde_json::json!({
            "date": "2034-06-10T16:30:00",
            "is_practice": false,
            "team1": {"Normal": 85, "unused": "drop"},
            "team2": {"Normal": 89},
            "replays": [1001, 1002, 1003],
            "running_state": {
                "End": {
                    "team1_score": 2,
                    "team2_score": 1,
                    "winner": 85,
                    "unused": {"large": true}
                }
            },
            "unused_match_payload": [1, 2, 3, 4]
        });

        let compact = global_compact_match_value(&source);
        assert_eq!(global_json_normal_team_id(&compact, "team1"), Some(85));
        assert_eq!(global_json_normal_team_id(&compact, "team2"), Some(89));
        assert!(global_json_is_completed_match(&compact));
        assert_eq!(compact["replays"].as_array().map(Vec::len), Some(3));
        assert_eq!(compact["running_state"]["End"]["winner"], 85);
        assert!(compact.get("unused_match_payload").is_none());
        assert!(compact["running_state"]["End"].get("unused").is_none());
    }

    #[test]
    fn team_match_index_scales_to_ten_thousand_records_and_filters_before_clone() {
        let mut snapshot = GlobalHistorySnapshot::default();
        for record_index in 0..GLOBAL_HISTORY_RECORD_CAP {
            let team_id = record_index % 200;
            snapshot.match_records.push(GlobalMatchRecord {
                json: format!(r#"{{"id":{record_index}}}"#),
                completed: record_index % 2 == 0,
            });
            index_global_match_for_team(
                &mut snapshot.team_match_indices,
                Some(team_id),
                record_index,
            );
        }

        assert_eq!(filter_global_team_records(&snapshot, 17, false).len(), 50);
        assert_eq!(filter_global_team_records(&snapshot, 17, true).len(), 0);
        assert_eq!(filter_global_team_records(&snapshot, 18, true).len(), 50);
        assert!(filter_global_team_records(&snapshot, 999, false).is_empty());
    }

    #[test]
    fn very_large_single_team_history_remains_bounded_by_snapshot_cap() {
        let mut snapshot = GlobalHistorySnapshot::default();
        for record_index in 0..GLOBAL_HISTORY_RECORD_CAP {
            snapshot.match_records.push(GlobalMatchRecord {
                json: format!(r#"{{"id":{record_index}}}"#),
                completed: true,
            });
            index_global_match_for_team(
                &mut snapshot.team_match_indices,
                Some(85),
                record_index,
            );
        }

        let records = filter_global_team_records(&snapshot, 85, true);
        assert_eq!(records.len(), GLOBAL_HISTORY_RECORD_CAP);
        assert!(records.first().is_some_and(|record| record.contains(r#""id":0"#)));
        assert!(records.last().is_some_and(|record| record.contains(r#""id":9999"#)));
    }

    #[test]
    fn streamed_hex_payload_matches_existing_wire_format() {
        let records = vec!["A|B".to_string(), "René".to_string(), "{}".to_string()];
        let legacy = records.iter().map(|record| hex_encode(record)).collect::<Vec<_>>().join(";");
        assert_eq!(hex_join_records(&records), legacy);
        assert_eq!(hex_join_records(&[]), "");
        assert_eq!(hex_join_records(&["x".to_string()]), hex_encode("x"));
    }
}

#[cfg(test)]
mod name_payload_tests {
    use super::*;

    #[test]
    fn entity_name_payload_round_trips_unicode_and_separator() {
        let values = EntityNameValue {
            name: "René | 홍길동".to_string(),
        };
        let payload = entity_name_payload(42, &values);
        let (entity_id, decoded) = parse_server_entity_name(&payload).unwrap();
        assert_eq!(entity_id, 42);
        assert_eq!(decoded.name, values.name);
    }

    #[test]
    fn entity_name_validation_trims_and_rejects_invalid_values() {
        assert_eq!(validate_entity_name("  Valid Name  ").unwrap(), "Valid Name");
        assert!(validate_entity_name("   ").is_err());
        assert!(validate_entity_name("line\nbreak").is_err());
        assert!(validate_entity_name(&"x".repeat(101)).is_err());
    }
}

#[cfg(test)]
mod team_quick_edit_payload_tests {
    use super::*;

    #[test]
    fn quick_contract_end_reuses_full_contract_editor_end_of_day_serialization() {
        assert_eq!(
            contract_end_text("2032-12-31").unwrap(),
            "2032-12-31T23:59:59"
        );
        assert_eq!(validate_contract_end_date_text("2032-12-31"), Ok("2032-12-31"));
        assert!(validate_contract_end_date_text("2032-12-31T23:59:59").is_err());
        assert_eq!(
            player_contract_end_payload(42, &PlayerContractEndValue {
                end_date: "2032-12-31".to_string(),
            }),
            b"42|2032-12-31"
        );
        assert_eq!(
            staff_contract_end_payload(7, &StaffContractEndValue {
                end_date: "2032-12-31".to_string(),
            }),
            b"7|2032-12-31"
        );
    }

    #[test]
    fn team_merchandise_payload_round_trips_only_target_fields() {
        let values = TeamMerchandiseWriteValue {
            product_type: "Uniform|Special".to_string(),
            athlete_id: 42,
            stock: "123".to_string(),
            sell_price: "49.5".to_string(),
        };
        let payload = team_merchandise_write_payload(85, &values);
        let (team_id, decoded) = parse_team_merchandise_write_payload(&payload).unwrap();
        assert_eq!(team_id, 85);
        assert_eq!(decoded.product_type, values.product_type);
        assert_eq!(decoded.athlete_id, 42);
        assert_eq!(decoded.stock, "123");
        assert_eq!(decoded.sell_price, "49.5");
    }

    #[test]
    fn team_fans_payload_round_trips_observed_satisfaction_text() {
        let values = TeamFansWriteValue {
            popularity: "74".to_string(),
            fan_count: "120000".to_string(),
            fan_expectation: "Lower|Observed".to_string(),
            fan_satisfaction: "VerySatisfied|Observed".to_string(),
            fan_momentum: "5".to_string(),
        };
        let payload = team_fans_write_payload(85, &values);
        let (team_id, decoded) = parse_team_fans_write_payload(&payload).unwrap();
        assert_eq!(team_id, 85);
        assert_eq!(decoded.popularity, "74");
        assert_eq!(decoded.fan_count, "120000");
        assert_eq!(decoded.fan_expectation, values.fan_expectation);
        assert_eq!(decoded.fan_satisfaction, values.fan_satisfaction);
        assert_eq!(decoded.fan_momentum, "5");
    }

    #[test]
    fn team_fans_payload_rejects_legacy_shape() {
        let old_payload = b"85|2|37193|4c6f776572|56657279536174697366696564";
        assert!(parse_team_fans_write_payload(old_payload).is_err());
    }

    #[test]
    fn team_fan_count_matches_game_composite_and_writes_back_to_base_team_fans() {
        let roster_player_fans = 5_083u128 + 7_015 + 3_118 + 55 + 2_993 + 3_832;
        assert_eq!(roster_player_fans, 22_096);
        assert_eq!(displayed_team_fan_count(15_097, roster_player_fans), Ok(37_193));
        assert_eq!(
            base_team_fan_count_from_displayed(40_000, roster_player_fans),
            Ok(17_904)
        );
        assert_eq!(
            base_team_fan_count_from_displayed(20_000, roster_player_fans),
            Err("FAN_COUNT_BELOW_PLAYER_FANS")
        );
    }

    #[test]
    fn team_fans_payload_includes_expectation_and_validated_momentum() {
        let values = TeamFansWriteValue {
            popularity: "2".to_string(),
            fan_count: "37193".to_string(),
            fan_expectation: "Upper".to_string(),
            fan_satisfaction: "VerySatisfied".to_string(),
            fan_momentum: "-4".to_string(),
        };
        let payload = String::from_utf8(team_fans_write_payload(85, &values)).unwrap();
        assert_eq!(
            payload,
            "85|2|37193|5570706572|56657279536174697366696564|-4"
        );
    }

    #[test]
    fn team_quick_edit_validation_rejects_invalid_numeric_values() {
        assert!(validate_team_merchandise_write(&TeamMerchandiseWriteValue {
            product_type: "Uniform".to_string(),
            athlete_id: 1,
            stock: "-1".to_string(),
            sell_price: "10".to_string(),
        }).is_err());
        assert!(validate_team_fans_write(&TeamFansWriteValue {
            popularity: "10".to_string(),
            fan_count: "-1".to_string(),
            fan_expectation: "Lower".to_string(),
            fan_satisfaction: "Satisfied".to_string(),
            fan_momentum: "0".to_string(),
        }).is_err());
        assert!(validate_team_fans_write(&TeamFansWriteValue {
            popularity: "2".to_string(),
            fan_count: "37193".to_string(),
            fan_expectation: "Lower".to_string(),
            fan_satisfaction: "Satisfied".to_string(),
            fan_momentum: "6".to_string(),
        }).is_err());
    }
}

#[cfg(test)]
mod staff_role_payload_tests {
    use super::*;

    #[test]
    fn replay_id_list_parser_accepts_unique_ids_and_rejects_empty_input() {
        assert_eq!(parse_replay_id_list("87,88,87"), Ok(vec![87, 88]));
        assert_eq!(parse_replay_id_list(""), Err("NO_REPLAY_IDS"));
        assert_eq!(parse_replay_id_list("87,nope"), Err("INVALID_REPLAY_ID"));
    }

    #[test]
    fn team_strategy_probe_payload_round_trips_team_id() {
        let payload = team_strategy_probe_payload(85);
        assert_eq!(parse_team_strategy_probe_payload(&payload), Ok(85));
        assert_eq!(
            parse_team_strategy_probe_payload(b"not-an-id"),
            Err("INVALID_ID")
        );
    }

    #[test]
    fn team_strategy_set_payload_round_trips_all_fields() {
        let raw = [
            "focused\tBottom",
            "early_jungle\tCounterJungle",
            "early_serpen\tFlexible",
            "early_serpen_top\tFlexible",
            "object_buildup\tFlexible",
            "object_battle\tPoking",
            "morgard_use\tGather",
            "tower_press\tPoking",
            "morgard_defense\tBattle",
            "object_finish\tKillPriority",
            "minion_wave\tWavePriority",
            "game_finish\tStable",
        ]
        .join("\n");
        let payload = team_strategy_set_payload(85, &raw);
        let (team_id, decoded) = parse_team_strategy_set_payload(&payload).unwrap();
        assert_eq!(team_id, 85);
        assert_eq!(decoded, raw);
        assert_eq!(parse_team_strategy_values(&decoded).unwrap().len(), 12);
    }

    #[test]
    fn move_staff_payload_accepts_optional_supported_role() {
        let (staff_id, team_id, role) =
            parse_move_staff_payload(b"42|85|Analyst").expect("role payload");
        assert_eq!(staff_id, 42);
        assert_eq!(team_id, 85);
        assert_eq!(role.as_deref(), Some("Analyst"));

        let (_, _, role) = parse_move_staff_payload(b"42|85").expect("legacy payload");
        assert_eq!(role, None);
    }

    #[test]
    fn staff_role_parser_accepts_exact_game_variants() {
        for role in ["HeadCoach", "TrainingCoach", "Scouter", "Analyst"] {
            assert!(parse_staff_role(role).is_ok(), "{role}");
        }
        assert!(parse_staff_role("Coach").is_err());
    }
}

fn init(_ctx: &GameCtx) -> ModRegistration {
    start_bridge_server();

    let mut reg = ModRegistration::new(MOD_ID);
    reg.set_extension(ModifierBridgeClient);
    reg.set_server_extension(ModifierBridgeServer);
    reg
}

declare_mod!(init);
