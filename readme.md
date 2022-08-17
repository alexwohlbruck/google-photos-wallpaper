<img src="https://i.imgur.com/tzSxumP.jpg" width="50">

# Google Photos Wallpaper

This is a utility that will update your desktop wallpaper from your Google Photos library on a custom schedule.

## Features

- Select the photo albums that you'd like to use, and your wallpaper will change automatically either in sequence or random order.
- Filter photos that are too small, have a poor aspect ratio, or create a custom blacklist.
- Previous, next, and random buttons to quickly change the wallpaper, or select one from the album list.

![App screenshot](https://i.imgur.com/mal73Lk.png)

## Installation

### Production version

There are no executable available for installation yet. They will be available soon on the [releases](https://github.com/alexwohlbruck/google-photos-wallpaper/releases) page.

### Development version

1. To compile the app, first clone the repository in the desired location on your computer.

```sh
git clone https://github.com/alexwohlbruck/google-photos-wallpaper.git

cd google-photos-wallpaper
```

2. Create and activate a python virtual environment.
```sh
python3 -m venv env
source env/Scripts/activate
```

3. Install packages using pip.
```sh
pip3 install -r requirements.txt
```

4. Create a [Google Cloud Platform](https://console.cloud.google.com/) project.
   - Enable the `Photos Library API`.
   - In the credentials tab, create an OAuth 2.0 client ID and download the secrets file
   - Name the file `client_secrets.json` and place in your /src directory.

5. Run the python app.
```sh
python3 main.py
```

### Build and distribute

Build the executable using pyupdater.
```sh
pyupdater build win.spec --app-version=X.X.X
```
Pass the flag `--pyinstaller-log-info` to see pyinstaller build logs.

The app will be built to `/pyu-data/new/gpwallpaper-win-X.X.X.zip`.

## FAQ

### How do I sign in?
The app will automatically open a browser tab with a Google sign in prompt. You may have to manually switch to your browser. After you have signed in, the your album library will be displayed.

### Can I select individual photos?
You can pick photo albums to be used, but I will add an option to add a custom search as a photo source.

### Why does it take so long to load albums?
When you select an album to use for your wallpaper, the app will download the metadata for every photo in the album, and can only do so in intervals of 50 photos. If the album is very large, it will take some time to download all of the data.

### What operating systems are supported?
I am currently developing the app for Windows. Support for Linux and MacOS is possible in the future, but after a full Windows release is done.

## Support

To get support, report a bug, or request features, go to the Issues page of this repository.

## Contribute

If you would like to contribute to the project, I will gladly look over any pull requests. Thank you!

## License

[MIT](https://github.com/alexwohlbruck/google-photos-wallpaper/blob/master/LICENSE)