# cLLm TODO

Work queued for cLLm, roughly in order. Fork rules live in the `cLLm` section of
[AGENTS.md](AGENTS.md).

## 1. CM 01/02 database importer

Turn a player's own CM 01/02 install into a cLLm world that starts in the 2001/02 season.

- [x] `src-tauri/crates/cm0102`: `cm0102-import <CM3_Data> <world.json>` writes a JSON world
      (2001/02 snapshot, real leagues, cups, European Cup) and reloads it through the game's loader.
- [x] New-game screen lists JSON worlds from the `databases` folder.
- [ ] Wages and money: CM pounds are converted to euros at 1.6; check they feel right in play.
- [ ] Player ratings come from attributes, so some stars rate lower than CM's ability says
      (Beckham 71, Totti 95). Consider blending in CM current ability.
- [x] Read `index.dat`: an 8-byte header, then 22 entries of 67 bytes (51-byte file name, then
      `u32` id, count, offset, version, little-endian).
- [x] Read the record tables. Sizes measured on the original 3.9.68 data:

  | File / section | Records | Bytes each |
  |---|---|---|
  | `staff.dat` people | 132,722 | 157 |
  | `staff.dat` players | 109,940 | 70 |
  | `staff.dat` non-players | 23,785 | 68 |
  | `club.dat` | 10,580 | 581 |
  | `nat_club.dat` | 426 | 581 |
  | `nation.dat` | 213 | 290 |
  | `stadium.dat` | 7,099 | 78 |
  | `officials.dat` | 3,124 | 43 |
  | `club_comp.dat` | 390 | 107 |
  | `first_names.dat`, `second_names.dat`, `common_names.dat` | 34,363 / 82,338 / 8,493 | 60 |

- [ ] Handle both `staff.dat` layouts: the original 3.9.68 data has no separate preferences
      section; databases saved by the community editor split it into four sections.
- [ ] Map CM attributes, positions, contracts, clubs, leagues and competitions onto the OFM
      world schema. List every CM field that has no home in the schema.
- [ ] Tests: synthetic `.dat` fixtures in the repo; real-data tests read `CLLM_CM0102_DATA`
      and skip when it is unset.
- [ ] Test against a community data update from champman0102.co.uk as well as the original.
- [ ] First-run flow: ask the player for their CM 01/02 folder and import it.
- [ ] Format references (read only, no code copied): `nckstwrt/CM0102Patcher`
      (`History Editor/HistoryLoader.cs`), `CoopApps/0201CM`, `agevak/CM0102`. MIT and
      reusable: `Russta/cm0102-regen-notes`.

## 2. Match engine

- [ ] Put a `MatchEngine` interface at the `ofm_core/turn` bridge, with the current OFM
      engine as the default implementation.
- [ ] Compare engines on numbers with `sim-bench`: goals, shots and possession per match.
- [ ] Prototype an adapter for the `ZOXEXIVO/open-football` engine (Apache-2.0).
- [ ] Watch upstream's engine overhaul (OFM #571, release 0.6.0) and its known defects
      (#347 role modifiers, #607 match ratings never computed).

## 3. Commentary

- [ ] Template commentary baseline in the Bygfoot style: each match event picks from weighted
      phrases with conditions and tokens (Bygfoot is GPL-2.0-or-later, so reusable).
- [ ] Study CM's `events_*.cfg` format for structure. Its text is copyrighted and stays out of
      the repo.

## 4. LLM layer

- [ ] LLM commentary generated from the match event stream, with the templates as fallback.
- [ ] LLM rival managers, board, press conferences and player talks through the MCP server
      (`--features mcp`).
- [ ] Local model support alongside hosted models.

## Housekeeping

- [ ] Merge `upstream/develop` regularly.
- [ ] CI workflows in `.github/workflows/` still trigger on `develop`, so none run on `main`.
      Decide which to keep; the nightly and release ones publish builds and need secrets.
- [ ] Discord button still opens Openfoot Manager's Discord (`src/lib/communityLinks.ts`).
- [ ] Settings still shows "Sturdy Robot" as publisher (`app.publisher` in every locale).
- [ ] Old Openfoot Manager logo files in `public/` and `images/` are unused.
- [ ] `docs/BETA_TESTING_GUIDE_*.md` still give the old `com.sturdyrobot.openfootmanager`
      data paths.
- [ ] `src/pages/Dashboard.test.tsx` "clears a stale active save id…" fails on upstream too.
- [ ] Ask the `CoopApps/0201CM` author whether they will add an open source license.
- [ ] The desktop app needs WebKitGTK and a display; on a headless machine use the crates,
      `ofm-cli` and `sim-bench`.
