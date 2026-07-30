use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use mod_api::*;
use game_core::{Contract, PaperState};

const MOD_ID: &str = "tfm2_modifier_bridge";
const BRIDGE_ADDR: &str = "127.0.0.1:28452";
const BRIDGE_VERSION: &str = "0.2.17";

#[derive(Debug, Clone, Copy)]
struct EconomyValues {
    money: f64,
    transfer_budget: f64,
    salary_budget: f64,
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
struct TeamListEntry {
    id: usize,
    display_name: String,
    manager_name: String,
    league_id: usize,
    is_player_team: bool,
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
    contract_end_date: String,
    primary_region: String,
    communication_raw: String,
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

#[derive(Debug, Clone, Copy)]
struct PlayerCommunicationValue {
    region_id: usize,
    level: i32,
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
    GetTeams {
        reply: Sender<String>,
    },
    MovePlayerToTeam {
        athlete_id: usize,
        team_id: usize,
        reply: Sender<String>,
    },
    GetPlayer {
        athlete_id: usize,
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
}

static SERVER_STARTED: AtomicBool = AtomicBool::new(false);
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

fn response_ok_economy(values: EconomyValues) -> String {
    format!(
        "OK|ECONOMY|{}|{}|{}",
        values.money,
        values.transfer_budget,
        values.salary_budget
    )
}

fn hex_encode(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let bytes = value.as_bytes();
    let mut encoded = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }

    encoded
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

fn response_ok_teams(teams: &[TeamListEntry]) -> String {
    let payload = teams
        .iter()
        .map(|team| format!(
            "{}:{}:{}:{}:{}",
            team.id,
            hex_encode(&team.display_name),
            hex_encode(&team.manager_name),
            team.league_id,
            if team.is_player_team { 1 } else { 0 }
        ))
        .collect::<Vec<_>>()
        .join(";");

    format!("OK|TEAMS|{}|{}", teams.len(), payload)
}

fn response_ok_player(player: PlayerSnapshot) -> String {
    format!(
        concat!(
            "OK|PLAYER|{}|{}|",
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|",
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}"
        ),
        player.id,
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
        hex_encode(&player.contract_end_date),
        player.primary_region,
        hex_encode(&player.communication_raw),
    )
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

fn read_teams(scene: &mut Scene) -> Result<Vec<TeamListEntry>, &'static str> {
    let Scene::InGame { data } = scene else {
        return Err("NOT_IN_GAME");
    };

