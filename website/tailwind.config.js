/** @type {import('tailwindcss').Config} */
module.exports = {
  presets: [require('../../soma-ui/web/theme/tailwind.preset.js')],
  content: [
    "./src/**/*.rs",
    "../../soma-ui/web/packages/ui/src/**/*.rs",
  ],
  plugins: [],
};
