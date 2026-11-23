// The closing: the question, the sticker, the commands, and the two things a reader can give back:
// a star, or a report of a wrong finding.
import Link from 'next/link';
import { InstallCommand } from '@/components/install-command';
import { StarLink } from '@/components/star-link';
import { Sticker } from '@/components/sticker';
import { installCommand, product, runCommand } from '@/lib/product';
import { stickers } from '@/lib/rules';
import { findingCount, repos } from './runs';
import { fill, type LandingCopy } from './copy/types';
import { Band } from './shared';

export function Closing({ copy }: { copy: LandingCopy }) {
  return (
    <Band ground="plum">
      <div className="grid items-center gap-10 md:grid-cols-2">
        <Sticker art={{ ...stickers.cleanRun, alt: copy.stickers.cleanRun }} className="mx-auto w-full max-w-sm" />
        <div className="home-reveal flex min-w-0 flex-col gap-5">
          <h2 className="text-3xl font-semibold tracking-tight sm:text-4xl">
            {fill(copy.closing.title, { product: product.name })}
          </h2>
          <p className="leading-relaxed text-fd-muted-foreground">
            {fill(copy.closing.text, { repos: repos.length, findings: findingCount })}
          </p>
          <InstallCommand command={installCommand} labels={copy.install} />
          <InstallCommand command={runCommand} labels={copy.install} />
          <Link href="/docs/install" className="text-sm font-medium text-glow hover:underline">
            {copy.closing.link}
          </Link>
          <div className="mt-2 flex flex-col gap-4 border-t border-fd-border pt-6">
            <p className="text-sm leading-relaxed text-fd-muted-foreground">
              {fill(copy.closing.ask, { product: product.name })}
            </p>
            <div className="flex flex-wrap items-center gap-x-5 gap-y-3">
              <StarLink>{copy.closing.star}</StarLink>
              <a href={`${product.repoUrl}/issues/new?template=false-positive.yml`} className="text-sm font-medium text-glow hover:underline">
                {copy.closing.report}
              </a>
            </div>
          </div>
        </div>
      </div>
    </Band>
  );
}
