// The five family pictures. Each is hairlines on the tile's surface and still at rest; each plays
// one change when its tile is active (./tile-grid). Amber appears only then, so a view never shows
// more than one found thing. The test names are real findings from the runs in hero-runs.json, the
// guards are the four fail-closed-setup checks, and the assertion is the flake-candidate example.
'use client';
import { animate, motion, useMotionValue, useReducedMotion, useTransform } from 'motion/react';
import { useEffect } from 'react';
import contours from '@/data/contours.json';
import type { FamilyId } from '@/lib/rules';
import { useActive } from './tile-grid';

const AMBER = '#F5A524';
const GLOW = '#D7B2F5';
const EDGE = 'rgb(187 162 205 / 0.2)';
const INK = 'var(--ink)';
const TEXT = 'var(--ink-text)';
const FILL = 'var(--ink-fill)';
const settle = { type: 'spring', visualDuration: 0.5, bounce: 0 } as const;

function useTiming() {
  return useReducedMotion() ? { duration: 0 } : settle;
}

/** The brand's orbit element, drawn closed around the found thing. */
function Orbit({ x, y, scale = 1, wait = 0.25 }: { x: number; y: number; scale?: number; wait?: number }) {
  const active = useActive();
  const still = useReducedMotion();
  const draw = { duration: still ? 0 : 0.7, ease: 'easeOut' as const, delay: still || !active ? 0 : wait };
  const fade = { duration: still ? 0 : 0.4, delay: still || !active ? 0 : wait + 0.35 };
  const line = 1.25 / scale;
  const thin = 0.8 / scale;
  return (
    <g transform={`translate(${x - 110 * scale} ${y - 85 * scale}) scale(${scale})`} stroke={GLOW} strokeLinecap="round" fill="none">
      <motion.path d="M28 98 A84 62 -12 0 1 176 34" strokeWidth={line} initial={false} animate={{ pathLength: active ? 1 : 0, opacity: active ? 1 : 0 }} transition={draw} />
      <motion.path d="M188 62 A84 62 -12 0 1 52 142" strokeWidth={line} strokeDasharray="14 9" initial={false} animate={{ opacity: active ? 1 : 0 }} transition={fade} />
      <motion.path d="M14 92 A98 72 -12 0 1 120 12" strokeWidth={thin} strokeDasharray="4 10" initial={false} animate={{ opacity: active ? 0.7 : 0 }} transition={fade} />
      <motion.path d="M204 84 A98 72 -12 0 1 120 158" strokeWidth={thin} strokeDasharray="22 10" initial={false} animate={{ opacity: active ? 0.7 : 0 }} transition={fade} />
    </g>
  );
}

const rows = [
  ['hides the item count tag when itemCount is 0', 'should stay closed when index is -1', 'the day %s should be hidden'],
  ['does not render an unvisited lazy tab', 'should not display the previous button', 'hides the search input when hideSearch is true'],
  ['search bar not rendered when options not provided', 'non-custom leftItems render nothing in children', 'should not populate textLabel prop when kind is not inline'],
  ['does not render horizontal modifier when layout="vertical"', 'when Link.Preview is not used, then does not render LinkNativeView, LinkNativePreview and LinkNativeTrigger'],
];
const FOUND = 'should not display the previous button';
const FOUND_ROW = 1;

function Tick({ found }: { found: boolean }) {
  const active = useActive();
  const timing = useTiming();
  return (
    <svg viewBox="0 0 16 16" className="size-4 shrink-0" fill="none" strokeLinecap="round" strokeLinejoin="round">
      <motion.path d="M3.5 8.5l3 3 6-7" stroke={TEXT} strokeWidth="1.5" initial={false} animate={{ opacity: found && active ? 0 : 1 }} transition={timing} />
      {found && <motion.circle cx="8" cy="8" r="5" stroke={AMBER} strokeWidth="1.5" initial={false} animate={{ opacity: active ? 1 : 0, pathLength: active ? 1 : 0 }} transition={timing} />}
    </svg>
  );
}

