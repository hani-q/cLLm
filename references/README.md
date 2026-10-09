# References

Local clones of projects cLLm studies. Only this file is committed; the commands at the bottom clone the rest. Licenses differ: code from a project without a
license is reference only (see the `cLLm` section of AGENTS.md).

| Folder | Source | Language | License | Why |
|---|---|---|---|---|
| Player12 | https://github.com/FlavioMili/Player12 | C++ | GPL-3.0 | Open source football management game |
| open-football | https://github.com/ZOXEXIVO/open-football | Rust | Apache-2.0 | Match engine and world simulation |
| open-football-database | https://github.com/ZOXEXIVO/open-football-database | Rust/data | none | Schema reference only |
| bygfoot | https://gitlab.com/bygfoot/bygfoot | C | GPL-2.0-or-later | Management loop, template commentary |
| footballSimulationEngine | https://github.com/GallagherAiden/footballSimulationEngine | JS | MIT | Small match engine |
| Cm93 | https://github.com/lifebeyondfife/Cm93 | C# | GPL-3.0 | Early-CM style manager |
| fbsim | https://github.com/IanTayler/fbsim | Rust | MIT | Small football sim |
| football | https://github.com/google-research/football | C++/Python | Apache-2.0 | Physics match sim (archived) |
| 0201CM | https://github.com/CoopApps/0201CM | Rust | none | CM 01/02 reimplementation, reference only |
| CM0102Patcher | https://github.com/nckstwrt/CM0102Patcher | C# | none | CM 01/02 record layouts, reference only |
| CM0102 | https://github.com/agevak/CM0102 | C# | none | CM 01/02 database tools, reference only |
| cm0102-regen-notes | https://github.com/Russta/cm0102-regen-notes | Python | MIT | CM 01/02 save format notes |
| TransferTool | https://github.com/archibalduk/TransferTool | C++ | none | CM 01/02 .dat editing, reference only |
| cm-scout-next | https://github.com/billytalbutt/cm-scout-next | TypeScript | none | CM 01/02 scouting reader, reference only |

Clone them all with:

```bash
cd references
for r in FlavioMili/Player12 ZOXEXIVO/open-football ZOXEXIVO/open-football-database \
  GallagherAiden/footballSimulationEngine lifebeyondfife/Cm93 IanTayler/fbsim google-research/football \
  CoopApps/0201CM nckstwrt/CM0102Patcher agevak/CM0102 Russta/cm0102-regen-notes \
  archibalduk/TransferTool billytalbutt/cm-scout-next; do
  git clone --depth 1 "https://github.com/$r.git"
done
git clone --depth 1 https://gitlab.com/bygfoot/bygfoot.git
```
