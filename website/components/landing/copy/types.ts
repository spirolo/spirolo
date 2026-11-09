// Every word the home page shows, per language. A page passes one LandingCopy down to its bands, so
// / and /zh share one composition and can only differ in words. `{name}` marks a value filled at
// render time (numbers from data/, the product name): a translation keeps every placeholder.
import type { FamilyId } from '@/lib/rules';
import type { Tier } from '@/lib/status';

interface Point {
  lead: string;
  title: string;
  text: string;
}

interface Layer {
  name: string;
  job: string;
  who: string;
}

export interface LandingCopy {
  /** BCP 47 tag for the page's lang attribute. */
  lang: string;
  /** Number grouping for counts. */
  numbers: string;
  meta: { title: string; description: string };
  nav: { docs: string; language: { text: string; url: string } };
  footer: { licence: string; docs: string; rules: string; source: string };
  install: { copy: string; copied: string; copyCommand: string };
  stickers: { bugInAJar: string; cleanRun: string };
  hero: { eyebrow: string; title: string; lead: string; link: string };
  replay: {
    findings: string;
    total: string;
    read: string;
    kinds: { 'absence-anchor': string; 'fail-closed-setup': string; 'dead-skip': string; 'duplicate-test': string };
    table: { project: string; tests: string; findings: string; readIn: string };
    caption: string;
  };
  value: { title: string; intro: string; points: [Point, Point, Point, Point, Point, Point] };
  families: {
    title: string;
    intro: string;
    link: string;
    after: string;
    /** `strong` must appear verbatim in `question`: it is the part set in bright text. */
    tiles: Record<FamilyId, { title: string; question: string; strong: string; stat: string }>;
  };
  health: { title: string; text: string; link: string; gradeLabel: string; outOf: string };
  layers: { title: string; intro: string; checkedBy: string; layers: [Layer, Layer, Layer, Layer] };
  defaultView: {
    title: string;
    intro: string;
    link: string;
    after: string;
    viewLabel: string;
    tierHeading: Record<Tier, string>;
    tierName: Record<Tier, string>;
    none: string;
    folded: string;
    foldedGroup: string;
    foldedJoin: string;
    foldedTail: string;
  };
  questions: { title: string; faqs: { question: string; answer: string }[] };
  closing: { title: string; text: string; link: string };
}

/** Fills each `{name}` from `values`; an unknown name is left as is, so a typo shows on the page. */
export function fill(template: string, values: Record<string, string | number> = {}): string {
  return template.replace(/\{(\w+)\}/g, (whole, name: string) => (name in values ? String(values[name]) : whole));
}
