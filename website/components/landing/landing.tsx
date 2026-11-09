// The home page: the promise, a replay of real runs, why it matters now, the five questions, the
// health score, the missing layer, what the alpha shows, the questions, and the install again. Every
// number comes from the data/ files of real runs; every word from the LandingCopy it is given, so
// each language renders the same page.
import { Closing } from './closing';
import type { LandingCopy } from './copy/types';
import { DefaultView } from './default-view';
import { Families } from './families';
import { HealthScore } from './health';
import { Hero } from './hero';
import { LayersBand } from './layers';
import { Questions } from './questions';
import { RunReplay } from './replay';
import { Value } from './value';

export function Landing({ copy }: { copy: LandingCopy }) {
  return (
    <main className="flex flex-1 flex-col">
      <Hero copy={copy} />
      <RunReplay copy={{ ...copy.replay, numbers: copy.numbers }} />
      <Value copy={copy} />
      <Families copy={copy} />
      <HealthScore copy={copy.health} />
      <LayersBand copy={copy.layers} />
      <DefaultView copy={copy.defaultView} families={copy.families.tiles} />
      <Questions copy={copy} />
      <Closing copy={copy} />
    </main>
  );
}
