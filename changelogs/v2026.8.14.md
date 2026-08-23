# v2026.8.14

## Improved PvP Support

- Improved match boundary detection for Warzones and Areans. Warzones should now be displayed as a single entry. Arenas are segmented into rounds.
- Friendly/Enemy players are identified and moved into separate tables in the data explorer overview
- Effect/Ability triggers can now have the source and target filtered by friendly and enemy players
- Discipline will now be inferred based on player ability casts. (Fury and Rage currently difficult to disambiguate)

## Audio Settings

- Revamped audio options. TTS fallback can now be disabled globally
- App volume can now scale up to 200
- Added volume/TTS preview buttons in application settings menu

## Other

- Eliminated false positive encounter timeouts caused by players receiving revive immunity while still alive
- Added encounter reset detection for Zorn and Toth and Coratanni
- Fixed HTPS values in data explorer sidebar
- Temporary alacrity buffs are now included in alacrity calculations (local player only)
- Added overlay for tracking interrupt casts (interrupt ability only, leap/charge interrupts not tracked)

# v2026.8.9

## Raid Frame Auto-assignment

Raid frame positions can now be automatically assigned by clicking the icon in the top-right corner of the frame in `Rearrange` mode or by
binding the command to a hotkey in the application settings.

BARAS will download a small (12mb) image recognition model and process a screenshot of the space underneath your raid frames overlay.

Provisional names will be assigned and filled in as the players are detected in the logs.

Supported on **Windows and Linux**.

## Other

- Raid frame overlay is now exempted from snap-to-grid alignment.
- Operations timer will now properly stop when flashpoint/PvP zone is exited
- Minor adjustments to the death recap display
