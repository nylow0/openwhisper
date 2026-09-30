/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/renderer/**/*.{svelte,html}'],
  theme: {
    extend: {
      colors: {
        // Blue-tinted near-black scale. 950 is the window background, 900/850/800
        // are raised surfaces, 700 is hairlines, 50 is primary text.
        ink: {
          950: '#0b0d10',
          900: '#0f1216',
          850: '#11151a',
          800: '#151a20',
          700: '#1b2027',
          600: '#2c333d',
          500: '#4a535e',
          400: '#5d6773',
          300: '#7c8792',
          200: '#9aa4b0',
          150: '#aab3bd',
          100: '#c5cdd6',
          50: '#e8edf2',
        },
      },
      backgroundImage: {
        // Signature accent: selection bars, radio dots, switched-on toggles.
        accent: 'linear-gradient(90deg, #3b82f6, #22d3ee)',
      },
      fontFamily: {
        sans: [
          '"Plus Jakarta Sans Variable"',
          '"Segoe UI Variable Text"',
          '"Segoe UI"',
          'system-ui',
          'sans-serif',
        ],
      },
      keyframes: {
        'ring-pulse': {
          '0%': { transform: 'scale(1)', opacity: '0.6' },
          '100%': { transform: 'scale(2.6)', opacity: '0' },
        },
        breathe: {
          '0%, 100%': { opacity: '0.35' },
          '50%': { opacity: '1' },
        },
      },
      animation: {
        'ring-pulse': 'ring-pulse 1.4s cubic-bezier(0, 0, 0.2, 1) infinite',
        breathe: 'breathe 1.6s ease-in-out infinite',
      },
    },
  },
  plugins: [],
};
