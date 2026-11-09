import type { BaseLayoutProps } from 'fumadocs-ui/layouts/shared';
import { product } from './product';

export function Wordmark() {
  return (
    <img src="/brand/logo/lockup-horizontal-glow-motion.svg" alt={product.name} width={94} height={32} />
  );
}

/** The home pages pass their language's labels and the switch to the other language; docs pages pass nothing. */
export function baseOptions(nav?: { docs: string; language: { text: string; url: string } }): BaseLayoutProps {
  return {
    nav: { title: <Wordmark /> },
    links: [
      { text: nav?.docs ?? 'Docs', url: '/docs', active: 'nested-url' },
      ...(nav ? [{ text: nav.language.text, url: nav.language.url }] : []),
    ],
    githubUrl: product.repoUrl,
    themeSwitch: { enabled: false },
  };
}
