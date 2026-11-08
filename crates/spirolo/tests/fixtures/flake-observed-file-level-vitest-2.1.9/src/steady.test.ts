import { expect, test } from "vitest";

test("is always green", () => {
  expect(1).toBe(1);
});

test.skip("is parked", () => {
  expect(1).toBe(2);
});
