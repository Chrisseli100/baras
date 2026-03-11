# v2026.3.10000

Many timer definitions are being refined and updated. This is an incremental process and it will take time to verify every one is accurate. Please report any timer issues on discord.

Expect timer updates to be the main focus of coming patches.

## General

- Replaced Shielding Given overlay with HTPS overlay
- Added a "pushes at" field to the entity definition allowing the health bar to be hidden at non-zero values
- Added option to disable bosses
- Added a "Final Boss" flag that will stop the operation timer once the final boss is killed
- Added group size field for timers
- Added ability to define different shield HP values for 8/16 man
- Added hotkey for returning to live file parsing

## UI

- Reformatted encounter selection sidebar
- Added burst healing taken to HP tracking chart
- Improved formatting of the damage taken summary block
- Added icons to overlay buttons
- Disabled elements will now show as greyed out

## Definitions

- Added/Corrected boss definitions for Objective Meridian (Republic), Spirit of Vengeance, Shrine of Silence, Manaan, Blood Hunt, Battle of Rishi, and Legacy of the Rakata flashpoints
- Updated timers for Sword Squadron, Underlurker
- Updated boss timers/definitions for R4
- Updated boss timers for SCYVA
- Updated some boss definitions/timers for EV/KP

Thank you Wolfy, Advieser, and Keetsune for contributing to the timers.

## Bugfixes

- Fixed issue causing multiple bosses to load in if names were the same
- Corrected effect uptime query when time range is filtered
- Negative threat values now display in the combat log
- Operations timer can no longer be triggered when in historical mode
- Fixed issue causing effects to flash to full duration on raid frames before expiring
