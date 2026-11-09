import Link from 'next/link';
import { HomeLayout } from 'fumadocs-ui/layouts/home';
import { baseOptions } from '@/lib/layout.shared';
import { product } from '@/lib/product';

function Footer() {
  return (
    <footer className="border-t border-fd-border">
      <div className="mx-auto flex w-full max-w-6xl flex-wrap items-center justify-between gap-4 px-4 py-8 text-sm text-fd-muted-foreground sm:px-6">
        <span>
          {product.name} {product.version} · Apache 2.0 licence
        </span>
        <nav className="flex gap-5">
          <Link href="/docs" className="hover:text-fd-foreground">
            Docs
          </Link>
          <Link href="/docs/rules" className="hover:text-fd-foreground">
            Rules
          </Link>
          <a href={product.repoUrl} className="hover:text-fd-foreground">
            Source
          </a>
          <a href="/llms.txt" className="hover:text-fd-foreground">
            llms.txt
          </a>
        </nav>
      </div>
    </footer>
  );
}

export default function Layout({ children }: LayoutProps<'/'>) {
  return (
    <HomeLayout {...baseOptions()}>
      {children}
      <Footer />
    </HomeLayout>
  );
}
