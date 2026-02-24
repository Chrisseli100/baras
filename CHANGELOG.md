# v2026.2.9000

## Encounter Classification

- Non-battle revives of the local player will now automatically end the combat encounter
- Encounters ended by exiting the Area are now classified as wipes unless there is a known special condition (e.g. TFB excluded)
- Healing actions are ignored when evaluating fallback encounter timeout

## Performance

- Cache data-explorer icons to reduce memory pressure from frequent lookups

## Timers

- Apex Vanguard Mass Target Lock P4 timer no longer fires on combat start
