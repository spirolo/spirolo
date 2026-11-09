// The replay band: one project at a time, the coil and the findings count run for exactly that
// project's measured wall-clock time, its table row lit, then a short hold and the next project. It only
// plays while in view; a reader who asked for less motion gets the first project, finished.
'use client';
import { useEffect, useRef, useState } from 'react';
import { Coil } from './coil';
import { findingCount, ofRule, repos, seconds, tests } from './runs';
import { fill, type LandingCopy } from './copy/types';
import { Band } from './shared';

const HOLD_MS = 1800;

// Play order, which is also the table order: fast first, then fast and slow in turn.
const shown = ['styled-components/styled-components', 'expo/expo', 'gatsbyjs/gatsby', 'mui/material-ui', 'carbon-design-system/carbon', 'mantinedev/mantine'];
const rows = shown.flatMap((name) => repos.filter((repo) => repo.name === name));
const kinds = ['absence-anchor', 'fail-closed-setup', 'dead-skip', 'duplicate-test'] as const;

const printed = (time: string) => time.replace(/(\d)(m?s)$/, '$1 $2');
const elapsed = (ms: number) => (ms < 1000 ? `${Math.round(ms)} ms` : `${(ms / 1000).toFixed(1)} s`);

export function RunReplay({ copy }: { copy: LandingCopy['replay'] & { numbers: string } }) {
  const root = useRef<HTMLElement>(null);
  const [{ index, progress }, setPlay] = useState({ index: 0, progress: 1 });

  useEffect(() => {
    if (matchMedia('(prefers-reduced-motion: reduce)').matches || !root.current) return;
    let inView = false;
    const observer = new IntersectionObserver(([entry]) => (inView = entry.isIntersecting), { threshold: 0.3 });
    observer.observe(root.current);
    let current = 0;
    let time = 0;
    let last = performance.now();
    let frame = requestAnimationFrame(function tick(now) {
      const step = Math.min(now - last, 100);
      last = now;
      if (inView) time += step;
      const run = rows[current].seconds * 1000;
      if (time >= run + HOLD_MS) {
        current = (current + 1) % rows.length;
        time = 0;
      }
      setPlay({ index: current, progress: Math.min(time / rows[current].seconds / 1000, 1) });
      frame = requestAnimationFrame(tick);
    });
    return () => {
      cancelAnimationFrame(frame);
      observer.disconnect();
    };
  }, []);

  const repo = rows[index];
  return (
    <Band ground="muted">
      <figure ref={root} className="grid items-center gap-8 md:grid-cols-[minmax(0,5fr)_minmax(0,6fr)] md:gap-12">
        <div className="min-w-0">
          <div className="flex items-center gap-4">
            <Coil progress={progress} className="size-20 shrink-0 sm:size-24" />
            <div className="min-w-0">
              <p className="text-5xl font-semibold tabular-nums tracking-tight sm:text-6xl">
                {Math.round(repo.findings * progress)}
                <span className="ml-2 text-xl font-normal tracking-normal text-fd-muted-foreground">{copy.findings}</span>
              </p>
              <p className="mt-1 text-sm tabular-nums text-fd-muted-foreground">
                <span className="font-medium text-fd-foreground">{repo.name.split('/')[1]}</span> ·{' '}
                <span className="text-glow">{progress < 1 ? elapsed(repo.seconds * 1000 * progress) : printed(repo.time)}</span>
              </p>
            </div>
          </div>
          <p className="mt-6 text-lg font-medium leading-snug text-fd-foreground">
            {fill(copy.total, { findings: findingCount, repos: repos.length })}
          </p>
          <p className="mt-1 text-sm leading-relaxed text-fd-muted-foreground">
            {fill(copy.read, { tests: tests.toLocaleString(copy.numbers), seconds })}
          </p>
          <dl className="mt-4 grid grid-cols-2 gap-2">
            {kinds.map((rule) => (
              <div key={rule} className="rounded-xl border border-fd-border bg-night/40 px-3 py-2">
                <dt className="sr-only">{copy.kinds[rule]}</dt>
                <dd className="text-xl font-semibold tabular-nums text-fd-foreground">{ofRule(rule)}</dd>
                <dd className="text-sm text-fd-muted-foreground">{copy.kinds[rule]}</dd>
              </div>
            ))}
          </dl>
        </div>
        <table className="w-full min-w-0 text-left text-sm">
          <thead>
            <tr className="text-xs text-fd-muted-foreground">
              <th className="px-2 pb-2 font-medium">{copy.table.project}</th>
              <th className="px-2 pb-2 text-right font-medium">{copy.table.tests}</th>
              <th className="px-2 pb-2 text-right font-medium">{copy.table.findings}</th>
              <th className="px-2 pb-2 text-right font-medium">{copy.table.readIn}</th>
            </tr>
          </thead>
          <tbody className="tabular-nums">
            {rows.map((row, rowIndex) => {
              const on = rowIndex === index;
              const fill = `${progress * 100}%`;
              return (
                <tr
                  key={row.name}
                  aria-current={on ? 'true' : undefined}
                  className={`border-t border-fd-border transition-opacity duration-300 ${on ? '' : 'opacity-60'}`}
                  style={on ? { background: `linear-gradient(to right, rgb(215 178 245 / 0.16) ${fill}, transparent ${fill})` } : undefined}
                >
                  <td className="px-2 py-2">
                    <a href={row.url} className="font-medium text-fd-foreground hover:text-glow">
                      {row.name.split('/')[1]}
                    </a>
                  </td>
                  <td className="px-2 py-2 text-right text-fd-muted-foreground">{row.tests.toLocaleString(copy.numbers)}</td>
                  <td className="px-2 py-2 text-right text-fd-foreground">{row.findings}</td>
                  <td className="px-2 py-2 text-right text-glow">{printed(row.time)}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
        <figcaption className="text-xs leading-relaxed text-fd-muted-foreground md:col-span-2">
          {copy.caption}
        </figcaption>
      </figure>
    </Band>
  );
}
