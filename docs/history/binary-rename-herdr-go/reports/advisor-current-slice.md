# Advisor consult: binary identity rename to herdr-go

Advisor resolution: unconfigured for Codex runtime.

Evidence bundle: user explicitly requested a full no-compatibility rename before
first release; decision 178345a6 records the retired `herdctl` identity; plan
and validation report bound the one-cell slice; schedule has one wave and zero
cycles.

Recommendation: proceed with the atomic rename slice. Keep no alias/fallback,
run focused active-surface name checks plus full verify, and leave historical
evidence untouched.
