import type { Config } from "tailwindcss";

export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        ink: {
          50: "#f7f7f8",
          100: "#ececef",
          200: "#d4d4da",
          300: "#a8a8b3",
          400: "#7a7a87",
          500: "#54545f",
          600: "#3a3a43",
          700: "#26262d",
          800: "#1a1a1f",
          900: "#0e0e12",
        },
        // Pastel palette introduced in v0.3.0. The old `accent` was a
        // single dusty blue; the logo already carried a richer
        // gradient that nothing else used. v0.3.0 promotes a soft
        // pastel lavender as primary accent and adds the gradient
        // stops + pastel status hues as `brand.*` tokens. White text
        // reads poorly on these lightnesses, so primary buttons use
        // dark ink text (see `.btn-primary` in index.css).
        accent: {
          DEFAULT: "#B5ACE5", // pastel lavender
          hover: "#9F94D6", // deeper lavender for hover
        },
        brand: {
          sky: "#A7C7E7", // pastel sky blue
          lavender: "#B5ACE5",
          blush: "#F2C6D1", // pastel blush — recording dot
          mint: "#A8D5BA", // pastel sage — granted/success
          amber: "#E8C97C", // pastel honey — not_determined
          rose: "#E08E96", // muted rose — denied
        },
      },
      fontFamily: {
        sans: [
          "ui-sans-serif",
          "-apple-system",
          "BlinkMacSystemFont",
          "Inter",
          "system-ui",
          "sans-serif",
        ],
      },
    },
  },
  plugins: [],
} satisfies Config;