/** A wall of ticks, and the one that has nothing behind it. */
function WeakTests() {
  const active = useActive();
  const timing = useTiming();
  const still = useReducedMotion();
  return (
    <div className="absolute inset-0 flex flex-col justify-center gap-2 pt-4">
      {rows.map((row, index) => (
        <motion.div
          key={row[1]}
          className={`relative gap-2 whitespace-nowrap font-mono text-[12.5px] ${index === FOUND_ROW ? 'z-10 grid grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)]' : 'flex justify-center'}`}
          style={{ left: [-46, 0, 58, -24][index] }}
          initial={false}
          animate={{ x: active && !still && index !== FOUND_ROW ? [-26, 0, 26, -18][index] : 0 }}
          transition={{ duration: active ? 7 : 0.8, ease: 'easeOut' }}
        >
          {row.map((name, column) => {
            const found = name === FOUND;
            return (
              <motion.div
                key={name}
                className={`relative flex h-11 shrink-0 items-center gap-2.5 rounded-lg border px-3.5 ${index === FOUND_ROW ? ['justify-self-end', '', 'justify-self-start'][column] : ''}`}
                style={{ background: FILL, color: TEXT }}
                initial={false}
                animate={{ opacity: active && !found ? 0.32 : 1, borderColor: found && active ? AMBER : EDGE }}
                transition={timing}
              >
                <Tick found={found} />
                {name}
                {found && (
                  <motion.div className="absolute left-5 top-full" initial={false} animate={{ opacity: active ? 1 : 0, y: active ? 0 : -6 }} transition={timing}>
                    <span className="ml-[3px] block h-2.5 w-px" style={{ background: AMBER }} />
                    <span className="block rounded-lg border px-3.5 py-2" style={{ background: FILL, borderColor: 'rgb(245 165 36 / 0.45)', color: AMBER }}>
                      asserts only that DOM is absent
                    </span>
                  </motion.div>
                )}
              </motion.div>
            );
          })}
        </motion.div>
      ))}
    </div>
  );
}

// Laid out inside the middle 330 units: a phone tile crops the lanes down to that.
const guards = ['mock reset', 'console guard', 'unhandled requests', 'global server'];
const MISSING = 1;
const POST = { left: 130, width: 146 };
const MAIN = { x: 372, y: 129, width: 60 };
const laneY = (index: number) => 48 + index * 54;
const OUT = POST.left + POST.width;
const curve = (y: number) => `M${OUT} ${y}C${OUT + 54} ${y} ${OUT + 46} ${MAIN.y} ${MAIN.x} ${MAIN.y}`;
const STRAIGHT = 0.72;

/** Where the dot that gets through is, from the left edge (0) to main (1). */
function along(progress: number, y: number): [number, number] {
  if (progress <= STRAIGHT) return [-10 + (OUT + 10) * (progress / STRAIGHT), y];
  const t = (progress - STRAIGHT) / (1 - STRAIGHT);
  const u = 1 - t;
  const blend = (a: number, b: number, c: number, d: number) => u * u * u * a + 3 * u * u * t * b + 3 * u * t * t * c + t * t * t * d;
  return [blend(OUT, OUT + 54, OUT + 46, MAIN.x), blend(y, y, MAIN.y, MAIN.y)];
}

function Guard({ name, index }: { name: string; index: number }) {
  const active = useActive();
  const still = useReducedMotion();
  const y = laneY(index);
  const missing = index === MISSING;
  const hit = 0.8 + index * 0.1;
  return (
    <g>
      <path d={`M-10 ${y}H${POST.left}`} stroke={INK} strokeDasharray="1 7" strokeLinecap="round" strokeWidth="1.5" />
      <path d={curve(y)} stroke={INK} strokeDasharray="1 7" strokeLinecap="round" strokeWidth="1.5" opacity={missing ? 1 : 0.45} />
      <motion.rect
        x={POST.left}
        y={y - 17}
        width={POST.width}
        height={34}
        rx={17}
        style={{ fill: FILL }}
        strokeDasharray={missing ? '4 5' : undefined}
        initial={false}
        animate={{ stroke: missing ? (active ? AMBER : 'rgb(187 162 205 / 0.42)') : active && !still ? ['rgb(187 162 205 / 0.42)', GLOW, 'rgb(187 162 205 / 0.42)'] : 'rgb(187 162 205 / 0.42)' }}
        transition={missing ? { duration: 0.25, delay: still || !active ? 0 : 0.6 } : { duration: 0.7, delay: still || !active ? 0 : hit, times: [0, 0.2, 1] }}
      />
      <text x={POST.left + POST.width / 2} y={y + 4} textAnchor="middle" className="font-mono" fontSize="12.5" style={{ fill: TEXT }}>
        {name}
      </text>
      {!missing && (
        <motion.circle
          cy={y}
          r={4.5}
          fill={GLOW}
          initial={false}
          animate={{ cx: active && !still ? [-10, POST.left - 9] : POST.left - 9 }}
          transition={{ duration: still ? 0 : 0.9, delay: still || !active ? 0 : index * 0.1, ease: 'easeOut' }}
        />
      )}
    </g>
  );
}

