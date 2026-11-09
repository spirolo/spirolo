// the default view against --all. A two-way toggle above the sixteen rules; under `check` the
// rules the default view leaves out fold into one counted line, under `--all` every rule shows under
// its tier. Tiers and counts come from lib/status.ts over the embedded status table.
'use client';
import Link from 'next/link';
import { AnimatePresence, motion } from 'motion/react';
import { useState } from 'react';
import { TierBadge } from '@/components/rule-status';
import { product } from '@/lib/product';
import { familyOf, rules } from '@/lib/rules';
import { statusOf, tierOrder, type Tier } from '@/lib/status';
import { Band, Heading } from './shared';

type Mode = 'check' | 'all';

const shownByDefault: Tier[] = ['proven', 'advisory'];

const rows = rules.map((rule) => ({ rule, family: familyOf(rule).title, status: statusOf(rule.id) }));
const byTier = tierOrder.map((tier) => ({ tier, rows: rows.filter((row) => row.status.tier === tier) }));
const folded = byTier.filter((group) => !shownByDefault.includes(group.tier));
const foldedCount = folded.reduce((sum, group) => sum + group.rows.length, 0);

const tierHeading: Record<Tier, string> = {
  proven: 'Proven: verdict PASS',
  advisory: 'Advisory: at least 15 decided, at least 80% true',
  experimental: 'Experimental: every other rule that is not FAIL or DROPPED',
  hidden: 'Hidden: verdict FAIL or DROPPED',
};

const modes: { id: Mode; label: string }[] = [
  { id: 'check', label: `${product.cli} check` },
  { id: 'all', label: `${product.cli} check --all` },
];

export function DefaultView() {
  const [mode, setMode] = useState<Mode>('check');
  const groups = byTier.filter((group) => group.rows.length > 0 && (mode === 'all' || shownByDefault.includes(group.tier)));

  return (
    <Band>
      <Heading title="This is an alpha. More rules are coming soon.">
        No rule is proven yet, so the default view stays cautious: a rule shows only once at least 15 of its sampled findings were decided and at least 80% were true. The others still run, and --all shows them, labelled.{' '}
        <Link href="/docs/trust" className="text-glow hover:underline">
          How rules are measured
        </Link>
        .
      </Heading>

      <div role="group" aria-label="View" className="mt-8 inline-flex rounded-full border border-fd-border p-1">
        {modes.map((option) => {
          const on = mode === option.id;
          return (
            <button
              key={option.id}
              type="button"
              aria-pressed={on}
              onClick={() => setMode(option.id)}
              className={`rounded-full px-3 py-1.5 font-mono text-[13px] transition-colors sm:px-4 ${
                on ? 'bg-glow text-night' : 'text-fd-muted-foreground hover:text-fd-foreground'
              }`}
            >
              {option.label}
            </button>
          );
        })}
      </div>

      <ul className="mt-6 flex flex-col gap-1.5">
        <AnimatePresence initial={false} mode="popLayout">
          {groups.flatMap((group) => [
            mode === 'all' && (
              <motion.li
                key={`tier-${group.tier}`}
                layout
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                transition={{ duration: 0.2 }}
                className="pt-4 text-[11px] font-semibold uppercase tracking-[0.08em] text-fd-muted-foreground/80 first:pt-0"
              >
                {tierHeading[group.tier]} · {group.rows.length}
              </motion.li>
            ),
            ...group.rows.map((row) => (
              <motion.li
                key={row.rule.id}
                layout
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                transition={{ duration: 0.2 }}
                className="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-x-4 gap-y-1 rounded-xl border border-fd-border bg-night/40 px-4 py-2.5 sm:grid-cols-[12rem_minmax(0,1fr)_auto_5.5rem]"
              >
                <Link href={`/docs/rules/${row.rule.id}`} className="truncate font-mono text-[13px] text-fd-foreground hover:text-glow">
                  {row.rule.id}
                </Link>
                <span className="order-3 text-sm text-fd-muted-foreground sm:order-none">{row.family}</span>
                <span className="justify-self-end sm:justify-self-auto">
                  <TierBadge tier={row.status.tier} />
                </span>
                <span className="order-4 text-right font-mono text-[13px] tabular-nums text-fd-muted-foreground sm:order-none">
                  {row.status.decided === 0 ? 'none' : `${row.status.truths} / ${row.status.decided}`}
                </span>
              </motion.li>
            )),
          ])}
          {mode === 'check' && (
            <motion.li
              key="folded"
              layout
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              transition={{ duration: 0.2 }}
              className="mt-2 text-sm leading-relaxed text-fd-muted-foreground"
            >
              {foldedCount} rules run but are not shown:{' '}
              {folded.map((group) => `${group.rows.length} ${group.tier}`).join(', ')}.{' '}
              <code className="font-mono">{product.cli} check --all</code> shows them, each labelled.
            </motion.li>
          )}
        </AnimatePresence>
      </ul>
    </Band>
  );
}
