const { scopedPreflightStyles } = require('tailwindcss-scoped-preflight');

/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["./src/**/*.rs"],
  theme: {
    extend: {
      colors: {
        'background': '#18181B', // zinc-900
        'foreground': '#FAFAFA', // zinc-50
        'accent': '#3F3F46',     // zinc-700
        'border': '#52525B',     // zinc-600
        'success-background': '#166534', // green-800
        'success-foreground': '#ECFCCB', // green-100
        'error-background': '#991B1B',   // red-800
        'error-foreground': '#FEE2E2',   // red-100
        'info-background': '#075985',    // sky-800
        'info-foreground': '#E0F2FE',    // sky-100
      },
    },
  },
   plugins: [
    scopedPreflightStyles({
        cssSelector: '.leptos-query-devtools', // or .tailwind-preflight or even [data-twp=true] - any valid CSS selector of your choice
        mode: 'matched only', // it's the default
    }),
],
}