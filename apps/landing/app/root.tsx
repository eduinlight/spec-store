import "@workspace/ui/globals.css";
import type { ReactNode } from "react";
import { Links, Meta, Outlet, Scripts, ScrollRestoration, useLocation } from "react-router";

export function Layout({ children }: { children: ReactNode }) {
  const location = useLocation();
  const language = location.pathname.startsWith("/es") ? "es" : "en";
  return (
    <html lang={language}>
      <head>
        <meta charSet="utf-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1" />
        <link rel="icon" type="image/png" sizes="16x16" href="/favicon-16.png" />
        <link rel="icon" type="image/png" sizes="32x32" href="/favicon-32.png" />
        <link rel="icon" type="image/png" sizes="64x64" href="/favicon-64.png" />
        <link rel="apple-touch-icon" sizes="180x180" href="/apple-touch-icon.png" />
        <meta name="theme-color" content="#1F4FE0" />
        <Meta />
        <Links />
      </head>
      <body>
        {children}
        <ScrollRestoration />
        <Scripts />
      </body>
    </html>
  );
}

export default function Root() {
  return <Outlet />;
}