/** A test on its way to main: three guards stop it, the missing one does not. */
function MergeGate() {
  const active = useActive();
  const still = useReducedMotion();
  const y = laneY(MISSING);
  const progress = useMotionValue(0.9);
  const cx = useTransform(() => along(progress.get(), y)[0]);
  const cy = useTransform(() => along(progress.get(), y)[1]);
  const trail = useTransform(progress, [STRAIGHT, 1], [0, 1]);
  useEffect(() => {
    if (!active || still) {
      progress.set(active ? 1 : 0.9);
      return;
    }
    const run = animate(progress, [0, 1], { duration: 1.7, ease: 'linear' });
    return () => run.stop();
  }, [active, still, progress]);
  return (
    <svg viewBox="0 0 556 251" preserveAspectRatio="xMidYMax slice" className="absolute inset-0 h-full w-full" fill="none">
      <path d={`M${MAIN.x + MAIN.width} ${MAIN.y}H566`} stroke={INK} strokeDasharray="1 7" strokeLinecap="round" strokeWidth="1.5" />
      <motion.circle cx={cx} cy={cy} r={4.5} initial={false} animate={{ fill: active ? AMBER : GLOW }} transition={{ duration: 0.2, delay: still || !active ? 0 : 1 }} />
      {guards.map((name, index) => (
        <Guard key={name} name={name} index={index} />
      ))}
      <motion.path d={curve(y)} stroke={AMBER} strokeWidth="1.5" strokeLinecap="round" style={{ pathLength: trail }} initial={false} animate={{ opacity: active ? 1 : 0 }} transition={{ duration: 0.2 }} />
      <rect x={MAIN.x} y={MAIN.y - 15} width={MAIN.width} height={30} rx={15} style={{ fill: FILL }} stroke="rgb(187 162 205 / 0.42)" />
      <text x={MAIN.x + MAIN.width / 2} y={MAIN.y + 4} textAnchor="middle" className="font-mono" fontSize="12.5" style={{ fill: TEXT }}>
        main
      </text>
      <Orbit x={MAIN.x + MAIN.width / 2} y={MAIN.y} scale={0.44} wait={1.5} />
    </svg>
  );
}

// Each cell is the silhouette of a test body: indent and length of its lines, as the duplicate rule
// compares them once names and values are normalised away.
const bodies = [
  [[0, 26], [6, 20], [6, 30], [0, 12]],
  [[0, 30], [6, 14], [12, 18], [6, 24], [0, 10]],
  [[0, 18], [6, 28], [0, 22]],
  [[0, 24], [6, 26], [12, 12], [0, 16]],
  [[0, 32], [6, 10], [6, 22], [6, 16], [0, 8]],
  [[0, 20], [12, 18], [6, 28]],
];
const TWIN = 1;
const grid = [
  [0, 2, 3, 5, 4, 0],
  [3, TWIN, 4, 0, 2, 5],
  [4, 5, 0, TWIN, 3, 2],
];
const CELL = 68;
const GAP = 7;
const cell = (row: number, column: number) => ({ x: -26 + column * (CELL + GAP), y: 24 + row * (CELL + GAP) });

