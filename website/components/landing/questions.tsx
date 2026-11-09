// The seven questions as cards. Nothing is hidden behind a click; run numbers come from ./runs.
import { product } from '@/lib/product';
import { repos, seconds, tests } from './runs';
import { Band, Heading } from './shared';

const faqs = [
  {
    question: `What is ${product.name}?`,
    answer: `${product.name} reads a JavaScript or TypeScript test suite with its configs, setup files, mocks and CI, and points at the tests that pass without proving anything. It is a native binary from npm, Apache 2.0 licensed, in advisory alpha.`,
  },
  {
    question: 'Does it run my tests?',
    answer: `No. It reads the files and never executes a test, so it needs no browser, database or network, only the project installed so imports resolve. It read ${tests.toLocaleString('en-US')} tests across ${repos.length} projects in ${seconds} seconds on Linux x64.`,
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
    answer: `Not until you ask: every finding reports at info and the exit code stays 0. Give a rule warn or error in ${product.config}, or run ratchet once so CI fails only on new findings.`,
  },
];

export function Questions() {
  return (
    <Band ground="muted">
      <Heading title="Questions" />
      <dl className="home-reveal mt-10 grid gap-4 md:grid-cols-2">
        {faqs.map((faq) => (
          <div key={faq.question} className="flex min-w-0 flex-col gap-2 rounded-2xl border border-fd-border p-5 sm:p-6">
            <dt className="font-semibold leading-snug">{faq.question}</dt>
            <dd className="text-sm leading-relaxed text-fd-muted-foreground sm:text-base">{faq.answer}</dd>
          </div>
        ))}
      </dl>
    </Band>
  );
}
