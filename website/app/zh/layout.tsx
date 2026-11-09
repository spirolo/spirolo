import { HomeLayout } from 'fumadocs-ui/layouts/home';
import { DocumentLang } from '@/components/document-lang';
import { zh } from '@/components/landing/copy/zh';
import { SiteFooter } from '@/components/landing/site-footer';
import { baseOptions } from '@/lib/layout.shared';

export default function Layout({ children }: LayoutProps<'/zh'>) {
  return (
    <div lang={zh.lang} className="flex flex-1 flex-col">
      <DocumentLang lang={zh.lang} />
      <HomeLayout {...baseOptions(zh.nav)}>
        {children}
        <SiteFooter copy={zh} />
      </HomeLayout>
    </div>
  );
}
