// The hero: the message on the left, the mascot on the right with what it caught. On a phone the mascot
// comes first, large.
import Link from 'next/link';
import { InstallCommand } from '@/components/install-command';
import { Sticker } from '@/components/sticker';
import { installCommand, product, runCommand } from '@/lib/product';
import { stickers } from '@/lib/rules';
import { fill, type LandingCopy } from './copy/types';

export function Hero({ copy }: { copy: LandingCopy }) {
  return (
    <div className="mx-auto w-full max-w-6xl px-4 pb-16 pt-8 sm:px-6 sm:pb-20 sm:pt-14">
      <div className="grid items-center gap-10 lg:grid-cols-[minmax(0,3fr)_minmax(0,2fr)]">
        <div className="flex flex-col items-start gap-6 text-left">
          <p className="text-sm text-fd-muted-foreground">{copy.hero.eyebrow}</p>
          <h1 className="text-4xl font-semibold leading-[1.08] tracking-tight sm:text-6xl">
            {copy.hero.title}
          </h1>
          <p className="text-lg leading-relaxed text-fd-muted-foreground">
            {fill(copy.hero.lead, { product: product.name })}
          </p>
          <div className="flex w-full max-w-md flex-col gap-3">
            <InstallCommand command={installCommand} labels={copy.install} />
            <InstallCommand command={runCommand} labels={copy.install} />
          </div>
          <Link href="/docs/install" className="text-sm font-medium text-glow hover:underline">
            {copy.hero.link}
          </Link>
        </div>
        <Sticker art={{ ...stickers.bugInAJar, alt: copy.stickers.bugInAJar }} className="order-first mx-auto w-full max-w-sm lg:order-none lg:max-w-md" />
      </div>
    </div>
  );
}
