import type { Metadata } from 'next';
import { IBM_Plex_Mono, IBM_Plex_Sans } from 'next/font/google';
import { Provider } from '@/components/provider';
import { en } from '@/components/landing/copy/en';
import { fill } from '@/components/landing/copy/types';
import { product } from '@/lib/product';
import './global.css';

// Plex Sans sets every page text and heading. The wordmark is outlined in the logo files, so Fredoka is not loaded.
const plexSans = IBM_Plex_Sans({ subsets: ['latin'], weight: ['400', '600'], variable: '--font-plex-sans' });
const plexMono = IBM_Plex_Mono({ subsets: ['latin'], weight: ['400', '500'], variable: '--font-plex-mono' });

const description = fill(en.meta.description, { product: product.name });

// Vercel names the production host; anything else (a local build) falls back to localhost.
const siteUrl = process.env.VERCEL_PROJECT_PRODUCTION_URL
  ? `https://${process.env.VERCEL_PROJECT_PRODUCTION_URL}`
  : 'http://localhost:3000';

export const metadata: Metadata = {
  metadataBase: new URL(siteUrl),
  title: { default: fill(en.meta.title, { product: product.name }), template: `%s · ${product.name}` },
  description,
  openGraph: { title: product.name, description, images: '/brand/scenes/bug-in-a-jar.webp' },
};

export default function Layout({ children }: LayoutProps<'/'>) {
  return (
    <html lang="en" className={`dark ${plexSans.variable} ${plexMono.variable}`} suppressHydrationWarning>
      <body className="flex min-h-screen flex-col bg-night font-sans">
        <Provider>{children}</Provider>
      </body>
    </html>
  );
}
