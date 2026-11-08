import { describe, expect, test } from "vitest"
import transform from "../src/transforms/props/as-props"
import { applyTransform } from "./test-utils"

describe("as-props transform", () => {
  describe("Text component with simple DOM elements", () => {
    test("keeps Text as='p' without asChild", async () => {
      const input = `
import { Text } from '@chakra-ui/react'

export default function App() {
  return <Text as='p' fontSize='md'>Paragraph text</Text>
}
      `

      const output = await applyTransform(transform, input)
      expect(output).toMatchInlineSnapshot(`
        "import { Text } from '@chakra-ui/react'

        export default function App() {
          return (
            <Text as="p" fontSize="md">
              Paragraph text
            </Text>
          )
        }
        "
      `)
    })
  })
})
