// The four layers of components/layers.tsx as an exploded stack of hairline plates, the job and who
// covers it beside them. Once in view the plates settle and a dot reads down the stack: it passes the
// two layers other tools cover and each layer nobody else covers lights as it arrives. The motion
// lives in home.css, driven by data-phase; the rest state is the finished picture.
'use client';
import { useRef, type CSSProperties, type RefObject } from 'react';
import { product } from '@/lib/product';
import { Band, Heading, usePlayOnce } from './shared';

const layers = [
  {
    name: 'Mistakes inside a test file',
    job: 'A test with no assertion, a focused test, an unawaited query.',
    who: 'Checked by ESLint test plugins',
    ours: false,
  },
  {
    name: 'The setup every test runs on',
    job: 'Whether a console error, a leaked mock or an unmatched request fails the test, or quietly lets it pass.',
    who: 'no other tool checks it',
    ours: true,
    at: '1300ms',
  },
  {
    name: 'Whether the assertions catch a bug',
    job: 'Mutation testing plants small bugs and reruns the suite to see whether a test notices. Thorough, and slow on a big suite: one run per planted bug.',
    who: 'Checked by Stryker',
    ours: false,
  },
  {
    name: 'How the tests connect to the code',
    job: 'Mocks that match no real export, absence checks with nothing proving the render happened, skips nobody tracks, gates that never gate.',
    who: 'no other tool checks it',
    ours: true,
    at: '2050ms',
  },
];

const GLOW = '#D7B2F5';
const EDGE = 'rgb(187 162 205 / 0.42)';
const CENTRE = 160;
const HALF_WIDTH = 130;
const HALF_DEPTH = 38;
const THICKNESS = 8;
const TOP = 62;
const STEP = 96;
const DOT_START = 6;
const centre = (index: number) => TOP + index * STEP;

const face = (y: number) =>
  `M${CENTRE - HALF_WIDTH} ${y} L${CENTRE} ${y - HALF_DEPTH} L${CENTRE + HALF_WIDTH} ${y} L${CENTRE} ${y + HALF_DEPTH} Z`;
const edge = (y: number) =>
  `M${CENTRE - HALF_WIDTH} ${y} v${THICKNESS} L${CENTRE} ${y + HALF_DEPTH + THICKNESS} L${CENTRE + HALF_WIDTH} ${y + THICKNESS} v${-THICKNESS}`;

function Stack({ root }: { root: RefObject<HTMLElement | null> }) {
  return (
    <figure ref={root} className="mx-auto w-full max-w-[17rem] md:max-w-none" aria-hidden>
      <svg
        viewBox="0 0 320 400"
        className="w-full"
        fill="none"
        style={
          {
            '--second': `${centre(1) - DOT_START}px`,
            '--last': `${centre(3) - DOT_START}px`,
          } as CSSProperties
        }
      >
        {[CENTRE - HALF_WIDTH, CENTRE + HALF_WIDTH].map((x) => (
          <line key={x} x1={x} x2={x} y1={centre(0)} y2={centre(3)} stroke={EDGE} strokeDasharray="2 6" />
        ))}
        {layers.map((layer, index) => {
          const y = centre(index);
          return (
            <g key={layer.name}>
              <g className="home-plate" style={{ '--i': index } as CSSProperties}>
                <path d={face(y)} fill="#1F1829" stroke={EDGE} strokeLinejoin="round" />
                <path d={edge(y)} stroke={EDGE} strokeLinejoin="round" />
              </g>
              {layer.ours && (
                <>
                  <g className="home-lit" style={{ '--at': layer.at } as CSSProperties}>
                    <path d={face(y)} fill="rgb(215 178 245 / 0.1)" stroke={GLOW} strokeLinejoin="round" />
                    <path d={edge(y)} stroke={GLOW} strokeLinejoin="round" />
                    <image
                      href="/brand/logo/symbol-small-glow.svg"
                      width="64"
                      height="64"
                      x="-32"
                      y="-32"
                      transform={`translate(${CENTRE} ${y}) matrix(0.75 0.22 -0.75 0.22 0 0)`}
                    />
                  </g>
                  <ellipse
                    className="home-ring"
                    cx={CENTRE}
                    cy={y}
                    rx="44"
                    ry="13"
                    stroke={GLOW}
                    style={{ '--at': layer.at } as CSSProperties}
                  />
                </>
              )}
            </g>
          );
        })}
        <g className="home-dot">
          <line x1={CENTRE} x2={CENTRE} y1={DOT_START - 24} y2={DOT_START - 4} stroke={GLOW} opacity="0.45" strokeLinecap="round" />
          <circle cx={CENTRE} cy={DOT_START} r="3.5" fill={GLOW} />
        </g>
      </svg>
    </figure>
  );
}

export function LayersBand() {
  const root = useRef<HTMLElement>(null);
  const phase = usePlayOnce(root, 0.5);

  return (
    <Band ground="muted">
      <Heading title="Green is not the same as tested">
        A passing suite rests on four layers. Most teams check the first and believe they have the rest. Two of them have no tool. Together they are the missing layer, and that is where {product.name} looks.
      </Heading>
      <div data-phase={phase} className="mt-10 grid items-center gap-8 md:grid-cols-[minmax(0,2fr)_minmax(0,3fr)] md:gap-14">
        <Stack root={root} />
        <ol className="flex flex-col">
          {layers.map((layer) => (
            <li key={layer.name} className="flex min-w-0 flex-col gap-1 border-t border-fd-border py-4 first:border-t-0 md:first:pt-0">
              <div className="flex flex-wrap items-baseline gap-x-3 gap-y-1">
                <h3 className={`font-semibold ${layer.ours ? 'text-fd-foreground' : 'text-fd-muted-foreground'}`}>{layer.name}</h3>
                <p className="flex items-center gap-2 text-sm">
                  {layer.ours ? (
                    <>
                      <span className="home-lit font-medium text-glow" style={{ '--at': layer.at } as CSSProperties}>
                        Checked by {product.name}
                      </span>
                      <span className="text-fd-muted-foreground">· {layer.who}</span>
                    </>
                  ) : (
                    <span className="text-fd-muted-foreground">{layer.who}</span>
                  )}
                </p>
              </div>
              <p className="leading-relaxed text-fd-muted-foreground">{layer.job}</p>
            </li>
          ))}
        </ol>
      </div>
    </Band>
  );
}
