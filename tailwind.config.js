/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    './templates/**/*.html',
  ],
  darkMode: 'class',
  theme: {
    extend: {
      fontFamily: {
        mono: ['Courier Prime', 'monospace'],
      }
    },
  },
  plugins: [],
  corePlugins: {}
}
