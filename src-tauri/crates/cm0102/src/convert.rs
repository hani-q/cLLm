//! Converts a decoded CM 01/02 database into a cLLm world that starts on 1 July 2001.
//!
//! The world is assembled as JSON and then parsed with the game's own loader, so anything the
//! game would reject fails here first. Attribute scales: CM rates 1-20, the game 0-100.

use std::collections::{BTreeMap, HashMap, HashSet};

use serde_json::{Value, json};

use crate::dat::{Club, CmDate, Database, PlayerAttr, PlayerRecord, Staff};

/// Pounds to euros at the 2001 rate. The game counts money in euros.
const GBP_TO_EUR: f64 = 1.6;
const SNAPSHOT_DATE: &str = "2001-07-01";
/// CM job code for a club manager.
const JOB_MANAGER: u8 = 5;
/// A CM rating at or above this counts as a position the player can play.
const COMPETENT: i8 = 15;

/// Which part of the CM world to import.
#[derive(Debug, Clone)]
pub struct ImportOptions {
    /// CM nation names, each optionally with its own tier depth: "England:4", "Spain".
    pub nations: Vec<String>,
    /// Tier depth for a nation listed without one (1 = top division only).
    pub max_tiers: usize,
    /// Best unattached players (by current ability) from the chosen nations to add as free agents.
    pub free_agents: usize,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            nations: [
                "England:4",
                "Scotland:4",
                "Italy",
                "Spain",
                "Germany",
                "France",
                "Holland",
                "Portugal",
            ]
            .map(String::from)
            .to_vec(),
            max_tiers: 2,
            free_agents: 1_500,
        }
    }
}

/// What an import produced, for the command line summary.
#[derive(Debug, Clone, Default)]
pub struct ImportSummary {
    pub leagues: Vec<String>,
    pub clubs: usize,
    pub players: usize,
    pub managers: usize,
}

/// One league tier picked for import.
struct Tier {
    competition_id: i32,
    name: String,
    country: String,
    priority: u32,
}

