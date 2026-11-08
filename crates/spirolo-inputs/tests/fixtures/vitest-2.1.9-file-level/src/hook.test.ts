import { beforeAll, expect, test } from "vitest";

// Its setup throws when SPIROLO_FLIP is set: vitest reports the test skipped and the file failed.
beforeAll(() => {
  if (process.env.SPIROLO_FLIP) throw new Error("setup flaked");
});

test("reads the seeded rows", () => {
  expect(1).toBe(1);
});