/** Tests as shapes: two of them are the same test. */
function UnusedTests() {
  const active = useActive();
  const timing = useTiming();
  const still = useReducedMotion();
  const from = cell(1, 1);
  const to = cell(2, 3);
  const [startX, startY] = [from.x + CELL / 2, from.y + CELL];
  const [endX, endY] = [to.x, to.y + CELL / 2];
  const link = `M${startX} ${startY}V${endY - 10}Q${startX} ${endY} ${startX + 10} ${endY}H${endX}`;
  return (
    <svg viewBox="0 0 372 251" preserveAspectRatio="xMinYMax slice" className="absolute inset-0 h-full w-full" fill="none">
      {grid.flatMap((cells, row) =>
        cells.map((body, column) => {
          const { x, y } = cell(row, column);
          const twin = body === TWIN;
          return (
            <motion.g key={`${row}-${column}`} initial={false} animate={{ opacity: active && !twin ? 0.28 : 1 }} transition={timing}>
              <motion.rect x={x} y={y} width={CELL} height={CELL} rx={11} style={{ fill: FILL }} initial={false} animate={{ stroke: twin && active ? GLOW : EDGE }} transition={timing} />
              {bodies[body].map(([indent, length], line) => (
                <motion.path
                  key={line}
                  d={`M${x + 16 + indent} ${y + 18 + line * 8}h${length * 0.85}`}
                  stroke={TEXT}
                  strokeWidth="2"
                  strokeLinecap="round"
                  initial={false}
                  animate={{ pathLength: twin && active && !still ? [0, 1] : 1 }}
                  transition={{ duration: 0.35, delay: active ? 0.1 + line * 0.09 : 0, ease: 'easeOut' }}
                />
              ))}
            </motion.g>
          );
        }),
      )}
      <motion.path d={link} stroke={AMBER} strokeWidth="1.5" strokeLinecap="round" initial={false} animate={{ pathLength: active ? 1 : 0, opacity: active ? 1 : 0 }} transition={{ duration: still ? 0 : 0.6, ease: 'easeOut', delay: still || !active ? 0 : 0.6 }} />
    </svg>
  );
}

const DIAL = { x: 186, y: 262, r: 196 };
const START = Math.PI * 1.08;
const SWEEP = Math.PI * 0.84;
const BUDGET = 0.6;
const REST = 0.42;
const LANDED = 0.78;
const earlier = [0.29, 0.35, 0.39, 0.47, 0.53];
const polar = (share: number, radius: number) => {
  const angle = START + SWEEP * share;
  return [DIAL.x + radius * Math.cos(angle), DIAL.y + radius * Math.sin(angle)] as const;
};
const point = (share: number, radius: number) => polar(share, radius).map((n) => n.toFixed(2)).join(' ');
const arc = (from: number, to: number, radius: number) => `M${point(from, radius)}A${radius} ${radius} 0 0 1 ${point(to, radius)}`;
const ticks = Array.from({ length: 61 }, (_, index) => index / 60);

/** A test that times itself: the same code lands either side of its budget. */
function Flaky() {
  const active = useActive();
  const still = useReducedMotion();
  const share = useMotionValue(REST);
  const x1 = useTransform(() => polar(share.get(), 66)[0]);
  const y1 = useTransform(() => polar(share.get(), 66)[1]);
  const x2 = useTransform(() => polar(share.get(), DIAL.r - 22)[0]);
  const y2 = useTransform(() => polar(share.get(), DIAL.r - 22)[1]);
  const late = { duration: 0.3, delay: still || !active ? 0 : 1.45 };
  useEffect(() => {
    if (still) {
      share.set(active ? LANDED : REST);
      return;
    }
    const sweep = active
      ? animate(share, [REST, 0.53, 0.33, LANDED], { duration: 1.9, times: [0, 0.3, 0.58, 1], ease: 'easeInOut' })
      : animate(share, REST, { duration: 0.5, ease: 'easeOut' });
    return () => sweep.stop();
  }, [active, still, share]);
  return (
    <svg viewBox="0 0 372 251" preserveAspectRatio="xMidYMax slice" className="absolute inset-0 h-full w-full" fill="none">
      {ticks.map((tick, index) => (
        <path key={index} d={`M${point(tick, DIAL.r)}L${point(tick, DIAL.r - (index % 5 === 0 ? 13 : 7))}`} stroke={INK} strokeWidth="1" strokeLinecap="round" />
      ))}
      <path d={arc(0, 1, DIAL.r - 34)} stroke={INK} />
      <motion.path d={arc(0, 1, DIAL.r - 34)} stroke={GLOW} strokeOpacity="0.75" style={{ pathLength: share }} />
      {earlier.map((run) => (
        <path key={run} d={`M${point(run, DIAL.r - 34)}L${point(run, DIAL.r - 24)}`} stroke={GLOW} strokeOpacity="0.55" strokeLinecap="round" />
      ))}
      <path d={`M${point(BUDGET, DIAL.r + 4)}L${point(BUDGET, DIAL.r - 44)}`} stroke={GLOW} strokeWidth="1.5" strokeLinecap="round" />
      <motion.path d={arc(BUDGET, LANDED, DIAL.r - 34)} stroke={AMBER} strokeWidth="1.5" strokeLinecap="round" initial={false} animate={{ opacity: active ? 1 : 0 }} transition={late} />
      <motion.line x1={x1} y1={y1} x2={x2} y2={y2} stroke={GLOW} strokeWidth="1.5" strokeLinecap="round" />
      <motion.circle cx={x2} cy={y2} r={3.5} initial={false} animate={{ fill: active ? AMBER : GLOW }} transition={late} />
      <text x={polar(BUDGET, DIAL.r + 16)[0]} y={polar(BUDGET, DIAL.r + 16)[1]} textAnchor="middle" className="font-mono" fontSize="11" style={{ fill: TEXT }}>
        50 ms
      </text>
      <rect x={20} y={205} width={332} height={32} rx={16} style={{ fill: FILL }} stroke="rgb(187 162 205 / 0.3)" />
      <text x={186} y={225} textAnchor="middle" className="font-mono" fontSize="11" style={{ fill: TEXT }}>
        expect(Date.now() - start)
        <motion.tspan initial={false} animate={{ fill: active ? AMBER : GLOW }} transition={late}>
          .toBeLessThan(50)
        </motion.tspan>
      </text>
    </svg>
  );
}