pub fn build_world(
    db: &Database,
    options: &ImportOptions,
) -> Result<(Value, ImportSummary), String> {
    let depth_by_name: HashMap<String, usize> = options
        .nations
        .iter()
        .map(|entry| match entry.split_once(':') {
            Some((name, depth)) => (
                name.trim().to_lowercase(),
                depth.trim().parse().unwrap_or(options.max_tiers),
            ),
            None => (entry.trim().to_lowercase(), options.max_tiers),
        })
        .collect();
    let chosen: Vec<_> = db
        .nations
        .iter()
        .filter(|n| depth_by_name.contains_key(&n.name.to_lowercase()))
        .collect();
    let nations: HashMap<i32, String> = chosen
        .iter()
        .map(|n| (n.id, nation_code(&n.name, &n.three_letter)))
        .collect();
    let depths: HashMap<i32, usize> = chosen
        .iter()
        .map(|n| (n.id, depth_by_name[&n.name.to_lowercase()]))
        .collect();
    if nations.len() != options.nations.len() {
        let known: Vec<&str> = db.nations.iter().map(|n| n.name.as_str()).collect();
        return Err(format!(
            "unknown nation in {:?}; CM nations include {:?}",
            options.nations,
            &known[..20]
        ));
    }

    let tiers = pick_tiers(db, &nations, &depths);
    let tier_by_comp: HashMap<i32, &Tier> = tiers.iter().map(|t| (t.competition_id, t)).collect();
    let clubs: Vec<&Club> = db
        .clubs
        .iter()
        .filter(|c| tier_by_comp.contains_key(&c.division))
        .collect();
    let club_ids: HashSet<i32> = clubs.iter().map(|c| c.id).collect();
    let staff_by_id: HashMap<i32, &Staff> = db.staff.iter().map(|s| (s.id, s)).collect();

    let mut players = Vec::new();
    let mut wage_bills: HashMap<i32, i64> = HashMap::new();
    for club in &clubs {
        // CM lets reserves share a first-teamer's squad number; the game's saves need one
        // holder per number per club, so later holders lose theirs.
        let mut numbers_taken = HashSet::new();
        for staff in club.squad.iter().filter_map(|id| staff_by_id.get(id)) {
            if let Some(mut player) = player_json(db, staff, Some(club.id)) {
                if let Some(number) = player.get("jersey_number").and_then(Value::as_u64)
                    && !numbers_taken.insert(number)
                    && let Some(fields) = player.as_object_mut()
                {
                    fields.remove("jersey_number");
                }
                *wage_bills.entry(club.id).or_default() += euros(staff.wage);
                players.push(player);
            }
        }
    }
    players.extend(free_agents(db, &nations, &club_ids, options.free_agents));

    let mut used_codes = HashSet::new();
    let teams: Vec<Value> = clubs
        .iter()
        // A club takes the country of the league it plays in: Cardiff and Swansea are Welsh
        // clubs in the English pyramid, and the game needs every league member in its country.
        .map(|c| {
            team_json(
                db,
                c,
                &tier_by_comp[&c.division].country,
                wage_bills.get(&c.id).copied().unwrap_or(0),
                &mut used_codes,
            )
        })
        .collect();
    let managers: Vec<Value> = clubs
        .iter()
        .filter_map(|c| manager_json(db, c, &staff_by_id))
        .collect();
    let competitions = competitions_json(&tiers, &clubs, &nations);

    let summary = ImportSummary {
        leagues: tiers.iter().map(|t| t.name.clone()).collect(),
        clubs: teams.len(),
        players: players.len(),
        managers: managers.len(),
    };
    let world = json!({
        "name": "Championship Manager 01/02",
        "description": format!("The 2001/02 season from your CM 01/02 database: {} clubs in {} leagues.", teams.len(), tiers.len()),
        "teams": teams,
        "players": players,
        "staff": [],
        "managers": managers,
        "competitionDefinitions": { "formatVersion": 1, "competitions": competitions },
        "metadata": {
            "format_version": 1,
            "world_id": "cm0102-2001",
            "kind": "historicalSnapshot",
            "base_year": 2001,
            "snapshot_date": format!("{SNAPSHOT_DATE}T00:00:00Z"),
        },
    });
    Ok((world, summary))
}

/// League tiers per nation, best first. CM sometimes runs parallel regional divisions at one
/// level; the game needs a single ladder, so a division with the same reputation as one
/// already taken is skipped.
fn pick_tiers(
    db: &Database,
    nations: &HashMap<i32, String>,
    depths: &HashMap<i32, usize>,
) -> Vec<Tier> {
    let clubs_per_comp = db
        .clubs
        .iter()
        .fold(HashMap::<i32, usize>::new(), |mut acc, c| {
            *acc.entry(c.division).or_default() += 1;
            acc
        });
    let mut tiers = Vec::new();
    for (nation_id, code) in nations.iter().collect::<BTreeMap<_, _>>() {
        let mut comps: Vec<_> = db
            .competitions
            .iter()
            .filter(|c| {
                c.nation == *nation_id && clubs_per_comp.get(&c.id).copied().unwrap_or(0) >= 4
            })
            .collect();
        comps.sort_by_key(|c| (std::cmp::Reverse(c.reputation), c.id));
        let mut seen_reputation = HashSet::new();
        for comp in comps
            .into_iter()
            .filter(|c| seen_reputation.insert(c.reputation))
            .take(depths[nation_id])
        {
            tiers.push(Tier {
                competition_id: comp.id,
                name: comp.name.clone(),
                country: code.clone(),
                priority: u32::try_from(
                    tiers.iter().filter(|t: &&Tier| t.country == *code).count(),
                )
                .unwrap_or(0),
            });
        }
    }
    tiers
}

