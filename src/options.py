import json
from src.google_api import GoogleApi

# Read and update user-set options for the app
class Options():

    OPTIONS_PATH = 'storage/options.json'

    @classmethod
    def get_current_wallpaper(cls):
        # Get the set wallpaper data
        
        # NOTE: Access to `baseUrl` will expire after session ends,
        # Use GoogleApi.get_media_item to refresh it

        with open(cls.OPTIONS_PATH, 'r') as f:
            options = json.load(f)
            f.close()

        return options.get('currentWallpaper')

    @classmethod
    def set_current_wallpaper(cls, media_item):
        # Update the current wallpaper data
        with open(cls.OPTIONS_PATH, 'r') as f:
            options = json.load(f)
            f.close()

        options['currentWallpaper'] = media_item

        with open(cls.OPTIONS_PATH, 'w') as f:
            json.dump(options, f)
            f.close()
    
    @classmethod
    def get_user_options(cls):
        # Return the entire options config
        
        with open(cls.OPTIONS_PATH, 'r') as f:
            options = json.load(f)
            f.close()
        return options
    
    @classmethod
    def set_selected_albums(cls, selected_items):
        with open(cls.OPTIONS_PATH, 'r') as f:
            options = json.load(f)
            f.close()

        original_selection = options.get('selected_items', [])
        new_selection = []

        # Iterate through new selection
        for album_id in selected_items:
            # Search in original selection for item containing album_id
            search = match = next((d for d in original_selection if d['id'] == album_id), None)
            
            if search == None:
                # User has selected a new album
                if album_id == 'FAVORITES':
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

        with open(cls.OPTIONS_PATH, 'w') as f:
            json.dump(options, f)
            f.close()
