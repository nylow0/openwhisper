/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/renderer/**/*.{svelte,html}'],
  theme: {
    extend: {
      fontFamily: {
        sans: [
          'Inter',
          'ui-sans-serif',
          'system-ui',
          '-apple-system',
          'BlinkMacSystemFont',
          'Segoe UI',
          'sans-serif',
        ],
      },
      keyframes: {
        'ring-pulse': {
          '0%': { transform: 'scale(0.96)', opacity: '0.5' },
          '70%': { opacity: '0' },
          '100%': { transform: 'scale(1.5)', opacity: '0' },
        },
        breathe: {
          '0%, 100%': { opacity: '0.35' },
          '50%': { opacity: '1' },
        },
        rise: {
          '0%': { opacity: '0', transform: 'translateY(6px)' },
          '100%': { opacity: '1', transform: 'translateY(0)' },
        },
        waveform: {
          '0%, 100%': { transform: 'scaleY(0.35)' },
          '50%': { transform: 'scaleY(1)' },
        },
      },
      animation: {
        'ring-pulse': 'ring-pulse 1.8s cubic-bezier(0.4, 0, 0.6, 1) infinite',
        breathe: 'breathe 1.6s ease-in-out infinite',
        rise: 'rise 0.3s ease-out both',
        waveform: 'waveform 0.85s ease-in-out infinite',
      },
    },
  },
  plugins: [],
};
