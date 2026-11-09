// The five families as a tile grid. Words and frame are in ./tile-grid, pictures in
// ./tile-art; counts come from ./runs.
import Link from 'next/link';
import { rules } from '@/lib/rules';
import { fill, type LandingCopy } from './copy/types';
import { Band, Heading } from './shared';
import { tileArt } from './tile-art';
import { TileGrid } from './tile-grid';

export function Families({ copy }: { copy: LandingCopy }) {
  return (
    <Band ground="muted">
      <Heading title={copy.families.title}>
        {fill(copy.families.intro, { count: rules.length })}
        <Link href="/docs/rules" className="text-glow hover:underline">
          {copy.families.link}
        </Link>
        {copy.families.after}
      </Heading>
      <TileGrid art={tileArt} tiles={copy.families.tiles} />
    </Band>
  );
}
