import { HomeLayout } from 'fumadocs-ui/layouts/home';
import { en } from '@/components/landing/copy/en';
import { SiteFooter } from '@/components/landing/site-footer';
import { baseOptions } from '@/lib/layout.shared';

export default function Layout({ children }: LayoutProps<'/'>) {
  return (
    <HomeLayout {...baseOptions(en.nav)}>
      {children}
      <SiteFooter copy={en} />
    </HomeLayout>
  );
}
