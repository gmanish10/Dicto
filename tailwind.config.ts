import type { Config } from "tailwindcss";

export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        // Warm-minimal palette introduced in v0.3.1. The pastel
        // lavender/blush/sky that shipped in v0.3.0 was replaced —
        // a dictation app the user keeps open all day reads better
        // with a calmer neutral substrate + a single warm accent.
        //
        // Ink scale is slightly warm-shifted (a touch of brown in the
        // grays) so it harmonizes with the amber accent. Pure cool
        // grays clashed with the amber and made the UI feel
        // uncoordinated.
        ink: {
          50: "#E8F4E1", // light pastel green — main app bg (light mode)
          100: "#F1ECE3",
          200: "#E5DDD0",
          300: "#C7BBA8",
          400: "#8B7E6C",
          500: "#5E5444",
          600: "#3F382C",
          700: "#2A2520", // body text (light), card bg (dark)
          800: "#1F1B18",
          900: "#14110F", // body text (dark), darkest surfaces
        },
        accent: {
          DEFAULT: "#D4894A", // warm amber — single brand color
          hover: "#B8723D",
        },
        brand: {
          // Status pill hues — chosen so they coexist with amber
          // without landing in any of the excluded families
          // (blue / pink / purple / green). Granted is a warm
          // sage-on-the-warm-side that reads as positive without
          // being green; denied is a muted brick; pending is a
          // soft taupe.
          cream: "#F5E9D7", // gradient mid-stop, subtle accent bg
          sand: "#E6D2B5", // hover surfaces, secondary chips
          amber: "#D4894A",
          terracotta: "#A85A38", // recording dot, "configured" pills
          olive: "#8B9670", // "granted" pill (warm, not green-coded)
          brick: "#A04848", // "denied" pill
          taupe: "#8B7960", // "not_determined" pill
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
