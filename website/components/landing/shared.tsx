// What the home page's bands share: the band, its heading, and the play-once hook. The server render
// is always the finished state; motion starts only on the client, only when the reader has not asked
// for less of it, and only once the block scrolls into view.
'use client';
import { useEffect, useLayoutEffect, useState, type RefObject } from 'react';

type Ground = 'night' | 'muted' | 'plum';

export function Band({ children, ground = 'night' }: { children: React.ReactNode; ground?: Ground }) {
  const grounds: Record<Ground, string> = {
    night: '',
    muted: 'bg-fd-muted/60 border-y border-fd-border',
    plum: 'bg-[radial-gradient(ellipse_at_top,rgb(112_66_146/0.35),transparent_60%)]',
  };
  return (
    <div className={grounds[ground]}>
      <div className="mx-auto w-full max-w-6xl px-4 py-14 sm:px-6 sm:py-20">{children}</div>
    </div>
  );
}

export function Heading({ title, children }: { title: string; children?: React.ReactNode }) {
  return (
    <div className="max-w-2xl">
      <h2 className="text-2xl font-semibold tracking-tight sm:text-3xl">{title}</h2>
      {children && <p className="mt-3 leading-relaxed text-fd-muted-foreground">{children}</p>}
    </div>
  );
}

type Phase = 'idle' | 'play' | 'done';

const useIsomorphicLayoutEffect = typeof window === 'undefined' ? useEffect : useLayoutEffect;

/** Plays once: 'done' on the server and for readers who asked for less motion, else 'idle' until in view, then 'play'. */
export function usePlayOnce<T extends HTMLElement>(ref: RefObject<T | null>, threshold = 0.3): Phase {
  const [phase, setPhase] = useState<Phase>('done');
  useIsomorphicLayoutEffect(() => {
    if (matchMedia('(prefers-reduced-motion: reduce)').matches) return;
    setPhase('idle');
    const observer = new IntersectionObserver(
      (entries) => {
        if (!entries.some((entry) => entry.isIntersecting)) return;
        observer.disconnect();
        setPhase('play');
      },
      { threshold },
    );
    if (ref.current) observer.observe(ref.current);
    return () => observer.disconnect();
  }, []);
  return phase;
}
