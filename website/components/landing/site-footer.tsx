// The home pages' footer, in the page's language, with the switch to the other one.
import Link from 'next/link';
import { product } from '@/lib/product';
import type { LandingCopy } from './copy/types';

export function SiteFooter({ copy }: { copy: LandingCopy }) {
  return (
    <footer className="border-t border-fd-border">
      <div className="mx-auto flex w-full max-w-6xl flex-wrap items-center justify-between gap-4 px-4 py-8 text-sm text-fd-muted-foreground sm:px-6">
        <span>
          {product.name} {product.version} · {copy.footer.licence}
        </span>
        <nav className="flex flex-wrap gap-5">
          <Link href="/docs" className="hover:text-fd-foreground">
            {copy.footer.docs}
          </Link>
          <Link href="/docs/rules" className="hover:text-fd-foreground">
            {copy.footer.rules}
          </Link>
          <a href={product.repoUrl} className="hover:text-fd-foreground">
            {copy.footer.source}
          </a>
          <a href="/llms.txt" className="hover:text-fd-foreground">
            llms.txt
          </a>
          <Link href={copy.nav.language.url} hrefLang={copy.nav.language.url === '/zh' ? 'zh-CN' : 'en'} className="hover:text-fd-foreground">
            {copy.nav.language.text}
          </Link>
        </nav>
      </div>
    </footer>
  );
}
