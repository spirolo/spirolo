const { greet } = require("../greet");

test("greets by name", () => {
  expect(greet("ada")).toBe("hello ada");
});
