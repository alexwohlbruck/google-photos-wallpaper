import json
import ctypes
import urllib
import os
import random
import eel

from src.google_api import GoogleApi
from src.resource_path import resource_path

OPTIONS_PATH = resource_path('storage/options.json')
FAVORITES = 'FAVORITES'

# Helper methods
def find_in_list_by_val(list_, key, val):
    return next((d for d in list_ if d[key] == val), None)
    
def find_index_in_list_by_val(list_, key, val):
    return next((i for i, d in enumerate(list_) if d[key] == val), None)

def get_large_url(media_item):
    # Helper function to get the max size image url
    try:
        width = media_item['mediaMetadata']['width']
        height = media_item['mediaMetadata']['height']
    except (AttributeError, KeyError):
        width = height = '16383'

    return media_item['baseUrl'] + '=w' + width + '-h' + height

# Read and update user-set options for the app
class Options():

    # Generic options methods

    @classmethod
    def get_user_options(cls):
        # Get the set wallpaper data
        
        # NOTE: Access to `baseUrl` will expire after session ends,
        # Use GoogleApi.get_media_item to refresh it

        with open(OPTIONS_PATH, 'r') as f:
            options = json.load(f)
            f.close()

        return options
    
    @classmethod
    def save_options(cls, options):
        with open(OPTIONS_PATH, 'w') as f:
            json.dump(options, f)
            f.close()
    
    # Scheduler options
    @classmethod
    def set_schedule(cls, interval, unit):
        options = cls.get_user_options()
        options['schedule']['interval'] = interval
        options['schedule']['unit'] = unit
        cls.save_options(options)


    # Wallpaper methods
    # TODO: Move these into Wallpaper class

    @classmethod
    def get_current_wallpaper(cls):
        options = cls.get_user_options()

        selected_albums = options.get('selectedAlbums', None)
        new_album = random.choice(selected_albums)

        new_album_id = new_album.get('id')
        album_media_items = new_album.get('mediaItems', None)

        # TODO: Store album title in options.json instead of retrieving it each time
        if (new_album_id == FAVORITES):
            new_album['title'] = 'Favorites'
        else:
            new_album = GoogleApi.get_album(new_album_id)

        new_wall = random.choice(album_media_items)

        # Refresh base url
        new_wall = GoogleApi.get_media_item(new_wall.get('id'))
        new_album.pop('mediaItems', None)
        new_wall['source'] = new_album

        cls.set_current_wallpaper(new_wall)
        return new_wall

    @classmethod
    def set_current_wallpaper(cls, media_item):
        
        print('Setting wallpaper: ' + media_item['filename']);

        # Update the current wallpaper data
        options = cls.get_user_options()

        print(eel._js_functions)
        if (hasattr(eel, 'wallpaper_changed')):
            print('running func')
            eel.wallpaper_changed(media_item)

        # TODO: This should only work on windows. Ensure multiplatform compatibility
        # Download full res image and change wallpaper
        image_url = get_large_url(media_item)
        path = resource_path('storage/wall.jpg')
        urllib.request.urlretrieve(image_url, path)
        path = path#os.path.abspath('storage/wall.jpg')
        SPI = 20
        SPIF = 2
        ctypes.windll.user32.SystemParametersInfoW(SPI, 0, path, SPIF)


        options['currentWallpaper'] = media_item

        cls.save_options(options)

    @classmethod
    def set_wallpaper_next(cls):
        return cls.set_wallpaper_by_direction(direction = 'next')
    
    @classmethod
    def set_wallpaper_prev(cls):
        return cls.set_wallpaper_by_direction(direction = 'prev')

    @classmethod
    def set_wallpaper_by_direction(cls, direction = 'next'):

        options = cls.get_user_options()
        current_wall = options.get('currentWallpaper', None)

        if (current_wall == None):
            # No current wallpaper set
            return None

        current_album = current_wall.get('source')
        current_album_id = current_album.get('id')
        current_wall_id = current_wall.get('id')

        # Get the album that the current wallpaper is in
        current_album_items = find_in_list_by_val(
            options.get('selectedAlbums'),
            'id',
            current_album_id
        ).get('mediaItems')

        # Get the index of the current wallpaper
        current_wall_index = find_index_in_list_by_val(
            current_album_items,
            'id',
            current_wall_id
        )

        if (direction == 'next'):
            index = current_wall_index + 1
        elif (direction == 'prev'):
            index = current_wall_index - 1

        if (index == len(current_album_items) and direction == 'next') or (index < 0 and direction == 'prev'):
            # Out of bounds
            # TODO: Change to next album when going out of bounds
            return
        else:
            new_wall_id = current_album_items[index].get('id')
            new_wall = GoogleApi.get_media_item(new_wall_id)
            new_wall['source'] = current_album

            cls.set_current_wallpaper(new_wall)
            return new_wall
    
    @classmethod
    def set_wallpaper_random(cls):

        options = cls.get_user_options()

        selected_albums = options.get('selectedAlbums', None)
        new_album = random.choice(selected_albums)

        new_album_id = new_album.get('id')
        album_media_items = new_album.get('mediaItems', None)

        # TODO: Store album title in options.json instead of retrieving it each time
        if (new_album_id == 'FAVORITES'):
            new_album['title'] = 'Favorites'
        else:
            new_album = GoogleApi.get_album(new_album_id)

        new_wall = random.choice(album_media_items)

        # Refresh base url
        new_wall = GoogleApi.get_media_item(new_wall.get('id'))
        new_album.pop('mediaItems', None)
        new_wall['source'] = new_album

        cls.set_current_wallpaper(new_wall)
        return new_wall
    

    # Selected albums methods

    @classmethod
    def set_selected_albums(cls, selected_items):
        with open(OPTIONS_PATH, 'r') as f:
            options = json.load(f)
            f.close()

        original_selection = options.get('selectedAlbums', [])
        new_selection = []

        # Iterate through new selection
        for album_id in selected_items:
            # Search in original selection for item containing album_id
            search = find_in_list_by_val(original_selection, 'id', album_id)

            if search == None:
                # User has selected a new album
                if album_id == FAVORITES:
                    media_items = GoogleApi.get_all_favorites()
                else:
                    media_items = GoogleApi.get_all_album_media_items(album_id)
                
                # Remove these baseUrl and productUrl - not needed for storage
                for item in media_items:
                    item.pop('baseUrl')
                    item.pop('productUrl')
                
                new_selection.append({
                    'id': album_id,
                    'mediaItems': media_items
                })
            else:
                # Album already selected, use old data
                new_selection.append(search)
        
        options['selectedAlbums'] = new_selection

        with open(OPTIONS_PATH, 'w') as f:
            json.dump(options, f)
            f.close()
