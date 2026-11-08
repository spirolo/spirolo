import { expect, test } from "vitest";

// The module throws at load when SPIROLO_FLIP is set: vitest reports the file failed, no tests.
if (process.env.SPIROLO_FLIP) throw new Error("module flaked at load");

test("imports cleanly", () => {
  expect(1).toBe(1);
});
