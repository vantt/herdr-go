import { describe, it, expect } from "vitest";
import { appVersion } from "../src/version";

describe("appVersion", () => {
  it("returns a non-empty semver-ish string", () => {
    expect(appVersion()).toMatch(/^\d+\.\d+\.\d+$/);
  });
});
