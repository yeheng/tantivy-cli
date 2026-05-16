/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{vue,js,ts}",
    "node_modules/flowbite/**/*.js",
    "node_modules/flowbite-vue/**/*.{vue,js,ts}",
  ],
  darkMode: 'class',
  theme: {
    extend: {},
  },
  plugins: [],
}
