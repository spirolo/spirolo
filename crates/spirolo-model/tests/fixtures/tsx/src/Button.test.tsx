import { expect, it } from "vitest";

import { Button } from "./Button";

it("renders a label", () => {
  expect(<Button label="ok" />).toBeTruthy();
});
