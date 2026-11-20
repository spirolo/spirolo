// corpus/rule-status.json is the one source of every published measurement. The binary embeds
// it, the README tables are generated from it and the website reads a copy of it at build time.
// What is left is prose and sample output written by hand; this module holds those to the same
// table, so a refreshed measurement cannot leave an older figure or claim standing somewhere.
export const tiers = ['proven', 'advisory', 'experimental', 'hidden'];

/** The floors of the default view, read from the Rust source the binary is built from. */
export function tierFloors(statusSource) {
  const constant = name => {
    const match = statusSource.match(new RegExp(`pub const ${name}: [^=]+= ([^;]+);`));
    if (!match) throw new Error(`crates/spirolo-rules/src/status.rs no longer declares ${name}`);
    return match[1];
  };
  const [numerator, denominator] = constant('SHOWN_PRECISION_FLOOR').match(/^\((\d+), (\d+)\)$/).slice(1).map(Number);
  return { decided: Number(constant('SHOWN_DECIDED_FLOOR')), numerator, denominator };
}

export function tierOf(row, floors) {
  if (row.verdict === 'PASS') return 'proven';
  if (['FAIL', 'DROPPED'].includes(row.verdict)) return 'hidden';
  const decided = row.truths + row.falses;
  return decided >= floors.decided && row.truths * floors.denominator >= decided * floors.numerator
    ? 'advisory' : 'experimental';
}

/** The trust label `spirolo check` prints after a rule's banner. */
export const trustLabel = (row, floors) =>
  `${tierOf(row, floors)}: ${row.truths}/${row.truths + row.falses} (${row.verdict})`;

// A rule banner in a sample report: `● rule (2 files, 3 findings) · ... · advisory: 18/18 (INSUFFICIENT)`.
const banner = new RegExp(`(● ([a-z][a-z-]*) \\([^)\\n]*\\)[^\\n]*? · )(?:${tiers.join('|')}): \\d+/\\d+ \\([A-Z]+\\)`, 'g');
// Sentences that are only true while no rule has passed.
const nothingProven = [/no rule is proven/i, /还没有(?:任何)?规则达到/, /尚无规则达到/];
const otherVersion = /\b\d+\.\d+\.\d+-[a-z]+\.\d+\b/g;

/**
 * Brings the hand-written text of one file in line with the table. Returns the corrected text
 * (banner labels are rewritten) and the claims only a person can rewrite.
 */
export function alignClaims(path, text, { status, floors, version }) {
  const problems = [];
  const aligned = text.replace(banner, (whole, lead, rule) => {
    const row = status.rules[rule];
    if (!row) {
      problems.push(`${path}: sample output names ${rule}, which the status table does not measure`);
      return whole;
    }
    return `${lead}${trustLabel(row, floors)}`;
  });
  const proven = Object.entries(status.rules).filter(([, row]) => row.verdict === 'PASS').map(([name]) => name);
  aligned.split('\n').forEach((line, index) => {
    if (proven.length > 0 && nothingProven.some(pattern => pattern.test(line))) {
      problems.push(`${path}:${index + 1} says no rule is proven, but the table has ${proven.join(', ')} at PASS`);
    }
    // A tag in an instruction (`v0.1.0-alpha.1`) is an example, not a statement about this release.
    for (const [found] of line.matchAll(otherVersion)) {
      if (found !== version && !line.includes(`v${found}`)) {
        problems.push(`${path}:${index + 1} names version ${found}; this release is ${version}`);
      }
    }
  });
  return { aligned, problems };
}

const number = value => value === null ? 'n/a' : value.toFixed(3);
const words = {
  en: {
    intro: (status, floors) => [
      `Measured at spirolo commit \`${status.spirolo_commit}\` across ${status.entries.length} OSS dev entries.`,
      `Advisory: at least ${floors.decided} decided and shown precision at least ${(floors.numerator / floors.denominator).toFixed(2)}.`,
    ],
    head: '| Rule | Tier | Status | True / decided | Precision | 95% Wilson interval | Measured entries |',
  },
  zh: {
    intro: (status, floors) => [
      `测量基于 Spirolo 提交 \`${status.spirolo_commit}\`，覆盖 ${status.entries.length} 个开源项目开发集条目。`,
      `提示级：至少有 ${floors.decided} 个已判定的发现项，且表中精确率至少为 ${(floors.numerator / floors.denominator).toFixed(2)}。`,
    ],
    head: '| 规则 | 级别 | 状态 | 判定为正确 / 已判定 | 精确率 | 95% Wilson 置信区间 | 测量条目数 |',
  },
};

/** The precision table of a README, in its language. */
export function precisionTable(status, floors, language) {
  const rows = Object.entries(status.rules)
    .sort(([a, x], [b, y]) => tiers.indexOf(tierOf(x, floors)) - tiers.indexOf(tierOf(y, floors)) || a.localeCompare(b))
    .map(([name, row]) =>
      `| ${name} | ${tierOf(row, floors)} | ${row.verdict} | ${row.truths} / ${row.truths + row.falses} | ${number(row.precision)} | ${row.interval ? row.interval.map(number).join('–') : 'n/a'} | ${row.measured_on?.length ?? 0} |`);
  return [...words[language].intro(status, floors), '', words[language].head,
    '| --- | --- | --- | ---: | ---: | --- | ---: |', ...rows].join('\n');
}

/**
 * Files whose words are published and speak about this release: docs, READMEs, site copy. The
 * changelog is history and the security policy lists versions on purpose.
 */
export const speaksOfRelease = file =>
  (/\.mdx?$/.test(file) || /^website\/components\/landing\/copy\//.test(file))
  && !['CHANGELOG.md', 'SECURITY.md'].includes(file) && !/(?:^|\/)fixtures\//.test(file) && !file.startsWith('corpus/');
