// The health line as the one thing the orbit circles. "Health C (61)" from the real run: once in
// view the arcs draw, the score counts up through the real grades and the dashes of the outer arcs
// keep travelling while the picture is on screen. The plain truth sits beside it: a provisional
// number to watch over time, which nothing gates on.
'use client';
import { animate, motion, useInView, useMotionValue, useTransform } from 'motion/react';
import Link from 'next/link';
import { useEffect, useRef, type CSSProperties } from 'react';
import { report } from './report';
import { Band, usePlayOnce } from './shared';

const health = report.health.match(/^Health (\S+) \((\d+)\)/);
const grade = health?.[1] ?? '';
const score = Number(health?.[2] ?? 0);

// The thresholds of crates/spirolo-report/src/score.rs.
const gradeOf = (value: number): string => (value >= 85 ? 'A' : value >= 70 ? 'B' : value >= 55 ? 'C' : value >= 40 ? 'D' : 'F');

// branding/elements/orbit.svg, its four arcs. `period` is a dash plus its gap, on the arcs that travel.
const arcs = [
  { d: 'M28 98 A84 62 -12 0 1 176 34', width: 2.5, opacity: 1 },
  { d: 'M188 62 A84 62 -12 0 1 52 142', width: 2.5, dash: '14 9', opacity: 1 },
  { d: 'M14 92 A98 72 -12 0 1 120 12', width: 1.5, dash: '4 10', opacity: 0.7, period: 14 },
  { d: 'M204 84 A98 72 -12 0 1 120 158', width: 1.5, dash: '22 10', opacity: 0.7, period: 32 },
];
const UNITS_PER_SECOND = 4.6;

export function HealthScore() {
  const root = useRef<HTMLElement>(null);
  const phase = usePlayOnce(root, 0.6);
  const inView = useInView(root);
  const count = useMotionValue(score);
  const shownScore = useTransform(() => Math.round(count.get()));
  const shownGrade = useTransform(() => gradeOf(Math.round(count.get())));

  useEffect(() => {
    if (phase === 'idle') count.set(0);
    if (phase !== 'play') return;
    const run = animate(count, score, { duration: 1.6, delay: 0.2, ease: [0.3, 0.2, 0.3, 1] });
    return () => run.stop();
  }, [phase, count]);

  const hidden = { opacity: phase === 'idle' ? 0 : 1 };
  return (
    <Band>
      <div className="grid items-center gap-10 md:grid-cols-2 md:gap-14">
        <figure ref={root} className="mx-auto w-full max-w-xs">
          <div className="relative aspect-[220/170]">
            <svg viewBox="0 0 220 170" className="absolute inset-0 size-full" aria-hidden style={hidden}>
              <defs>
                {arcs.map((arc, index) => (
                  <mask key={index} id={`home-orbit-${index}`} maskUnits="userSpaceOnUse" x="0" y="0" width="220" height="170">
                    <path
                      d={arc.d}
                      pathLength={1}
                      fill="none"
                      stroke="#fff"
                      strokeWidth={arc.width + 2}
                      strokeLinecap="round"
                      className={phase === 'play' ? 'home-orbit-draw' : ''}
                      style={{ '--i': index } as CSSProperties}
                    />
                  </mask>
                ))}
              </defs>
              <g fill="none" stroke="#D7B2F5" strokeLinecap="round">
                {arcs.map((arc, index) => (
                  <g key={index} mask={`url(#home-orbit-${index})`}>
                    <path
                      d={arc.d}
                      strokeWidth={arc.width}
                      strokeDasharray={arc.dash}
                      opacity={arc.opacity}
                      className={phase === 'play' && arc.period ? 'home-orbit-flow' : ''}
                      style={
                        arc.period
                          ? ({
                              '--loop': -arc.period,
                              '--turn': `${arc.period / UNITS_PER_SECOND}s`,
                              animationPlayState: inView ? 'running' : 'paused',
                            } as CSSProperties)
                          : undefined
                      }
                    />
                  </g>
                ))}
              </g>
            </svg>
            <div className="absolute inset-0 flex flex-col items-center justify-center" style={hidden}>
              <p className="font-mono text-6xl font-medium tracking-tight sm:text-7xl">
                <motion.span aria-hidden>{shownGrade}</motion.span>
                <span className="sr-only">{grade} grade</span>
              </p>
              <p className="font-mono text-sm tabular-nums text-fd-muted-foreground">
                <motion.span aria-hidden>{shownScore}</motion.span>
                <span className="sr-only">{score}</span> of 100
              </p>
            </div>
          </div>
          <figcaption className="mt-2 text-center font-mono text-xs text-fd-muted-foreground">{report.health}</figcaption>
        </figure>
        <div className="flex min-w-0 flex-col gap-5">
          <h2 className="text-2xl font-semibold tracking-tight sm:text-3xl">One number to watch, not to gate on</h2>
          <p className="leading-relaxed text-fd-muted-foreground">
            Every report opens with a provisional health score out of 100. Each family takes off points for the share
            of its subjects that have findings, up to a cap per family, and rules outside the default view count half.
            It is a summary to watch over time: nothing gates on it, and the machine formats do not carry it.
          </p>
          <Link href="/docs/reading-a-report" className="text-sm font-medium text-glow hover:underline">
            Reading a report
          </Link>
        </div>
      </div>
    </Band>
  );
}
