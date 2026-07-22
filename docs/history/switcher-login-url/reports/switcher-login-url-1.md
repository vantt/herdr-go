# switcher-login-url-1

[DONE] Switcher and login now have their own bookmarkable `/switcher` and `/login` URLs, symmetric with PBI-025's `/terminal/<pane_id>`; `/` canonicalizes silently; D7's login-transition replaceState exception closes the review-flagged Back-button gap.

**Files touched:** `web/src/main.ts`, `web/test/main.test.ts`

**Verify:** `cd web && npm run typecheck && npm run test -- --run` — passed (tsc clean; vitest 79/79, main.test.ts 28/28).

**Commit:** `cff01ad` — feat(switcher-login-url-1): give switcher and login their own bookmarkable URLs

Full trace and evidence: `.bee/cells/switcher-login-url-1.json`
