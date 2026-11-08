import { CodecError } from "../errors"

export type Shape = "circle" | "square"

export function decodeShape(raw: string): Shape {
  if (raw === "circle" || raw === "square") return raw
  throw new CodecError(`unknown shape: ${raw}`)
}

export function decodeShapes(raw: string): Shape[] {
  const items: unknown = JSON.parse(raw)
  if (!Array.isArray(items)) throw new CodecError("expected an array")
  return items.map((item) => decodeShape(String(item)))
}
