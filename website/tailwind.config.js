/** @type {import('tailwindcss').Config} */
module.exports = {
  presets: [require('./theme/tailwind.preset.js')],
  content: [
    "./src/**/*.rs",
    "./index.html",
  ],
  plugins: [],
};
