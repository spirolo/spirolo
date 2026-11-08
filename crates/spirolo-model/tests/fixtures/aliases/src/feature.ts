import { BASE_URL, slugify } from "@/util";
import { helper } from "~/helper";

export function feature(name: string): string {
  return `${BASE_URL}/${slugify(name)}#${helper()}`;
}
