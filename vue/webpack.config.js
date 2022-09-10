module.exports = {
  rules: [
    {
      test: /\.pug$/,
      loader: 'pug-plain-loader'
    },
    {
      test: /\.scss$/,
      use: [
        'vue-style-loader',
        'css-loader',
        {
          loader: 'sass-loader',
          options: {
            data: "@import '@/styles/variables.scss';"
          },
        },
      ],
    },
  ]
}