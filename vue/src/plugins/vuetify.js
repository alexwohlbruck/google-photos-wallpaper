import Vue from 'vue';
import Vuetify, { colors } from 'vuetify/lib';

Vue.use(Vuetify);

export default new Vuetify({
  theme: {
    dark: true,
    options: {
      customProperties: true,
    },
    themes: {
      light: {
        primary: colors.blue.darken1,
        secondary: colors.blue.darken4,
        accent: colors.blue.accent2,
      },
      dark: {
        primary: colors.blue.darken1,
        secondary: colors.blue.darken4,
        accent: colors.blue.accent2,
      },
    },
  },
});
