if (process.env.TZ) {
  console.log(`Running tests in ${process.env.TZ} timezone`);
}

import type { Config } from "@jest/types";

const sharedConfig: Config.InitialOptions = {
  clearMocks: true,
  testEnvironment: "jsdom",
  setupFilesAfterEnv: ["<rootDir>/test/setup.ts"],
  transform: {
    "^.+\\.[mc]?[jt]sx?$": [
      "@swc/jest",
      { jsc: { transform: { react: { runtime: "automatic" } } } },
    ],
    "^.+\\.css$": "jest-transform-css",
  },
};

const config: Config.InitialOptions = {
  coverageReporters: ["lcov", "text", "clover"],
  projects: [
    {
      ...sharedConfig,
      displayName: "src",
      roots: ["<rootDir>/packages/react-day-picker/src"],
      moduleNameMapper: {
        "@/test/(.*)": ["<rootDir>/test/$1"],
        "@daypicker/react/locale/(.*)\\.js": [
          "<rootDir>/packages/react-day-picker/src/locale/$1.ts",
        ],
        "@daypicker/react/locale/(.*)": [
          "<rootDir>/packages/react-day-picker/src/locale/$1",
        ],
        "@daypicker/react/locale": ["<rootDir>/packages/react/src/locale.ts"],
        "@daypicker/react/style.module.css": [
          "<rootDir>/packages/react-day-picker/src/style.module.css",
        ],
        "@daypicker/react/style.css": [
          "<rootDir>/packages/react-day-picker/src/style.css",
        ],
        "@daypicker/react": ["<rootDir>/packages/react/src/index.ts"],
        "react-day-picker/locale/(.*)\\.js": [
          "<rootDir>/packages/react-day-picker/src/locale/$1.ts",
        ],
        "react-day-picker/locale/(.*)": [
          "<rootDir>/packages/react-day-picker/src/locale/$1",
        ],
        "react-day-picker/locale": [
          "<rootDir>/packages/react-day-picker/src/locale.ts",
        ],
        "react-day-picker": [
          "<rootDir>/packages/react-day-picker/src/index.ts",
        ],
        "^(\\.\\.?\\/.+)\\.jsx?$": "$1", // see https://github.com/kulshekhar/ts-jest/issues/1057
      },
    },
    {
      ...sharedConfig,
      displayName: "examples",
      roots: ["<rootDir>/examples"],
      testPathIgnorePatterns: ["<rootDir>/examples/timezone/"],
      moduleNameMapper: {
        "@/test/(.*)": ["<rootDir>/test/$1"],
        "@daypicker/buddhist": ["<rootDir>/packages/buddhist/src/index.tsx"],
        "@daypicker/ethiopic": ["<rootDir>/packages/ethiopic/src/index.tsx"],
        "@daypicker/hebrew": ["<rootDir>/packages/hebrew/src/index.tsx"],
        "@daypicker/hijri": ["<rootDir>/packages/hijri/src/index.tsx"],
        "@daypicker/persian": ["<rootDir>/packages/persian/src/index.tsx"],
        "@daypicker/react/locale/(.*)\\.js": [
          "<rootDir>/packages/react-day-picker/src/locale/$1.ts",
        ],
        "@daypicker/react/locale/(.*)": [
          "<rootDir>/packages/react-day-picker/src/locale/$1",
        ],
        "@daypicker/react/locale": ["<rootDir>/packages/react/src/locale.ts"],
        "@daypicker/react/style.module.css": [
          "<rootDir>/packages/react-day-picker/src/style.module.css",
        ],
        "@daypicker/react/style.css": [
          "<rootDir>/packages/react-day-picker/src/style.css",
        ],
        "@daypicker/react": ["<rootDir>/packages/react/src/index.ts"],
        "react-day-picker/locale/(.*)\\.js": [
          "<rootDir>/packages/react-day-picker/src/locale/$1.ts",
        ],
        "react-day-picker/locale/(.*)": [
          "<rootDir>/packages/react-day-picker/src/locale/$1",
        ],
        "react-day-picker/locale": [
          "<rootDir>/packages/react-day-picker/src/locale.ts",
        ],
        "react-day-picker": [
          "<rootDir>/packages/react-day-picker/src/index.ts",
        ],
        "^(\\.\\.?\\/.+)\\.jsx?$": "$1", // see https://github.com/kulshekhar/ts-jest/issues/1057
      },
    },
    {
      ...sharedConfig,
      displayName: "packages",
      roots: [
        "<rootDir>/packages/buddhist",
        "<rootDir>/packages/ethiopic",
        "<rootDir>/packages/hebrew",
        "<rootDir>/packages/hijri",
        "<rootDir>/packages/persian",
        "<rootDir>/packages/react",
      ],
      moduleNameMapper: {
        "@/test/(.*)": ["<rootDir>/test/$1"],
        "@daypicker/buddhist": ["<rootDir>/packages/buddhist/src/index.tsx"],
        "@daypicker/ethiopic": ["<rootDir>/packages/ethiopic/src/index.tsx"],
        "@daypicker/hebrew": ["<rootDir>/packages/hebrew/src/index.tsx"],
        "@daypicker/hijri": ["<rootDir>/packages/hijri/src/index.tsx"],
        "@daypicker/persian": ["<rootDir>/packages/persian/src/index.tsx"],
        "@daypicker/react/locale/(.*)\\.js": [
          "<rootDir>/packages/react-day-picker/src/locale/$1.ts",
        ],
        "@daypicker/react/locale/(.*)": [
          "<rootDir>/packages/react-day-picker/src/locale/$1",
        ],
        "@daypicker/react/locale": ["<rootDir>/packages/react/src/locale.ts"],
        "@daypicker/react/style.module.css": [
          "<rootDir>/packages/react-day-picker/src/style.module.css",
        ],
        "@daypicker/react/style.css": [
          "<rootDir>/packages/react-day-picker/src/style.css",
        ],
        "@daypicker/react": ["<rootDir>/packages/react/src/index.ts"],
        "react-day-picker/locale/(.*)\\.js": [
          "<rootDir>/packages/react-day-picker/src/locale/$1.ts",
        ],
        "react-day-picker/locale/(.*)": [
          "<rootDir>/packages/react-day-picker/src/locale/$1",
        ],
        "react-day-picker/locale": [
          "<rootDir>/packages/react-day-picker/src/locale.ts",
        ],
        "react-day-picker": [
          "<rootDir>/packages/react-day-picker/src/index.ts",
        ],
        "^(\\.\\.?\\/.+)\\.jsx?$": "$1", // see https://github.com/kulshekhar/ts-jest/issues/1057
      },
    },
  ],
};

export default config;
