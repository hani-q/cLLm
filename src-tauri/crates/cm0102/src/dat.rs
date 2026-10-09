//! Readers for the Championship Manager 01/02 pre-game database (`CM3_Data/*.dat`).
//!
//! Every table is a run of fixed-size little-endian records whose id equals its index.
//! `index.dat` says where each table starts inside its file and how many records it holds.
//! The layouts here were measured against the original 3.9.68 data; the record shapes agree
//! with the structures documented by the CM 01/02 community (see `references/README.md`).
//! No code was copied from those projects.

use std::fs;
use std::path::Path;

/// Index entry: an 8-byte header precedes 67-byte entries
/// (51-byte file name, then `u32` table type, count, offset, version).
const INDEX_HEADER: usize = 8;
const INDEX_ENTRY: usize = 67;

const TABLE_STAFF: i32 = 6;
const TABLE_NON_PLAYER: i32 = 9;
const TABLE_PLAYER: i32 = 10;

const NAME_RECORD: usize = 60;
const NATION_RECORD: usize = 290;
const COMP_RECORD: usize = 107;
const CLUB_RECORD: usize = 581;
const STADIUM_RECORD: usize = 78;
const CITY_RECORD: usize = 56;
const COLOUR_RECORD: usize = 58;
const PLAYER_RECORD: usize = 70;
const NON_PLAYER_RECORD: usize = 68;
/// Staff records are 157 bytes in version 1 (the original data, preferences folded in)
/// and 110 bytes in version 2 (databases rewritten by the community editor).
const STAFF_RECORD_V1: usize = 157;
const STAFF_RECORD_V2: usize = 110;
/// A club's squad list holds at most this many staff ids.
pub const SQUAD_SLOTS: usize = 50;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexEntry {
    pub file: String,
    pub table: i32,
    pub count: usize,
    pub offset: usize,
    pub version: i32,
}

/// A calendar date as CM stores it: day of the year (0-based) and year.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CmDate {
    pub day_of_year: i16,
    pub year: i16,
}

impl CmDate {
    /// Year 0 and the 1900 placeholder mean "unknown".
    pub fn is_known(&self) -> bool {
        self.year > 1900 && (0..366).contains(&self.day_of_year)
    }
}

#[derive(Debug, Clone)]
pub struct Nation {
    pub id: i32,
    pub name: String,
    pub three_letter: String,
}

#[derive(Debug, Clone)]
pub struct Competition {
    pub id: i32,
    pub name: String,
    pub short_name: String,
    pub scope: i8,
    pub nation: i32,
    pub reputation: i16,
}

#[derive(Debug, Clone)]
pub struct Club {
    pub id: i32,
    pub name: String,
    pub short_name: String,
    pub nation: i32,
    pub division: i32,
    pub cash: i32,
    pub stadium: i32,
    pub reputation: u16,
    /// Kit colours as `colour.dat` ids: (foreground, background).
    pub home_colours: (i32, i32),
    pub manager: i32,
    pub squad: Vec<i32>,
}

#[derive(Debug, Clone)]
pub struct Stadium {
    pub id: i32,
    pub name: String,
    pub city: i32,
    pub capacity: i32,
}

#[derive(Debug, Clone)]
pub struct Colour {
    pub rgb: (u8, u8, u8),
}

#[derive(Debug, Clone)]
pub struct Staff {
    pub id: i32,
    pub first_name: i32,
    pub second_name: i32,
    pub common_name: i32,
    pub date_of_birth: CmDate,
    pub year_of_birth: u16,
    pub nation: i32,
    pub club: i32,
    pub club_job: u8,
    pub joined_club: CmDate,
    pub contract_expires: CmDate,
    pub wage: i32,
    pub value: i32,
    pub player: i32,
    pub non_player: i32,
}

/// Player abilities. Ability and reputation use CM's own scales (ability 1-200, reputation
/// 1-10000); every attribute and position rating is CM's 1-20 scale.
#[derive(Debug, Clone)]
pub struct PlayerRecord {
    pub id: i32,
    pub squad_number: u8,
    pub current_ability: u16,
    pub potential_ability: i16,
    pub world_reputation: u16,
    /// GK, SW, D, DM, M, AM, F, WB.
    pub positions: [i8; 8],
    /// Right, left, centre.
    pub sides: [i8; 3],
    /// Attributes in file order: see [`PlayerAttr`].
    pub attributes: [i8; 42],
}