fn competitions_json(
    tiers: &[Tier],
    clubs: &[&Club],
    nations: &HashMap<i32, String>,
) -> Vec<Value> {
    let mut out: Vec<Value> = tiers
        .iter()
        .map(|tier| {
            let members: Vec<String> = clubs
                .iter()
                .filter(|c| c.division == tier.competition_id)
                .map(|c| team_id(c.id))
                .collect();
            json!({
                "id": format!("cm-league-{}", tier.competition_id),
                "name": tier.name,
                "type": "League",
                "scope": "Domestic",
                "countryId": tier.country,
                "priority": tier.priority,
                "format": { "kind": "LeagueTable", "legs": 2 },
                "participants": { "explicit": members },
                "seasonStartMonth": 8,
                "seasonStartDay": 18,
            })
        })
        .collect();
    let countries: std::collections::BTreeSet<&String> = nations.values().collect();
    for country in countries {
        out.push(json!({
            "id": format!("cm-cup-{}", country.to_lowercase()),
            "name": format!("{} Cup", ofm_core::nations::nation_display_name(country)),
            "type": "Cup",
            "scope": "Domestic",
            "countryId": country,
            "format": { "kind": "Knockout", "legs": 1 },
            "participants": { "selector": { "kind": "allInCountry", "country": country } },
            "seasonStartMonth": 8,
            "seasonStartDay": 18,
        }));
    }
    out.push(json!({
        "id": "cm-european-cup",
        "name": "European Champions Cup",
        "type": "ContinentalClub",
        "scope": "Continental",
        "regionId": "europe",
        "format": { "kind": "GroupAndKnockout", "legs": 2, "groupSize": 4, "qualifiersPerGroup": 2 },
        "participants": { "explicit": european_cup_entrants(clubs) },
        "seasonStartMonth": 9,
        "seasonStartDay": 11,
    }));
    out
}

/// The 32 best-regarded clubs, at most four per league, as a stand-in for the 2001/02 field.
fn european_cup_entrants(clubs: &[&Club]) -> Vec<String> {
    let mut ranked: Vec<&&Club> = clubs.iter().collect();
    ranked.sort_by_key(|c| (std::cmp::Reverse(c.reputation), c.id));
    let mut per_league: HashMap<i32, usize> = HashMap::new();
    ranked
        .into_iter()
        .filter(|c| {
            let taken = per_league.entry(c.division).or_default();
            *taken += 1;
            *taken <= 4
        })
        .take(32)
        .map(|c| team_id(c.id))
        .collect()
}

fn team_json(
    db: &Database,
    club: &Club,
    country: &str,
    weekly_wages: i64,
    used: &mut HashSet<String>,
) -> Value {
    let stadium = usize::try_from(club.stadium)
        .ok()
        .and_then(|i| db.stadiums.get(i));
    let city = stadium
        .and_then(|s| usize::try_from(s.city).ok())
        .and_then(|i| db.cities.get(i))
        .cloned()
        .unwrap_or_else(|| club.short_name.clone());
    let (primary, secondary) = kit_colours(db, club.home_colours);
    let finance = euros(club.cash).max(weekly_wages * 26);
    json!({
        "id": team_id(club.id),
        "name": club.name,
        "short_name": unique_code(&club.short_name, used),
        "country": country,
        "football_nation": country,
        "city": city,
        "stadium_name": stadium.map_or_else(|| format!("{} Ground", club.short_name), |s| s.name.clone()),
        "stadium_capacity": stadium.map_or(5_000, |s| s.capacity.max(1_000)),
        "finance": finance,
        "reputation": (u32::from(club.reputation) / 10).clamp(1, 1_000),
        "wage_budget": (weekly_wages * 100 + 89) / 90,
        "transfer_budget": (finance / 4).max(0),
        "season_income": 0,
        "season_expenses": 0,
        "formation": "4-4-2",
        "play_style": "Balanced",
        "founded_year": 1900,
        "colors": { "primary": primary, "secondary": secondary },
        "history": [],
    })
}

