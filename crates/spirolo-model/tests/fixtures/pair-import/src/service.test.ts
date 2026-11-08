import { describe, expect, it } from "vitest";

import { service } from "./service";

describe("service", () => {
  it("works", () => {
    expect(service()).toBe("ok");
  });
});
