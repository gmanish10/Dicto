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
        accent: {
          DEFAULT: "#5b8def",
          hover: "#4a7be0",
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
