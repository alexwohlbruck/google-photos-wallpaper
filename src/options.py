import json 
import os

from src.google_api import GoogleApi
from src.helpers import resource_path, find_in_list_by_val

OPTIONS_PATH = resource_path('storage/options.json')
FAVORITES = 'FAVORITES'
DEFAULT_OPTIONS = {
    'schedule': {
        'interval': 5,
        'unit': 'minutes',
    }
}

# Read and update user-set options for the app
class Options():

    @classmethod
    def get_user_options(this):
        # Get the set wallpaper data
        
        # NOTE: Access to `baseUrl` will expire after session ends,
        # Use GoogleApi.get_media_item to refresh it
        
        if os.path.exists(OPTIONS_PATH):
            with open(OPTIONS_PATH, 'r') as options_file:
                options = json.load(options_file)
        else:
            options = DEFAULT_OPTIONS

        return options
    
    @classmethod
    def save_options(this, options):
        # If /storage doesn't exist, create it
        if not os.path.exists(resource_path('storage')):
            os.makedirs(resource_path('storage'))
        
        with open(OPTIONS_PATH, 'w') as f:
            json.dump(options, f)
            f.close()
    
    # Scheduler options
    @classmethod
    def set_schedule(this, interval, unit):
        options = this.get_user_options()
        options['schedule']['interval'] = interval
        options['schedule']['unit'] = unit
        this.save_options(options)

    @classmethod
    def set_selected_albums(this, selected_items):
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
