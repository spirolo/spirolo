// The value band: why this matters now that agents write tests, as six plain points with no
// card chrome (the family cards sit right below). Run numbers come from ./runs; the rest is what
// docs/ci, docs/mcp, docs/configuration and docs/index already state.
import { Band, Heading } from './shared';
import { ofRule, repos, tests } from './runs';

const points = [
  {
    lead: 'Super fast',
    title: 'Keeps up with the volume',
    text: `Written in Rust. It read ${tests.toLocaleString('en-US')} tests across ${repos.length} projects, fast enough to run on every change, by every agent.`,
  },
  {
    lead: '0 tests run',
    title: 'Adds no load to CI',
    text: `It reads files, so it needs no browser, no database and no second test run. The ${ofRule('duplicate-test')} duplicate tests it found are time CI spends twice.`,
  },
  {
    lead: '5 MCP tools',
    title: 'Agents check their own tests',
    text: 'An agent can call the same check while it writes, and fix a weak test before anyone reviews it.',
  },
  {
    lead: `${ofRule('fail-closed-setup')} missing guards`,
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
];

export function Value() {
  return (
    <Band>
      <Heading title="Agents write tests faster than anyone can check them">
        An agent can add hundreds of tests in an afternoon, and they all come back green. A reviewer cannot read them
        all, and CI only reports that they passed. Verification has to keep up with the volume.
      </Heading>
      <ul className="mt-10 grid gap-x-10 gap-y-9 sm:grid-cols-2 lg:grid-cols-3">
        {points.map((point) => (
          <li key={point.title} className="home-reveal min-w-0 border-t border-fd-border pt-4">
            <p className="text-2xl font-semibold tracking-tight text-glow sm:text-3xl">{point.lead}</p>
            <h3 className="mt-2 font-semibold">{point.title}</h3>
            <p className="mt-1.5 text-sm leading-relaxed text-fd-muted-foreground">{point.text}</p>
          </li>
        ))}
      </ul>
    </Band>
  );
}
