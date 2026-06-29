import './globals.css';

// Root layout is intentionally a pass-through: the real <html>/<body> and all
// locale-aware metadata live in [locale]/layout.tsx so next-intl can resolve
// the request locale. (Same split as reality-web.)
export default function RootLayout({ children }: { children: React.ReactNode }) {
  return children;
}
