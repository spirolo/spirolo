// The frame the five family tiles share: a flat surface, the words at the top, the picture running
// off the bottom edge. Each picture is still until the reader reaches it: on hover, or on touch
// screens once the tile is on screen, it plays one change. Hover also carries a light that follows
// the pointer and brightens the lines under it.
'use client';
import { useInView } from 'motion/react';
import { createContext, useContext, useEffect, useRef, useState, type CSSProperties, type PointerEvent, type ReactNode } from 'react';
import { Icon } from '@/components/icon';
import { families, type Family, type FamilyId } from '@/lib/rules';
import { fill, type LandingCopy } from './copy/types';
import { ofRule } from './runs';

const Active = createContext(false);
/** True while this tile is the one being looked at. */
export const useActive = () => useContext(Active);

// The frame of each tile and the count its stat quotes; its words are in copy.families.tiles.
const layout: Record<FamilyId, { span: string; count?: number; wide?: boolean }> = {
  audit: { wide: true, span: 'lg:col-span-3', count: ofRule('absence-anchor') },
  gate: { wide: true, span: 'lg:col-span-3', count: ofRule('fail-closed-setup') },
  hygiene: { span: 'lg:col-span-2', count: ofRule('duplicate-test') },
  flake: { span: 'lg:col-span-2' },
  impact: { span: 'lg:col-span-2' },
};

type Words = LandingCopy['families']['tiles'][FamilyId];

function Question({ text, strong }: { text: string; strong: string }) {
  const [before, after] = text.split(strong);
  return (
    <p className="mt-3 text-[15px] leading-relaxed text-fd-muted-foreground">
      {before}
      <strong className="font-normal text-fd-foreground">{strong}</strong>
      {after}
    </p>
  );
}

/** One pointer position for every tile, so the light on the edges reaches the neighbours too. */
function follow(event: PointerEvent<HTMLUListElement>) {
  const tiles = event.currentTarget.children;
  for (let index = 0; index < tiles.length; index++) {
    const tile = tiles[index] as HTMLElement;
    const box = tile.getBoundingClientRect();
    tile.style.setProperty('--x', `${event.clientX - box.left}px`);
    tile.style.setProperty('--y', `${event.clientY - box.top}px`);
  }
}

const INK = { '--ink': 'rgb(187 162 205 / 0.3)', '--ink-text': 'rgb(187 162 205 / 0.62)', '--ink-fill': '#1F1829' } as CSSProperties;
const LIT = { '--ink': 'rgb(215 178 245 / 0.85)', '--ink-text': '#EBDCF9', '--ink-fill': '#261D33' } as CSSProperties;

function Tile({ family, words, art, wide }: { family: Family; words: Words; art: ReactNode; wide: boolean }) {
  const ref = useRef<HTMLLIElement>(null);
  const [hover, setHover] = useState(false);
  const [touch, setTouch] = useState(false);
  const centred = useInView(ref, { amount: 0.7 });
  useEffect(() => setTouch(matchMedia('(hover: none)').matches), []);
  const look = layout[family.id];
  const mask = 'radial-gradient(170px circle at var(--x) calc(var(--y) - var(--art-top)), #000 20%, transparent 100%)';
  return (
    <li
      ref={ref}
      onPointerEnter={() => setHover(true)}
      onPointerLeave={() => setHover(false)}
      style={{ '--x': '50%', '--y': '0px' } as CSSProperties}
      className={`home-reveal group relative isolate min-w-0 h-[26rem] [--art-top:150px] ${look.wide ? 'lg:h-[23rem] lg:[--art-top:104px]' : ''} overflow-hidden rounded-2xl bg-[#191320] shadow-[inset_0_0_0_1px_rgb(255_255_255/0.08)] ${look.span} ${wide ? 'sm:col-span-2' : ''}`}
    >
      <span aria-hidden className="home-edge" />
      <div className="relative z-10 px-6 pt-6">
        <p className="flex items-center gap-2.5 text-fd-foreground">
          <Icon src={family.icon} className="size-[18px] text-fd-foreground" />
          <span className="font-medium">{words.title}</span>
          <span className="ml-auto font-mono text-xs text-fd-muted-foreground">{fill(words.stat, { count: look.count ?? '' })}</span>
        </p>
        <Question text={words.question} strong={words.strong} />
      </div>
      <Active.Provider value={hover || (touch && centred)}>
        <div aria-hidden className="absolute inset-x-0 bottom-0 top-[var(--art-top)] [mask-image:linear-gradient(to_bottom,transparent,#000_34px)]">
          <div className="absolute inset-0" style={INK}>
            {art}
          </div>
          <div
            className="absolute inset-0 opacity-0 transition-opacity duration-300 group-hover:opacity-100"
            style={{ ...LIT, maskImage: mask, WebkitMaskImage: mask }}
          >
            {art}
          </div>
        </div>
      </Active.Provider>
    </li>
  );
}

export function TileGrid({ art, tiles }: { art: Record<FamilyId, ReactNode>; tiles: LandingCopy['families']['tiles'] }) {
  return (
    <ul onPointerMove={follow} className="group/grid mt-10 grid gap-3 sm:grid-cols-2 lg:grid-cols-6">
      {families.map((family, index) => (
        <Tile key={family.id} family={family} words={tiles[family.id]} art={art[family.id]} wide={index === 4} />
      ))}
    </ul>
  );
}
