import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "OCR Scan Tool",
  description: "Capture a screen region and auto-detect barcode/QR or OCR text",
};

export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    <html lang="en" className="h-full antialiased">
      <body className="min-h-full flex flex-col bg-zinc-950">{children}</body>
    </html>
  );
}
