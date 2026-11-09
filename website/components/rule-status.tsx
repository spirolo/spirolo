import Link from 'next/link';
import { familyOf, rules } from '@/lib/rules';
import { statusOf, tierOrder, type RuleStatus, type Tier } from '@/lib/status';

const tierLabel: Record<Tier, string> = {
  proven: 'proven',
  advisory: 'advisory',
  experimental: 'experimental',
  hidden: 'hidden',
};

const tierView: Record<Tier, string> = {
  proven: 'Shown by default; gates at its own severity.',
  advisory: 'Shown by default at info. Gates only if you raise its severity.',
  experimental: 'Shown with --all. Gates only if you raise its severity.',
  hidden: 'Shown with --all. Never gates, even with an override.',
};

export function TierBadge({ tier }: { tier: Tier }) {
  const style =
    tier === 'proven' || tier === 'advisory'
      ? 'border-glow/40 text-glow'
      : 'border-fd-border text-fd-muted-foreground';
  return <span className={`rounded-full border px-2 py-0.5 font-sans text-xs ${style}`}>{tierLabel[tier]}</span>;
}

function score(status: RuleStatus): string {
  return status.decided === 0 ? 'No decided findings' : `${status.truths} / ${status.decided} true`;
}

/** The measured status of one rule, for its docs page. */
export function RuleStatusCard({ rule }: { rule: string }) {
  const status = statusOf(rule);
  const interval = status.interval
    ? `, 95% Wilson interval ${status.interval[0].toFixed(2)} to ${status.interval[1].toFixed(2)}`
    : '';

  return (
    <div className="not-prose my-6 rounded-xl border border-fd-border bg-fd-card p-4 text-sm">
      <div className="flex flex-wrap items-center gap-2">
        <TierBadge tier={status.tier} />
        <span className="font-mono text-xs text-fd-muted-foreground">{status.verdict}</span>
      </div>
      <p className="mt-3 text-fd-foreground">
        {score(status)}
        {status.decided > 0 ? interval : ''}
        {status.measuredOn.length > 0
          ? `, on ${status.measuredOn.length} open-source ${status.measuredOn.length === 1 ? 'project' : 'projects'}`
          : ''}
        .
      </p>
      <p className="mt-1 text-fd-muted-foreground">{tierView[status.tier]}</p>
    </div>
  );
}

/** Every rule with its family, tier and how often it was right, ordered by tier. */
export function RuleTable() {
  const rows = rules
    .map((rule) => ({ rule, status: statusOf(rule.id) }))
    .sort((a, b) => tierOrder.indexOf(a.status.tier) - tierOrder.indexOf(b.status.tier));

  return (
    <div className="not-prose my-6 overflow-x-auto rounded-xl border border-fd-border">
      <table className="w-full text-left text-sm">
        <thead className="text-fd-muted-foreground">
          <tr className="border-b border-fd-border">
            <th className="px-4 py-2.5 font-normal">Rule</th>
            <th className="px-4 py-2.5 font-normal">Family</th>
            <th className="px-4 py-2.5 font-normal">Tier</th>
            <th className="px-4 py-2.5 text-right font-normal">True / decided</th>
          </tr>
        </thead>
        <tbody>
          {rows.map(({ rule, status }) => (
            <tr key={rule.id} className="border-b border-fd-border last:border-0">
              <td className="px-4 py-2.5 font-mono text-[13px]">
                <Link href={`/docs/rules/${rule.id}`} className="text-fd-foreground hover:text-glow">
                  {rule.id}
                </Link>
              </td>
              <td className="px-4 py-2.5 text-fd-muted-foreground">{familyOf(rule).title}</td>
              <td className="px-4 py-2.5">
                <TierBadge tier={status.tier} />
              </td>
              <td className="px-4 py-2.5 text-right font-mono text-[13px] tabular-nums text-fd-muted-foreground">
                {status.decided === 0 ? 'none' : `${status.truths} / ${status.decided}`}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

/** Why a rule is hidden, shown only while its measured tier is hidden. */
export function HiddenReason({ rule, children }: { rule: string; children: React.ReactNode }) {
  if (statusOf(rule).tier !== 'hidden') return null;
  return (
    <div className="not-prose my-6 rounded-xl border border-fd-border p-4 text-sm leading-relaxed text-fd-muted-foreground">
      <p className="mb-1 font-medium text-fd-foreground">Hidden by default</p>
      {children}
      <p className="mt-2">
        It still runs: <code className="font-mono">spirolo check --all</code> shows its findings, labelled, and it never
        fails the build, even with an override.
      </p>
    </div>
  );
}
