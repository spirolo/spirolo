import { double, NAME } from "@scope/pkg-b";

import { local } from "./local";

export function quadruple(value: number): number {
  return double(double(value)) + local + NAME.length;
}
