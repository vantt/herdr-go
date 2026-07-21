# Context: agent presets in config + doctor

Slice 3 of `new-shell-new-agent`, run as its own lane. The mobile sheet will show
one action row per configured agent preset; this slice is where those presets
come from and how an operator edits them. The mobile UI only renders the list
(D4 of the parent feature).

## The open question, decided

`plan.md:88` left slice 3 to decide: **is an empty `argv` a config-load failure,
or a preset that renders disabled?**

**Decision: a load failure.** Reasons, in order of weight:

1. **The frozen plan's own exit criterion already says so** — "a malformed `argv`
   is rejected at config load, not at agent start". Slice 2 made
   `InvalidAgentArgv` reachable so either was implementable; this is choosing the
   one the plan wrote down.
2. **It matches the posture the config layer already takes everywhere else.** An
   unknown key is a hard error, an empty `allowed_roots` fails closed rather than
   meaning "allow everything", and a token placed in the config document is
   refused outright (`src/config/mod.rs:1-14`). A preset that silently renders
   disabled would be the first place this file shrugs.
3. **A disabled row is a failure nobody sees.** The operator's next contact with
   it is a row that does nothing, on a phone, away from the machine — the worst
   possible place to discover a typo made at a desk.

The counter-argument was taken seriously: a bad preset then stops the gateway
booting, and the gateway is how the operator reaches everything. It is
outweighed because **the recovery path already exists and is first class** —
`doctor` diagnoses an invalid config, backs it up, and repairs it field by field
(`src/config/write.rs:85-94,150-190`), and this slice's editor refuses to save an
invalid preset in the first place. A config that cannot boot is loud, recoverable
and already tooled for; a preset that quietly does nothing is neither.

## Locked decisions

| # | Decision |
|---|---|
| P1 | `agent_presets` is a list of `{ label, argv }`. Nothing else per entry — no icon, no cwd, no env. The mobile sheet shows the label and the port runs the argv. |
| P2 | The field defaults to an empty list. A gateway with no presets is normal and boots fine; the sheet simply offers `Shell` alone. This also keeps the two hand-written default-config literals (`config/mod.rs:759-764`, `doctor/checks.rs:790-793`) untouched. |
| P3 | A preset is invalid when its label is empty, its `argv` is empty, or its first `argv` element is empty. Any invalid preset fails config load, naming the offending entry by index and label. |
| P4 | Labels must be unique. Two rows reading the same word in the sheet, doing different things, is a defect the operator cannot diagnose from the phone. |
| P5 | The doctor editor supports **listing, adding and removing** a preset. Editing an entry in place is add-then-remove; a per-field editor for a list of structs is more machinery than the operator needs today. |
| P6 | Nothing here validates that the `argv` names a real program. The gateway does not know what is installed on the desktop's PATH, and herdr reports a spawn failure at start time (`agent_start_failed`) which the port already surfaces. Claiming to check would be theatre. |

## Where a new config field has to be touched

Established from the existing `allowed_roots` field (the only other list-valued
setting). Missing any one of these is the ordinary way a config field ends up
half-supported:

1. `Config` struct — `src/config/mod.rs:27-41`
2. `RawConfig` + its serde default — `src/config/mod.rs:90-106`
3. Validation and construction in `Config::load_str` — `src/config/mod.rs:164-211`
4. `CONFIG_FIELDS` allowlist — `src/config/write.rs:16-25` (drives the doctor menu
   **and** what any editor-driven write is allowed to keep; a field missing here
   is silently dropped on every save)
5. Per-field validator in `invalid_field_names` — `src/config/write.rs:192-218`
6. Doctor's input coercion `field_json_value` — `src/doctor/checks.rs:769-777`
7. The runtime consumer — here, slice 4's endpoint

The two hand-written default-config JSON literals (7th and 8th sites for a
required field) are deliberately **not** touched, because P2 makes the field
optional with an empty default.

## Precedent to follow, and where it runs out

`allowed_roots` gives the shape for a list-valued setting end to end: validator,
doctor list-editor (`src/doctor/edit.rs:127-159`), append-one prompt, and the
menu's by-name dispatch (`src/doctor/edit.rs:93-95`).

It runs out at the element type: `allowed_roots` is a list of strings, and
`agent_presets` is a list of two-field objects. The add prompt therefore asks two
questions instead of one, and removal needs an index or label to name the entry.
No existing code does that; it is new, small, and belongs in the same file.

## Known trap

`repair_fields` rebuilds the config document from the `CONFIG_FIELDS` allowlist
only (`src/config/write.rs:165-171`). If `agent_presets` is added to the struct
but not to that list, every doctor-driven save silently deletes the operator's
presets. This is the single most expensive mistake available in this slice.
