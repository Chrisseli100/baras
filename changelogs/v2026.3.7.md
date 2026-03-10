# Hotfix v2026.3.7001

- Fixed issue with phase ended trigger firing twice on same event, breaking Firebrand and Stormcaller phase logic
- Custom boss creation on default areas now works properly
- Fixed issue with Calphayus burn phase trigger
- Fixed issue with IP-CPT burn phase trigger
- Dynamic background now renders properly when `Stack from Bottom` option is selected for metrics overlays

# v2026.3.7

## General

- The application is no longer susceptible to time drift or clock skew relative to the SWTOR server time
- Replaced timer enable/disable on card header with visibility toggle.
- Separated the ability to enable/disable timers, phases and counters into a separate section with warning
- Added support for NOT keyword in the combat log filter

## Timer System

- Enabled counter to counter comparison condition in timers
- Added trigger type on any counter change
- Added counter mode for tracking effect stacks

## Bugfixes

- Players registered are now reset on area change to prevent them from leaking into the next encounter
- Fixed issue with `Any Phase Entered` trigger not firing for timers
- `Boss HP Below` trigger is now more consistent and sensitive to small HP movements
- Removed duplicate time remaining from effects overlay when in vertical mode
- Fixed issue with cooldown overlay source clipping into other entries
- Removed unsupported target set and time elapsed triggers from phases in UI
- Removed "Training Dummy" invalid area type from encounter builder area creation menu
- Fixed issues causing SCYVA timers chains not to fire
- Adding notes to built-in boss files or creating a new boss on a built-in file will no longer attempt to write to the built-in directory
- Fixed issue causing save/duplicate buttons in the timer editor to shift if a built-in timer is modified
- Fixed issue with overlapping Boss Name and target text on the HP overlay
- `AbilityCast` triggers now respect the target filter
- Non-local player effects should now properly track and refresh
- Users are no longer restricted from navigating away from the parsely upload file modal
- CounterReaches triggers now recognize decrements
- Conditions relying on timer time remaining now evaluate based on latest signal state
- DOT refreshes are now calculated from the first damage event after the ability cast instead of the cast event
