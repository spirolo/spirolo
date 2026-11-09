// The seven questions as cards. Nothing is hidden behind a click; run numbers come from ./runs.
import { product } from '@/lib/product';
import { repos, seconds, tests } from './runs';
import { fill, type LandingCopy } from './copy/types';
import { Band, Heading } from './shared';

export function Questions({ copy }: { copy: LandingCopy }) {
  const values = {
    product: product.name,
    tests: tests.toLocaleString(copy.numbers),
    repos: repos.length,
    seconds,
    config: product.config,
  };
  const faqs = copy.questions.faqs.map((faq) => ({ question: fill(faq.question, values), answer: fill(faq.answer, values) }));
  return (
    <Band ground="muted">
      <Heading title={copy.questions.title} />
      <dl className="home-reveal mt-10 grid gap-4 md:grid-cols-2">
        {faqs.map((faq) => (
          <div key={faq.question} className="flex min-w-0 flex-col gap-2 rounded-2xl border border-fd-border p-5 sm:p-6">
            <dt className="font-semibold leading-snug">{faq.question}</dt>
            <dd className="text-sm leading-relaxed text-fd-muted-foreground sm:text-base">{faq.answer}</dd>
          </div>
        ))}
      </dl>
    </Band>
  );
}
