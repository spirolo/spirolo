import { describe, expect, it, vi } from "vitest";

import { add, BASE } from "./math";

describe("add", () => {
  it("adds", () => {
    expect(add(1, 2)).toBe(3);
    expect(BASE).not.toBe("");
  });

  it.each([
    [1, 1, 2],
    [2, 3, 5],
  ])("adds %i and %i", (a, b, sum) => {
    vi.spyOn(console, "log");
    expect(add(a, b)).toBe(sum);
  });

  it.skip("is not run", () => {
    expect(add(0, 0)).toBeDefined();
  });
});
