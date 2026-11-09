import Link from 'next/link';
import { Sticker } from '@/components/sticker';
import { stickers } from '@/lib/rules';

export default function NotFound() {
  return (
    <main className="flex flex-1 flex-col items-center justify-center gap-6 px-4 py-24 text-center">
      <Sticker art={stickers.unimpressed} className="w-36" />
      <h1 className="text-2xl font-semibold">Nothing at this address.</h1>
      <Link href="/docs" className="text-glow hover:underline">
        Go to the docs
      </Link>
    </main>
  );
}
