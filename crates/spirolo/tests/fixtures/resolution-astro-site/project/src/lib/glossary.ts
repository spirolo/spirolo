/** Small helpers for a glossary page: the summary beside a term and its index letter. */

/** The first sentence of a definition, or the whole text if it has no sentence end. */
export function firstSentence(text: string): string {
  const end = /[.!?](?=\s+\p{Lu})/u.exec(text);
  return (end ? text.slice(0, end.index + 1) : text).trim();
}

/** The terms that begin with one letter, and that letter. */
export interface LetterGroup<T> {
  readonly letter: string;
  readonly entries: readonly T[];
}

/** Terms sorted by name and grouped by their upper-case initial. */
export function byInitial<T>(
  entries: readonly T[],
  name: (entry: T) => string,
  locale: string,
): LetterGroup<T>[] {
  const order = new Intl.Collator(locale);
  const groups: { letter: string; entries: T[] }[] = [];
  for (const entry of [...entries].sort((a, b) => order.compare(name(a), name(b)))) {
    const letter = [...name(entry).trim()][0]?.toLocaleUpperCase(locale) ?? '';
    const last = groups.at(-1);
    if (last && last.letter === letter) last.entries.push(entry);
    else groups.push({ letter, entries: [entry] });
  }
  return groups;
}