const others = [
  { rank: 2, x: 120, y: 170 },
  { rank: 3, x: 336, y: 208 },
  { rank: 4, x: 70, y: 52 },
];

/** The suite as a survey map, ranked, and the one spot where a new test pays first. */
function Health() {
  const active = useActive();
  const still = useReducedMotion();
  const [px, py] = contours.peak;
  const top = contours.levels.length - 1;
  const draw = { duration: still ? 0 : 0.6, ease: 'easeOut' as const };
  return (
    <svg viewBox={`0 0 ${contours.width} ${contours.height}`} preserveAspectRatio="xMidYMax slice" className="absolute inset-0 h-full w-full" fill="none">
      {contours.levels.map((d, index) => (
        <path key={index} d={d} stroke={INK} strokeWidth={index % 4 === 3 ? 1.1 : 0.7} />
      ))}
      {contours.levels.map((d, index) => (
        <motion.path
          key={index}
          d={d}
          stroke={GLOW}
          strokeWidth="1"
          initial={false}
          animate={{ opacity: active && !still ? [0, 0.8, 0] : 0 }}
          transition={active ? { duration: 0.8, delay: (top - index) * 0.045, times: [0, 0.3, 1] } : { duration: 0.2 }}
        />
      ))}
      <motion.path d={`M-10 ${py}H${px - 14}`} stroke={GLOW} strokeWidth="1" strokeDasharray="1 5" strokeLinecap="round" initial={false} animate={{ opacity: active ? 1 : 0 }} transition={draw} />
      <motion.path d={`M${px} ${contours.height + 10}V${py + 14}`} stroke={GLOW} strokeWidth="1" strokeDasharray="1 5" strokeLinecap="round" initial={false} animate={{ opacity: active ? 1 : 0 }} transition={draw} />
      {others.map(({ rank, x, y }) => (
        <g key={rank}>
          <circle cx={x} cy={y} r={2.5} fill={GLOW} fillOpacity="0.7" />
          <text x={x + 8} y={y + 4} className="font-mono" fontSize="11" style={{ fill: TEXT }}>
            {rank}
          </text>
        </g>
      ))}
      <motion.circle cx={px} cy={py} r={4} initial={false} animate={{ fill: active ? AMBER : GLOW }} transition={{ duration: 0.3 }} />
      <text x={px + 9} y={py + 4} className="font-mono" fontSize="11" style={{ fill: TEXT }}>
        1
      </text>
    </svg>
  );
}

export const tileArt: Record<FamilyId, React.ReactNode> = {
  audit: <WeakTests />,
  gate: <MergeGate />,
  hygiene: <UnusedTests />,
  flake: <Flaky />,
  impact: <Health />,
};