/// Positions of the 42 attribute bytes that follow the position ratings in a player record.
#[derive(Debug, Clone, Copy)]
pub enum PlayerAttr {
    Acceleration = 0,
    Aggression,
    Agility,
    Anticipation,
    Balance,
    Bravery,
    Consistency,
    Corners,
    Crossing,
    Decisions,
    Dirtiness,
    Dribbling,
    Finishing,
    Flair,
    FreeKicks,
    Handling,
    Heading,
    ImportantMatches,
    InjuryProneness,
    Jumping,
    Leadership,
    LeftFoot,
    LongShots,
    Marking,
    Movement,
    NaturalFitness,
    OneOnOnes,
    Pace,
    Passing,
    Penalties,
    Positioning,
    Reflexes,
    RightFoot,
    Stamina,
    Strength,
    Tackling,
    Teamwork,
    Technique,
    ThrowIns,
    Versatility,
    Vision,
    WorkRate,
}

impl PlayerRecord {
    pub fn attr(&self, attr: PlayerAttr) -> i8 {
        self.attributes[attr as usize]
    }
}

#[derive(Debug, Clone)]
pub struct NonPlayerRecord {
    pub id: i32,
    pub current_ability: u16,
    pub world_reputation: u16,
}

/// The whole pre-game database, decoded.
#[derive(Debug, Clone)]
pub struct Database {
    pub first_names: Vec<String>,
    pub second_names: Vec<String>,
    pub common_names: Vec<String>,
    pub nations: Vec<Nation>,
    pub competitions: Vec<Competition>,
    pub clubs: Vec<Club>,
    pub stadiums: Vec<Stadium>,
    pub cities: Vec<String>,
    pub colours: Vec<Colour>,
    pub staff: Vec<Staff>,
    pub players: Vec<PlayerRecord>,
    pub non_players: Vec<NonPlayerRecord>,
}

impl Database {
    /// Read every table the importer needs from a `CM3_Data` folder.
    pub fn load(dir: &Path) -> Result<Self, String> {
        let read =
            |name: &str| fs::read(dir.join(name)).map_err(|e| format!("cannot read {name}: {e}"));
        let index = parse_index(&read("index.dat")?)?;
        let staff_bytes = read("staff.dat")?;
        let staff_entry = find_entry(&index, TABLE_STAFF)?;
        let staff_size = match staff_entry.version {
            1 => STAFF_RECORD_V1,
            _ => STAFF_RECORD_V2,
        };
        let player_entry = find_entry(&index, TABLE_PLAYER)?;
        let non_player_entry = find_entry(&index, TABLE_NON_PLAYER)?;

        Ok(Self {
            first_names: parse_names(&read("first_names.dat")?),
            second_names: parse_names(&read("second_names.dat")?),
            common_names: parse_names(&read("common_names.dat")?),
            nations: records(&read("nation.dat")?, NATION_RECORD)
                .map(parse_nation)
                .collect(),
            competitions: records(&read("club_comp.dat")?, COMP_RECORD)
                .map(parse_competition)
                .collect(),
            clubs: records(&read("club.dat")?, CLUB_RECORD)
                .map(parse_club)
                .collect(),
            stadiums: records(&read("stadium.dat")?, STADIUM_RECORD)
                .map(parse_stadium)
                .collect(),
            cities: records(&read("city.dat")?, CITY_RECORD)
                .map(|r| text(&r[4..30]))
                .collect(),
            colours: records(&read("colour.dat")?, COLOUR_RECORD)
                .map(|r| Colour {
                    rgb: (r[55], r[56], r[57]),
                })
                .collect(),
            staff: section(&staff_bytes, staff_entry, staff_size)?
                .map(|r| parse_staff(r, staff_size))
                .collect(),
            players: section(&staff_bytes, player_entry, PLAYER_RECORD)?
                .map(parse_player)
                .collect(),
            non_players: section(&staff_bytes, non_player_entry, NON_PLAYER_RECORD)?
                .map(parse_non_player)
                .collect(),
        })
    }

    /// The display name CM uses: the common name ("Ronaldo") when set, else first and second.
    pub fn person_names(&self, staff: &Staff) -> (String, String, String) {
        let first = lookup(&self.first_names, staff.first_name);
        let second = lookup(&self.second_names, staff.second_name);
        let common = lookup(&self.common_names, staff.common_name);
        (first, second, common)
    }
}

