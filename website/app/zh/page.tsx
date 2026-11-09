import type { Metadata } from 'next';
import { zh } from '@/components/landing/copy/zh';
import { fill } from '@/components/landing/copy/types';
import { Landing } from '@/components/landing/landing';
import { product } from '@/lib/product';

const description = fill(zh.meta.description, { product: product.name });

export const metadata: Metadata = {
  title: { absolute: fill(zh.meta.title, { product: product.name }) },
  description,
  alternates: { canonical: '/zh', languages: { en: '/', 'zh-CN': '/zh' } },
  openGraph: { title: product.name, description, locale: 'zh_CN', images: '/brand/scenes/bug-in-a-jar.webp' },
};

export default function ChinesePage() {
  return <Landing copy={zh} />;
}
