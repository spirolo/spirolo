import { expect, it } from "vitest";

import { helper } from "./helper";
import { service } from "./service";

it("uses both", () => {
  expect(service()).toBe(helper());
});
