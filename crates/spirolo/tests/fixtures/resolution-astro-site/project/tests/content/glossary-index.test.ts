/** The glossary helpers in `src/lib/glossary.ts`, asked directly. */

import { describe, expect, it } from 'vitest';

import { byInitial, firstSentence } from '../../src/lib/glossary';

describe('firstSentence', () => {
  it('stops at the end of the first sentence', () => {
    expect(firstSentence('A proof the editor signs. It is then final.')).toBe(
      'A proof the editor signs.',
    );
  });

  it('keeps the whole text when no sentence ends', () => {
    expect(firstSentence('A paper without coating')).toBe('A paper without coating');
  });
});

describe('byInitial', () => {
  it('groups terms under their initial, in order', () => {
    const groups = byInitial(['binding', 'Apex', 'bleed'], (term) => term, 'en');
    expect(groups.map((group) => group.letter)).toEqual(['A', 'B']);
    expect(groups[1].entries).toEqual(['binding', 'bleed']);
  });
});
