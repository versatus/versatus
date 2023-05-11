/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    './pages/**/*.{js,ts,jsx,tsx}',
    './components/**/*.{js,ts,jsx,tsx}',
    './app/**/*.{js,ts,jsx,tsx}',
  ],
  theme: {
    colors: {
      // VRRB
      'neon-earth': '#BDFF51',
      earth: '#88DC00',
      'earth-low': '#00372D',
      'neon-mars': '#FF5E96',
      mars: '#EE2C70',
      'mars-low': '#48001A',
      'neon-venus': '#37E7FF',
      venus: '#37C7FF',
      'venus-low': '#061072',
      neptune: '#6F54F7',
      'neptune-low': '#260037',
      saturn: '#FFE059',
      'saturn-low': '#371100',
      obsidian: '#22192C',

      white: '#FFFFFF',
      paper: '#F2F2F2',
      grey: {
        100: '#A2A2A2',
        200: '#6D6D6D',
        300: '#3E3E3E',
        400: '#1C1C1C',
      },
    },
    extend: {
      borderColor: (theme) => ({
        DEFAULT: theme('colors.grey.300'),
      }),
      backgroundImage: (theme) => ({
        solar: `linear-gradient(332.11deg, ${theme(
          'colors.mars'
        )} 29.84%,  ${theme('colors.saturn')} 75%)`,
        'neon-solar': `linear-gradient(332.11deg, ${theme(
          'colors.mars'
        )} 29.84%,  ${theme('colors.neon-mars')} 75%)`,
        energy: `linear-gradient(332.11deg, ${theme(
          'colors.earth'
        )} 29.84%,  ${theme('colors.saturn')} 75%)`,
        'neon-energy': `linear-gradient(332.11deg, ${theme(
          'colors.earth'
        )} 29.84%,  ${theme('colors.neon-earth')} 75%)`,
        bonding: `linear-gradient(332.11deg, ${theme(
          'colors.neptune'
        )} 29.84%,  ${theme('colors.venus')} 75%)`,
        'neon-bonding': `linear-gradient(332.11deg, ${theme(
          'colors.venus'
        )} 29.84%,  ${theme('colors.neon-venus')} 75%)`,
      }),
    },
  },
  plugins: [require('daisyui')],
}
