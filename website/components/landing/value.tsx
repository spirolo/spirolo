// The value band: why this matters now that agents write tests, as six plain points with no
// card chrome (the family cards sit right below). Run numbers come from ./runs; the rest is what
// docs/ci, docs/mcp, docs/configuration and docs/index already state.
import { fill, type LandingCopy } from './copy/types';
import { ofRule, repos, tests } from './runs';
import { Band, Heading } from './shared';

export function Value({ copy }: { copy: LandingCopy }) {
  const values = {
    tests: tests.toLocaleString(copy.numbers),
    repos: repos.length,
    duplicates: ofRule('duplicate-test'),
    guards: ofRule('fail-closed-setup'),
  };
  return (
    <Band>
      <Heading title={copy.value.title}>{copy.value.intro}</Heading>
      <ul className="mt-10 grid gap-x-10 gap-y-9 sm:grid-cols-2 lg:grid-cols-3">
        {copy.value.points.map((point) => (
          <li key={point.title} className="home-reveal min-w-0 border-t border-fd-border pt-4">
            <p className="text-2xl font-semibold tracking-tight text-glow sm:text-3xl">{fill(point.lead, values)}</p>
            <h3 className="mt-2 font-semibold">{point.title}</h3>
            <p className="mt-1.5 text-sm leading-relaxed text-fd-muted-foreground">{fill(point.text, values)}</p>
          </li>
        ))}
      </ul>
    </Band>
  );
}
