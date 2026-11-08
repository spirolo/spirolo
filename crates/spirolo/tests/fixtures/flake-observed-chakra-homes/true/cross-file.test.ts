import { describe, expect, it } from "vitest"
import colorPalette from "../src/transforms/props/color-palette"
import { findCrossFileReExports } from "../src/utils/chakra-tracker"
import { applyTransformFiles, createTestProject } from "./test-utils"

describe("findCrossFileReExports", () => {
  it("reports the file location, name, and source of each resolved re-export", () => {
    const project = createTestProject()
    project.createSourceFile(
      "ui.ts",
      `export { Button } from '@chakra-ui/react'\n`,
    )
    const app = project.createSourceFile(
      "app.tsx",
      `import { Button } from './ui'\nconst a = <Button />\n`,
    )
    expect(findCrossFileReExports(app)).toEqual([
      { name: "Button", from: "./ui", line: 1 },
    ])
  })
})

describe("cross-file barrel resolution (prop transforms)", () => {