fn manager_json(db: &Database, club: &Club, staff: &HashMap<i32, &Staff>) -> Option<Value> {
    let s = staff
        .get(&club.manager)
        .filter(|s| s.club_job == JOB_MANAGER)?;
    let (first, second, common) = db.person_names(s);
    let reputation = usize::try_from(s.non_player)
        .ok()
        .and_then(|i| db.non_players.get(i))
        .map_or(300, |np| u32::from(np.world_reputation) / 10);
    Some(json!({
        "id": format!("cm-m-{}", s.id),
        "first_name": if first.is_empty() { common.clone() } else { first },
        "last_name": if second.is_empty() { common } else { second },
        "date_of_birth": birth_date(s, 1955),
        "nationality": nationality(db, s.nation),
        "reputation": reputation.clamp(1, 1_000),
        "satisfaction": 60,
        "career_stats": { "matches_managed": 0, "wins": 0, "draws": 0, "losses": 0, "trophies": 0 },
        "career_history": [],
        "team_id": team_id(club.id),
    }))
}

fn free_agents(
    db: &Database,
    nations: &HashMap<i32, String>,
    club_ids: &HashSet<i32>,
    limit: usize,
) -> Vec<Value> {
    let mut pool: Vec<(&Staff, &PlayerRecord)> = db
        .staff
        .iter()
        .filter(|s| nations.contains_key(&s.nation) && !club_ids.contains(&s.club) && s.club < 0)
        .filter_map(|s| {
            usize::try_from(s.player)
                .ok()
                .and_then(|i| db.players.get(i))
                .map(|p| (s, p))
        })
        .collect();
    pool.sort_by_key(|(_, p)| std::cmp::Reverse(p.current_ability));
    pool.into_iter()
        .take(limit)
        .filter_map(|(s, _)| player_json(db, s, None))
        .collect()
}

fn player_json(db: &Database, staff: &Staff, club: Option<i32>) -> Option<Value> {
    let p = db.players.get(usize::try_from(staff.player).ok()?)?;
    let (first, second, common) = db.person_names(staff);
    let full_name = [first.as_str(), second.as_str()]
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(" ");
    let match_name = if !common.is_empty() {
        common.clone()
    } else if let Some(initial) = first.chars().next() {
        format!("{initial}. {second}")
    } else {
        second.clone()
    };
    let (position, alternates) = positions(p);
    let (footedness, weak_foot) = feet(p);
    let (contract_start, contract_end) = contract(staff);
    let mut player = json!({
        "id": format!("cm-p-{}", staff.id),
        "match_name": match_name,
        "full_name": if full_name.is_empty() { common } else { full_name },
        "date_of_birth": birth_date(staff, 1980),
        "nationality": nationality(db, staff.nation),
        "position": position,
        "natural_position": position,
        "alternate_positions": alternates,
        "footedness": footedness,
        "weak_foot": weak_foot,
        "attributes": attributes(p),
        "condition": 100,
        "morale": 70,
        "fitness": scale(p.attr(PlayerAttr::NaturalFitness)).max(60),
        "injury": null,
        "team_id": club.map(team_id),
        "market_value": u64::try_from(euros(staff.value)).unwrap_or(0).max(10_000),
        "wage": u32::try_from(euros(staff.wage)).unwrap_or(0).max(100),
        "contract_start": contract_start,
        "contract_end": contract_end,
        "movement_history": [],
        "stats": {},
        "career": [],
        "potential": potential_target(p),
    });
    if club.is_some() && (1..=99).contains(&p.squad_number) {
        player["jersey_number"] = json!(p.squad_number);
    }
    Some(player)
}

/// CM potential ability (1-200) on the game's 1-99 scale. The final potential is never below the
/// player's current rating; see [`finish_world`].
///
/// CM marks some youngsters with a negative potential that the game rolls at career start: -2 is
/// the wider, higher range that made Cherno Samba, -1 the usual one. Those resolve here from the
/// player id, so every import gives the same answer.
fn potential_target(p: &PlayerRecord) -> i32 {
    let roll = |low: i32, high: i32| {
        low + i32::try_from(u32::try_from(p.id).unwrap_or(0).wrapping_mul(2_654_435_761) % 1_000)
            .unwrap_or(0)
            * (high - low)
            / 1_000
    };
    let pa = match p.potential_ability {
        -2 => roll(150, 200),
        -1 => roll(130, 195),
        pa if pa < 0 => roll(110, 180),
        pa => i32::from(pa).max(i32::from(p.current_ability)),
    };
    (35 + pa * 3 / 10).clamp(1, 99)
}

