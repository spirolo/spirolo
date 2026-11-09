// The two lines of the real report the page quotes, read from data/sample-check.json.
import sample from '@/data/sample-check.json';

const lines = sample.output.split('\n');
const line = (match: RegExp) => lines.find((text) => match.test(text)) ?? '';

export const report = { health: line(/^Health /) };
