import eel

from src.google_api import GoogleApi
from src.options import Options
from src.wallpaper import Wallpaper
from src.scheduler import Scheduler

# These methods are exposed to the Vue app
# They are a bridge between JS and Python code

@eel.expose
def get_system_info():
    return Options.get_system_info()

@eel.expose
def is_authenticated():
    # Check if user has valid credentials
    return GoogleApi.is_authenticated()

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
def get_user_options():
    return Options.get_user_options()

@eel.expose
def set_schedule(interval, unit):
    Scheduler.start(interval, unit)


@eel.expose
def set_wallpaper(media_item):    
    Wallpaper.set_current_wallpaper(media_item)
    return True

@eel.expose
def set_wallpaper_by_direction(direction):
    return Wallpaper.set_wallpaper_by_direction(direction)

@eel.expose
def set_wallpaper_random():
    return Wallpaper.set_wallpaper_random()

@eel.expose
def get_current_wallpaper():
    # Get current wall from user options
    return Wallpaper.get_current_wallpaper()