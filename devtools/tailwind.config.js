const { scopedPreflightStyles } = require('tailwindcss-scoped-preflight');
const colors = require('tailwindcss/colors')

/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["./src/**/*.rs"],
  theme: {
    extend: {
      colors: {
        'background': colors.zinc[900],
        'foreground': colors.zinc[100], 
        'accent': colors.zinc[800], 
        'border': colors.zinc[700],     

        'input': colors.zinc[700],      
        'input-foreground': colors.zinc[300] 
      },
    },
  },
   plugins: [
    require("@tailwindcss/forms")({
      strategy: 'class', // only generate classes
    }),
    scopedPreflightStyles({
        cssSelector: '.leptos-query-devtools', // or .tailwind-preflight or even [data-twp=true] - any valid CSS selector of your choice
        mode: 'matched only', // it's the default
    }),
],
}