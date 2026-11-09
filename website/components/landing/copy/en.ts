// The home page in English, the source every other language translates.
import type { LandingCopy } from './types';

export const en: LandingCopy = {
  lang: 'en',
  numbers: 'en-US',
  meta: {
    title: '{product}: the missing layer of your test suite',
    description:
      'A passing suite tells you the tests ran. {product} reads the whole suite, with its setup files and CI, and shows where a pass proves nothing. It never runs your tests.',
  },
  nav: { docs: 'Docs', language: { text: '简体中文', url: '/zh' } },
  footer: { licence: 'Apache 2.0 licence', docs: 'Docs', rules: 'Rules', source: 'Source' },
  install: { copy: 'Copy', copied: 'Copied', copyCommand: 'Copy {command}' },
  stickers: {
    bugInAJar: 'Spirolo sprints past with a grumpy beetle caught in a jar.',
    cleanRun: 'Spirolo plants a flag on a hill of passing test cards, arms crossed.',
  },
  hero: {
    eyebrow: 'Advisory alpha · JavaScript and TypeScript test suites',
    title: 'The missing layer of your test suite.',
    lead: 'A passing suite tells you the tests ran. {product} reads the whole suite, with its setup files and CI, and shows where a pass proves nothing. It never runs your tests.',
    link: 'Install and first run',
  },
  replay: {
    findings: 'findings',
    total: '{findings} findings across {repos} open-source projects.',
    read: '{tests} tests read in about {seconds} seconds.',
    kinds: {
      'absence-anchor': 'weak tests',
      'fail-closed-setup': 'missing guards',
      'dead-skip': 'skips nobody tracks',
      'duplicate-test': 'duplicate tests',
    },
    table: { project: 'Project', tests: 'Tests', findings: 'Findings', readIn: 'Read in' },
    caption:
      'Replayed at real speed: full runs of the alpha, start to exit, median of seven on one Linux machine at pinned commits. Findings are advisory, not confirmed bugs.',
  },
  value: {
    title: 'Agents write tests faster than anyone can check them',
    intro:
      'An agent can add hundreds of tests in an afternoon, and they all come back green. A reviewer cannot read them all, and CI only reports that they passed. Verification has to keep up with the volume.',
    points: [
      {
        lead: 'Super fast',
        title: 'Keeps up with the volume',
        text: 'Written in Rust. It read {tests} tests across {repos} projects, fast enough to run on every change, by every agent.',
      },
      {
        lead: '0 tests run',
        title: 'Adds no load to CI',
        text: 'It reads files, so it needs no browser, no database and no second test run. The {duplicates} duplicate tests it found are time CI spends twice.',
      },
      {
        lead: '5 MCP tools',
        title: 'Agents check their own tests',
        text: 'An agent can call the same check while it writes, and fix a weak test before anyone reviews it.',
      },
      {
        lead: '{guards} missing guards',
        title: 'Green means something again',
        text: 'Setups with no guard against a console error or a leaked mock, so a test can pass when it should fail. When nobody reads every test, the merge gate has to hold.',
      },
      {
        lead: 'New findings only',
        title: 'Holds the line',
        text: 'Ratchet once and CI fails only on what is new. No cleanup project first, and every new batch of tests meets the same bar.',
      },
      {
        lead: 'Deterministic',
        title: 'A second opinion that is not an LLM',
        text: 'Same code, same report, every run. No model and no network: your code stays on your machine, and the tests are not checked by the same kind of tool that wrote them.',
      },
    ],
  },
  families: {
    title: 'Five questions a green suite does not answer',
    intro: 'Every rule carries a public score; all {count} are on ',
    link: 'the rules page',
    after: '.',
    tiles: {
      audit: {
        title: 'Weak tests',
        question: 'Would this test notice if the code were wrong?',
        strong: 'notice if the code were wrong',
        stat: '{count} weak tests',
      },
      gate: {
        title: 'Merge gate',
        question: 'Does a broken test actually block the merge?',
        strong: 'actually block the merge',
        stat: '{count} missing guards',
      },
      hygiene: {
        title: 'Unused tests',
        question: 'Which tests add nothing the suite does not already check?',
        strong: 'add nothing',
        stat: '{count} duplicate tests',
      },
      flake: {
        title: 'Flaky',
        question: 'Which tests can pass on one run and fail on the next?',
        strong: 'pass on one run and fail on the next',
        stat: 'Experimental',
      },
      impact: {
        title: 'Health',
        question: 'Where does a new test pay first?',
        strong: 'a new test pay first',
        stat: 'Experimental',
      },
    },
  },
  health: {
    title: 'One number to watch, not to gate on',
    text: 'Every report opens with a provisional health score out of 100. Each family takes off points for the share of its subjects that have findings, up to a cap per family, and rules outside the default view count half. It is a summary to watch over time: nothing gates on it, and the machine formats do not carry it.',
    link: 'Reading a report',
    gradeLabel: '{grade} grade',
    outOf: 'of 100',
  },
  layers: {
    title: 'Green is not the same as tested',
    intro:
      'A passing suite rests on four layers. Most teams check the first and believe they have the rest. Two of them have no tool. Together they are the missing layer, and that is where {product} looks.',
    checkedBy: 'Checked by {product}',
    layers: [
      {
        name: 'Mistakes inside a test file',
        job: 'A test with no assertion, a focused test, an unawaited query.',
        who: 'Checked by ESLint test plugins',
      },
      {
        name: 'The setup every test runs on',
        job: 'Whether a console error, a leaked mock or an unmatched request fails the test, or quietly lets it pass.',
        who: 'no other tool checks it',
      },
      {
        name: 'Whether the assertions catch a bug',
        job: 'Mutation testing plants small bugs and reruns the suite to see whether a test notices. Thorough, and slow on a big suite: one run per planted bug.',
        who: 'Checked by Stryker',
      },
      {
        name: 'How the tests connect to the code',
        job: 'Mocks that match no real export, absence checks with nothing proving the render happened, skips nobody tracks, gates that never gate.',
        who: 'no other tool checks it',
      },
    ],
  },
  defaultView: {
    title: 'This is an alpha. More rules are coming soon.',
    intro:
      'No rule is proven yet, so the default view stays cautious: a rule shows only once at least 15 of its sampled findings were decided and at least 80% were true. The others still run, and --all shows them, labelled. ',
    link: 'How rules are measured',
    after: '.',
    viewLabel: 'View',
    tierHeading: {
      proven: 'Proven: verdict PASS',
      advisory: 'Advisory: at least 15 decided, at least 80% true',
      experimental: 'Experimental: every other rule that is not FAIL or DROPPED',
      hidden: 'Hidden: verdict FAIL or DROPPED',
    },
    tierName: { proven: 'proven', advisory: 'advisory', experimental: 'experimental', hidden: 'hidden' },
    none: 'none',
    folded: '{count} rules run but are not shown: {groups}. ',
    foldedGroup: '{count} {tier}',
    foldedJoin: ', ',
    foldedTail: ' shows them, each labelled.',
  },
  questions: {
    title: 'Questions',
    faqs: [
      {
        question: 'What is {product}?',
        answer:
          '{product} reads a JavaScript or TypeScript test suite with its configs, setup files, mocks and CI, and points at the tests that pass without proving anything. It is a native binary from npm, Apache 2.0 licensed, in advisory alpha.',
      },
      {
        question: 'Does it run my tests?',
        answer:
          'No. It reads the files and never executes a test, so it needs no browser, database or network, only the project installed so imports resolve. It read {tests} tests across {repos} projects in {seconds} seconds on Linux x64.',
      },
      {
        question: 'How is it different from the ESLint test plugins?',
        answer:
          'ESLint sees one file at a time, and most of the ways a suite lies sit between files: a setup that swallows console errors, a mock that matches no export, a skip nobody tracks. Keep ESLint for the inside of a file; Spirolo reads the wiring.',
      },
      {
        question: 'How is it different from mutation testing?',
        answer:
          'Stryker plants small bugs and reruns the suite once per bug, which is thorough and slow on a big suite. Spirolo reads once and points at the structure that lets a bug through, and it can read a Stryker report to find tests that kill the same mutants.',
      },
      {
        question: 'Which suites does it read?',
        answer:
          'Jest, Vitest and Playwright, with Testing Library, MSW and nock, on Linux and macOS (x64 and arm64). The rules are measured on React and plain Node or TypeScript projects; Cypress, Vue and Windows are outside the alpha.',
      },
      {
        question: 'What about false positives?',
        answer:
          'A rule shows by default only once at least 15 of its sampled findings were decided and at least 80% were true, and that score is public. A wrong finding, reported with the file and the line, is how a rule gets better.',
      },
      {
        question: 'Can it fail my build?',
        answer:
          'Not until you ask: every finding reports at info and the exit code stays 0. Give a rule warn or error in {config}, or run ratchet once so CI fails only on new findings.',
      },
    ],
  },
  closing: {
    title: 'What would {product} find in your test suite?',
    text: 'Your tests are the last check before production. In {repos} popular open-source projects it found {findings} findings, each project read in seconds. Install your project’s dependencies first, then run it from the root.',
    link: 'Install and first run',
  },
};
