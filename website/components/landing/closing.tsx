// The closing: the question, the sticker, and the commands.
import Link from 'next/link';
import { InstallCommand } from '@/components/install-command';
import { Sticker } from '@/components/sticker';
import { installCommand, product, runCommand } from '@/lib/product';
import { stickers } from '@/lib/rules';
import { findingCount, repos } from './runs';
import { Band } from './shared';

export function Closing() {
  return (
    <Band ground="plum">
      <div className="grid items-center gap-10 md:grid-cols-2">
        <Sticker art={stickers.cleanRun} className="mx-auto w-full max-w-sm" />
        <div className="home-reveal flex min-w-0 flex-col gap-5">
          <h2 className="text-3xl font-semibold tracking-tight sm:text-4xl">
            What would {product.name} find in your test suite?
          </h2>
          <p className="leading-relaxed text-fd-muted-foreground">
            Your tests are the last check before production. In {repos.length} popular open-source projects it found{' '}
            {findingCount} findings, each project read in seconds. Install your project&rsquo;s dependencies first, then
            run it from the root.
          </p>
          <InstallCommand command={installCommand} />
          <InstallCommand command={runCommand} />
          <Link href="/docs/install" className="text-sm font-medium text-glow hover:underline">
            Install and first run
          </Link>
        </div>
      </div>
    </Band>
  );
}
