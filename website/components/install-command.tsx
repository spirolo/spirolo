'use client';
import { useState } from 'react';

const english = { copy: 'Copy', copied: 'Copied', copyCommand: 'Copy {command}' };

export function InstallCommand({ command, labels = english }: { command: string; labels?: typeof english }) {
  const [copied, setCopied] = useState(false);

  return (
    <div className="flex max-w-full items-center gap-2 rounded-xl border border-fd-border bg-fd-muted py-2 pl-3 pr-1.5 font-mono text-[13px] sm:gap-3 sm:pl-4 sm:pr-2 sm:text-sm">
      <span className="select-none text-fd-muted-foreground" aria-hidden>
        $
      </span>
      <code className="min-w-0 flex-1 overflow-x-auto whitespace-nowrap">{command}</code>
      {/* Icon only on phones, so the whole command stays readable at 390 px. */}
      <button
        type="button"
        aria-label={copied ? labels.copied : labels.copyCommand.replace('{command}', command)}
        className="shrink-0 rounded-lg p-1.5 font-sans text-xs text-fd-muted-foreground transition-colors hover:bg-fd-accent hover:text-fd-foreground sm:px-3 sm:py-1"
        onClick={() => {
          void navigator.clipboard.writeText(command).then(() => {
            setCopied(true);
            setTimeout(() => setCopied(false), 1500);
          });
        }}
      >
        <svg viewBox="0 0 16 16" className="size-4 sm:hidden" fill="none" stroke="currentColor" strokeWidth="1.5" aria-hidden>
          {copied ? (
            <path d="M3.5 8.5l3 3 6-7" strokeLinecap="round" strokeLinejoin="round" />
          ) : (
            <>
              <rect x="5.5" y="5.5" width="8" height="8" rx="1.5" />
              <path d="M10.5 5.5v-2a1 1 0 0 0-1-1h-6a1 1 0 0 0-1 1v6a1 1 0 0 0 1 1h2" />
            </>
          )}
        </svg>
        <span className="hidden sm:inline">{copied ? labels.copied : labels.copy}</span>
      </button>
    </div>
  );
}
