//! `cm0102-import <CM3_Data folder> <output.json> [--nations England:4,Spain] [--tiers 2]`
//!
//! Writes a cLLm world JSON that starts the 2001/02 season. Put the file in the game's
//! `databases` folder and pick it on the new-game screen.

use std::path::PathBuf;
use std::process::ExitCode;

use cm0102::{Database, ImportOptions, build_world, finish_world};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("cm0102-import: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: cm0102-import <CM3_Data folder> <output.json> [--nations A,B] [--tiers N] [--free-agents N]";
    let data_dir = PathBuf::from(args.next().ok_or(usage)?);
    let output = PathBuf::from(args.next().ok_or(usage)?);
    let mut options = ImportOptions::default();
    while let Some(flag) = args.next() {
        let value = args.next().ok_or(usage)?;
        match flag.as_str() {
            "--nations" => {
                options.nations = value.split(',').map(|s| s.trim().to_string()).collect()
            }
            "--tiers" => options.max_tiers = value.parse().map_err(|_| usage)?,
            "--free-agents" => options.free_agents = value.parse().map_err(|_| usage)?,
            _ => return Err(usage.to_string()),
        }
    }

    let db = Database::load(&data_dir)?;
    println!(
        "read {} clubs, {} players, {} staff from {}",
        db.clubs.len(),
        db.players.len(),
        db.staff.len(),
        data_dir.display()
    );
    let (world, summary) = build_world(&db, &options)?;
    let world = finish_world(world)?;
    let json = ofm_core::generator::export_world_to_json(&world)?;
    std::fs::write(&output, json).map_err(|e| format!("cannot write {}: {e}", output.display()))?;
    println!(
        "wrote {}: {} leagues, {} clubs, {} players, {} managers",
        output.display(),
        summary.leagues.len(),
        summary.clubs,
        summary.players,
        summary.managers
    );
    for league in &summary.leagues {
        println!("  {league}");
    }
    verify(&output)
}

/// Reload the written file the way the game does and build the 2001/02 competitions, so a world
/// the game would reject fails here instead of on the new-game screen.
fn verify(path: &std::path::Path) -> Result<(), String> {
    let world = ofm_core::generator::load_world_from_path(path)?;
    // Saves store one holder per squad number per club.
    let mut numbers = std::collections::HashSet::new();
    for player in &world.players {
        if let (Some(team), Some(number)) = (&player.team_id, player.jersey_number)
            && !numbers.insert((team.clone(), number))
        {
            return Err(format!("{team} has two players wearing {number}"));
        }
    }
    let definitions = world
        .competition_definitions
        .as_ref()
        .ok_or("the written world has no league definitions")?;
    let start = chrono::DateTime::parse_from_rfc3339("2001-07-01T00:00:00Z")
        .map_err(|e| e.to_string())?
        .with_timezone(&chrono::Utc);
    let competitions = ofm_core::generator::resolve_definitions(definitions, &world, 2001, start);
    let fixtures: usize = competitions.iter().map(|c| c.fixtures.len()).sum();
    if competitions.is_empty() || fixtures == 0 {
        return Err(
            "the game built no competitions or fixtures from the written world".to_string(),
        );
    }
    println!(
        "verified: the game loads it and builds {} competitions with {fixtures} fixtures",
        competitions.len()
    );
    Ok(())
}