fn attributes(p: &PlayerRecord) -> Value {
    use PlayerAttr as A;
    let a = |attr| scale(p.attr(attr));
    let avg = |x: PlayerAttr, y: PlayerAttr| (a(x) + a(y)) / 2;
    json!({
        "pace": avg(A::Pace, A::Acceleration),
        "stamina": a(A::Stamina),
        "strength": a(A::Strength),
        "agility": avg(A::Agility, A::Balance),
        "passing": avg(A::Passing, A::Technique),
        "shooting": avg(A::Finishing, A::LongShots),
        "tackling": a(A::Tackling),
        "dribbling": avg(A::Dribbling, A::Flair),
        "defending": avg(A::Marking, A::Positioning),
        "positioning": avg(A::Movement, A::Anticipation),
        "vision": a(A::Vision),
        "decisions": a(A::Decisions),
        "composure": avg(A::ImportantMatches, A::Consistency),
        "aggression": a(A::Aggression),
        "teamwork": avg(A::Teamwork, A::WorkRate),
        "leadership": a(A::Leadership),
        "handling": a(A::Handling),
        "reflexes": avg(A::Reflexes, A::OneOnOnes),
        "aerial": avg(A::Heading, A::Jumping),
    })
}

/// CM 1-20 to the game's 0-100.
fn scale(value: i8) -> u32 {
    u32::try_from(i32::from(value).clamp(1, 20) * 5 - 2).unwrap_or(1)
}

const GK: usize = 0;
const SW: usize = 1;
const D: usize = 2;
const DM: usize = 3;
const M: usize = 4;
const AM: usize = 5;
const F: usize = 6;
const WB: usize = 7;

/// The best position, plus up to three others the player is rated competent in.
fn positions(p: &PlayerRecord) -> (&'static str, Vec<&'static str>) {
    let side = |s: usize| p.sides[s];
    let best_side = if side(0) >= side(1) && side(0) >= side(2) {
        0
    } else if side(1) >= side(2) {
        1
    } else {
        2
    };
    let named = |role: usize, s: usize| match (role, s) {
        (GK, _) => "Goalkeeper",
        (SW | D, 0) => "RightBack",
        (SW | D, 1) => "LeftBack",
        (SW | D, _) => "CenterBack",
        (WB, 1) => "LeftWingBack",
        (WB, _) => "RightWingBack",
        (DM, _) => "DefensiveMidfielder",
        (M, 0) => "RightMidfielder",
        (M, 1) => "LeftMidfielder",
        (M, _) => "CentralMidfielder",
        (AM, 0) => "RightWinger",
        (AM, 1) => "LeftWinger",
        (AM, _) => "AttackingMidfielder",
        _ => "Striker",
    };
    let mut roles: Vec<usize> = (0..8).collect();
    roles.sort_by_key(|r| std::cmp::Reverse(p.positions[*r]));
    let primary = named(roles[0], best_side);
    let mut alternates = Vec::new();
    for role in roles
        .iter()
        .copied()
        .filter(|r| p.positions[*r] >= COMPETENT)
    {
        for s in (0..3).filter(|s| side(*s) >= COMPETENT) {
            let name = named(role, s);
            if name != primary && !alternates.contains(&name) && alternates.len() < 3 {
                alternates.push(name);
            }
        }
    }
    let _ = F;
    (primary, alternates)
}

fn feet(p: &PlayerRecord) -> (&'static str, u8) {
    let left = p.attr(PlayerAttr::LeftFoot);
    let right = p.attr(PlayerAttr::RightFoot);
    let weak = u8::try_from(left.min(right).clamp(1, 20))
        .unwrap_or(1)
        .div_ceil(4)
        .clamp(1, 5);
    let foot = if left >= COMPETENT && right >= COMPETENT {
        "Both"
    } else if left > right {
        "Left"
    } else {
        "Right"
    };
    (foot, weak)
}

