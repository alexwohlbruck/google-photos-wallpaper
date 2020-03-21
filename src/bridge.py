import eel
import ctypes
import urllib
import os

from src.google_api import GoogleApi
from src.options import Options

def get_large_url(media_item):
    # Helper function to get the max size image url
    try:
        width = media_item['mediaMetadata']['width']
        height = media_item['mediaMetadata']['height']
    except (AttributeError, KeyError):
        width = height = '16383'

    return media_item['baseUrl'] + '=w' + width + '-h' + height


# These methods are exposed to the Vue app
# They are a bridge between JS and Python code

@eel.expose
def is_authenticated():
    # Check if user has valid credentials
    return GoogleApi.is_authenicated()

@eel.expose
def sign_in():
    # If creds are invalid, send user to sign in flow
    GoogleApi.ensure_valid_token()


@eel.expose
def get_favorites():
    # Get user's favorited media items
    return GoogleApi.get_favorites()

@eel.expose
def get_albums():
    # Get user's photo albums
    return GoogleApi.get_albums()

@eel.expose
def get_album_media_items(album_id):
    # Get media items from a photo album
    return GoogleApi.get_album_media_items(album_id)

@eel.expose
def get_media_item(media_item_id):
    # Retreive data for a media item
    return GoogleApi.get_media_item(media_item_id)


@eel.expose
def set_selected_albums(selected_albums):
    # User has changed the album selection
    Options.set_selected_albums(selected_albums)

@eel.expose
def get_current_wallpaper():
    # Get current wall from user options
    return Options.get_current_wallpaper()

@eel.expose
def get_user_options():
    return Options.get_user_options()

@eel.expose
def set_wallpaper(media_item):
    # Set a media item as the user's wallpaper

    # TODO: This should only work on windows. Ensure multiplatform compatibility

    image_url = get_large_url(media_item)

    # Download full res image
    urllib.request.urlretrieve(image_url, 'storage/wall.jpg')
    path = os.path.abspath('storage/wall.jpg')
    SPI = 20
    SPIF = 2
    ctypes.windll.user32.SystemParametersInfoW(SPI, 0, path, SPIF)
    
    Options.set_current_wallpaper(media_item)

    return True