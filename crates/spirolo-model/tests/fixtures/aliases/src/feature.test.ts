import { describe, expect, it } from "vitest";

import { feature } from "@/feature";

describe("feature", () => {
  it("builds a url", () => {
    expect(feature("Hi")).toContain("example.test");
  });
});
