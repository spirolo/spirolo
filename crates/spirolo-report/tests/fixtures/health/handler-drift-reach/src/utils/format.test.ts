import { describe, it, expect } from "vitest";

describe("format", () => {
  it("joins a code and a name", () => {
    expect(["ORD-001", "Alpha"].join(" ")).toBe("ORD-001 Alpha");
  });
});
