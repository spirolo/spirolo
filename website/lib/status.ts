// Reads the measured rule table the binary embeds and places each rule in the tier the default
// view uses. The floors mirror SHOWN_DECIDED_FLOOR and SHOWN_PRECISION_FLOOR in
// crates/spirolo-rules/src/status.rs; docs/tiers.md explains them.
import table from './generated/rule-status.json';

export type Tier = 'proven' | 'advisory' | 'experimental' | 'hidden';

export interface RuleStatus {
  rule: string;
  verdict: string;
  tier: Tier;
  truths: number;
  decided: number;
  precision: number | null;
  interval: [number, number] | null;
  measuredOn: string[];
}

interface Row {
  verdict: string;
  precision: number | null;
  interval: number[] | null;
  truths: number;
  falses: number;
  measured_on: string[];
}

const DECIDED_FLOOR = 15;

function tierOf(verdict: string, truths: number, decided: number): Tier {
  if (verdict === 'PASS') return 'proven';
  if (verdict === 'FAIL' || verdict === 'DROPPED') return 'hidden';
  if (decided >= DECIDED_FLOOR && truths * 5 >= decided * 4) return 'advisory';
  return 'experimental';
}

const rows = new Map<string, Row>(Object.entries(table.rules));

export const measuredAt = { commit: table.spirolo_commit as string, entries: (table.entries as string[]).length };

export function statusOf(rule: string): RuleStatus {
  const row = rows.get(rule);
  if (!row) {
    return { rule, verdict: 'UNMEASURED', tier: 'experimental', truths: 0, decided: 0, precision: null, interval: null, measuredOn: [] };
  }
  const decided = row.truths + row.falses;
  return {
    rule,
    verdict: row.verdict,
    tier: tierOf(row.verdict, row.truths, decided),
    truths: row.truths,
    decided,
    precision: row.precision,
    interval: row.interval ? [row.interval[0], row.interval[1]] : null,
    measuredOn: row.measured_on,
  };
}

export const tierOrder: Tier[] = ['proven', 'advisory', 'experimental', 'hidden'];
