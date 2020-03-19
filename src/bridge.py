import eel
import ctypes
import urllib
import os

from src.google_api import GoogleApi

# TODO: Potentially redundant.
# TODO: Can just import eel in GoogleApi class and decorate that method
@eel.expose
def is_authenticated():
    return GoogleApi.is_authenicated()

@eel.expose
def sign_in():
    GoogleApi.ensure_valid_token()

@eel.expose
def get_albums():
    return []

@eel.expose
def get_album_media_items(album_id):
    return []

@eel.expose
def get_media_item(item_id):
    return {}

@eel.expose
def set_wallpaper(image_url):

    print('change image to ' + image_url)

    # Download full res image
    urllib.request.urlretrieve(image_url, 'storage/wall.jpg')
    path = os.path.abspath('storage/wall.jpg')
    SPI = 20
    SPIF = 2
    ctypes.windll.user32.SystemParametersInfoW(SPI, 0, path, SPIF)