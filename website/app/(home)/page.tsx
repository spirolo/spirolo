// The home page: the promise, a replay of real runs, why it matters now, the five questions, the
// health score, the missing layer, what the alpha shows, the questions, and the install again. Every
// number comes from the data/ files of real runs.
import { Closing } from '@/components/landing/closing';
import { DefaultView } from '@/components/landing/default-view';
import { Families } from '@/components/landing/families';
import { HealthScore } from '@/components/landing/health';
import { Hero } from '@/components/landing/hero';
import { LayersBand } from '@/components/landing/layers';
import { Questions } from '@/components/landing/questions';
import { RunReplay } from '@/components/landing/replay';
import { Value } from '@/components/landing/value';
import './home.css';

export default function HomePage() {
  return (
    <main className="flex flex-1 flex-col">
      <Hero />
      <RunReplay />
      <Value />
      <Families />
      <HealthScore />
      <LayersBand />
      <DefaultView />
      <Questions />
      <Closing />
    </main>
  );
}
