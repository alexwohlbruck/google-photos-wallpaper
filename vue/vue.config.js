const path = require('path')

module.exports = {
  outputDir: path.resolve(__dirname, '../web'),
  transpileDependencies: [
    'vuetify'
  ],
}