fn contract(staff: &Staff) -> (String, String) {
    let start = if staff.joined_club.is_known() {
        iso(staff.joined_club)
    } else {
        "2000-07-01".to_string()
    };
    let end = if staff.contract_expires.is_known() {
        iso(staff.contract_expires)
    } else {
        "2003-06-30".to_string()
    };
    // A deal that ran out before the season starts would leave the player a free agent on day one.
    let end = if end.as_str() <= SNAPSHOT_DATE {
        "2002-06-30".to_string()
    } else {
        end
    };
    let start = if start >= end {
        "2000-07-01".to_string()
    } else {
        start
    };
    (start, end)
}

fn birth_date(staff: &Staff, fallback_year: i32) -> String {
    if staff.date_of_birth.is_known() {
        iso(staff.date_of_birth)
    } else if staff.year_of_birth > 1900 {
        format!("{}-07-01", staff.year_of_birth)
    } else {
        format!("{fallback_year}-01-01")
    }
}

/// CM dates are a 0-based day of the year.
fn iso(date: CmDate) -> String {
    let year = i32::from(date.year);
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let lengths = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut day = i32::from(date.day_of_year).clamp(0, if leap { 365 } else { 364 });
    let mut month = 0;
    while day >= lengths[month] {
        day -= lengths[month];
        month += 1;
    }
    format!("{year:04}-{:02}-{:02}", month + 1, day + 1)
}

fn nationality(db: &Database, nation_id: i32) -> String {
    usize::try_from(nation_id)
        .ok()
        .and_then(|i| db.nations.get(i))
        .map_or_else(
            || "ENG".to_string(),
            |n| nation_code(&n.name, &n.three_letter),
        )
}

/// CM nation names to the game's codes (ISO alpha-2, plus ENG/SCO/WAL/NIR).
pub fn nation_code(name: &str, three_letter: &str) -> String {
    let alias = match name {
        "Holland" => "Netherlands",
        "USA" => "United States",
        "Korea Republic" | "South Korea" => "Korea Republic",
        "Czech Republic" => "Czechia",
        "Yugoslavia" => "Serbia",
        "Turkey" => "Türkiye",
        "C.I.S." => "Russia",
        other => other,
    };
    let normalized = domain::identity::normalize_football_nation_code(alias);
    if matches!(normalized.as_str(), "ENG" | "SCO" | "WAL" | "NIR" | "IE") {
        return normalized;
    }
    ofm_core::nations::nation_by_name(alias)
        .map_or_else(|| three_letter.to_ascii_uppercase(), |n| n.code.to_string())
}

fn kit_colours(db: &Database, (fore, back): (i32, i32)) -> (String, String) {
    let hex = |id: i32| {
        usize::try_from(id)
            .ok()
            .and_then(|i| db.colours.get(i))
            .map(|c| {
                let (r, g, b) = c.rgb;
                format!("#{r:02x}{g:02x}{b:02x}")
            })
    };
    let primary = hex(back).unwrap_or_else(|| "#ffffff".to_string());
    let mut secondary = hex(fore).unwrap_or_else(|| "#000000".to_string());
    if secondary == primary {
        secondary = if primary == "#ffffff" {
            "#000000".to_string()
        } else {
            "#ffffff".to_string()
        };
    }
    (primary, secondary)
}

/// Three-letter club code from CM's short name, made unique across the world.
fn unique_code(short_name: &str, used: &mut HashSet<String>) -> String {
    let letters: Vec<char> = short_name
        .chars()
        .filter(char::is_ascii_alphabetic)
        .map(|c| c.to_ascii_uppercase())
        .collect();
    let base: String = letters.iter().take(3).collect::<String>();
    let base = format!("{base:X<3}");
    let mut candidates = vec![base.clone()];
    for c in letters
        .iter()
        .skip(3)
        .chain(('A'..='Z').collect::<Vec<_>>().iter())
    {
        candidates.push(format!("{}{c}", &base[..2]));
    }
    let code = candidates
        .into_iter()
        .find(|c| !used.contains(c))
        .unwrap_or_else(|| format!("{base}{}", used.len()));
    used.insert(code.clone());
    code
}

