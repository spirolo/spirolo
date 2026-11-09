// The sixteen rules, in the families the report groups them under (crates/spirolo-rules: Family).
// `signal` is the README's "What each rule claims" line.

/** A die-cut sticker: transparent ground, so it sits on the page without a box. */
export interface Art {
  src: string;
  width: number;
  height: number;
  alt: string;
}

export const stickers = {
  bugInAJar: { src: '/brand/stickers/bug-in-a-jar.webp', width: 1024, height: 580, alt: 'Spirolo sprints past with a grumpy beetle caught in a jar.' },
  cleanRun: { src: '/brand/stickers/clean-run.webp', width: 1024, height: 742, alt: 'Spirolo plants a flag on a hill of passing test cards, arms crossed.' },
  unimpressed: { src: '/brand/stickers/face-unimpressed.webp', width: 434, height: 543, alt: 'Spirolo, unimpressed.' },
} satisfies Record<string, Art>;

export type FamilyId = 'audit' | 'gate' | 'hygiene' | 'flake' | 'impact';

export interface Family {
  id: FamilyId;
  title: string;
  question: string;
  /** Its interface icon, one of Spirolo's props (see branding/elements/). */
  icon: string;
}

export const families: Family[] = [
  {
    id: 'audit',
    title: 'Weak tests',
    question: 'Would this test notice if the code were wrong?',
    icon: '/brand/elements/icons/jar.svg',
  },
  {
    id: 'gate',
    title: 'Merge gate',
    question: 'Does a broken test actually block the merge?',
    icon: '/brand/elements/icons/flag.svg',
  },
  {
    id: 'hygiene',
    title: 'Unused tests',
    question: 'Which tests add nothing the suite does not already check?',
    icon: '/brand/elements/icons/magnifier.svg',
  },
  {
    id: 'flake',
    title: 'Flaky',
    question: 'Which tests can pass on one run and fail on the next?',
    icon: '/brand/elements/icons/lantern.svg',
  },
  {
    id: 'impact',
    title: 'Health',
    question: 'Where does a new test pay first?',
    icon: '/brand/elements/icons/map.svg',
  },
];

export interface Rule {
  id: string;
  family: FamilyId;
  signal: string;
}

export const rules: Rule[] = [
  { id: 'absence-anchor', family: 'audit', signal: 'An absence assertion lacks a positive anchor for the thing queried.' },
  { id: 'weak-assertion', family: 'audit', signal: 'Assertions check existence or type without pinning a produced value.' },
  { id: 'handler-drift', family: 'audit', signal: 'A request and the handlers intended to intercept it disagree.' },
  { id: 'data-layer-mock', family: 'audit', signal: 'A module mock replaces the data layer rather than the network boundary.' },
  { id: 'generator-walk', family: 'audit', signal: 'A saga test pins generator steps instead of its outcome.' },
  { id: 'fail-closed-setup', family: 'gate', signal: 'Runner setup lacks guards that make broken tests fail.' },
  { id: 'dead-skip', family: 'gate', signal: 'A skip or focus has no linked issue or visible condition watching it.' },
  { id: 'gate-truth', family: 'gate', signal: 'Claimed merge gates disagree with configs and CI workflows.' },
  { id: 'duplicate-test', family: 'hygiene', signal: 'Test bodies have the same normalized structure.' },
  { id: 'redundant-test', family: 'hygiene', signal: 'Two tests kill the same mutants in an optional mutation report.' },
  { id: 'unused-mock', family: 'hygiene', signal: 'A mock is unreachable or has no modeled consumer.' },
  { id: 'orphan-test', family: 'hygiene', signal: 'A test is not collected by a runner gate, or a project import is unresolved.' },
  { id: 'stale-snapshot', family: 'hygiene', signal: 'A stored snapshot has no modeled test left to produce it.' },
  { id: 'flake-candidate', family: 'flake', signal: 'A test asserts an elapsed-time budget or a calendar value it re-reads from the clock.' },
  { id: 'flake-observed', family: 'flake', signal: 'An optional run ledger records both pass and fail on the same tree.' },
  { id: 'untested-hotspot', family: 'impact', signal: 'Churn and complexity rank a module above the strength of its tests.' },
];

export function familyOf(rule: Rule): Family {
  return families.find((family) => family.id === rule.family)!;
}
