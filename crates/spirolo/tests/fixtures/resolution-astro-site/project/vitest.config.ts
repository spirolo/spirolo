import { defineConfig } from 'vitest/config';

// Assertions about the content helpers, run without a browser.
export default defineConfig({
  test: {
    include: ['tests/{build,content}/**/*.test.ts'],
    environment: 'node',
  },
});
