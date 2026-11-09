// The logo's coil, drawn: the same shape, uncovered along its centre line from the tip out. Both
// come from the same generator as branding/logo/, so the replay and the logo are one mark.
'use client';
import { useId } from 'react';
import coil from '@/data/coil.json';

/** `progress` runs from 0 to 1. The dim shape behind is the whole coil, waiting to be filled. */
export function Coil({ progress, className = '' }: { progress: number; className?: string }) {
  const mask = `coil${useId().replace(/[^a-zA-Z0-9]/g, '')}`;
  return (
    <svg viewBox="0 0 256 256" className={className} aria-hidden>
      <mask id={mask} maskUnits="userSpaceOnUse" x="0" y="0" width="256" height="256">
        <path
          d={coil.line}
          pathLength={1}
          fill="none"
          stroke="#fff"
          strokeWidth="38"
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeDasharray="1 1"
          strokeDashoffset={1 - progress}
          opacity={progress > 0 ? 1 : 0}
        />
      </mask>
      <path d={coil.shape} fill="#704292" opacity={0.35} />
      <path d={coil.shape} fill="#D7B2F5" mask={`url(#${mask})`} />
    </svg>
  );
}