fn team_id(club_id: i32) -> String {
    format!("cm-club-{club_id}")
}

fn euros(pounds: i32) -> i64 {
    (f64::from(pounds.max(0)) * GBP_TO_EUR).round() as i64
}

/// Parse the assembled world with the game's own loader, rate every player the way the game
/// does, set potentials from CM's ability gap, and check the league definitions.
pub fn finish_world(world: Value) -> Result<ofm_core::generator::WorldData, String> {
    let growth: HashMap<String, i32> = world["players"]
        .as_array()
        .map(|players| {
            players
                .iter()
                .filter_map(|p| {
                    Some((
                        p["id"].as_str()?.to_string(),
                        i32::try_from(p["potential"].as_i64()?).ok()?,
                    ))
                })
                .collect()
        })
        .unwrap_or_default();
    // The loader rejects bad league definitions with one opaque code, so they are checked
    // separately below, where the individual errors can be reported.
    let mut world = world;
    let definitions = world
        .as_object_mut()
        .and_then(|w| w.remove("competitionDefinitions"));
    let text = serde_json::to_string(&world).map_err(|e| e.to_string())?;
    let mut data = ofm_core::generator::load_world_from_json(&text)?;
    if let Some(definitions) = definitions {
        data.competition_definitions = Some(
            serde_json::from_value(definitions).map_err(|e| format!("league definitions: {e}"))?,
        );
    }
    for player in &mut data.players {
        let ovr = ofm_core::player_rating::ovr_from_attributes(
            &player.attributes,
            &player.natural_position,
        )
        .round();
        let target = growth.get(&player.id).copied().unwrap_or(0);
        player.potential = (ovr as i32).max(target).clamp(1, 99) as u8;
        ofm_core::player_rating::refresh_player_derived(player, 2001);
    }
    if let Some(defs) = &data.competition_definitions {
        let errors = ofm_core::generator::validate_definitions_for_world(defs, &data);
        if !errors.is_empty() {
            return Err(format!("league definitions rejected: {errors:?}"));
        }
    }
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Given CM's day-of-year dates, when formatted, then leap years and month ends are right.
    #[test]
    fn cm_dates_become_calendar_dates() {
        assert_eq!(
            iso(CmDate {
                day_of_year: 121,
                year: 1975
            }),
            "1975-05-02"
        );
        assert_eq!(
            iso(CmDate {
                day_of_year: 59,
                year: 2000
            }),
            "2000-02-29"
        );
        assert_eq!(
            iso(CmDate {
                day_of_year: 364,
                year: 2001
            }),
            "2001-12-31"
        );
    }

    /// Given CM nation names, when mapped, then home nations keep their football codes and
    /// renamed nations resolve to the game's catalogue.
    #[test]
    fn nation_names_map_to_game_codes() {
        assert_eq!(nation_code("England", "ENG"), "ENG");
        assert_eq!(nation_code("Holland", "HOL"), "NL");
        assert_eq!(nation_code("France", "FRA"), "FR");
    }

    /// Given two clubs with the same short name, when coded, then both get distinct three-letter codes.
    #[test]
    fn club_codes_stay_unique() {
        let mut used = HashSet::new();
        let a = unique_code("Man Utd", &mut used);
        let b = unique_code("Man City", &mut used);
        assert_eq!(a, "MAN");
        assert_ne!(a, b);
        assert_eq!(b.len(), 3);
    }

    /// Given CM's 1-20 ratings, when scaled, then the ends map inside the game's 0-100 range.
    #[test]
    fn ratings_scale_into_game_range() {
        assert_eq!((scale(1), scale(20), scale(-5)), (3, 98, 3));
    }
}
