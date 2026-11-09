import type { Metadata } from 'next';
import { en } from '@/components/landing/copy/en';
import { Landing } from '@/components/landing/landing';
import './home.css';

export const metadata: Metadata = {
  alternates: { canonical: '/', languages: { en: '/', 'zh-CN': '/zh' } },
};

export default function HomePage() {
  return <Landing copy={en} />;
}