pub fn parse_index(bytes: &[u8]) -> Result<Vec<IndexEntry>, String> {
    if bytes.len() < INDEX_HEADER {
        return Err("index.dat is too short".to_string());
    }
    Ok(bytes[INDEX_HEADER..]
        .chunks_exact(INDEX_ENTRY)
        .map(|e| IndexEntry {
            file: text(&e[..51]),
            table: i32_at(e, 51),
            count: usize::try_from(i32_at(e, 55)).unwrap_or(0),
            offset: usize::try_from(i32_at(e, 59)).unwrap_or(0),
            version: i32_at(e, 63),
        })
        .collect())
}

fn find_entry(index: &[IndexEntry], table: i32) -> Result<&IndexEntry, String> {
    index
        .iter()
        .find(|e| e.table == table)
        .ok_or_else(|| format!("index.dat has no table of type {table}"))
}

fn section<'a>(
    bytes: &'a [u8],
    entry: &IndexEntry,
    size: usize,
) -> Result<impl Iterator<Item = &'a [u8]>, String> {
    let end = entry.offset + entry.count * size;
    let slice = bytes.get(entry.offset..end).ok_or_else(|| {
        format!(
            "{} table {} runs past the end of the file",
            entry.file, entry.table
        )
    })?;
    Ok(slice.chunks_exact(size))
}

fn records(bytes: &[u8], size: usize) -> impl Iterator<Item = &[u8]> {
    bytes.chunks_exact(size)
}

fn parse_names(bytes: &[u8]) -> Vec<String> {
    records(bytes, NAME_RECORD)
        .map(|r| text(&r[..51]))
        .collect()
}

fn parse_nation(r: &[u8]) -> Nation {
    Nation {
        id: i32_at(r, 0),
        name: text(&r[4..55]),
        three_letter: text(&r[83..87]),
    }
}

fn parse_competition(r: &[u8]) -> Competition {
    Competition {
        id: i32_at(r, 0),
        name: text(&r[4..55]),
        short_name: text(&r[56..82]),
        scope: r[87].cast_signed(),
        nation: i32_at(r, 93),
        reputation: i16_at(r, 105),
    }
}

fn parse_club(r: &[u8]) -> Club {
    Club {
        id: i32_at(r, 0),
        name: text(&r[4..55]),
        short_name: text(&r[56..82]),
        nation: i32_at(r, 83),
        division: i32_at(r, 87),
        cash: i32_at(r, 101),
        stadium: i32_at(r, 105),
        reputation: u16_at(r, 128),
        home_colours: (i32_at(r, 131), i32_at(r, 135)),
        manager: i32_at(r, 207),
        squad: (0..SQUAD_SLOTS)
            .map(|i| i32_at(r, 215 + i * 4))
            .filter(|id| *id >= 0)
            .collect(),
    }
}

fn parse_stadium(r: &[u8]) -> Stadium {
    Stadium {
        id: i32_at(r, 0),
        name: text(&r[4..55]),
        city: i32_at(r, 56),
        capacity: i32_at(r, 60),
    }
}

fn parse_staff(r: &[u8], size: usize) -> Staff {
    // The two versions share every field up to 0x61; after that version 1 inlines twelve
    // preference ids before the player and non-player links.
    let (player_at, non_player_at) = if size == STAFF_RECORD_V1 {
        (0x91, 0x99)
    } else {
        (0x61, 0x69)
    };
    Staff {
        id: i32_at(r, 0),
        first_name: i32_at(r, 4),
        second_name: i32_at(r, 8),
        common_name: i32_at(r, 12),
        date_of_birth: date_at(r, 16),
        year_of_birth: u16_at(r, 24),
        nation: i32_at(r, 26),
        club: i32_at(r, 0x39),
        club_job: r[0x3D],
        joined_club: date_at(r, 0x3E),
        contract_expires: date_at(r, 0x46),
        wage: i32_at(r, 0x4E),
        value: i32_at(r, 0x52),
        player: i32_at(r, player_at),
        non_player: i32_at(r, non_player_at),
    }
}

fn parse_player(r: &[u8]) -> PlayerRecord {
    let signed = |i: usize| r[i].cast_signed();
    PlayerRecord {
        id: i32_at(r, 0),
        squad_number: r[4],
        current_ability: u16_at(r, 5),
        potential_ability: i16_at(r, 7),
        world_reputation: u16_at(r, 13),
        positions: std::array::from_fn(|i| signed(0x0F + i)),
        sides: std::array::from_fn(|i| signed(0x17 + i)),
        attributes: std::array::from_fn(|i| signed(0x1B + i)),
    }
}

