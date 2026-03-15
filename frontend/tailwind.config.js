/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{vue,js}'],
  theme: {
    extend: {
      colors: {
        panel: {
          bg: '#1a1a2e',
          header: '#16213e',
          border: '#0f3460',
          text: '#e0e0e0',
          accent: '#e94560',
          muted: '#888',
        },
      },
      fontFamily: {
        mono: ['"JetBrains Mono"', '"Fira Code"', 'Consolas', 'monospace'],
      },
    },
  },
  plugins: [],
}
