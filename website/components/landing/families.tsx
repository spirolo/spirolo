// The five families as a tile grid. Words and frame are in ./tile-grid, pictures in
// ./tile-art; counts come from ./runs.
import Link from 'next/link';
import { rules } from '@/lib/rules';
import { Band, Heading } from './shared';
import { tileArt } from './tile-art';
import { TileGrid } from './tile-grid';

export function Families() {
  return (
    <Band ground="muted">
      <Heading title="Five questions a green suite does not answer">
        Every rule carries a public score; all {rules.length} are on{' '}
        <Link href="/docs/rules" className="text-glow hover:underline">
          the rules page
        </Link>
        .
      </Heading>
      <TileGrid art={tileArt} />
    </Band>
  );
}