fn parse_non_player(r: &[u8]) -> NonPlayerRecord {
    NonPlayerRecord {
        id: i32_at(r, 0),
        current_ability: u16_at(r, 4),
        world_reputation: u16_at(r, 12),
    }
}

fn lookup(names: &[String], id: i32) -> String {
    usize::try_from(id)
        .ok()
        .and_then(|i| names.get(i))
        .cloned()
        .unwrap_or_default()
}

/// Strings are NUL-terminated Latin-1.
fn text(bytes: &[u8]) -> String {
    bytes
        .iter()
        .take_while(|b| **b != 0)
        .map(|b| char::from(*b))
        .collect::<String>()
        .trim()
        .to_string()
}

fn date_at(r: &[u8], at: usize) -> CmDate {
    CmDate {
        day_of_year: i16_at(r, at),
        year: i16_at(r, at + 2),
    }
}

fn i32_at(r: &[u8], at: usize) -> i32 {
    i32::from_le_bytes([r[at], r[at + 1], r[at + 2], r[at + 3]])
}

fn i16_at(r: &[u8], at: usize) -> i16 {
    i16::from_le_bytes([r[at], r[at + 1]])
}

fn u16_at(r: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([r[at], r[at + 1]])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put_i32(buf: &mut [u8], at: usize, value: i32) {
        buf[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }

    /// Given an index.dat with one staff entry, when it is parsed, then the file name,
    /// table, count, offset and version come back as written.
    #[test]
    fn index_entries_round_trip_their_fields() {
        let mut bytes = vec![0u8; INDEX_HEADER + INDEX_ENTRY];
        bytes[8..17].copy_from_slice(b"staff.dat");
        put_i32(&mut bytes, 8 + 51, TABLE_PLAYER);
        put_i32(&mut bytes, 8 + 55, 3);
        put_i32(&mut bytes, 8 + 59, 140);
        put_i32(&mut bytes, 8 + 63, 2);

        let index = parse_index(&bytes).expect("parse");

        assert_eq!(
            index,
            vec![IndexEntry {
                file: "staff.dat".into(),
                table: TABLE_PLAYER,
                count: 3,
                offset: 140,
                version: 2
            }]
        );
    }

    /// Given a version-1 staff record, when it is parsed, then the player link is read from
    /// after the inlined preferences rather than from the version-2 position.
    #[test]
    fn version_one_staff_reads_the_player_link_after_inlined_preferences() {
        let mut r = vec![0u8; STAFF_RECORD_V1];
        put_i32(&mut r, 0, 51_514);
        put_i32(&mut r, 0x39, 5_670);
        put_i32(&mut r, 0x61, 5_670); // a favourite club in version 1, the player link in version 2
        put_i32(&mut r, 0x91, 43_125);
        put_i32(&mut r, 0x99, -1);

        let staff = parse_staff(&r, STAFF_RECORD_V1);

        assert_eq!(
            (staff.id, staff.club, staff.player, staff.non_player),
            (51_514, 5_670, 43_125, -1)
        );
    }

    /// Given a player record, when it is parsed, then position ratings and attributes land in
    /// the slots their names say.
    #[test]
    fn player_record_maps_ratings_and_attributes_by_name() {
        let mut r = vec![0u8; PLAYER_RECORD];
        put_i32(&mut r, 0, 7);
        r[5..7].copy_from_slice(&180u16.to_le_bytes());
        r[0x13] = 20; // midfielder
        r[0x17] = 20; // right side
        r[0x1B + PlayerAttr::Crossing as usize] = 20;
        r[0x1B + PlayerAttr::WorkRate as usize] = 18;

        let p = parse_player(&r);

        assert_eq!(
            (p.id, p.current_ability, p.positions[4], p.sides[0]),
            (7, 180, 20, 20)
        );
        assert_eq!(
            (p.attr(PlayerAttr::Crossing), p.attr(PlayerAttr::WorkRate)),
            (20, 18)
        );
    }

    /// Given Latin-1 bytes with trailing garbage after the terminator, when decoded, then only
    /// the text before the NUL is kept and accents survive.
    #[test]
    fn text_stops_at_nul_and_keeps_latin1_accents() {
        assert_eq!(text(b"M\xfcller\0junk"), "Müller");
    }
}