    let player_team_id = data.player_team_id();
    let db = data.db();
    let mut teams = db
        .teams
        .iter()
        .map(|(id, team)| TeamListEntry {
            id: *id,
            display_name: db.team_display_name(team).to_string(),
            manager_name: team.manager_name.to_string(),
            league_id: team.league_id,
            is_player_team: *id == player_team_id,
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
        if db.teams.get(&destination_team_id).is_none() {
            return Err("TEAM_NOT_FOUND");
        }
        if db.athletes.get(&athlete_id).is_none() {
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

fn communication_raw(athlete: &Athlete) -> String {
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
    for (_, athlete) in db.athletes.iter() {
        if let Some(region_id) = athlete.get_primary_region() {
            region_ids.push(region_id);
        }
        region_ids.extend(athlete.training_exp.language_by_region.keys().copied());
    }
    region_ids.sort_unstable();
    region_ids.dedup();
    Ok(region_ids)
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

fn transfer_fee_raw(athlete: &Athlete) -> String {
    match &athlete.contract {
        Contract::InContract { transfer_fee, .. } => transfer_fee.to_string(),
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

fn apply_contract_end_to_athlete(
    athlete: &mut Athlete,
    values: &PlayerContractEndValue,
) -> Result<(), &'static str> {
    let requested = validate_contract_end_date_text(&values.end_date)?;
    match &mut athlete.contract {
        Contract::InContract { end_date, .. } => {
            // Preserve any time/timezone suffix used by the concrete game_core date type.
            // This lets the editor expose a simple YYYY-MM-DD field while parsing back into
            // exactly the same type that TFM2 uses internally.
            let current = end_date.to_string();
            let candidate = if current.len() > 10 {
                format!("{}{}", requested, &current[10..])
            } else {
                requested.to_string()
            };
            *end_date = candidate.parse().map_err(|_| "INVALID_CONTRACT_DATE")?;
            Ok(())
        }
        Contract::FreeAgent { .. } => Err("PLAYER_FREE_AGENT"),
    }
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
        contract_end_date: contract_end_date_raw(athlete),
        primary_region: primary_region_raw(athlete),
        communication_raw: communication_raw(athlete),
    })
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
        contract_end_date: contract_end_date_raw(athlete),
        primary_region: primary_region_raw(athlete),
        communication_raw: communication_raw(athlete),
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
    if values.level < 0 {
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
        athlete.training_exp.language_by_region.remove(&values.region_id);
    } else {
        athlete
            .training_exp
            .language_by_region
            .insert(values.region_id, values.level);
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
        level: parts.next().ok_or("MISSING_VALUE")?.parse::<i32>().map_err(|_| "INVALID_COMMUNICATION")?,
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
            athlete.training_exp.language_by_region.insert(*region_id, 100);
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
            GameRequest::GetTeams { reply } => {
                let response = match read_teams(scene) {
                    Ok(teams) => response_ok_teams(&teams),
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
            GameRequest::GetPlayer { athlete_id, reply } => {
                let response = match read_player(scene, athlete_id) {
                    Ok(player) => response_ok_player(player),
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
        "PING" => format!("OK|PONG|{BRIDGE_VERSION}"),
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
        "GET_TEAMS" => send_game_request(request_tx, |reply| GameRequest::GetTeams { reply }),
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
        "GET_PLAYER" => match parse_usize(parts.next()) {
            Ok(athlete_id) => send_game_request(request_tx, |reply| GameRequest::GetPlayer {
                athlete_id,
                reply,
            }),
            Err(reason) => format!("ERR|{reason}"),
        },
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
        "SET_PLAYER_COMMUNICATION" => {
            let parsed: Result<(usize, PlayerCommunicationValue), &'static str> = (|| {
                let athlete_id = parse_usize(parts.next())?;
                let region_id = parse_usize(parts.next())?;
                let level = parts.next().ok_or("MISSING_VALUE")?.parse::<i32>().map_err(|_| "INVALID_COMMUNICATION")?;
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
    fn on_server_start(&self, _ctx: &mut ServerModContext) {
        clear_contract_end_overrides();

        // Keep the two runtime toggle preferences across save/career loads. The
        // destination team is save-specific, so mark the team IDs stale; the client
        // extension will re-send enabled settings for the newly loaded player team.
        TRANSFER_ALWAYS_SUCCESS_TEAM_ID.store(usize::MAX, Ordering::SeqCst);
        RECRUITMENT_INSTANT_RETRY_TEAM_ID.store(usize::MAX, Ordering::SeqCst);
    }

    fn before_management_tick(&self, ctx: &mut ServerModContext) {
        enforce_contract_end_overrides(ctx, false);
        force_transfer_request_success(ctx);
        clear_player_recruitment_cooldowns(ctx);
    }

    fn after_management_tick(&self, ctx: &mut ServerModContext) {
        enforce_contract_end_overrides(ctx, true);
        force_transfer_request_success(ctx);
        clear_player_recruitment_cooldowns(ctx);
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

        if command.command == "move_player_to_team" {
            let Ok((athlete_id, team_id)) = parse_move_player_payload(&command.payload) else {
                return ModServerCommandResult::Handled;
            };
            let _ = move_player_to_team_server(ctx, athlete_id, team_id);
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

fn init(_ctx: &GameCtx) -> ModRegistration {
    start_bridge_server();

    let mut reg = ModRegistration::new(MOD_ID);
    reg.set_extension(ModifierBridgeClient);
    reg.set_server_extension(ModifierBridgeServer);
    reg
}

declare_mod!(init);
