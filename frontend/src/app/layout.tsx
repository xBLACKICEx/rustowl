import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "visuarust",
  description: "Visualize ownership and lifetimes in Rust",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
