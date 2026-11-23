import type { BaseLayoutProps } from 'fumadocs-ui/layouts/shared';
import { StarIcon } from '@/components/star-link';
import { product } from './product';

export function Wordmark() {
  return (
    <img src="/brand/logo/lockup-horizontal-glow-motion.svg" alt={product.name} width={94} height={32} />
  );
}

/** The home pages pass their language's labels and the switch to the other language; docs pages pass nothing. */
export function baseOptions(nav?: { docs: string; star: string; language: { text: string; url: string } }): BaseLayoutProps {
  return {
    nav: { title: <Wordmark /> },
    links: [
      { text: nav?.docs ?? 'Docs', url: '/docs', active: 'nested-url' },
      ...(nav ? [{ text: nav.language.text, url: nav.language.url }] : []),
      // In place of the bare GitHub icon: the same link, saying what it asks for.
      {
        type: 'button',
        secondary: true,
        url: product.repoUrl,
        external: true,
        // The layout draws a button's `icon` only in the phone menu, so the star sits in the text.
        text: (
          <span className="inline-flex items-center gap-1.5">
            <StarIcon />
            {nav?.star ?? 'Star on GitHub'}
          </span>
        ),
      },
    ],
    themeSwitch: { enabled: false },
  };
}
