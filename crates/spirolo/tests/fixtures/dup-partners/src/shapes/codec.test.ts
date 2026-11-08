import { describe, it, expect } from "vitest"
import { decodeShape, decodeShapes } from "./codec"
import { CodecError } from "../errors"

describe("decodeShape", () => {
  it("reads a known shape", () => {
    expect(decodeShape("circle")).toBe("circle")
  })

  it("rejects a shape it has never heard of", () => {
    expect(() => decodeShape("hexagon")).toThrowError(CodecError)
  })
})

describe("decodeShapes", () => {
  it("reads an empty array", () => {
    expect(decodeShapes("[]")).toEqual([])
  })

  it("rejects an object instead of an array", () => {
    expect(() => decodeShapes("{}")).toThrowError(CodecError)
  })
})

describe("edge cases", () => {
  it("does not mistake an empty list for an error", () => {
    expect(decodeShapes("[]")).toEqual([])
  })

  it("still rejects an unknown shape asked for alone", () => {
    expect(() => decodeShape("hexagon")).toThrowError(CodecError)
  })
})
